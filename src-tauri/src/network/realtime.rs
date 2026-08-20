//! WebSocket realtime connection manager
//!
//! Speaks the Supabase Realtime protocol (Phoenix channels over WebSocket):
//!   - joins `realtime:postgres_changes` for the `messages` table
//!   - joins `realtime:presence` for presence fanout
//!   - joins `realtime:broadcast` for typing events
//!   - sends Phoenix heartbeats to keep the socket alive
//!
//! Incoming message rows are forwarded to the UI as Tauri events; the
//! frontend pulls the ciphertext rows into the local store via sync_messages.
//! Only ciphertext/metadata crosses this connection — never plaintext.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, info, warn};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const RECONNECT_BASE: Duration = Duration::from_secs(2);
const RECONNECT_MAX: Duration = Duration::from_secs(60);
/// Shared topic for client→client broadcasts (typing, ephemeral signals).
const BROADCAST_TOPIC: &str = "realtime:broadcast:veilanon";

#[derive(Clone)]
pub struct RealtimeManager {
    ws_url: String,
    apikey: String,
    connected: Arc<AtomicBool>,
    /// Outbound broadcast pipe — set while a session is live.
    outbound: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<WsMessage>>>>,
    /// User JWT for authenticated realtime connections.
    /// Without this, postgres_changes won't deliver row notifications (RLS requires auth.uid()).
    access_token: Arc<Mutex<Option<String>>>,
}

impl RealtimeManager {
    pub fn new() -> Self {
        let base = config::var("VEILANON_SUPABASE_URL").unwrap_or_default();
        let apikey = config::var("VEILANON_SUPABASE_ANON_KEY").unwrap_or_default();
        // Realtime endpoint requires the /realtime/v1/websocket path, else the
        // bare project origin answers 400 (Cloudflare).
        let with_path = if base.contains("/realtime/v1/websocket") {
            base
        } else {
            format!("{}/realtime/v1/websocket", base.trim_end_matches('/'))
        };
        let ws_url = with_path
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        Self {
            ws_url,
            apikey,
            connected: Arc::new(AtomicBool::new(false)),
            outbound: Arc::new(Mutex::new(None)),
            access_token: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.access_token.lock() {
            *guard = token;
        }
    }

    /// Force the realtime WebSocket to drop and reconnect with the current token.
    ///
    /// Dropping the outbound sender causes `rx.recv()` in `run_once` to yield
    /// `None`, which returns an error and causes the `run` loop to reconnect —
    /// this time using the freshly-set JWT.
    pub fn force_reconnect(&self) {
        if let Ok(mut sender) = self.outbound.lock() {
            if let Some(s) = sender.take() {
                drop(s); // triggers disconnect → reconnect in run() loop
                info!("Realtime force-reconnect: dropped outbound pipe");
            }
        }
    }

    /// Publish a client-to-client broadcast (typing, ephemeral signals).
    /// No-op while disconnected — signals are ephemeral by design.
    pub fn broadcast(&self, payload: Value) {
        let msg = WsMessage::Text(
            json!({
                "topic": BROADCAST_TOPIC,
                "event": "broadcast",
                "payload": payload,
                "ref": "bcast"
            })
            .to_string(),
        );
        if let Ok(guard) = self.outbound.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(msg);
            }
        }
    }

    #[allow(dead_code)] // surfaced by the connection-status UI in a later iteration
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Whether a realtime endpoint is configured at all.
    pub fn is_configured(&self) -> bool {
        !self.ws_url.is_empty() && !self.apikey.is_empty()
    }

    /// Connect, join channels, pump messages, and reconnect with exponential
    /// backoff until the task is dropped. Runs forever.
    pub async fn run(&self, app: AppHandle) {
        let mut backoff = RECONNECT_BASE;
        loop {
            if self.is_configured() {
                let started = std::time::Instant::now();
                match self.run_once(&app).await {
                    Ok(_) => {}
                    Err(e) => warn!("Realtime session ended: {}", e),
                }
                // A session that lived a while proves the endpoint works —
                // reset backoff so healthy reconnects stay snappy.
                if started.elapsed() > Duration::from_secs(60) {
                    backoff = RECONNECT_BASE;
                }
            } else {
                debug!("Realtime not configured — skipping connection");
                sleep(Duration::from_secs(30)).await;
                continue;
            }
            self.connected.store(false, Ordering::Relaxed);
            *self.outbound.lock().unwrap() = None;
            let _ = app.emit("veilanon:realtime-status", serde_json::json!({ "connected": false }));
            sleep(backoff).await;
            backoff = (backoff * 2).min(RECONNECT_MAX);
        }
    }

    async fn run_once(&self, app: &AppHandle) -> anyhow::Result<()> {
        let token = self.access_token.lock().unwrap().clone();
        let url = format!(
            "{}?apikey={}&vsn=1.0.0",
            self.ws_url, self.apikey
        );
        let mut request = url.into_client_request()?;
        request.headers_mut().insert(
            "User-Agent",
            tokio_tungstenite::tungstenite::http::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
            ),
        );
        if let Some(ref jwt) = token {
            if let Ok(val) = tokio_tungstenite::tungstenite::http::HeaderValue::from_str(
                &format!("Bearer {}", jwt),
            ) {
                request.headers_mut().insert("Authorization", val);
            }
        }
        debug!("Realtime request: {} {:?}", request.method(), request.headers());
        let (mut socket, _resp) = match connect_ws(request).await {
            Ok(pair) => pair,
            Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
                let body = resp.body().clone().unwrap_or_default();
                let status = resp.status();
                let body_str = String::from_utf8_lossy(&body);
                warn!(
                    "Realtime handshake rejected ({}): {}",
                    status,
                    body_str.chars().take(300).collect::<String>()
                );
                return Err(anyhow::anyhow!("handshake rejected with {}", status));
            }
            Err(e) => return Err(e.into()),
        };
        info!("Realtime connected");

        // Phoenix handshake + channel joins.
        let mut ref_counter: u32 = 0;
        let mut next_ref = || {
            ref_counter += 1;
            ref_counter.to_string()
        };

        socket
            .send(WsMessage::Text(
                json!({"topic": "realtime:dev", "event": "phx_join", "payload": {}, "ref": next_ref()})
                    .to_string(),
            ))
            .await?;

        let join_payload = json!({
            "config": {
                "postgres_changes": [
                    {"event": "*", "schema": "public", "table": "messages"},
                    {"event": "*", "schema": "public", "table": "friendships"},
                    {"event": "*", "schema": "public", "table": "channels"},
                    {"event": "*", "schema": "public", "table": "channel_members"},
                    {"event": "*", "schema": "public", "table": "spaces"},
                    {"event": "*", "schema": "public", "table": "memberships"},
                    {"event": "*", "schema": "public", "table": "roles"},
                    {"event": "*", "schema": "public", "table": "role_members"},
                    {"event": "*", "schema": "public", "table": "presence"},
                    {"event": "*", "schema": "public", "table": "users"}
                ]
            }
        });
        socket
            .send(WsMessage::Text(
                json!({
                    "topic": "realtime:postgres_changes",
                    "event": "phx_join",
                    "payload": join_payload,
                    "ref": next_ref()
                })
                .to_string(),
            ))
            .await?;

        socket
            .send(WsMessage::Text(
                json!({
                    "topic": "realtime:presence",
                    "event": "phx_join",
                    "payload": {"config": {"presence": {"key": "veilanon-device"}}},
                    "ref": next_ref()
                })
                .to_string(),
            ))
            .await?;

        socket
            .send(WsMessage::Text(
                json!({
                    "topic": BROADCAST_TOPIC,
                    "event": "phx_join",
                    "payload": {"config": {"broadcast": {"self": false}}},
                    "ref": next_ref()
                })
                .to_string(),
            ))
            .await?;

        self.connected.store(true, Ordering::Relaxed);
        let _ = app.emit("veilanon:realtime-status", serde_json::json!({ "connected": true }));

        // Split into sink + stream so outbound broadcasts and inbound frames
        // can be polled concurrently without fighting over the socket.
        let (mut sink, mut stream) = socket.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WsMessage>();
        *self.outbound.lock().unwrap() = Some(tx);

        let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    sink.send(WsMessage::Text(
                        json!({"topic": "phoenix", "event": "heartbeat", "payload": {}, "ref": next_ref()})
                            .to_string()
                    )).await?;
                }
                outbound = rx.recv() => {
                    match outbound {
                        Some(msg) => sink.send(msg).await?,
                        None => return Err(anyhow::anyhow!("broadcast pipe closed")),
                    }
                }
                msg = stream.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            self.handle_frame(app, &text).await;
                        }
                        Some(Ok(WsMessage::Ping(payload))) => {
                            sink.send(WsMessage::Pong(payload)).await?;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => return Err(e.into()),
                        None => return Err(anyhow::anyhow!("connection closed by server")),
                    }
                }
            }
        }
    }

    async fn handle_frame(&self, app: &AppHandle, text: &str) {
        let Ok(parsed) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let topic = parsed.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("");

        if topic.starts_with("realtime:postgres_changes") {
            let schema = parsed.pointer("/payload/data/schema").and_then(|v| v.as_str()).unwrap_or("");
            let table = parsed.pointer("/payload/data/table").and_then(|v| v.as_str()).unwrap_or("");
            let record = parsed.pointer("/payload/data/record").cloned();
            if schema != "public" {
                return;
            }
            match table {
                "messages" => {
                    if let Some(rec) = record {
                        let _ = app.emit("veilanon:realtime-message", rec);
                        debug!("Realtime message row received");
                    }
                }
                "presence" => {
                    let rec = record.clone().unwrap_or(Value::Null);
                    let _ = app.emit("presence:changed", rec.clone());
                    let _ = app.emit("members:changed", rec);
                    info!("Realtime presence change received");
                }
                "users" => {
                    let rec = record.clone().unwrap_or(Value::Null);
                    let _ = app.emit("user:updated", rec.clone());
                    let _ = app.emit("members:changed", rec);
                    info!("Realtime user profile change received");
                }
                "friendships" => {
                    let _ = app.emit("friends:changed", record.unwrap_or(Value::Null));
                    info!("Realtime friendship change received");
                }
                "channels" | "channel_members" => {
                    let _ = app.emit("channels:changed", record.unwrap_or(Value::Null));
                    info!("Realtime channel change received");
                }
                "spaces" | "memberships" => {
                    let _ = app.emit("spaces:changed", record.clone().unwrap_or(Value::Null));
                    let _ = app.emit("members:changed", record.unwrap_or(Value::Null));
                    info!("Realtime space/membership change received");
                }
                "roles" | "role_members" => {
                    let _ = app.emit("roles:changed", record.clone().unwrap_or(Value::Null));
                    let _ = app.emit("members:changed", record.unwrap_or(Value::Null));
                    info!("Realtime role change received");
                }
                _ => {
                    if let Some(rec) = record {
                        let _ = app.emit("veilanon:realtime-message", rec);
                    }
                }
            }
            return;
        }

        if topic.starts_with("realtime:presence") && event == "presence_state" {
            if let Some(state) = parsed.pointer("/payload/presence") {
                let _ = app.emit("veilanon:presence", state);
            }
            return;
        }

        if topic.starts_with("realtime:presence") && event == "presence_diff" {
            if let Some(state) = parsed.get("payload") {
                let _ = app.emit("veilanon:presence", state);
            }
            return;
        }

        if topic.starts_with("realtime:broadcast") && event == "broadcast" {
            let raw_payload = parsed.get("payload").cloned().unwrap_or(Value::Null);
            let unwrapped = if let Some(inner) = raw_payload.get("payload") {
                inner.clone()
            } else {
                raw_payload
            };
            let _ = app.emit("veilanon:broadcast", unwrapped);
        }
    }
}

/// Connect with native-tls (schannel/OpenSSL): Cloudflare in front of
/// Supabase flags rustls' TLS fingerprint and answers the handshake with 400.
async fn connect_ws(
    request: tokio_tungstenite::tungstenite::http::Request<()>,
) -> Result<
    (
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        tokio_tungstenite::tungstenite::http::Response<Option<Vec<u8>>>,
    ),
    tokio_tungstenite::tungstenite::Error,
> {
    let tls = native_tls::TlsConnector::new().map_err(|e| {
        tokio_tungstenite::tungstenite::Error::Io(std::io::Error::other(
            e,
        ))
    })?;
    let connector = tokio_tungstenite::Connector::NativeTls(tls);
    connect_async_tls_with_config(request, None, false, Some(connector)).await
}


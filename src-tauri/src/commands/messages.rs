//! Message IPC commands
//! Encryption happens here in Rust before network transmission.
//! Decryption happens here after receiving — UI only sees plaintext via IPC.

use tauri::{Emitter, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, debug};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::state::AppState;
use crate::config;
use crate::error::{VeilError, VeilResult};
use crate::models::message::{Message, MessageType, MessageStatus, Reaction, AttachmentRef, QueuedMessage};
use crate::models::channel::ChannelType;
use crate::crypto::{decrypt_aes_gcm, derive_channel_message_key, derive_message_key, encrypt_aes_gcm};
use crate::crypto::ratchet::{MessageHeader, RatchetState};

// ── DM ratchet helpers ──────────────────────────────────────────────────────

/// Deterministic root key for a 1:1 DM: HKDF over the identity-level X25519
/// shared secret, salted with the canonically-ordered user ids so both sides
/// derive identical material without any pre-key exchange round trip.
fn derive_dm_root_key(dh_shared: &[u8; 32], a: &uuid::Uuid, b: &uuid::Uuid) -> VeilResult<[u8; 32]> {
    use ring::hkdf;
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    let mut salt = Vec::with_capacity(16 + 16 + 16);
    salt.extend_from_slice(b"veilanon-dm-v1");
    salt.extend_from_slice(lo.as_bytes());
    salt.extend_from_slice(hi.as_bytes());
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, &salt).extract(dh_shared);
    let mut okm = [0u8; 32];
    prk.expand(&[b"veilanon-root"], hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut okm)
        .map_err(|_| VeilError::KeyDerivationError)?;
    Ok(okm)
}

fn channel_is_dm(db: &crate::db::Database, channel_id: &uuid::Uuid) -> VeilResult<bool> {
    let ct = db.channel_type_of(channel_id)?;
    Ok(matches!(ct.as_deref(), Some("dm" | "direct_message" | "directmessage")))
}

fn channel_is_e2ee(db: &crate::db::Database, channel_id: &uuid::Uuid) -> VeilResult<bool> {
    let rows = db.query_row(
        "SELECT is_e2ee FROM channels WHERE id = ?1",
        rusqlite::params![channel_id.to_string()],
        |r| r.get::<_, i64>(0),
    );
    Ok(rows.unwrap_or(0) != 0)
}

/// Load the persisted ratchet session for a DM channel, or establish a fresh
/// one from the identity-level DH agreement. Returns (state, peer_id).
async fn load_or_create_ratchet(
    state: &AppState,
    channel_id: &uuid::Uuid,
) -> VeilResult<(RatchetState, uuid::Uuid)> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;
    let identity = state
        .identity
        .read()
        .await
        .clone()
        .ok_or(VeilError::Unauthenticated)?;

    let db = state.db.read().await;
    if let Some((peer, json)) = db.load_dm_session(channel_id, Some(&db_key))? {
        let ratchet: RatchetState =
            serde_json::from_str(&json).map_err(|_| VeilError::SerializationError)?;
        return Ok((ratchet, peer));
    }

    let members = db.list_channel_members(channel_id)?;
    let mut peer_opt = members.iter().find(|m| **m != identity.id).copied();

    if peer_opt.is_none() && config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let cm_filter = format!("channel_id=eq.{}", channel_id);
        if let Ok(cm_rows) = network.api.select::<serde_json::Value>("channel_members", &cm_filter, None, Some(10)).await {
            for r in cm_rows {
                if let Some(uid_str) = r.get("user_id").and_then(|v| v.as_str()) {
                    if let Ok(uid) = Uuid::parse_str(uid_str) {
                        let _ = db.add_channel_member(channel_id, &uid);
                        if uid != identity.id {
                            peer_opt = Some(uid);
                        }
                    }
                }
            }
        }
    }
    let peer = peer_opt.ok_or_else(|| VeilError::InvalidInput("DM channel has no peer member".into()))?;

    let mut peer_dh = db.get_profile_dh_public(&peer)?;
    if peer_dh.is_none() || peer_dh.as_deref() == Some("") {
        if config::configured("VEILANON_SUPABASE_URL") {
            let network = state.network.read().await;
            // 1. Fetch user profile from users table
            let u_filter = format!("id=eq.{}&select=id,username,display_name,avatar_hash", peer);
            if let Ok(u_rows) = network.api.select::<serde_json::Value>("users", &u_filter, None, Some(1)).await {
                if let Some(u) = u_rows.first() {
                    let uname = u.get("username").and_then(|v| v.as_str()).unwrap_or("");
                    let disp = u.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
                    let av = u.get("avatar_hash").and_then(|v| v.as_str());
                    let _ = db.upsert_profile(&peer, uname, disp, av, None, None, None, None, None);
                }
            }

            // 2. Fetch device public key from devices table (registered public key for DH)
            let dev_filter = format!("user_id=eq.{}&select=public_key,signing_public_key", peer);
            if let Ok(dev_rows) = network.api.select::<serde_json::Value>("devices", &dev_filter, Some("created_at.desc"), Some(1)).await {
                if let Some(dev) = dev_rows.first() {
                    let pk = dev.get("public_key").and_then(|v| v.as_str()).map(str::to_string);
                    let spk = dev.get("signing_public_key").and_then(|v| v.as_str());
                    if let Some(ref pk_str) = pk {
                        let profile = db.get_profile_by_id(&peer).ok().flatten();
                        let uname = profile.as_ref().map(|p| p.0.as_str()).unwrap_or("");
                        let disp = profile.as_ref().map(|p| p.1.as_str()).unwrap_or("");
                        let av = profile.as_ref().and_then(|p| p.2.as_deref());
                        let _ = db.upsert_profile(&peer, uname, disp, av, Some(pk_str), spk, None, None, None);
                    }
                    peer_dh = pk;
                }
            }
        }
    }
    let peer_dh = match peer_dh {
        Some(key) => key,
        None => {
            // Fallback: peer'in DH key'i hala yoksa, peer'in identity key'ini kullan
            // Bu durumda peer henüz cihaz anahtarı kaydetmemiş olabilir
            let db = state.db.read().await;
            let profile = db.get_profile_by_id(&peer).ok().flatten();
            drop(db);
            
            if let Some((_uname, _disp, _av, Some(dh_key), _spk)) = profile {
                dh_key
            } else {
                info!("DM peer {} has no public key yet; message will be queued", peer);
                return Err(VeilError::PeerKeyMissing);
            }
        }
    };

    let device = state
        .device_identity
        .read()
        .await
        .clone()
        .ok_or(VeilError::Unauthenticated)?;
    let shared = device.dh_agree(&peer_dh)?;
    let root = derive_dm_root_key(&shared, &identity.id, &peer)?;
    let ratchet = RatchetState::new(
        device.dh_private_bytes(),
        &identity.identity_key_public,
        &peer_dh,
        root,
    )?;
    Ok((ratchet, peer))
}

/// Encrypt a message with the DM ratchet; returns (ciphertext_b64, iv_b64, header_json).
async fn ratchet_encrypt(
    state: &AppState,
    channel_id: &uuid::Uuid,
    plaintext: &[u8],
) -> VeilResult<(String, String, String)> {
    let (mut ratchet, peer) = load_or_create_ratchet(state, channel_id).await?;
    let (header, ciphertext, nonce) = ratchet.encrypt(plaintext)?;
    let header_json =
        serde_json::to_string(&header).map_err(|_| VeilError::SerializationError)?;
    let db = state.db.read().await;
    let ratchet_json = serde_json::to_string(&ratchet).map_err(|_| VeilError::SerializationError)?;
    db.save_dm_session(
        channel_id,
        &peer,
        &ratchet_json,
        state.get_db_key().await.as_ref(),
    )?;
    Ok((B64.encode(&ciphertext), B64.encode(&nonce), header_json))
}

/// Decrypt one message through a loaded ratchet session (may advance chains).
/// The derived message key is cached so later reads don't advance the chain.
fn ratchet_decrypt(
    db: &crate::db::Database,
    db_key: &[u8; 32],
    ratchet: &mut RatchetState,
    msg: &Message,
) -> Option<String> {
    if let Ok(Some(key)) = db.get_message_key(&msg.id, Some(db_key)) {
        let ct = B64.decode(&msg.ciphertext).ok()?;
        let nonce = B64.decode(&msg.iv).ok()?;
        let plaintext = decrypt_aes_gcm(&key, &ct, &nonce).ok()?;
        return String::from_utf8(plaintext).ok();
    }

    let meta = msg.crypto_meta.as_deref()?;
    let header: MessageHeader = serde_json::from_str(meta).ok()?;
    let ct = B64.decode(&msg.ciphertext).ok()?;
    let nonce = B64.decode(&msg.iv).ok()?;
    let (plaintext, msg_key) = ratchet.decrypt(&header, &ct, &nonce).ok()?;
    let _ = db.save_message_key(&msg.id, &msg_key, Some(db_key));
    String::from_utf8(plaintext).ok()
}

fn response_from_message(msg: &Message, content: String, sender: SenderInfo) -> MessageResponse {
    MessageResponse {
        id: msg.id.to_string(),
        channel_id: msg.channel_id.to_string(),
        sender_id: msg.sender_id.to_string(),
        sender_name: sender.name,
        sender_avatar_hash: sender.avatar_hash,
        sender_role_color: sender.role_color,
        content: Some(content),
        message_type: format!("{:?}", msg.message_type).to_lowercase(),
        status: format!("{:?}", msg.status).to_lowercase(),
        reply_to_id: msg.reply_to_id.map(|id| id.to_string()),
        pinned: msg.pinned,
        reactions: msg.reactions.clone(),
        attachments: msg.attachments.clone(),
        edited_at: msg.edited_at.map(|dt| dt.timestamp()),
        created_at: msg.created_at.timestamp(),
        deleted_at: msg.deleted_at.map(|dt| dt.timestamp()),
        disappears_at: msg.disappears_at.map(|dt| dt.timestamp()),
    }
}

/// Sender display metadata attached to every message response — lets the UI
/// render real names/avatars and role colors without extra IPC round-trips.
#[derive(Default, Clone)]
pub struct SenderInfo {
    pub name: Option<String>,
    pub avatar_hash: Option<String>,
    pub role_color: Option<String>,
}

/// Resolve a sender's display name, avatar and (in space channels) their
/// highest-coloured role for the channel's space. Best-effort: any missing
/// piece falls back to None and the UI renders the sender id.
fn resolve_sender_info(
    db: &crate::db::Database,
    channel_id: &Uuid,
    sender_id: &Uuid,
    identity: Option<&crate::models::user::Identity>,
) -> SenderInfo {
    let mut info = SenderInfo::default();
    if let Some(identity) = identity {
        if identity.id == *sender_id {
            info.name = Some(identity.display_name.clone());
            info.avatar_hash = identity.avatar_hash.clone();
        }
    }
    if info.name.is_none() {
        if let Ok(Some((_, display_name, avatar_hash, _, _))) = db.get_profile_by_id(sender_id) {
            info.name = Some(display_name);
            info.avatar_hash = avatar_hash;
        }
    }
    if let Ok(Some(channel)) = db.get_channel(channel_id, None) {
        if let Some(space_id) = channel.space_id {
            if let Ok(members) = db.list_space_members(&space_id) {
                if let Some(member) = members.iter().find(|m| m.user_id == *sender_id) {
                    if info.name.is_none() && !member.display_name.is_empty() {
                        info.name = Some(member.display_name.clone());
                    } else if info.name.is_none() && !member.username.is_empty() {
                        info.name = Some(member.username.clone());
                    }
                    if info.avatar_hash.is_none() && member.avatar_hash.is_some() {
                        info.avatar_hash = member.avatar_hash.clone();
                    }
                    if let Ok(roles) = db.list_roles(&space_id) {
                        for role_id in &member.role_ids {
                            if let Some(role) = roles.iter().find(|r| r.id == *role_id) {
                                if let Some(color) = &role.color {
                                    if !color.is_empty() {
                                        info.role_color = Some(color.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    info
}

/// Decrypt a message batch, advancing/persisting the DM ratchet when the
/// channel is a 1:1 DM. Messages that cannot be decrypted are skipped.
async fn decrypt_batch(
    state: &AppState,
    channel_id: &uuid::Uuid,
    messages: Vec<Message>,
) -> VeilResult<Vec<MessageResponse>> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;
    let db = state.db.read().await;
    let mut is_dm = channel_is_dm(&db, channel_id)?;
    if !is_dm && messages.iter().any(|m| m.crypto_meta.as_deref().map(|s| s.contains("dh_public")).unwrap_or(false)) {
        is_dm = true;
    }
    let is_e2ee = !is_dm && channel_is_e2ee(&db, channel_id)?;
    drop(db);

    let mut ratchet: Option<(RatchetState, uuid::Uuid)> = None;
    let mut mls_session: Option<crate::crypto::group::MlsGroupSession> = None;
    if is_dm {
        ratchet = Some(load_or_create_ratchet(state, channel_id).await?);
    } else if is_e2ee {
        mls_session = Some(super::mls::load_session(state, channel_id).await?);
    }

    let mut responses = Vec::with_capacity(messages.len());
    let db_guard = state.db.read().await;
    let mut sender_cache: std::collections::HashMap<Uuid, SenderInfo> = std::collections::HashMap::new();
    let identity = state.get_or_restore_identity().await.clone();
    for msg in messages {
        let content_str = if is_dm {
            ratchet
                .as_mut()
                .and_then(|(r, _)| ratchet_decrypt(&db_guard, &db_key, r, &msg))
        } else if is_e2ee {
            mls_session.as_mut().and_then(|s| {
                let ct = B64.decode(&msg.ciphertext).ok()?;
                s.decrypt_message(&ct).ok().and_then(|p| String::from_utf8(p).ok())
            })
        } else {
            decrypt_message_content(&db_key, &msg)
        };
        let content = if let Some(c) = content_str {
            c
        } else if !msg.attachments.is_empty() {
            String::new()
        } else {
            continue;
        };
        let sender = sender_cache
            .entry(msg.sender_id)
            .or_insert_with(|| {
                resolve_sender_info(&db_guard, channel_id, &msg.sender_id, identity.as_ref())
            })
            .clone();
        responses.push(response_from_message(&msg, content, sender));
    }
    drop(db_guard);

    if let Some((ratchet, peer)) = ratchet {
        let db = state.db.read().await;
        let ratchet_json =
            serde_json::to_string(&ratchet).map_err(|_| VeilError::SerializationError)?;
        db.save_dm_session(channel_id, &peer, &ratchet_json, Some(&db_key))?;
    }
    if let Some(session) = mls_session {
        super::mls::save_session(state, channel_id, &session).await?;
    }
    Ok(responses)
}

/// Purge locally expired disappearing messages; best-effort.
pub(crate) async fn purge_expired(state: &AppState) {
    let removed = state.db.read().await.purge_expired_messages();
    if let Ok(n) = removed {
        if n > 0 {
            info!("Purged {} expired message(s)", n);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // message_type part of IPC contract; parsed by client
pub struct SendMessageInput {
    pub channel_id: String,
    pub content: String, // Plaintext — encrypted here in Rust
    pub message_type: Option<String>,
    pub reply_to_id: Option<String>,
    pub disappear_seconds: Option<u64>,
    /// Optional encrypted-file attachments (uploaded via upload_file first).
    #[serde(default)]
    pub attachments: Vec<AttachmentRef>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponse {
    pub id: String,
    pub channel_id: String,
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub sender_avatar_hash: Option<String>,
    pub sender_role_color: Option<String>,
    pub content: Option<String>,  // Decrypted content — only in IPC response
    pub message_type: String,
    pub status: String,
    pub reply_to_id: Option<String>,
    pub pinned: bool,
    pub reactions: Vec<Reaction>,
    pub attachments: Vec<AttachmentRef>,
    pub edited_at: Option<i64>,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
    pub disappears_at: Option<i64>,
}

/// Send an encrypted message
/// Content is encrypted in Rust before leaving this command
#[tauri::command]
pub async fn send_message(
    input: SendMessageInput,
    state: State<'_, AppState>,
) -> Result<MessageResponse, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?.clone();
    
    if input.content.trim().is_empty() && input.attachments.is_empty() {
        return Err(VeilError::InvalidInput("Mesaj içeriği veya dosya eklenmelidir".into()));
    }
    if input.content.len() > 4000 {
        return Err(VeilError::InvalidInput("Mesaj çok uzun (en fazla 4000 karakter)".into()));
    }
    
    let channel_id = Uuid::parse_str(&input.channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;
    
    // Message ID binds the deterministic per-message key
    let message_id = Uuid::new_v4();

    // 1:1 DMs use the Double-Ratchet session (forward secrecy); E2EE group
    // channels use MLS; other space channels use the deterministic per-message key.
    let db = state.db.read().await;
    let is_dm = channel_is_dm(&db, &channel_id)?;
    let is_e2ee = !is_dm && channel_is_e2ee(&db, &channel_id)?;

    // Alan moderasyonu: kanal bir topluluğa aitse susturma (timeout) kontrolü.
    // Süre dolmamış üye mesaj gönderemez; yasaklı üyeler zaten üye değildir.
    if !is_dm {
        let space_id = db
            .get_channel(&channel_id, None)
            .ok()
            .flatten()
            .and_then(|c| c.space_id);
        if let Some(space_id) = space_id {
            if let Some(until) = db.get_member_timeout(&space_id, &identity.id)? {
                if until > chrono::Utc::now().timestamp() {
                    return Err(VeilError::PermissionDenied);
                }
            }
        }
    }
    drop(db);

    let (ciphertext_b64, iv_b64, crypto_meta) = if is_dm {
        match ratchet_encrypt(&state, &channel_id, input.content.as_bytes()).await {
            Ok((ct, nonce, meta)) => (ct, nonce, Some(meta)),
            Err(VeilError::PeerKeyMissing) => {
                let db = state.db.read().await;
                let reply_uuid = input.reply_to_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
                let disappears_ts = input.disappear_seconds.map(|secs| {
                    (chrono::Utc::now() + chrono::Duration::seconds(secs as i64)).timestamp()
                });
                let db_key = state.get_db_key().await;
                (*db).insert_pending_dm_encrypted(
                    &message_id,
                    &channel_id,
                    &peer_id_from_dm_channel(&state, &channel_id).await.unwrap_or_default(),
                    &input.content,
                    "text",
                    reply_uuid.as_ref(),
                    &input.attachments,
                    disappears_ts,
                    db_key.as_ref(),
                )?;
                drop(db);
                let temp_msg = Message {
                    id: message_id,
                    channel_id,
                    sender_id: identity.id,
                    sender_device_id: identity.device_id,
                    content: Some(input.content.clone()),
                    ciphertext: String::new(),
                    iv: String::new(),
                    crypto_meta: None,
                    message_type: MessageType::Text,
                    status: MessageStatus::Queued,
                    reply_to_id: reply_uuid,
                    pinned: false,
                    reactions: Vec::new(),
                    attachments: input.attachments.clone(),
                    edited_at: None,
                    created_at: chrono::Utc::now(),
                    deleted_at: None,
                    disappears_at: disappears_ts.map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now)),
                    schema_version: 1,
                };
                let db = state.db.read().await;
                let _ = db.insert_message(&temp_msg);
                drop(db);
                let sender = {
                    let db = state.db.read().await;
                    resolve_sender_info(&db, &channel_id, &identity.id, Some(&identity))
                };
                return Ok(MessageResponse {
                    id: message_id.to_string(),
                    channel_id: input.channel_id,
                    sender_id: identity.id.to_string(),
                    sender_name: sender.name,
                    sender_avatar_hash: sender.avatar_hash,
                    sender_role_color: sender.role_color,
                    content: Some(input.content),
                    message_type: "text".into(),
                    status: "queued".into(),
                    reply_to_id: input.reply_to_id,
                    pinned: false,
                    reactions: Vec::new(),
                    attachments: input.attachments,
                    edited_at: None,
                    created_at: chrono::Utc::now().timestamp(),
                    deleted_at: None,
                    disappears_at: disappears_ts,
                });
            }
            Err(e) => return Err(e),
        }
    } else if is_e2ee {
        let mut session = super::mls::load_session(&state, &channel_id).await?;
        let ct = session.encrypt_message(input.content.as_bytes())?;
        super::mls::save_session(&state, &channel_id, &session).await?;
        (B64.encode(&ct), String::new(), Some("mls".to_string()))
    } else {
        let msg_key = derive_channel_message_key(&channel_id, &message_id)?;
        let (ct, nonce) = encrypt_aes_gcm(&msg_key, input.content.as_bytes())?;
        (B64.encode(&ct), B64.encode(&nonce), None)
    };
    
    let now = Utc::now();
    let disappears_at = input.disappear_seconds
        .filter(|&secs| secs > 0)
        .map(|secs| now + chrono::Duration::seconds(secs as i64));
    
    let msg_type = if let Some(first_att) = input.attachments.first() {
        if let Some(ref mime) = first_att.mime_type_hint {
            if mime.starts_with("image/") {
                MessageType::Image
            } else if mime.starts_with("video/") {
                MessageType::Video
            } else if mime.starts_with("audio/") {
                MessageType::Audio
            } else {
                MessageType::File
            }
        } else {
            MessageType::File
        }
    } else {
        MessageType::Text
    };

    let message = Message {
        id: message_id,
        channel_id,
        sender_id: identity.id,
        sender_device_id: identity.device_id,
        content: Some(input.content.clone()), // Only in memory, not stored
        ciphertext: ciphertext_b64,
        iv: iv_b64,
        crypto_meta,
        message_type: msg_type,
        status: MessageStatus::Sending,
        reply_to_id: input.reply_to_id.as_ref().and_then(|id| Uuid::parse_str(id).ok()),
        pinned: false,
        reactions: Vec::new(),
        attachments: input.attachments.clone(),
        edited_at: None,
        created_at: now,
        deleted_at: None,
        disappears_at,
        schema_version: 1,
    };
    
    // Store in local DB (ciphertext only)
    {
        let db = state.db.read().await;
        db.insert_message(&message)?;
    }

    // Attempt network delivery (best-effort) — queue offline if unavailable.
    let network_ok = {
        let network = state.network.read().await;
        let url_set = config::configured("VEILANON_SUPABASE_URL");

        if url_set {
            let payload = serde_json::json!({
                "id": message_id.to_string(),
                "channel_id": channel_id.to_string(),
                "sender_id": identity.id.to_string(),
                "sender_device_id": identity.device_id.to_string(),
                "ciphertext": message.ciphertext,
                "iv": message.iv,
                "crypto_meta": message.crypto_meta,
                "message_type": match message.message_type {
                    MessageType::File => "file",
                    MessageType::Image => "image",
                    MessageType::Video => "video",
                    MessageType::Audio => "audio",
                    MessageType::System => "system",
                    MessageType::Call => "call",
                    MessageType::Text => "text",
                },
                "reply_to_id": message.reply_to_id.map(|r| r.to_string()),
                "pinned": message.pinned,
                "attachments": message.attachments,
                "schema_version": 1,
                "client_created_at": message.created_at.to_rfc3339(),
                "disappears_at": message.disappears_at.map(|dt| dt.to_rfc3339()),
            });

            let payload = match payload {
                serde_json::Value::Object(map) => serde_json::Value::Object(
                    map.into_iter().filter(|(_, v)| !v.is_null()).collect(),
                ),
                other => other,
            };

            let res = network.api.insert("messages", &payload).await;
            if res.is_ok() {
                // Ensure sender has channel_members row in Supabase (for realtime delivery).
                // This is critical: without this row, the sender won't receive realtime
                // notifications for messages in this channel.
                let _ = network.api.upsert(
                    "channel_members",
                    &serde_json::json!({
                        "channel_id": channel_id.to_string(),
                        "user_id": identity.id.to_string(),
                    }),
                    "channel_id,user_id",
                ).await;
                true
            } else {
                // Recover: channel or membership might not be in Supabase yet
                let db_key = state.get_db_key().await;
                let db = state.db.read().await;
                if let Ok(Some(ch)) = db.get_channel(&channel_id, db_key.as_ref()) {
                    let _ = network.api.upsert(
                        "channels",
                        &serde_json::json!({
                            "id": channel_id.to_string(),
                            "space_id": ch.space_id.map(|s| s.to_string()),
                            "name": ch.name,
                            "channel_type": match ch.channel_type {
                                ChannelType::Voice => "voice",
                                ChannelType::DirectMessage => "dm",
                                ChannelType::GroupDirectMessage => "group_dm",
                                ChannelType::Announcement => "announcement",
                                ChannelType::Forum => "forum",
                                _ => "text",
                            },
                            "position": ch.position,
                            "is_e2ee": ch.is_e2ee,
                        }),
                        "id",
                    ).await;

                    // For DM channels, use the create_dm_channel RPC to ensure
                    // both members are registered (required by RLS policies).
                    if matches!(ch.channel_type, ChannelType::DirectMessage) {
                        if let Ok(members) = db.list_channel_members(&channel_id) {
                            if let Some(peer) = members.iter().find(|m| **m != identity.id) {
                                let _ = network.api.rpc_void(
                                    "create_dm_channel",
                                    &serde_json::json!({
                                        "p_channel_id": channel_id.to_string(),
                                        "p_peer_user_id": peer.to_string(),
                                    }),
                                ).await;
                            }
                        }
                    } else if let Ok(members) = db.list_channel_members(&channel_id) {
                        for m in members {
                            let _ = network.api.upsert(
                                "channel_members",
                                &serde_json::json!({
                                    "channel_id": channel_id.to_string(),
                                    "user_id": m.to_string(),
                                }),
                                "channel_id,user_id",
                            ).await;
                        }
                    }
                }
                drop(db);
                // Retry message insert after channel upsert
                network.api.insert("messages", &payload).await.is_ok()
            }
        } else {
            false
        }
    };

    let status = if network_ok {
        {
            let db = state.db.read().await;
            db.update_message_status(&message_id, &MessageStatus::Sent)?;
        }
        "sent"
    } else {
        {
            let db = state.db.read().await;
            let queued = QueuedMessage {
                id: message_id,
                message: message.clone(),
                retry_count: 0,
                queued_at: now,
                next_retry_at: None,
            };
            db.enqueue_message(&queued)?;
            db.update_message_status(&message_id, &MessageStatus::Queued)?;
        }
        "queued"
    };

    debug!("Message delivery status: {}", status); // No content or IDs in log

    // Discord köprüsü: webhook yapılandırılmışsa mesajı yansıt (best-effort).
    super::discord::mirror_message(
        &state,
        &input.channel_id,
        &identity.display_name,
        &input.content,
    )
    .await;

    let sender = {
        let db = state.db.read().await;
        resolve_sender_info(&db, &channel_id, &identity.id, Some(&identity))
    };

    Ok(MessageResponse {
        id: message_id.to_string(),
        channel_id: input.channel_id,
        sender_id: identity.id.to_string(),
        sender_name: sender.name,
        sender_avatar_hash: sender.avatar_hash,
        sender_role_color: sender.role_color,
        content: Some(input.content),
        message_type: "text".into(),
        status: status.into(),
        reply_to_id: input.reply_to_id,
        pinned: false,
        reactions: Vec::new(),
        attachments: input.attachments,
        edited_at: None,
        created_at: now.timestamp(),
        deleted_at: None,
        disappears_at: disappears_at.map(|dt| dt.timestamp()),
    })
}

/// Load messages for a channel (decrypted)
#[tauri::command]
pub async fn load_messages(
    channel_id: String,
    before_id: Option<String>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<MessageResponse>, VeilError> {
    let channel_uuid = Uuid::parse_str(&channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;
    
    let before_uuid = before_id.as_ref()
        .and_then(|id| Uuid::parse_str(id).ok());
    
    purge_expired(&state).await;

    let db = state.db.read().await;
    let messages = db.get_messages(&channel_uuid, before_uuid.as_ref(), limit.unwrap_or(50))?;
    drop(db);

    info!(
        "Loaded {} message(s) for channel (limit {})",
        messages.len(),
        limit.unwrap_or(50)
    );

    // If local SQLite has no messages on initial channel load (e.g. after reinstall/update/first view),
    // automatically sync with remote backend and return the decrypted messages.
    if messages.is_empty() && before_uuid.is_none() && config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(synced) = sync_messages(channel_id.clone(), state.clone()).await {
            if !synced.is_empty() {
                return Ok(synced);
            }
        }
    }

    let mut responses = decrypt_batch(&state, &channel_uuid, messages).await?;
    // Veritabanı en yeni mesajı ilk sırada (DESC) döndürür. Sohbet akışında mesajların
    // kronolojik (en eski -> en yeni) dizilmesi için listeyi tersine çeviriyoruz.
    responses.reverse();
    Ok(responses)
}

fn decrypt_message_content(db_key: &[u8; 32], msg: &Message) -> Option<String> {
    let ciphertext = B64.decode(&msg.ciphertext).ok()?;
    let nonce = B64.decode(&msg.iv).ok()?;
    // 1. Try space channel key: derived from channel_id + msg.id (shared by all participants)
    if let Ok(chan_key) = derive_channel_message_key(&msg.channel_id, &msg.id) {
        if let Ok(plaintext) = decrypt_aes_gcm(&chan_key, &ciphertext, &nonce) {
            if let Ok(str_val) = String::from_utf8(plaintext) {
                return Some(str_val);
            }
        }
    }
    // 2. Fallback to local db_key (legacy local messages)
    if let Ok(key) = derive_message_key(db_key, &msg.id) {
        if let Ok(plaintext) = decrypt_aes_gcm(&key, &ciphertext, &nonce) {
            if let Ok(str_val) = String::from_utf8(plaintext) {
                return Some(str_val);
            }
        }
    }
    None
}

/// Edit a message
#[tauri::command]
pub async fn edit_message(
    message_id: String,
    new_content: String,
    state: State<'_, AppState>,
) -> Result<MessageResponse, VeilError> {
    let msg_uuid = Uuid::parse_str(&message_id)
        .map_err(|_| VeilError::InvalidInput("Invalid message ID".into()))?;

    if new_content.is_empty() {
        return Err(VeilError::InvalidInput("Message content cannot be empty".into()));
    }
    if new_content.len() > 4000 {
        return Err(VeilError::InvalidInput("Message too long (max 4000 chars)".into()));
    }

    let _db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;

    let msg = {
        let db = state.db.read().await;
        db.get_message(&msg_uuid)?
    };
    let msg = msg.ok_or(VeilError::InvalidInput("Message not found".into()))?;

    // DM channels re-encrypt through the ratchet (chain advances); E2EE group
    // channels re-encrypt through MLS; others use channel message key.
    let db = state.db.read().await;
    let is_dm = channel_is_dm(&db, &msg.channel_id)?;
    let is_e2ee = !is_dm && channel_is_e2ee(&db, &msg.channel_id)?;
    drop(db);
    let (ciphertext_b64, iv_b64, crypto_meta) = if is_dm {
        let (ct, nonce, meta) = ratchet_encrypt(&state, &msg.channel_id, new_content.as_bytes()).await?;
        (ct, nonce, Some(meta))
    } else if is_e2ee {
        let mut session = super::mls::load_session(&state, &msg.channel_id).await?;
        let ct = session.encrypt_message(new_content.as_bytes())?;
        super::mls::save_session(&state, &msg.channel_id, &session).await?;
        (B64.encode(&ct), String::new(), Some("mls".to_string()))
    } else {
        let msg_key = derive_channel_message_key(&msg.channel_id, &msg_uuid)?;
        let (ct, nonce) = encrypt_aes_gcm(&msg_key, new_content.as_bytes())?;
        (B64.encode(&ct), B64.encode(&nonce), None)
    };
    let now = Utc::now();

    {
        let db = state.db.read().await;
        db.update_message_ciphertext(&msg_uuid, &ciphertext_b64, &iv_b64, now.timestamp())?;
        db.delete_message_key(&msg_uuid)?;
        if let Some(ref meta) = crypto_meta {
            db.execute(
                "UPDATE messages SET crypto_meta = ?1 WHERE id = ?2",
                rusqlite::params![meta, msg_uuid.to_string()],
            )?;
        }
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let mut payload = serde_json::json!({
            "ciphertext": ciphertext_b64,
            "iv": iv_b64,
            "edited_at": now.to_rfc3339(),
        });
        if let Some(ref meta) = crypto_meta {
            payload["crypto_meta"] = serde_json::json!(meta);
        }
        let _ = network
            .api
            .update(
                "messages",
                &format!("id=eq.{}", message_id),
                &payload,
            )
            .await;
    }

    let sender = {
        let db = state.db.read().await;
        resolve_sender_info(&db, &msg.channel_id, &msg.sender_id, None)
    };

    Ok(MessageResponse {
        id: msg_uuid.to_string(),
        channel_id: msg.channel_id.to_string(),
        sender_id: msg.sender_id.to_string(),
        sender_name: sender.name,
        sender_avatar_hash: sender.avatar_hash,
        sender_role_color: sender.role_color,
        content: Some(new_content),
        message_type: format!("{:?}", msg.message_type).to_lowercase(),
        status: format!("{:?}", msg.status).to_lowercase(),
        reply_to_id: msg.reply_to_id.map(|id| id.to_string()),
        pinned: msg.pinned,
        reactions: msg.reactions.clone(),
        attachments: msg.attachments.clone(),
        edited_at: Some(now.timestamp()),
        created_at: msg.created_at.timestamp(),
        deleted_at: None,
        disappears_at: msg.disappears_at.map(|dt| dt.timestamp()),
    })
}

/// Delete a message
#[tauri::command]
pub async fn delete_message(
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let msg_uuid = Uuid::parse_str(&message_id)
        .map_err(|_| VeilError::InvalidInput("Invalid message ID".into()))?;
    let db = state.db.read().await;
    db.soft_delete_message(&msg_uuid)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .update(
                "messages",
                &format!("id=eq.{}", message_id),
                &serde_json::json!({ "deleted_at": Utc::now().to_rfc3339() }),
            )
            .await;
    }

    info!("Message deleted");
    Ok(())
}

/// Clear all messages in a channel (local soft delete + remote tombstone)
#[tauri::command]
pub async fn clear_channel_messages(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let ch_uuid = Uuid::parse_str(&channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;

    let db = state.db.read().await;
    db.clear_channel_messages(&ch_uuid)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .update(
                "messages",
                &format!("channel_id=eq.{}", channel_id),
                &serde_json::json!({ "deleted_at": Utc::now().to_rfc3339() }),
            )
            .await;
    }

    info!("Channel messages cleared: {}", channel_id);
    Ok(())
}

/// Add a reaction
#[tauri::command]
pub async fn add_reaction(
    message_id: String,
    emoji: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let msg_uuid = Uuid::parse_str(&message_id)
        .map_err(|_| VeilError::InvalidInput("Invalid message ID".into()))?;
    if emoji.is_empty() {
        return Err(VeilError::InvalidInput("Emoji cannot be empty".into()));
    }

    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let actor_id = identity.id;

    let db = state.db.read().await;
    let msg = db
        .get_message(&msg_uuid)?
        .ok_or(VeilError::InvalidInput("Message not found".into()))?;

    let mut reactions = msg.reactions;
    mutate_reactions(&mut reactions, &emoji, actor_id, true);
    db.update_message_reactions(&msg_uuid, &reactions)?;
    let reactions_val = serde_json::to_value(&reactions).unwrap_or(serde_json::json!([]));
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .update(
                "messages",
                &format!("id=eq.{}", message_id),
                &serde_json::json!({ "reactions": reactions_val }),
            )
            .await;
    }

    Ok(())
}

/// Remove a reaction
#[tauri::command]
pub async fn remove_reaction(
    message_id: String,
    emoji: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let msg_uuid = Uuid::parse_str(&message_id)
        .map_err(|_| VeilError::InvalidInput("Invalid message ID".into()))?;

    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let actor_id = identity.id;

    let db = state.db.read().await;
    let msg = db
        .get_message(&msg_uuid)?
        .ok_or(VeilError::InvalidInput("Message not found".into()))?;

    let mut reactions = msg.reactions;
    mutate_reactions(&mut reactions, &emoji, actor_id, false);
    db.update_message_reactions(&msg_uuid, &reactions)?;
    let reactions_val = serde_json::to_value(&reactions).unwrap_or(serde_json::json!([]));
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .update(
                "messages",
                &format!("id=eq.{}", message_id),
                &serde_json::json!({ "reactions": reactions_val }),
            )
            .await;
    }

    Ok(())
}

/// Apply an actor's reaction to the list, adding (dedup) or removing it.
/// Removed reactions with no remaining users are dropped.
fn mutate_reactions(reactions: &mut Vec<Reaction>, emoji: &str, actor_id: Uuid, add: bool) {
    if add {
        if let Some(r) = reactions.iter_mut().find(|r| r.emoji == emoji) {
            if !r.user_ids.contains(&actor_id) {
                r.user_ids.push(actor_id);
            }
            r.count = r.user_ids.len() as u32;
        } else {
            reactions.push(Reaction {
                emoji: emoji.to_string(),
                user_ids: vec![actor_id],
                count: 1,
            });
        }
    } else {
        if let Some(r) = reactions.iter_mut().find(|r| r.emoji == emoji) {
            r.user_ids.retain(|u| *u != actor_id);
            r.count = r.user_ids.len() as u32;
        }
        reactions.retain(|r| !r.user_ids.is_empty());
    }
}

/// Pin a message
#[tauri::command]
pub async fn pin_message(
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    Uuid::parse_str(&message_id)
        .map_err(|_| VeilError::InvalidInput("Invalid message ID".into()))?;
    
    let db = state.db.read().await;
    db.execute(
        "UPDATE messages SET pinned = 1 WHERE id = ?1",
        rusqlite::params![message_id],
    )?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .update(
                "messages",
                &format!("id=eq.{}", message_id),
                &serde_json::json!({ "pinned": true }),
            )
            .await;
    }

    Ok(())
}

/// Unpin a message
#[tauri::command]
pub async fn unpin_message(
    message_id: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    Uuid::parse_str(&message_id)
        .map_err(|_| VeilError::InvalidInput("Invalid message ID".into()))?;
    
    let db = state.db.read().await;
    db.execute(
        "UPDATE messages SET pinned = 0 WHERE id = ?1",
        rusqlite::params![message_id],
    )?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .update(
                "messages",
                &format!("id=eq.{}", message_id),
                &serde_json::json!({ "pinned": false }),
            )
            .await;
    }

    Ok(())
}

/// Mark channel messages as read
#[tauri::command]
pub async fn mark_as_read(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let channel_uuid = Uuid::parse_str(&channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;
    let db = state.db.read().await;
    db.mark_channel_read(&channel_uuid)?;
    Ok(())
}

/// Get pinned messages for a channel
#[tauri::command]
pub async fn get_pinned_messages(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MessageResponse>, VeilError> {
    let channel_uuid = Uuid::parse_str(&channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;

    let messages = {
        let db = state.db.read().await;
        db.get_pinned_messages(&channel_uuid)?
    };

    decrypt_batch(&state, &channel_uuid, messages).await
}

/// Search messages locally (never sends content to remote)
#[tauri::command]
pub async fn search_messages(
    channel_id: Option<String>,
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<MessageResponse>, VeilError> {
    // Local search only — decrypts and searches in memory
    // E2EE content NEVER sent to remote search index by default
    if query.len() < 2 {
        return Err(VeilError::InvalidInput("Search query too short".into()));
    }

    let needle = query.to_lowercase();
    let result_limit = limit.unwrap_or(50).min(100) as usize;

    let messages = {
        let db = state.db.read().await;
        match channel_id.as_deref() {
            Some(cid) => {
                let channel_uuid = Uuid::parse_str(cid)
                    .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;
                db.get_messages(&channel_uuid, None, 100)?
            }
            None => db.get_all_messages(1000)?,
        }
    };

    // Group by channel so DM batches decrypt through their ratchet session.
    let mut by_channel: std::collections::HashMap<Uuid, Vec<Message>> =
        std::collections::HashMap::new();
    for msg in messages {
        by_channel.entry(msg.channel_id).or_default().push(msg);
    }

    let mut responses = Vec::new();
    for (channel, msgs) in by_channel {
        let decrypted = decrypt_batch(&state, &channel, msgs).await?;
        for resp in decrypted {
            let Some(content) = resp.content.as_deref() else { continue };
            if !content.to_lowercase().contains(&needle) {
                continue;
            }
            responses.push(resp);
            if responses.len() >= result_limit {
                break;
            }
        }
        if responses.len() >= result_limit {
            break;
        }
    }

    Ok(responses)
}

/// Pull remote ciphertext rows for a channel and merge them into the local
/// store (idempotent by message id). Returns the freshly inserted rows,
/// decrypted, so the UI can render them without a full reload.
#[tauri::command]
pub async fn sync_messages(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MessageResponse>, VeilError> {
    let channel_uuid = Uuid::parse_str(&channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;

    let url_set = config::configured("VEILANON_SUPABASE_URL");
    if !url_set {
        return Ok(Vec::new());
    }

    // Ensure channel and its members exist locally in SQLite
    {
        let db = state.db.read().await;
        let has_channel = db.channel_type_of(&channel_uuid)?.is_some();
        drop(db);

        if !has_channel {
            let network = state.network.read().await;
            let ch_filter = format!("id=eq.{}", channel_uuid);
            if let Ok(ch_rows) = network.api.select::<serde_json::Value>("channels", &ch_filter, None, Some(1)).await {
                if let Some(ch) = ch_rows.first() {
                    let cname = ch.get("name").and_then(|v| v.as_str()).unwrap_or("Sohbet");
                    let ctype_str = ch.get("channel_type").and_then(|v| v.as_str()).unwrap_or("dm");
                    let ctype = if ctype_str == "group_dm" || ctype_str == "group_direct_message" {
                        ChannelType::GroupDirectMessage
                    } else if ctype_str == "voice" {
                        ChannelType::Voice
                    } else if ctype_str == "announcement" {
                        ChannelType::Announcement
                    } else if ctype_str == "forum" {
                        ChannelType::Forum
                    } else if ctype_str == "dm" || ctype_str == "direct_message" {
                        ChannelType::DirectMessage
                    } else {
                        ChannelType::Text
                    };
                    let pos = ch.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let is_nsfw = ch.get("is_nsfw").and_then(|v| v.as_bool()).unwrap_or(false);
                    let is_e2ee = ch.get("is_e2ee").and_then(|v| v.as_bool()).unwrap_or(true);
                    let space_id = ch.get("space_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok());
                    let channel = crate::models::channel::Channel {
                        id: channel_uuid,
                        space_id,
                        name: cname.to_string(),
                        channel_type: ctype,
                        position: pos,
                        topic: None,
                        is_nsfw,
                        is_e2ee,
                        slow_mode_seconds: 0,
                        permission_overrides: Vec::new(),
                        created_at: chrono::Utc::now(),
                        last_message_id: None,
                        unread_count: 0,
                        mentioned: false,
                    };
                    let db = state.db.read().await;
                    let db_key = state.get_db_key().await;
                    let _ = db.upsert_channel(&channel, db_key.as_ref());
                }
            }

            let cm_filter = format!("channel_id=eq.{}", channel_uuid);
            if let Ok(cm_rows) = network.api.select::<serde_json::Value>("channel_members", &cm_filter, None, Some(50)).await {
                let db = state.db.read().await;
                for m in cm_rows {
                    if let Some(uid_str) = m.get("user_id").and_then(|v| v.as_str()) {
                        if let Ok(uid) = Uuid::parse_str(uid_str) {
                            let _ = db.add_channel_member(&channel_uuid, &uid);
                        }
                    }
                }
            }

            if let Some(identity) = state.get_or_restore_identity().await.as_ref() {
                let _ = network.api.upsert(
                    "channel_members",
                    &serde_json::json!({
                        "channel_id": channel_uuid.to_string(),
                        "user_id": identity.id.to_string(),
                    }),
                    "channel_id,user_id",
                ).await;
            }
        }
    }

    let mut fresh: Vec<Message> = Vec::new();

    // Remote rows: ciphertext only — never plaintext crosses the wire.
    type RemoteRow = serde_json::Value;
    let rows: Vec<RemoteRow> = {
        let network = state.network.read().await;
        let filter = format!("channel_id=eq.{}&deleted_at=is.null&select=id,channel_id,sender_id,sender_device_id,ciphertext,iv,crypto_meta,message_type,attachments,reactions,schema_version,client_created_at,disappears_at,reply_to_id,pinned,deleted_at", channel_uuid);
        network.api.select("messages", &filter, Some("client_created_at.asc"), Some(200)).await?
    };

    // Resolve device → owning user once, as fallback for older rows
    let mut device_owners: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();
    {
        let network = state.network.read().await;
        let devices: Vec<serde_json::Value> = network
            .api
            .select("devices", "select=id,user_id", None, Some(500))
            .await
            .unwrap_or_default();
        for d in devices {
            if let (Some(id), Some(uid)) = (
                d.get("id").and_then(|v| v.as_str()),
                d.get("user_id").and_then(|v| v.as_str()),
            ) {
                if let (Ok(dev), Ok(usr)) = (Uuid::parse_str(id), Uuid::parse_str(uid)) {
                    device_owners.insert(dev.to_string(), usr);
                }
            }
        }
    }

    for row in rows {
        let Some(id_str) = row.get("id").and_then(|v| v.as_str()) else { continue };
        let Ok(id) = Uuid::parse_str(id_str) else { continue };
        let Some(ciphertext) = row.get("ciphertext").and_then(|v| v.as_str()) else { continue };
        let Some(iv) = row.get("iv").and_then(|v| v.as_str()) else { continue };
        let schema_version = row.get("schema_version").and_then(|v| v.as_i64()).unwrap_or(1) as u8;
        let crypto_meta = row.get("crypto_meta").and_then(|v| v.as_str()).map(str::to_string);
        let sender_device = row.get("sender_device_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let sender_id = row.get("sender_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .or_else(|| device_owners.get(&sender_device).copied())
            .unwrap_or_else(Uuid::nil);
        let reply_to_id = row.get("reply_to_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());
        let pinned = row.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false);
        let created_ts = row.get("client_created_at").and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);
        let disappears_ts = row.get("disappears_at").and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let msg_type_str = row.get("message_type").and_then(|v| v.as_str()).unwrap_or("text");
        let msg_type = match msg_type_str {
            "image" => MessageType::Image,
            "video" => MessageType::Video,
            "audio" => MessageType::Audio,
            "file" => MessageType::File,
            "system" => MessageType::System,
            "call" => MessageType::Call,
            _ => MessageType::Text,
        };

        let attachments: Vec<AttachmentRef> = row.get("attachments")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let reactions: Vec<Reaction> = row.get("reactions")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        // Skip rows we already have locally (idempotent merge).
        let exists: bool = {
            let db = state.db.read().await;
            db.query_row(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE id = ?1)",
                rusqlite::params![id.to_string()],
                |r| r.get(0),
            )?
        };
        if exists {
            continue;
        }

        // Cache sender profile if not in SQLite
        if sender_id != Uuid::nil() {
            let has_prof = {
                let db = state.db.read().await;
                db.get_profile_by_id(&sender_id).ok().flatten().is_some()
            };
            if !has_prof {
                let network = state.network.read().await;
                let u_filter = format!("id=eq.{}&select=id,username,display_name,avatar_hash", sender_id);
                if let Ok(u_rows) = network.api.select::<serde_json::Value>("users", &u_filter, None, Some(1)).await {
                    if let Some(u) = u_rows.first() {
                        let uname = u.get("username").and_then(|v| v.as_str()).unwrap_or("");
                        let disp = u.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
                        let av = u.get("avatar_hash").and_then(|v| v.as_str());
                        let db = state.db.read().await;
                        let _ = db.upsert_profile(&sender_id, uname, disp, av, None, None, None, None, None);
                    }
                }
            }
        }

        let message = Message {
            id,
            channel_id: channel_uuid,
            sender_id,
            sender_device_id: Uuid::parse_str(&sender_device).unwrap_or_default(),
            content: None,
            ciphertext: ciphertext.to_string(),
            iv: iv.to_string(),
            crypto_meta,
            message_type: msg_type,
            status: MessageStatus::Sent,
            reply_to_id,
            pinned,
            reactions,
            attachments,
            edited_at: None,
            created_at: created_ts,
            deleted_at: None,
            disappears_at: disappears_ts,
            schema_version,
        };

        {
            let db = state.db.read().await;
            if let Err(e) = db.insert_message(&message) {
                debug!("Sync insert skipped: {}", e);
                continue;
            }
        }
        fresh.push(message);
    }

    let responses = decrypt_batch(&state, &channel_uuid, fresh).await?;

    info!("Synced {} new message(s) for channel", responses.len());
    Ok(responses)
}

async fn peer_id_from_dm_channel(state: &AppState, channel_id: &Uuid) -> VeilResult<Uuid> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let db = state.db.read().await;
    let members = db.list_channel_members(channel_id)?;
    drop(db);
    members.iter()
        .find(|m| **m != identity.id)
        .copied()
        .ok_or(VeilError::InvalidInput("DM channel has no peer".into()))
}

pub(crate) async fn flush_pending_dm_messages(state: &AppState) {
    let identity = match state.get_or_restore_identity().await {
        Some(id) => id.clone(),
        None => return,
    };

    let peer_ids: Vec<Uuid> = {
        let db = state.db.read().await;
        match db.query_map(
            "SELECT DISTINCT peer_id FROM pending_dm_messages",
            [],
            |row| {
                let s: String = row.get(0)?;
                Ok(Uuid::parse_str(&s).unwrap_or_default())
            },
        ) {
            Ok(ids) => ids.into_iter().filter(|id| !id.is_nil()).collect(),
            Err(_) => return,
        }
    };

    for peer_id in peer_ids {
        let db_key = state.get_db_key().await;
        let pending = {
            let db = state.db.read().await;
            match (*db).get_pending_dms_by_peer_decrypted(&peer_id, db_key.as_ref()) {
                Ok(p) => p,
                Err(_) => continue,
            }
        };

        if pending.is_empty() {
            continue;
        }

        for (msg_id, channel_id, content, _msg_type, reply_to_id, attachments, disappears_at) in pending {
            match ratchet_encrypt(state, &channel_id, content.as_bytes()).await {
                Ok((ciphertext_b64, iv_b64, crypto_meta)) => {
                    let reply_uuid = reply_to_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
                    let disappears_dt = disappears_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
                    let now = Utc::now();

                    let message = Message {
                        id: msg_id,
                        channel_id,
                        sender_id: identity.id,
                        sender_device_id: identity.device_id,
                        content: Some(content),
                        ciphertext: ciphertext_b64.clone(),
                        iv: iv_b64.clone(),
                        crypto_meta: Some(crypto_meta.clone()),
                        message_type: MessageType::Text,
                        status: MessageStatus::Sending,
                        reply_to_id: reply_uuid,
                        pinned: false,
                        reactions: Vec::new(),
                        attachments: attachments.clone(),
                        edited_at: None,
                        created_at: now,
                        deleted_at: None,
                        disappears_at: disappears_dt,
                        schema_version: 1,
                    };

                    {
                        let db = state.db.read().await;
                        let _ = db.insert_message(&message);
                    }

                    let network_ok = {
                        let network = state.network.read().await;
                        if config::configured("VEILANON_SUPABASE_URL") {
                            let payload = serde_json::json!({
                                "id": msg_id.to_string(),
                                "channel_id": channel_id.to_string(),
                                "sender_id": identity.id.to_string(),
                                "sender_device_id": identity.device_id.to_string(),
                                "ciphertext": ciphertext_b64,
                                "iv": iv_b64,
                                "crypto_meta": crypto_meta,
                                "schema_version": 1,
                                "client_created_at": now.to_rfc3339(),
                                "disappears_at": disappears_dt.map(|dt| dt.to_rfc3339()),
                            });
                            let payload = match payload {
                                serde_json::Value::Object(map) => serde_json::Value::Object(
                                    map.into_iter().filter(|(_, v)| !v.is_null()).collect(),
                                ),
                                other => other,
                            };
                            network.api.insert("messages", &payload).await.is_ok()
                        } else {
                            false
                        }
                    };

                    {
                        let db = state.db.read().await;
                        if network_ok {
                            let _ = db.update_message_status(&msg_id, &MessageStatus::Sent);
                        } else {
                            let _ = db.update_message_status(&msg_id, &MessageStatus::Queued);
                        }
                        let _ = (*db).delete_pending_dm(&msg_id);
                    }

                    let _ = state.app.emit("veilanon:realtime-message", serde_json::json!({
                        "channel_id": channel_id.to_string(),
                        "id": msg_id.to_string(),
                    }));

                    info!("Flushed pending DM message for peer {}", peer_id);
                }
                Err(_) => {
                    debug!("Still cannot encrypt for peer {}; will retry later", peer_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dm_root_key_is_symmetric() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let shared = [7u8; 32];
        let k1 = derive_dm_root_key(&shared, &a, &b).unwrap();
        let k2 = derive_dm_root_key(&shared, &b, &a).unwrap();
        assert_eq!(k1, k2);
        assert_ne!(k1, [0u8; 32]);
    }

    #[test]
    fn add_reaction_dedupes_same_actor() {
        let actor = Uuid::new_v4();
        let mut reactions: Vec<Reaction> = Vec::new();

        mutate_reactions(&mut reactions, "👍", actor, true);
        mutate_reactions(&mut reactions, "👍", actor, true);

        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].user_ids, vec![actor]);
        assert_eq!(reactions[0].count, 1);
    }

    #[test]
    fn remove_reaction_drops_empty_entries() {
        let actor = Uuid::new_v4();
        let mut reactions = vec![Reaction {
            emoji: "👍".to_string(),
            user_ids: vec![actor],
            count: 1,
        }];

        mutate_reactions(&mut reactions, "👍", actor, false);

        assert!(reactions.is_empty());
    }

    #[test]
    fn remove_reaction_keeps_other_actors() {
        let actor_a = Uuid::new_v4();
        let actor_b = Uuid::new_v4();
        let mut reactions = vec![Reaction {
            emoji: "🔥".to_string(),
            user_ids: vec![actor_a, actor_b],
            count: 2,
        }];

        mutate_reactions(&mut reactions, "🔥", actor_a, false);

        assert_eq!(reactions.len(), 1);
        assert_eq!(reactions[0].user_ids, vec![actor_b]);
        assert_eq!(reactions[0].count, 1);
    }
}

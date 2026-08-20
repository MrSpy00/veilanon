//! Client-side diagnostics IPC.
//!
//! Routes WebView errors/warnings into the app's tracing file log so that
//! issues in the UI layer are visible in the same log stream as the Rust core.
//!
//! SECURITY: the message is sanitized against log injection (control
//! characters stripped), capped in length, and known PII shapes (e-mail
//! addresses) are masked before anything reaches the log file.

#[cfg(unix)]
use crate::error::VeilError;
use crate::error::VeilResult;
use crate::config;
use serde::Serialize;
use tauri::State;
use tracing::{debug as log_debug, error as log_error, info as log_info, warn as log_warn};

use crate::state::AppState;

const MAX_CLIENT_LOG_CHARS: usize = 2000;

fn mask_pii(input: &str) -> String {
    // E-mail addresses → `a***@example.com` (keep domain, mask local part).
    let email_re = regex::Regex::new(
        r"(?i)\b([a-z0-9._%+-])[a-z0-9._%+-]*@([a-z0-9.-]+\.[a-z]{2,})\b",
    )
    .expect("static email regex");
    let masked = email_re.replace_all(input, |caps: &regex::Captures<'_>| {
        format!("{}***@{}", &caps[1], &caps[2])
    });
    masked.into_owned()
}

#[tauri::command]
pub fn log_client_error(level: Option<String>, message: String) -> VeilResult<()> {
    let clean: String = message
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_CLIENT_LOG_CHARS)
        .collect();
    if clean.is_empty() {
        return Ok(());
    }
    let clean = mask_pii(&clean);
    match level.as_deref() {
        Some("debug") => log_debug!("[client] {clean}"),
        Some("info") => log_info!("[client] {clean}"),
        Some("warn") => log_warn!("[client] {clean}"),
        _ => log_error!("[client] {clean}"),
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub app_version: String,
    pub platform: String,
    pub supabase_configured: bool,
    pub supabase_reachable: bool,
    pub livekit_configured: bool,
    pub r2_configured: bool,
    pub realtime_connected: bool,
    pub message_count: u64,
    pub friend_count: u64,
    pub space_count: u64,
    pub queued_count: u64,
    pub file_count: u64,
    pub database_size_bytes: u64,
    pub log_directory: String,
}

/// Non-sensitive runtime diagnostics for the About / support screen.
/// No keys, tokens, PII or message content — counts and flags only.
#[tauri::command]
pub async fn get_diagnostics(state: State<'_, AppState>) -> VeilResult<Diagnostics> {
    let (message_count, friend_count, space_count, queued_count, file_count) = {
        let db = state.db.read().await;
        let count = |sql: &str| -> u64 {
            db.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0).max(0) as u64
        };
        (
            count("SELECT COUNT(*) FROM messages WHERE deleted_at IS NULL"),
            count("SELECT COUNT(*) FROM friends WHERE status = 'accepted'"),
            count("SELECT COUNT(*) FROM spaces"),
            count("SELECT COUNT(*) FROM offline_queue"),
            count("SELECT COUNT(*) FROM file_metadata WHERE deleted_at IS NULL"),
        )
    };

    let realtime_connected = state.network.read().await.realtime.is_connected();

    let supabase_configured = config::configured("VEILANON_SUPABASE_URL");
    let supabase_reachable = supabase_configured && realtime_connected;

    let database_size_bytes = std::fs::metadata(
        tauri::Manager::path(&state.app)
            .app_data_dir()
            .map(|p| p.join("veilanon.db"))
            .unwrap_or_default(),
    )
    .map(|m| m.len())
    .unwrap_or(0);

    Ok(Diagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        supabase_configured,
        supabase_reachable,
        livekit_configured: config::configured("VEILANON_LIVEKIT_URL"),
        r2_configured: config::configured("VEILANON_SUPABASE_URL"),
        realtime_connected,
        message_count,
        friend_count,
        space_count,
        queued_count,
        file_count,
        database_size_bytes,
        log_directory: crate::default_log_dir().display().to_string(),
    })
}

/// Path of the rolling file log — the UI opens it with the system file
/// manager so users can inspect or share the full log stream.
#[tauri::command]
pub fn get_log_directory() -> VeilResult<String> {
    Ok(crate::default_log_dir().display().to_string())
}

/// Open the log directory in the system file manager. Uses the OS-native
/// opener (explorer/open/xdg-open) instead of WebView file:// URLs, which
/// WebView2 blocks or renders unreliably.
#[tauri::command]
pub fn open_log_folder(app: tauri::AppHandle) -> VeilResult<()> {
    let dir = crate::default_log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let log_file = dir.join("veilanon.log");
    if !log_file.exists() {
        let _ = std::fs::write(&log_file, b"--- veilanon diagnostic log initialized ---\n");
    }
    #[cfg(windows)]
    {
        let win_path = dir.to_string_lossy().replace('/', "\\");
        let spawned = std::process::Command::new("explorer.exe")
            .arg(&win_path)
            .spawn();
        if spawned.is_err() {
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("Start-Process -FilePath '{}'", win_path)])
                .spawn();
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&dir)
            .spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn();
    }
    let _ = tauri_plugin_opener::OpenerExt::opener(&app).reveal_item_in_dir(log_file.to_string_lossy().as_ref());
    Ok(())
}

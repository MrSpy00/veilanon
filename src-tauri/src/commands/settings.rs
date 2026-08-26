//! Settings IPC commands

use tauri::Manager;
use tauri::State;
use serde::Serialize;
use tracing::info;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::state::{AppState, AppSettings};
use crate::error::VeilError;
use crate::crypto::{decrypt_aes_gcm, encrypt_aes_gcm};

const EXPORT_PREFIX: &str = "VEILANON_EXPORT_1.";
const EXPORT_VERSION: u64 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub app_name: String,
    pub version: String,
    pub description: String,
    pub developer: String,
    pub developer_url: String,
    pub developer_github: String,
    pub project_github: String,
    pub support_url: String,
    pub license: String,
    pub build_date: String,
    pub rust_version: String,
    pub platform: String,
}

/// Get all application settings
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, VeilError> {
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

/// Update application settings (supports partial updates cleanly)
#[tauri::command]
pub async fn update_settings(
    new_settings: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<AppSettings, VeilError> {
    let data_dir = state.app.path().app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;

    let saved = {
        let mut settings = state.settings.write().await;
        let mut current_val = serde_json::to_value(&*settings)
            .map_err(|_| VeilError::SerializationError)?;
        
        if let (serde_json::Value::Object(ref mut cur_map), serde_json::Value::Object(new_map)) = (&mut current_val, new_settings) {
            for (k, v) in new_map {
                if !v.is_null() {
                    cur_map.insert(k, v);
                }
            }
        }
        
        let merged: AppSettings = serde_json::from_value(current_val)
            .map_err(|_| VeilError::SerializationError)?;
        *settings = merged;
        settings.save(&data_dir)?;
        settings.clone()
    };

    let proxy_url = saved.network_privacy.get_effective_proxy_url();
    {
        let mut network = state.network.write().await;
        if let Err(e) = network.api.apply_proxy(proxy_url.as_deref()) {
            tracing::warn!("Failed to apply network proxy dynamically: {}", e);
        }
    }

    info!("Settings updated");
    Ok(saved)
}

/// Get about information for the About screen
#[tauri::command]
pub async fn get_about_info() -> Result<AboutInfo, VeilError> {
    Ok(AboutInfo {
        app_name: "veilanon".into(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "veilanon, gizlilik, hız ve özgürlük odaklı olarak geliştirilen açık kaynaklı bir iletişim uygulamasıdır. Proje, aegisSoft tarafından geliştirilmiştir.".into(),
        developer: "aegisSoft".into(),
        developer_url: "https://www.aegissoft.com.tr/".into(),
        developer_github: "https://github.com/MrSpy00".into(),
        project_github: "https://github.com/MrSpy00/veilanon".into(),
        support_url: "https://buymeacoffee.com/aegissoft".into(),
        license: "AGPL-3.0".into(),
        build_date: option_env!("VEILANON_BUILD_DATE").unwrap_or("—").to_string(),
        rust_version: option_env!("RUSTC_VERSION").unwrap_or("stable").to_string(),
        platform: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
    })
}

/// Configure startup behavior
#[tauri::command]
pub async fn set_startup_behavior(
    start_on_login: bool,
    minimize_to_tray: bool,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    use tauri_plugin_autostart::ManagerExt;
    if start_on_login {
        let _ = state.app.autolaunch().enable();
    } else {
        let _ = state.app.autolaunch().disable();
    }
    let data_dir = state.app.path().app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let mut settings = state.settings.write().await;
    settings.start_on_login = start_on_login;
    settings.minimize_to_tray = minimize_to_tray;
    settings.save(&data_dir)?;
    Ok(())
}

/// Export user data (GDPR-compliant, encrypted archive)
#[tauri::command]
pub async fn export_data(
    output_path: String,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;

    let (tables, identity) = {
        let db = state.db.read().await;
        let tables = db.export_all_rows()?;
        let identity = tables
            .get("local_identity")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        (tables, identity)
    };

    let settings_val = {
        let settings = state.settings.read().await;
        serde_json::to_value(&*settings).map_err(|_| VeilError::SerializationError)?
    };

    let envelope = serde_json::json!({
        "version": EXPORT_VERSION,
        "exported_at": chrono::Utc::now().timestamp(),
        "identity": identity,
        "settings": settings_val,
        "tables": tables,
    });

    let plaintext = serde_json::to_vec(&envelope).map_err(|_| VeilError::SerializationError)?;
    let (ciphertext, nonce) = encrypt_aes_gcm(&db_key, &plaintext)?;

    let mut payload = ciphertext;
    payload.extend_from_slice(&nonce);
    let file_content = format!("{EXPORT_PREFIX}{}", B64.encode(&payload));

    std::fs::write(&output_path, file_content)?;

    info!("Data exported to {}", output_path);
    Ok(output_path)
}

/// Import user data from export archive
#[tauri::command]
pub async fn import_data(
    archive_path: String,
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let keys = state
        .keystore
        .load_keys(&passphrase)
        .map_err(|_| VeilError::InvalidPassphrase)?;

    let db_key = match state.get_db_key().await {
        Some(key) => key,
        None => {
            state.set_db_key(keys.db_key).await;
            keys.db_key
        }
    };

    let file_content = std::fs::read_to_string(&archive_path)?;
    let encoded = file_content
        .strip_prefix(EXPORT_PREFIX)
        .ok_or_else(|| VeilError::InvalidInput("invalid export archive".into()))?;
    let payload = B64
        .decode(encoded)
        .map_err(|_| VeilError::InvalidInput("invalid export archive".into()))?;
    if payload.len() < 12 {
        return Err(VeilError::InvalidInput("invalid export archive".into()));
    }
    let split = payload.len() - 12;
    let plaintext = decrypt_aes_gcm(&db_key, &payload[..split], &payload[split..])?;

    let envelope: serde_json::Value =
        serde_json::from_slice(&plaintext).map_err(|_| VeilError::SerializationError)?;

    let version = envelope
        .get("version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| VeilError::InvalidInput("missing archive version".into()))?;
    if version != EXPORT_VERSION {
        return Err(VeilError::InvalidInput(format!(
            "unsupported archive version: {version}"
        )));
    }

    let tables = envelope
        .get("tables")
        .ok_or_else(|| VeilError::InvalidInput("missing tables".into()))?;
    {
        let db = state.db.read().await;
        db.import_rows(tables)?;
    }

    if let Some(new_settings) = envelope
        .get("settings")
        .and_then(|s| serde_json::from_value::<AppSettings>(s.clone()).ok())
    {
        let data_dir = state.app.path().app_data_dir().map_err(|_| {
            VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir"))
        })?;
        let mut settings = state.settings.write().await;
        *settings = new_settings;
        settings.save(&data_dir)?;
    }

    info!("Data imported from {}", archive_path);
    Ok(())
}

/// Clear all local data (with confirmation)
#[tauri::command]
pub async fn clear_local_data(
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    // Require passphrase verification before wiping
    state.keystore.load_keys(&passphrase)
        .map_err(|_| VeilError::InvalidPassphrase)?;

    let data_dir = state.app.path().app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;

    // In-memory state first — identity, DB key, network token, keystore cache
    *state.identity.write().await = None;
    state.clear_db_key().await;
    state.network.write().await.api.clear_token();
    state.keystore.clear_cached_secrets();

    // Close the SQLite connection so the files can be removed, then delete DB files
    {
        let mut db = state.db.write().await;
        db.force_close()?;
    }
    let db_path = data_dir.join("veilanon.db");
    if db_path.exists() { std::fs::remove_file(&db_path)?; }
    let wal_path = data_dir.join("veilanon.db-wal");
    if wal_path.exists() { std::fs::remove_file(&wal_path)?; }
    let shm_path = data_dir.join("veilanon.db-shm");
    if shm_path.exists() { std::fs::remove_file(&shm_path)?; }

    // Delete settings.json
    let settings_path = data_dir.join("settings.json");
    if settings_path.exists() { std::fs::remove_file(&settings_path)?; }

    // Delete logs directory recursively
    let logs_dir = data_dir.join("logs");
    if logs_dir.exists() { std::fs::remove_dir_all(&logs_dir)?; }

    // Keychain entries + salt — last: the point of no return
    state.keystore.delete_keys()?;

    // Reset in-memory settings to defaults (do not save — the file is gone)
    *state.settings.write().await = AppSettings::default();

    info!("Local data cleared");
    Ok(())
}

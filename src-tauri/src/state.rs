//! Application state — managed by Tauri, shared across commands
//! 
//! SECURITY: AppState holds handles to subsystems.
//! Raw key material is NOT stored here — it lives in KeyStore only.

use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tauri::{AppHandle, Manager};
use anyhow::Result;
use zeroize::Zeroize;

use crate::db::Database;
use crate::crypto::KeyStore;
use crate::crypto::identity::DeviceIdentity;
use crate::network::NetworkManager;
use crate::models::Identity;

pub struct AppState {
    pub app: AppHandle,
    pub db: Arc<RwLock<Database>>,
    pub keystore: Arc<KeyStore>,
    pub network: Arc<RwLock<NetworkManager>>,
    pub identity: Arc<RwLock<Option<Identity>>>,
    /// Private device keys held in memory while authenticated — enables
    /// signing and Double-Ratchet key agreement without re-unlocking the
    /// keychain. Cleared on sign_out; never serialized or logged.
    pub device_identity: Arc<RwLock<Option<DeviceIdentity>>>,
    pub settings: Arc<RwLock<AppSettings>>,
    /// DB encryption key — set after create_identity/load_identity,
    /// cleared on sign_out. NEVER written to disk or logs.
    pub db_key: Arc<RwLock<Option<[u8; 32]>>>,
    /// Failed passphrase attempts since last success — in-memory only.
    /// Drives escalating lockout for brute-force resistance; resets on
    /// successful unlock, sign_out, or process restart.
    pub failed_attempts: AtomicU32,
    /// Persisted Supabase anonymous-session refresh token (mirror of the
    /// local_identity column). Lets bind_control_plane reuse the SAME
    /// anonymous user across restarts instead of creating a new one.
    pub supabase_refresh_token: Arc<RwLock<Option<String>>>,
    /// Recently modified role_members: key = "{space_id}:{user_id}", value = Instant of last local write.
    /// Prevents Supabase round-trip from overwriting a just-completed local role update.
    pub role_members_modified: Arc<RwLock<HashMap<String, Instant>>>,
    /// Recently modified spaces: key = space_id, value = Instant of last local write (e.g. ownership transfer).
    /// Prevents Supabase round-trip from overwriting a just-completed local space mutation.
    pub spaces_modified: Arc<RwLock<HashMap<String, Instant>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    // Privacy
    pub presence_visibility: PresenceVisibility,
    pub show_read_receipts: bool,
    pub show_typing_indicator: bool,
    pub auto_download_media: bool,
    pub link_previews: bool,
    pub notification_preview: NotificationPreview,
    pub telemetry_enabled: bool,
    pub local_ai_enabled: bool,
    pub discord_bridge_enabled: bool,
    /// Kayıt tarihi profilde görünsün mü? Açıkken herkes görür; kapalıyken
    /// yalnızca kullanıcının kendisi görür.
    pub show_join_date: bool,
    /// Ağ & Bağlantı Gizliliği (Tor / Proxy / WireGuard)
    pub network_privacy: NetworkPrivacySettings,

    // Appearance
    pub theme: Theme,
    pub font_size: u8,
    pub reduce_motion: bool,
    pub compact_mode: bool,
    pub accent_color: Option<String>,
    pub amoled_mode: bool,
    pub preset_theme_id: String,
    pub custom_theme_name: String,
    pub custom_css: String,
    pub custom_css_enabled: bool,
    pub custom_bg_image: String,
    pub custom_bg_video: String,
    pub custom_bg_opacity: f32,
    pub saved_themes: String,  // JSON string of saved themes array

    // Notifications
    pub desktop_notifications: bool,
    pub notification_sound: bool,
    pub mention_only: bool,
    pub notification_volume: Option<u8>,
    pub sound_messages: bool,
    pub sound_mentions: bool,
    pub sound_friends: bool,
    pub sound_calls: bool,
    pub dnd_suppress_notifications: bool,

    // Audio/Video
    pub input_device_id: Option<String>,
    pub output_device_id: Option<String>,
    pub video_device_id: Option<String>,
    pub noise_suppression: bool,
    pub echo_cancellation: bool,
    pub push_to_talk: bool,
    pub push_to_talk_key: Option<String>,
    pub mirror_camera: bool,

    // System
    pub start_on_login: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub hardware_acceleration: bool,
    pub language: String,
    /// Uygulama açılışında GitHub'dan yeni sürüm kontrolü yapılsın mı?
    pub auto_update_check: bool,
    /// Bu cihazda oturum parolası OS anahtar kasasında saklanıp açılışta
    /// sorulmadan oturum açılsın mı? (remember_passphrase)
    pub auto_unlock: bool,
    /// DM gizliliği: herkes, yalnızca arkadaşlar, aynı sunucudakiler, kimse
    pub dm_privacy: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProxyMode {
    #[default]
    Direct,
    Tor,
    CustomSocks,
    CustomHttp,
    Wireguard,
    CloudflareWarp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkPrivacySettings {
    pub mode: NetworkProxyMode,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub strict_mode: bool,
    pub route_app_only: bool,
    pub custom_proxy_url: Option<String>,
    pub wireguard_profile: Option<String>,
    pub auto_start_tor: bool,
    pub verify_exit_node: bool,
    pub tor_bridge_type: Option<String>,
    pub active_preset: Option<String>,
    pub wireguard_endpoint: Option<String>,
    pub wireguard_public_key: Option<String>,
    pub wireguard_allowed_ips: Option<String>,
}

impl Default for NetworkPrivacySettings {
    fn default() -> Self {
        Self {
            mode: NetworkProxyMode::Direct,
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 9050,
            strict_mode: false,
            route_app_only: true,
            custom_proxy_url: None,
            wireguard_profile: None,
            auto_start_tor: false,
            verify_exit_node: true,
            tor_bridge_type: None,
            active_preset: None,
            wireguard_endpoint: None,
            wireguard_public_key: None,
            wireguard_allowed_ips: None,
        }
    }
}

impl NetworkPrivacySettings {
    pub fn get_effective_proxy_url(&self) -> Option<String> {
        match self.mode {
            NetworkProxyMode::Direct | NetworkProxyMode::Wireguard => None,
            NetworkProxyMode::Tor => {
                let host = if self.proxy_host.trim().is_empty() { "127.0.0.1" } else { self.proxy_host.trim() };
                let port = if self.proxy_port == 0 { 9050 } else { self.proxy_port };
                Some(format!("socks5h://{}:{}", host, port))
            }
            NetworkProxyMode::CloudflareWarp => {
                // Cloudflare WARP local proxy mode (typically on localhost socks5 port or direct warp tunnel)
                if let Some(custom) = &self.custom_proxy_url {
                    if !custom.trim().is_empty() {
                        let trimmed = custom.trim();
                        if !trimmed.starts_with("socks5://") && !trimmed.starts_with("socks5h://") && !trimmed.starts_with("http://") {
                            return Some(format!("socks5h://{}", trimmed));
                        }
                        return Some(trimmed.to_string());
                    }
                }
                let host = if self.proxy_host.trim().is_empty() { "127.0.0.1" } else { self.proxy_host.trim() };
                let port = if self.proxy_port == 0 { 40000 } else { self.proxy_port };
                Some(format!("socks5h://{}:{}", host, port))
            }
            NetworkProxyMode::CustomSocks => {
                if let Some(custom) = &self.custom_proxy_url {
                    if !custom.trim().is_empty() {
                        let trimmed = custom.trim();
                        if !trimmed.starts_with("socks5://") && !trimmed.starts_with("socks5h://") {
                            return Some(format!("socks5h://{}", trimmed));
                        }
                        return Some(trimmed.to_string());
                    }
                }
                let host = if self.proxy_host.trim().is_empty() { "127.0.0.1" } else { self.proxy_host.trim() };
                let port = if self.proxy_port == 0 { 1080 } else { self.proxy_port };
                Some(format!("socks5h://{}:{}", host, port))
            }
            NetworkProxyMode::CustomHttp => {
                if let Some(custom) = &self.custom_proxy_url {
                    if !custom.trim().is_empty() {
                        let trimmed = custom.trim();
                        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
                            return Some(format!("http://{}", trimmed));
                        }
                        return Some(trimmed.to_string());
                    }
                }
                let host = if self.proxy_host.trim().is_empty() { "127.0.0.1" } else { self.proxy_host.trim() };
                let port = if self.proxy_port == 0 { 8080 } else { self.proxy_port };
                Some(format!("http://{}:{}", host, port))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPreview {
    Full,
    Sender,
    #[default]
    None,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PresenceVisibility {
    Everyone,
    ContactsOnly,
    Nobody,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for AppSettings {
    fn default() -> Self {
        // Privacy-first defaults
        Self {
            presence_visibility: PresenceVisibility::ContactsOnly,
            show_read_receipts: false,
            show_typing_indicator: true,
            auto_download_media: false,
            link_previews: false,
            notification_preview: NotificationPreview::None,
            telemetry_enabled: false,
            local_ai_enabled: false,
            discord_bridge_enabled: false,
            show_join_date: false,
            network_privacy: NetworkPrivacySettings::default(),
            theme: Theme::Dark,
            font_size: 14,
            reduce_motion: false,
            compact_mode: false,
            accent_color: None,
            amoled_mode: false,
            preset_theme_id: "veil-origin".to_string(),
            custom_theme_name: "Kişisel Tema".to_string(),
            custom_css: String::new(),
            custom_css_enabled: false,
            custom_bg_image: String::new(),
            custom_bg_video: String::new(),
            custom_bg_opacity: 0.26,
            saved_themes: String::new(),
            desktop_notifications: true,
            notification_sound: true,
            mention_only: false,
            notification_volume: Some(80),
            sound_messages: true,
            sound_mentions: true,
            sound_friends: true,
            sound_calls: true,
            dnd_suppress_notifications: true,
            input_device_id: None,
            output_device_id: None,
            video_device_id: None,
            noise_suppression: true,
            echo_cancellation: true,
            push_to_talk: false,
            push_to_talk_key: None,
            mirror_camera: true,
            start_on_login: false,
            minimize_to_tray: true,
            close_to_tray: true,
            hardware_acceleration: true,
            language: "tr".to_string(),
            auto_update_check: true,
            auto_unlock: false,
            dm_privacy: "everyone".to_string(),
        }
    }
}

impl AppState {
    pub fn new(app: AppHandle) -> Result<Self> {
        let data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        let db = Database::open(&data_dir.join("veilanon.db"))?;
        db.run_migrations()?;
        let keystore = KeyStore::new(&data_dir)?;
        let mut network = NetworkManager::new();
        let settings = AppSettings::load(&data_dir).unwrap_or_default();
        if let Some(proxy_url) = settings.network_privacy.get_effective_proxy_url() {
            let _ = network.api.apply_proxy(Some(&proxy_url));
        }

        Ok(Self {
            app,
            db: Arc::new(RwLock::new(db)),
            keystore: Arc::new(keystore),
            network: Arc::new(RwLock::new(network)),
            identity: Arc::new(RwLock::new(None)),
            device_identity: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(settings)),
            db_key: Arc::new(RwLock::new(None)),
            failed_attempts: AtomicU32::new(0),
            supabase_refresh_token: Arc::new(RwLock::new(None)),
            role_members_modified: Arc::new(RwLock::new(HashMap::new())),
            spaces_modified: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Current DB encryption key (authenticated sessions only)
    pub async fn get_db_key(&self) -> Option<[u8; 32]> {
        *self.db_key.read().await
    }

    /// Get the current authenticated identity, or restore from local SQLite database if available
    pub async fn get_or_restore_identity(&self) -> Option<Identity> {
        let current = self.identity.read().await.clone();
        if current.is_some() {
            return current;
        }

        // Only restore if keystore actually holds identity key material
        if !self.keystore.has_identity() {
            return None;
        }

        // Fallback: restore from local_identity table in SQLite
        let restored = {
            let db = self.db.read().await;
            db.query_row(
                "SELECT id, username, display_name, avatar_hash, banner_hash, dh_public_key, signing_public_key, device_id FROM local_identity LIMIT 1",
                [],
                |r| {
                    let id_str: String = r.get(0)?;
                    let id = uuid::Uuid::parse_str(&id_str).map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
                    let username: String = r.get(1)?;
                    let display_name: String = r.get(2)?;
                    let avatar_hash: Option<String> = r.get(3)?;
                    let banner_hash: Option<String> = r.get(4)?;
                    let identity_key_public: String = r.get(5)?;
                    let signing_key_public: String = r.get(6)?;
                    let device_id_str: String = r.get(7)?;
                    let device_id = uuid::Uuid::parse_str(&device_id_str).map_err(|e| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(e)))?;
                    Ok(Identity {
                        id,
                        username,
                        display_name,
                        avatar_hash,
                        banner_hash,
                        identity_key_public,
                        signing_key_public,
                        created_at: chrono::Utc::now(),
                        device_id,
                    })
                },
            ).ok()
        };

        if let Some(ref identity) = restored {
            *self.identity.write().await = Some(identity.clone());
        }

        if self.db_key.read().await.is_none() {
            if let Some(passphrase) = self.keystore.load_auto_unlock() {
                if let Ok(keys) = self.keystore.load_keys(&passphrase) {
                    *self.db_key.write().await = Some(keys.db_key);
                    *self.device_identity.write().await = Some(DeviceIdentity::from_bytes(keys.dh_private, keys.signing_private));
                }
            }
        }

        restored
    }

    /// Set the DB encryption key in memory
    pub async fn set_db_key(&self, key: [u8; 32]) {
        *self.db_key.write().await = Some(key);
    }

    /// Clear and zeroize the DB encryption key (sign out)
    pub async fn clear_db_key(&self) {
        let mut guard = self.db_key.write().await;
        if let Some(mut key) = guard.take() {
            key.zeroize();
        }
    }
}

impl AppSettings {
    fn load(data_dir: &std::path::Path) -> Option<Self> {
        let path = data_dir.join("settings.json");
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, data_dir: &std::path::Path) -> Result<()> {
        let path = data_dir.join("settings.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_preview_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&NotificationPreview::None).unwrap(),
            "\"none\""
        );
        assert_eq!(
            serde_json::to_string(&NotificationPreview::Full).unwrap(),
            "\"full\""
        );
        assert_eq!(
            serde_json::to_string(&NotificationPreview::Sender).unwrap(),
            "\"sender\""
        );
    }

    #[test]
    fn notification_preview_deserializes_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<NotificationPreview>("\"none\"").unwrap(),
            NotificationPreview::None
        );
        assert_eq!(
            serde_json::from_str::<NotificationPreview>("\"full\"").unwrap(),
            NotificationPreview::Full
        );
        assert_eq!(
            serde_json::from_str::<NotificationPreview>("\"sender\"").unwrap(),
            NotificationPreview::Sender
        );
    }

    #[test]
    fn presence_visibility_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&PresenceVisibility::ContactsOnly).unwrap(),
            "\"contacts_only\""
        );
    }

    #[test]
    fn app_settings_defaults_are_privacy_first() {
        let settings = AppSettings::default();
        assert_eq!(settings.notification_preview, NotificationPreview::None);
        assert!(settings.accent_color.is_none());
    }

    #[test]
    fn app_settings_json_round_trip() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, decoded);
    }

    #[test]
    fn app_settings_round_trip_with_accent_color() {
        let settings = AppSettings {
            accent_color: Some("#7c3aed".to_string()),
            ..AppSettings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.accent_color.as_deref(), Some("#7c3aed"));
        assert_eq!(settings, decoded);
    }

    #[test]
    fn app_settings_partial_deserialize_uses_defaults() {
        let json = r##"{"notificationPreview":"sender","accentColor":"#7c3aed"}"##;
        let settings: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.notification_preview, NotificationPreview::Sender);
        assert_eq!(settings.accent_color.as_deref(), Some("#7c3aed"));
        assert_eq!(settings.presence_visibility, PresenceVisibility::ContactsOnly);
        assert!(!settings.link_previews);
        assert_eq!(settings.network_privacy.mode, NetworkProxyMode::Direct);
    }

    #[test]
    fn network_privacy_effective_proxy_url_computation() {
        let tor_settings = NetworkPrivacySettings {
            mode: NetworkProxyMode::Tor,
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 9050,
            ..NetworkPrivacySettings::default()
        };
        assert_eq!(
            tor_settings.get_effective_proxy_url(),
            Some("socks5h://127.0.0.1:9050".to_string())
        );

        let custom_socks = NetworkPrivacySettings {
            mode: NetworkProxyMode::CustomSocks,
            custom_proxy_url: Some("socks5h://10.0.0.1:1080".to_string()),
            ..NetworkPrivacySettings::default()
        };
        assert_eq!(
            custom_socks.get_effective_proxy_url(),
            Some("socks5h://10.0.0.1:1080".to_string())
        );

        let direct = NetworkPrivacySettings {
            mode: NetworkProxyMode::Direct,
            ..NetworkPrivacySettings::default()
        };
        assert_eq!(direct.get_effective_proxy_url(), None);
    }

    #[test]
    fn app_settings_theme_fields_default_and_roundtrip() {
        let mut settings = AppSettings::default();
        assert_eq!(settings.preset_theme_id, "veil-origin");
        assert_eq!(settings.custom_theme_name, "Kişisel Tema");
        assert_eq!(settings.custom_css, "");
        assert!(!settings.custom_css_enabled);
        assert_eq!(settings.custom_bg_opacity, 0.26);

        settings.preset_theme_id = "obsidian-cyan".to_string();
        settings.custom_css = ":root { --veil-brand: #00ffff; }".to_string();
        settings.custom_css_enabled = true;
        settings.custom_bg_image = "https://example.com/bg.jpg".to_string();
        settings.custom_bg_opacity = 0.40;

        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.preset_theme_id, "obsidian-cyan");
        assert_eq!(decoded.custom_css, ":root { --veil-brand: #00ffff; }");
        assert!(decoded.custom_css_enabled);
        assert_eq!(decoded.custom_bg_image, "https://example.com/bg.jpg");
        assert_eq!(decoded.custom_bg_opacity, 0.40);
        assert_eq!(settings, decoded);
    }

    #[test]
    fn app_settings_legacy_json_without_theme_fields() {
        let legacy_json = r#"{"notificationPreview":"none","fontSize":16}"#;
        let decoded: AppSettings = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(decoded.font_size, 16);
        assert_eq!(decoded.preset_theme_id, "veil-origin");
        assert_eq!(decoded.custom_theme_name, "Kişisel Tema");
        assert_eq!(decoded.custom_css, "");
        assert!(!decoded.custom_css_enabled);
        assert_eq!(decoded.custom_bg_opacity, 0.26);
    }

    #[test]
    fn app_settings_all_theme_css_fields_default_empty() {
        let settings = AppSettings::default();
        assert!(settings.custom_css.is_empty(), "custom_css should default to empty string");
        assert!(settings.custom_bg_image.is_empty(), "custom_bg_image should default to empty");
        assert!(settings.custom_bg_video.is_empty(), "custom_bg_video should default to empty");
        assert_eq!(settings.custom_bg_opacity, 0.26, "custom_bg_opacity should default to 0.26");
        assert!(!settings.custom_css_enabled, "custom_css_enabled should default to false");
        assert_eq!(settings.preset_theme_id.as_str(), "veil-origin", "preset should default to veil-origin");
    }
}

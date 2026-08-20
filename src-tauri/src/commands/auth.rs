//! Authentication IPC commands
//! 
//! SECURITY: Passphrase never logged. Private keys never returned via IPC.
//! Only public keys and non-sensitive status information cross the IPC boundary.

use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{debug, info, warn};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::state::AppState;
use crate::config;
use crate::error::{VeilError, VeilResult};
use crate::crypto::identity::{fingerprint_for_keys, DeviceIdentity, DevicePublicIdentity};
use crate::crypto::keystore::{format_recovery_code, parse_recovery_code, DecryptedKeyBundle};
use crate::models::user::Identity;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIdentityInput {
    pub username: String,
    pub display_name: String,
    /// Passphrase — used for key derivation, NEVER stored
    pub passphrase: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCredentialsInput {
    pub username: String,
    pub passphrase: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileInput {
    pub display_name: String,
    pub username: Option<String>,
    pub avatar_hash: Option<String>,
    pub clear_avatar: Option<bool>,
    /// "Hakkımda" metni - yerel DB'de şifreli saklanır.
    pub bio: Option<String>,
    pub banner_hash: Option<String>,
    pub clear_banner: Option<bool>,
    pub custom_status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub banner_hash: Option<String>,
    pub device_id: String,
    pub public_key: DevicePublicIdentity,
    pub recovery_code: Option<String>,
    pub custom_status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    /// Host computer name — what the user actually calls this machine.
    pub name: String,
    /// Human-friendly OS label, e.g. "Windows 11".
    pub os: String,
    pub app_version: String,
    pub identity_fingerprint: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub device_id: String,
    pub name: String,
    pub platform: String,
    pub last_active_at: i64,
    pub is_current: bool,
}

/// Real host computer name (COMPUTERNAME on Windows, hostname elsewhere).
pub(crate) fn host_name() -> String {
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        if !name.trim().is_empty() {
            return name;
        }
    }
    sysinfo::System::host_name().unwrap_or_else(|| "Bu cihaz".to_string())
}

/// Human-friendly OS label ("Windows 11", "macOS 15", "Linux").
pub(crate) fn host_os_name() -> String {
    let base = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    };
    let version = sysinfo::System::os_version().unwrap_or_default();
    let version = version.trim();
    if version.is_empty() || version == base {
        base.to_string()
    } else {
        format!("{base} {version}")
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityHint {
    pub has_identity: bool,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_hash: Option<String>,
}

/// Failed passphrase attempts allowed before lockout. Argon2id already imposes
/// a persistent cost per attempt; this adds an in-memory ceiling.
const MAX_FAILED_ATTEMPTS: u32 = 10;

fn check_attempts(state: &AppState) -> Result<(), VeilError> {
    if state.failed_attempts.load(Ordering::Relaxed) >= MAX_FAILED_ATTEMPTS {
        return Err(VeilError::RateLimitError);
    }
    Ok(())
}

fn record_failure(state: &AppState) {
    state.failed_attempts.fetch_add(1, Ordering::Relaxed);
}

fn reset_attempts(state: &AppState) {
    state.failed_attempts.store(0, Ordering::Relaxed);
}

/// Create a new identity (first-time setup or clean reset)
/// Returns the recovery code ONCE — user must store it safely
#[tauri::command]
pub async fn create_identity(
    input: CreateIdentityInput,
    state: State<'_, AppState>,
) -> Result<IdentityResponse, VeilError> {
    let clean_username = input.username.trim().to_lowercase();
    if clean_username.len() < 2 || clean_username.len() > 32 {
        return Err(VeilError::InvalidInput("Kullanıcı adı 2 ile 32 karakter arasında olmalıdır.".into()));
    }
    if input.passphrase.len() < 8 {
        return Err(VeilError::InvalidInput("Parola en az 8 karakter olmalıdır.".into()));
    }

    // Kullanıcı adı benzersizliği kontrolü (Supabase public.users)
    if config::configured("VEILANON_SUPABASE_URL") {
        let rows: Vec<serde_json::Value> = state
            .network
            .read()
            .await
            .api
            .select(
                "users",
                &format!("username=eq.{}", clean_username),
                None,
                Some(1),
            )
            .await
            .unwrap_or_default();
        if !rows.is_empty() {
            return Err(VeilError::InvalidInput(
                "Bu kullanıcı adı başka bir kullanıcı tarafından alınmış. Lütfen farklı bir kullanıcı adı seçin veya 'Mevcut Kimliğimle Giriş Yap' seçeneğini kullanın.".into(),
            ));
        }
    }

    // Deterministik anahtar ve Supabase kimlik türetimi
    let (device_identity, db_key, recovery_entropy, auth_password) =
        crate::crypto::derive_identity_bundle(&clean_username, &input.passphrase)?;
    let (dh_priv, sign_priv) = device_identity.export_private_bytes();
    let public = device_identity.public_identity()?;
    let recovery_code = format_recovery_code(&recovery_entropy);

    let auth_email = format!("{}@user.veilanon.internal", clean_username);
    let mut auth_user_id: Option<Uuid> = None;

    if config::configured("VEILANON_SUPABASE_URL") {
        let auth = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            state.network.read().await.api.sign_up_with_password(&auth_email, &auth_password),
        )
        .await;

        match auth {
            Ok(Ok(auth)) => {
                if let Ok(uid) = Uuid::parse_str(&auth.user.id) {
                    auth_user_id = Some(uid);
                    let mut network = state.network.write().await;
                    network.api.set_access_token(auth.access_token.clone());
                    network.realtime.set_token(Some(auth.access_token));
                    *state.supabase_refresh_token.write().await = Some(auth.refresh_token);
                }
            }
            Ok(Err(_)) => {
                // Eğer GoTrue auth.users katmanında kullanıcı zaten varsa, aynı deterministik parola ile oturum açmayı dene
                let sign_in_res = tokio::time::timeout(
                    std::time::Duration::from_secs(8),
                    state.network.read().await.api.sign_in_with_password(&auth_email, &auth_password),
                )
                .await;

                match sign_in_res {
                    Ok(Ok(auth)) => {
                        if let Ok(uid) = Uuid::parse_str(&auth.user.id) {
                            auth_user_id = Some(uid);
                            let mut network = state.network.write().await;
                            network.api.set_access_token(auth.access_token.clone());
                            network.realtime.set_token(Some(auth.access_token));
                            *state.supabase_refresh_token.write().await = Some(auth.refresh_token);
                        }
                    }
                    _ => {
                        return Err(VeilError::InvalidInput(
                            "Bu kullanıcı adı başka bir kullanıcı tarafından alınmış. Lütfen farklı bir kullanıcı adı seçin veya 'Mevcut Kimliğimle Giriş Yap' seçeneğini kullanın.".into(),
                        ));
                    }
                }
            }
            Err(_) => {
                return Err(VeilError::InvalidInput(
                    "Kayıt sunucusuna bağlanırken zaman aşımı oluştu. Lütfen internet bağlantınızı kontrol edip tekrar deneyin.".into(),
                ));
            }
        }
    }

    // Yeni kimlik anahtarlarını kaydetmeden önce eski anahtarlık kaydını temizle
    let _ = state.keystore.delete_keys();
    let _ = state.keystore.clear_auto_unlock();

    let key_bundle = DecryptedKeyBundle {
        dh_private: dh_priv,
        signing_private: sign_priv,
        db_key,
        recovery_entropy,
    };

    // Save to OS keychain (encrypted with passphrase-derived key)
    state.keystore.save_keys(&input.passphrase, &key_bundle)?;

    // Keep the DB key in memory for column encryption
    state.set_db_key(db_key).await;
    // Keep private device keys in memory for signing + ratchet key agreement
    *state.device_identity.write().await = Some(device_identity);

    let device_id = Uuid::new_v4();
    let identity_id = auth_user_id.unwrap_or_else(|| stable_identity_id(&public.dh_public_key));
    let display_name = if input.display_name.trim().is_empty() {
        clean_username.clone()
    } else {
        input.display_name.trim().to_string()
    };

    let identity = Identity {
        id: identity_id,
        username: clean_username.clone(),
        display_name: display_name.clone(),
        avatar_hash: None,
        banner_hash: None,
        identity_key_public: public.dh_public_key.clone(),
        signing_key_public: public.signing_public_key.clone(),
        created_at: chrono::Utc::now(),
        device_id,
    };

    // Store identity in local DB
    {
        let db = state.db.read().await;
        let _ = db.execute(
            r#"INSERT INTO local_identity
               (id, username, display_name, dh_public_key, signing_public_key, device_id)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(id) DO UPDATE SET
                 username = excluded.username,
                 display_name = excluded.display_name"#,
            rusqlite::params![
                identity.id.to_string(),
                identity.username,
                identity.display_name,
                identity.identity_key_public,
                identity.signing_key_public,
                identity.device_id.to_string(),
            ],
        );
        let _ = db.execute(
            r#"INSERT INTO user_profiles (id, username, display_name, dh_public_key, signing_public_key, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, unixepoch())
               ON CONFLICT(id) DO UPDATE SET
                 username = excluded.username,
                 display_name = excluded.display_name,
                 updated_at = unixepoch()"#,
            rusqlite::params![
                identity.id.to_string(),
                identity.username,
                identity.display_name,
                identity.identity_key_public,
                identity.signing_key_public,
            ],
        );
    }

    // Store identity in app state
    *state.identity.write().await = Some(identity.clone());

    if let Some(token) = state.supabase_refresh_token.read().await.clone() {
        let db = state.db.read().await;
        let _ = db.save_supabase_refresh_token(&identity.id, &token);
    }

    info!("Identity created for user: {}", identity.username);

    bind_control_plane(&state).await;

    Ok(IdentityResponse {
        id: identity.id.to_string(),
        username: identity.username,
        display_name: identity.display_name,
        avatar_hash: None,
        banner_hash: None,
        device_id: identity.device_id.to_string(),
        public_key: public,
        recovery_code: Some(recovery_code),
        custom_status: None,
    })
}

/// Login with existing username + passphrase on any device
#[tauri::command]
pub async fn login_with_credentials(
    input: LoginCredentialsInput,
    state: State<'_, AppState>,
) -> Result<IdentityResponse, VeilError> {
    check_attempts(&state)?;

    let clean_username = input.username.trim().to_lowercase();
    if clean_username.len() < 2 || clean_username.len() > 32 {
        return Err(VeilError::InvalidInput("Kullanıcı adı 2 ile 32 karakter arasında olmalıdır.".into()));
    }
    if input.passphrase.len() < 8 {
        return Err(VeilError::InvalidInput("Parola en az 8 karakter olmalıdır.".into()));
    }

    // 1. Derive deterministic keys and Supabase credentials
    let (device_identity, db_key, recovery_entropy, auth_password) =
        crate::crypto::derive_identity_bundle(&clean_username, &input.passphrase)?;
    let (dh_priv, sign_priv) = device_identity.export_private_bytes();
    let public = device_identity.public_identity()?;

    let key_bundle = DecryptedKeyBundle {
        dh_private: dh_priv,
        signing_private: sign_priv,
        db_key,
        recovery_entropy,
    };

    // 2. Authenticate to Supabase GoTrue with deterministic credentials
    let auth_email = format!("{}@user.veilanon.internal", clean_username);
    let mut auth_user_id: Option<Uuid> = None;
    let mut remote_profile: Option<(String, String, Option<String>, Option<String>)> = None;

    if config::configured("VEILANON_SUPABASE_URL") {
        // Authenticate with GoTrue password
        let auth_res = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            state.network.read().await.api.sign_in_with_password(&auth_email, &auth_password),
        )
        .await;

        match auth_res {
            Ok(Ok(auth)) => {
                if let Ok(uid) = Uuid::parse_str(&auth.user.id) {
                    auth_user_id = Some(uid);
                }
                let mut network = state.network.write().await;
                network.api.set_access_token(auth.access_token.clone());
                network.realtime.set_token(Some(auth.access_token));
                *state.supabase_refresh_token.write().await = Some(auth.refresh_token);
            }
            _ => {
                record_failure(&state);
                return Err(VeilError::InvalidInput(
                    "Kullanıcı adı veya parola hatalı. Lütfen bilgilerinizi kontrol edin veya yeni bir kimlik oluşturun.".into(),
                ));
            }
        }

        // Now with authenticated JWT, fetch user's real profile from Supabase
        if let Some(uid) = &auth_user_id {
            let filter = format!("id=eq.{}", uid);
            let rows: Vec<serde_json::Value> = state
                .network
                .read()
                .await
                .api
                .select("users", &filter, None, Some(1))
                .await
                .unwrap_or_default();

            if let Some(row) = rows.first() {
                let un = row.get("username").and_then(|v| v.as_str()).unwrap_or(&clean_username).to_string();
                let disp = row.get("display_name").and_then(|v| v.as_str()).unwrap_or(&clean_username).to_string();
                let av = row.get("avatar_hash").and_then(|v| v.as_str()).map(str::to_string);
                let ban = row.get("banner_hash").and_then(|v| v.as_str()).map(str::to_string);
                remote_profile = Some((un, disp, av, ban));
            }
        }
    } else {
        if !state.keystore.has_identity() {
            record_failure(&state);
            return Err(VeilError::InvalidInput(
                "Bu cihazda kayıtlı kimlik bulunamadı. Lütfen önce kimlik oluşturun.".into(),
            ));
        }
        state.keystore.load_keys(&input.passphrase)?;
    }

    reset_attempts(&state);

    // Yeni oturum anahtarlarını kaydetmeden önce eski anahtarlık kaydını temizle
    let _ = state.keystore.delete_keys();
    let _ = state.keystore.clear_auto_unlock();

    // 3. Save keys to OS keychain for quick local unlock
    let _ = state.keystore.save_keys(&input.passphrase, &key_bundle);

    // 4. Set state keys
    state.set_db_key(db_key).await;
    *state.device_identity.write().await = Some(device_identity.clone());

    let (final_username, final_display, avatar_hash, banner_hash) = if let Some((u, d, a, b)) = remote_profile {
        (u, d, a, b)
    } else {
        (clean_username.clone(), clean_username.clone(), None, None)
    };

    let identity_id = auth_user_id.unwrap_or_else(|| stable_identity_id(&public.dh_public_key));
    let device_id = Uuid::new_v4();

    let identity = Identity {
        id: identity_id,
        username: final_username.clone(),
        display_name: final_display.clone(),
        avatar_hash: avatar_hash.clone(),
        banner_hash: banner_hash.clone(),
        identity_key_public: public.dh_public_key.clone(),
        signing_key_public: public.signing_public_key.clone(),
        created_at: chrono::Utc::now(),
        device_id,
    };

    // Store in local DB
    {
        let db = state.db.read().await;
        let _ = db.execute(
            r#"INSERT INTO local_identity
               (id, username, display_name, dh_public_key, signing_public_key, device_id, avatar_hash, banner_hash)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
               ON CONFLICT(id) DO UPDATE SET
                 username = excluded.username,
                 display_name = excluded.display_name,
                 avatar_hash = COALESCE(excluded.avatar_hash, local_identity.avatar_hash),
                 banner_hash = COALESCE(excluded.banner_hash, local_identity.banner_hash)"#,
            rusqlite::params![
                identity.id.to_string(),
                identity.username,
                identity.display_name,
                identity.identity_key_public,
                identity.signing_key_public,
                identity.device_id.to_string(),
                identity.avatar_hash,
                identity.banner_hash,
            ],
        );
        let _ = db.execute(
            r#"INSERT INTO user_profiles (id, username, display_name, avatar_hash, banner_hash, dh_public_key, signing_public_key, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())
               ON CONFLICT(id) DO UPDATE SET
                 username = excluded.username,
                 display_name = excluded.display_name,
                 avatar_hash = COALESCE(excluded.avatar_hash, user_profiles.avatar_hash),
                 banner_hash = COALESCE(excluded.banner_hash, user_profiles.banner_hash),
                 updated_at = unixepoch()"#,
            rusqlite::params![
                identity.id.to_string(),
                identity.username,
                identity.display_name,
                identity.avatar_hash,
                identity.banner_hash,
                identity.identity_key_public,
                identity.signing_key_public,
            ],
        );
    }

    *state.identity.write().await = Some(identity.clone());

    if let Some(token) = state.supabase_refresh_token.read().await.clone() {
        let db = state.db.read().await;
        let _ = db.save_supabase_refresh_token(&identity.id, &token);
    }

    let id_arc = state.identity.clone();
    let db_arc = state.db.clone();
    let net_arc = state.network.clone();
    let tok_arc = state.supabase_refresh_token.clone();
    tokio::spawn(async move {
        bind_control_plane_handles(id_arc, db_arc, net_arc, tok_arc).await;
    });

    info!("Identity logged in with credentials for user: {}", identity.username);

    let custom_status = {
        let db = state.db.read().await;
        db.query_row(
            "SELECT custom_status FROM local_identity WHERE id = ?1",
            rusqlite::params![identity.id.to_string()],
            |r| r.get::<_, Option<String>>(0),
        ).ok().flatten()
    };

    Ok(IdentityResponse {
        id: identity.id.to_string(),
        username: identity.username,
        display_name: identity.display_name,
        avatar_hash,
        banner_hash,
        device_id: identity.device_id.to_string(),
        public_key: public,
        recovery_code: None,
        custom_status,
    })
}

/// Update the local identity's display name / avatar hash (profile edit).
/// No key material is touched; only non-sensitive profile metadata.
#[tauri::command]
pub async fn update_profile(
    input: UpdateProfileInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let display_name = input.display_name.trim();
    if display_name.is_empty() || display_name.chars().count() > 32 {
        return Err(VeilError::InvalidInput(
            "Display name must be 1-32 characters".into(),
        ));
    }
    if let Some(hash) = &input.avatar_hash {
        if hash.len() > 512 {
            return Err(VeilError::InvalidInput("Avatar hash too long".into()));
        }
    }

    let (identity_id, old_username, old_avatar, old_banner) = {
        let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;
        (
            identity.id,
            identity.username.clone(),
            identity.avatar_hash.clone(),
            identity.banner_hash.clone(),
        )
    };

    let target_avatar = if input.clear_avatar.unwrap_or(false) {
        None
    } else if input.avatar_hash.is_some() {
        input.avatar_hash.clone()
    } else {
        old_avatar
    };

    let target_banner = if input.clear_banner.unwrap_or(false) {
        None
    } else if input.banner_hash.is_some() {
        input.banner_hash.clone()
    } else {
        old_banner
    };

    let target_username = if let Some(un) = &input.username {
        let u = un.trim().to_lowercase();
        if u.len() < 3 || u.len() > 32 || !u.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(VeilError::InvalidInput("Kullanıcı adı 3-32 karakter olmalı (harf, rakam, _, -)".into()));
        }
        // Unique check if changed
        if u != old_username.to_lowercase() && config::configured("VEILANON_SUPABASE_URL") {
            if let Ok(network) = state.network.try_read() {
                if let Ok(users) = network.api.select::<serde_json::Value>(
                    "users",
                    &format!("username=eq.{}", u),
                    None,
                    Some(10),
                ).await {
                    for row in users {
                        if let Some(owner_id) = row.get("id").and_then(|v| v.as_str()) {
                            if owner_id != identity_id.to_string() {
                                return Err(VeilError::InvalidInput("Bu kullanıcı adı başka bir kullanıcı tarafından alınmış".into()));
                            }
                        }
                    }
                }
            }
        }
        u
    } else {
        old_username
    };

    {
        let db = state.db.read().await;
        db.update_local_identity(&identity_id, display_name, target_avatar.as_deref())?;
        if input.username.is_some() {
            db.set_local_identity_username(&identity_id, &target_username)?;
        }
        db.set_local_identity_banner(&identity_id, target_banner.as_deref())?;
        if let Some(cs) = &input.custom_status {
            let clean_cs = cs.trim();
            let cs_opt = if clean_cs.is_empty() { None } else { Some(clean_cs) };
            db.update_custom_status(&identity_id, cs_opt)?;
        }
        // "Hakkımda" metni yerel profilde şifreli saklanır (DB anahtarıyla).
        if let Some(bio) = &input.bio {
            if bio.chars().count() > 200 {
                return Err(VeilError::InvalidInput("Bio must be 1-200 characters".into()));
            }
            db.update_local_bio(&identity_id, Some(bio.trim()))?;
            let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;
            let (ct, nonce) = crate::crypto::encrypt_aes_gcm(&db_key, bio.as_bytes())?;
            let mut payload = ct;
            payload.extend_from_slice(&nonce);
            db.update_profile_bio(&identity_id, Some(&B64.encode(&payload)))?;
        }
    }

    {
        let mut guard = state.identity.write().await;
        if let Some(identity) = guard.as_mut() {
            identity.display_name = display_name.to_string();
            identity.username = target_username.clone();
            identity.avatar_hash = target_avatar.clone();
            identity.banner_hash = target_banner.clone();
        }
    }

    // Mirror the updated profile to the control plane so friends on other
    // devices see the new name (best-effort, never fails the command).
    if config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(network) = state.network.try_read() {
            let mut payload = serde_json::json!({
                "id": identity_id.to_string(),
                "username": target_username,
                "display_name": display_name,
                "avatar_hash": target_avatar.as_deref(),
                "banner_hash": target_banner.as_deref(),
            });
            if let Some(ref cs) = input.custom_status {
                let clean_cs = cs.trim();
                if clean_cs.is_empty() {
                    payload["custom_status"] = serde_json::Value::Null;
                } else {
                    payload["custom_status"] = serde_json::json!(clean_cs);
                }
            }
            if let Some(ref b) = input.bio {
                payload["bio"] = serde_json::json!(b.trim());
            }
            let _ = network
                .api
                .upsert(
                    "users",
                    &payload,
                    "id",
                )
                .await;

            if let Some(ref cs) = input.custom_status {
                let clean_cs = cs.trim();
                let pres_payload = if clean_cs.is_empty() {
                    serde_json::json!({
                        "user_id": identity_id.to_string(),
                        "custom_status": serde_json::Value::Null,
                    })
                } else {
                    serde_json::json!({
                        "user_id": identity_id.to_string(),
                        "custom_status": clean_cs,
                    })
                };
                let _ = network.api.upsert("presence", &pres_payload, "user_id").await;
            }
        }
    }

    let _ = state.app.emit("user:updated", serde_json::json!({ "userId": identity_id.to_string() }));
    let _ = state.app.emit("presence:changed", serde_json::json!({ "userId": identity_id.to_string() }));
    info!("Profile updated");
    Ok(())
}

/// Check if a username is available globally (unique check)
#[tauri::command]
pub async fn check_username_available(
    username: String,
    state: State<'_, AppState>,
) -> Result<bool, VeilError> {
    let u = username.trim().to_lowercase();
    if u.len() < 3 || u.len() > 32 || !u.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Ok(false);
    }

    let my_info = {
        let guard = state.get_or_restore_identity().await;
        guard.map(|id| (id.id.to_string(), id.username.to_lowercase()))
    };

    if let Some((_, current_un)) = &my_info {
        if current_un == &u {
            return Ok(true);
        }
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(network) = state.network.try_read() {
            if let Ok(users) = network.api.select::<serde_json::Value>(
                "users",
                &format!("username=eq.{}", u),
                None,
                Some(10),
            ).await {
                for row in users {
                    if let Some(owner_id) = row.get("id").and_then(|v| v.as_str()) {
                        if let Some((my_id, _)) = &my_info {
                            if owner_id == my_id {
                                continue;
                            }
                        }
                        return Ok(false);
                    }
                }
            }
        }
    }

    Ok(true)
}

/// Load existing identity from keychain
#[tauri::command]
pub async fn load_identity(
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<IdentityResponse, VeilError> {
    debug!("load_identity: attempts check");
    check_attempts(&state)?;

    debug!("load_identity: loading keys");
    let keys = match state.keystore.load_keys(&passphrase) {
        Ok(keys) => keys,
        Err(_) => {
            // Deterministik HKDF yedeği: Cihaz tuzu/anahtarlık değişmiş veya hesap başka cihazda oluşturulmuş olsa bile,
            // yerel veritabanındaki kullanıcı adı ve açık anahtarla eşleşiyorsa parolanın doğruluğunu teyit et ve anahtarları yükle.
            let fallback_bundle = {
                let db = state.db.read().await;
                let un_opt: Option<(String, String)> = db.query_row(
                    "SELECT username, dh_public_key FROM local_identity LIMIT 1",
                    [],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                ).ok().or_else(|| {
                    db.query_row(
                        "SELECT username, dh_public_key FROM user_profiles LIMIT 1",
                        [],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                    ).ok()
                });

                if let Some((un, stored_pub_key)) = un_opt {
                    let clean_un = un.trim().to_lowercase();
                    if let Ok((dev, db_k, rec_entropy, _)) = crate::crypto::derive_identity_bundle(&clean_un, &passphrase) {
                        if let Ok(pub_id) = dev.public_identity() {
                            if pub_id.dh_public_key == stored_pub_key {
                                let (dh_p, sign_p) = dev.export_private_bytes();
                                let bundle = DecryptedKeyBundle {
                                    dh_private: dh_p,
                                    signing_private: sign_p,
                                    db_key: db_k,
                                    recovery_entropy: rec_entropy,
                                };
                                let _ = state.keystore.save_keys(&passphrase, &bundle);
                                Some(bundle)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(bundle) = fallback_bundle {
                bundle
            } else {
                record_failure(&state);
                return Err(VeilError::InvalidPassphrase);
            }
        }
    };
    reset_attempts(&state);

    debug!("load_identity: setting db key");
    // Keep the DB key in memory for column encryption; bundle zeroizes on drop
    state.set_db_key(keys.db_key).await;

    debug!("load_identity: rebuilding device identity");
    // Reconstruct identity from stored keys
    let device_identity = DeviceIdentity::from_bytes(keys.dh_private, keys.signing_private);
    let public = device_identity.public_identity()?;
    let device_id = Uuid::new_v4();

    debug!("load_identity: reading identity row");
    // Load identity record from DB (rebuilds a stable row if it went missing).
    let db = state.db.read().await;
    let (id, username, display_name, avatar_hash, banner_hash, device_id) =
        load_or_rebuild_identity_row(&db, &public, &device_id)?;
    drop(db);

    // Supabase sunucu doğrulaması: Hesap sunucudan silinmiş mi kontrol et
    if config::configured("VEILANON_SUPABASE_URL") {
        let clean_user = username.trim().to_lowercase();
        let auth_email = format!("{}@user.veilanon.internal", clean_user);
        let (_, _, _, auth_password) = crate::crypto::derive_identity_bundle(&clean_user, &passphrase)?;

        let auth_check = tokio::time::timeout(
            std::time::Duration::from_secs(6),
            state.network.read().await.api.sign_in_with_password(&auth_email, &auth_password),
        )
        .await;

        match auth_check {
            Ok(Ok(auth)) => {
                let mut network = state.network.write().await;
                network.api.set_access_token(auth.access_token.clone());
                network.realtime.set_token(Some(auth.access_token));
                *state.supabase_refresh_token.write().await = Some(auth.refresh_token.clone());
                let db = state.db.read().await;
                let _ = db.save_supabase_refresh_token(&id, &auth.refresh_token);
            }
            Ok(Err(e)) => {
                // Sunucu yanıt verdi fakat hesap bulunamadı / kimlik bilgisi geçersiz (veritabanı sıfırlanmış / kullanıcı silinmiş):
                // Yerel yetim veriyi temizle ve IdentityNotFound döndürerek tertemiz onboarding ekranına yönlendir!
                if matches!(e, VeilError::InvalidInput(_) | VeilError::Unauthenticated | VeilError::PermissionDenied) {
                    warn!("load_identity: account {} no longer exists on Supabase, wiping stale local data", username);
                    let _ = state.keystore.clear_auto_unlock();
                    let _ = state.keystore.delete_keys();
                    state.clear_db_key().await;
                    *state.identity.write().await = None;
                    *state.device_identity.write().await = None;
                    state.network.write().await.api.clear_token();
                    {
                        let db = state.db.read().await;
                        for table in crate::db::USER_TABLES {
                            let sql = format!("DELETE FROM {}", table);
                            let _ = db.execute(&sql, []);
                        }
                    }
                    return Err(VeilError::IdentityNotFound);
                }
                warn!("load_identity: server sign-in check failed ({:?}), proceeding with local verified identity for {}", e, username);
            }
            Err(_) => {
                // Zaman aşımı / çevrimdışı: yerel modda devam et
                warn!("load_identity: server check timed out, proceeding in offline mode for {}", username);
            }
        }
    }

    debug!("load_identity: setting identity state");
    let identity = crate::models::user::Identity {
        id,
        username: username.clone(),
        display_name: display_name.clone(),
        avatar_hash: avatar_hash.clone(),
        banner_hash: banner_hash.clone(),
        identity_key_public: public.dh_public_key.clone(),
        signing_key_public: public.signing_public_key.clone(),
        created_at: chrono::Utc::now(),
        device_id,
    };
    *state.identity.write().await = Some(identity);
    *state.device_identity.write().await = Some(device_identity);

    debug!("load_identity: binding control plane in background");
    let id_arc = state.identity.clone();
    let db_arc = state.db.clone();
    let net_arc = state.network.clone();
    let tok_arc = state.supabase_refresh_token.clone();
    tokio::spawn(async move {
        bind_control_plane_handles(id_arc, db_arc, net_arc, tok_arc).await;
    });

    info!("Identity loaded for user: {}", username);

    let custom_status = {
        let db = state.db.read().await;
        db.query_row(
            "SELECT custom_status FROM local_identity WHERE id = ?1",
            rusqlite::params![id.to_string()],
            |r| r.get::<_, Option<String>>(0),
        ).ok().flatten()
    };

    Ok(IdentityResponse {
        id: id.to_string(),
        username,
        display_name,
        avatar_hash,
        banner_hash,
        device_id: device_id.to_string(),
        public_key: public,
        recovery_code: None,
        custom_status,
    })
}

/// Attempt to auto-unlock the identity using the saved OS keychain token.
#[tauri::command]
pub async fn try_auto_unlock(state: State<'_, AppState>) -> Result<Option<IdentityResponse>, VeilError> {
    if let Some(passphrase) = state.keystore.load_auto_unlock() {
        debug!("try_auto_unlock: found saved passphrase in OS keychain, attempting unlock");
        match load_identity(passphrase, state.clone()).await {
            Ok(resp) => Ok(Some(resp)),
            Err(e) => {
                warn!("try_auto_unlock: failed to unlock with saved token: {:?}", e);
                let _ = state.keystore.clear_auto_unlock();
                if matches!(e, VeilError::IdentityNotFound | VeilError::InvalidPassphrase) {
                    let _ = state.keystore.delete_keys();
                }
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// Enable or disable auto-unlock by saving/clearing the session passphrase in the OS keychain.
#[tauri::command]
pub async fn set_auto_unlock(
    enabled: bool,
    passphrase: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    if enabled {
        let pass = passphrase.ok_or_else(|| VeilError::InvalidInput("Passphrase required to enable auto-unlock".into()))?;
        // Verify passphrase first
        let _ = state.keystore.load_keys(&pass)?;
        state.keystore.save_auto_unlock(&pass)?;
        info!("Auto-unlock enabled and saved");
    } else {
        state.keystore.clear_auto_unlock()?;
        info!("Auto-unlock disabled and cleared");
    }
    Ok(())
}

/// Check whether auto-unlock is configured on this device.
#[tauri::command]
pub async fn has_auto_unlock(state: State<'_, AppState>) -> Result<bool, VeilError> {
    Ok(state.keystore.has_auto_unlock())
}

/// Recover an identity with the recovery code and set a NEW passphrase.
/// Supports both local keychain recovery and cross-device recovery with username.
#[tauri::command]
pub async fn recover_identity(
    recovery_code: String,
    new_passphrase: String,
    username: Option<String>,
    state: State<'_, AppState>,
) -> Result<IdentityResponse, VeilError> {
    check_attempts(&state)?;

    if new_passphrase.len() < 8 {
        return Err(VeilError::InvalidInput("Yeni parola en az 8 karakter olmalıdır.".into()));
    }

    let supplied = parse_recovery_code(&recovery_code)?;
    if supplied.len() != 24 {
        return Err(VeilError::InvalidRecoveryCode);
    }
    let mut entropy = [0u8; 24];
    entropy.copy_from_slice(&supplied);

    let (keys, new_recovery_code) = if let Some(un) = &username {
        let clean_un = un.trim().to_lowercase();
        if clean_un.len() < 2 || clean_un.len() > 32 {
            return Err(VeilError::InvalidInput("Kullanıcı adı 2 ile 32 karakter arasında olmalıdır.".into()));
        }

        if config::configured("VEILANON_SUPABASE_URL") {
            let (_dev_id, _db_key, auth_password) =
                crate::crypto::derive_identity_bundle_from_recovery(&clean_un, &entropy)?;

            let auth_email = format!("{}@user.veilanon.internal", clean_un);
            let auth_res = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                state.network.read().await.api.sign_in_with_password(&auth_email, &auth_password),
            )
            .await;

            match auth_res {
                Ok(Ok(auth)) => {
                    let mut network = state.network.write().await;
                    network.api.set_access_token(auth.access_token.clone());
                    network.realtime.set_token(Some(auth.access_token));
                    *state.supabase_refresh_token.write().await = Some(auth.refresh_token);
                }
                _ => {
                    record_failure(&state);
                    return Err(VeilError::InvalidInput(
                        "Kurtarma kodu veya kullanıcı adı hatalı. Lütfen bilgilerinizi kontrol edip tekrar deneyin.".into(),
                    ));
                }
            }

            // Derive the NEW identity bundle from the new passphrase
            let (new_dev, new_db_k, new_rec_entropy, new_auth_password) =
                crate::crypto::derive_identity_bundle(&clean_un, &new_passphrase)?;

            // Update GoTrue password on Supabase with the new auth password so cross-device login works
            let _ = state.network.read().await.api.update_user_password(&new_auth_password).await;

            let (dh_priv, sign_priv) = new_dev.export_private_bytes();
            let bundle = DecryptedKeyBundle {
                dh_private: dh_priv,
                signing_private: sign_priv,
                db_key: new_db_k,
                recovery_entropy: new_rec_entropy,
            };
            state.keystore.save_keys(&new_passphrase, &bundle)?;
            (bundle, Some(format_recovery_code(&new_rec_entropy)))
        } else if state.keystore.has_identity() {
            let k = state.keystore.recover_keys(&recovery_code, &new_passphrase)?;
            let db = state.db.read().await;
            let local_un: Option<String> = db
                .query_row("SELECT username FROM local_identity LIMIT 1", [], |r| r.get(0))
                .ok();
            if let Some(lun) = local_un {
                if lun.to_lowercase() != clean_un {
                    record_failure(&state);
                    return Err(VeilError::InvalidInput(
                        "Girilen kurtarma kodu ve kullanıcı adı bu cihazdaki kayıtlı kimlikle uyuşmuyor.".into(),
                    ));
                }
            }
            (k, None)
        } else {
            record_failure(&state);
            return Err(VeilError::InvalidRecoveryCode);
        }
    } else if state.keystore.has_identity() {
        match state.keystore.recover_keys(&recovery_code, &new_passphrase) {
            Ok(k) => (k, None),
            Err(_) => {
                record_failure(&state);
                return Err(VeilError::InvalidRecoveryCode);
            }
        }
    } else {
        record_failure(&state);
        return Err(VeilError::InvalidInput(
            "Farklı bir cihazdaki veya sıfırlanmış hesabı kurtarmak için lütfen kullanıcı adınızı girin.".into(),
        ));
    };

    reset_attempts(&state);

    // Keep the DB key in memory for column encryption; bundle zeroizes on drop
    state.set_db_key(keys.db_key).await;

    // Reconstruct identity from recovered keys
    let device_identity = DeviceIdentity::from_bytes(keys.dh_private, keys.signing_private);
    let public = device_identity.public_identity()?;
    let device_id = Uuid::new_v4();

    // Load identity record from DB (rebuilds a stable row if it went missing).
    let db = state.db.read().await;
    let (id, un, display_name, avatar_hash, banner_hash, device_id) =
        load_or_rebuild_identity_row(&db, &public, &device_id)?;
    drop(db);

    let final_username = username.unwrap_or(un);
    let identity = crate::models::user::Identity {
        id,
        username: final_username.clone(),
        display_name: display_name.clone(),
        avatar_hash: avatar_hash.clone(),
        banner_hash: banner_hash.clone(),
        identity_key_public: public.dh_public_key.clone(),
        signing_key_public: public.signing_public_key.clone(),
        created_at: chrono::Utc::now(),
        device_id,
    };
    *state.identity.write().await = Some(identity);
    *state.device_identity.write().await = Some(device_identity);

    let id_arc = state.identity.clone();
    let db_arc = state.db.clone();
    let net_arc = state.network.clone();
    let tok_arc = state.supabase_refresh_token.clone();
    tokio::spawn(async move {
        bind_control_plane_handles(id_arc, db_arc, net_arc, tok_arc).await;
    });

    info!("Identity recovered for user: {}", final_username);

    Ok(IdentityResponse {
        id: id.to_string(),
        username: final_username,
        display_name,
        avatar_hash,
        banner_hash,
        device_id: device_id.to_string(),
        public_key: public,
        recovery_code: new_recovery_code,
        custom_status: None,
    })
}

// ── Control plane binding ───────────────────────────────────────────────────

/// Connects this device to the Supabase control plane.
/// Uses a refresh token if available; otherwise registers a new
/// anonymous user and persists its refresh token.
pub async fn bind_control_plane_handles(
    identity_arc: std::sync::Arc<tokio::sync::RwLock<Option<crate::models::Identity>>>,
    db_arc: std::sync::Arc<tokio::sync::RwLock<crate::db::Database>>,
    network_arc: std::sync::Arc<tokio::sync::RwLock<crate::network::NetworkManager>>,
    supabase_refresh_token_arc: std::sync::Arc<tokio::sync::RwLock<Option<String>>>,
) {
    use tracing::warn;
    let identity = {
        let guard = identity_arc.read().await;
        guard.clone()
    };
    let Some(identity) = identity else { return };

    // Read the lock only for the network call, then release it BEFORE taking
    // the write lock below — holding a read guard across a write acquisition
    // deadlocks tokio's RwLock.
    let stored_token = {
        let db = db_arc.read().await;
        db.supabase_refresh_token(&identity.id).ok().flatten()
    };

    let _clean_user = identity.username.trim().to_lowercase();
    let _auth_email = format!("{}@user.veilanon.internal", _clean_user);

    let auth = match stored_token {
        Some(refresh_token) => {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(8),
                network_arc.read().await.api.refresh_access_token(&refresh_token),
            )
            .await;
            match result {
                Ok(Ok(auth)) => Ok(auth),
                _ => {
                    warn!("Refresh token rejected or timed out");
                    Err(VeilError::Unauthenticated)
                }
            }
        }
        None => {
            Err(VeilError::Unauthenticated)
        }
    };

    match auth {
        Ok(auth) => {
            let auth_uid = Uuid::parse_str(&auth.user.id).ok();
            {
                let mut network = network_arc.write().await;
                network.api.set_access_token(auth.access_token.clone());
                network.realtime.set_token(Some(auth.access_token));
            }
            // Refresh rotates the refresh token too — keep the stored one current.
            let fresh_token = auth.refresh_token;
            *supabase_refresh_token_arc.write().await = Some(fresh_token.clone());
            {
                let db = db_arc.read().await;
                let _ = db.save_supabase_refresh_token(&identity.id, &fresh_token);
            }
            // Register the user row (id must equal auth.uid for RLS) and the
            // device's public keys (E2EE key distribution registry).
            let network = network_arc.read().await;
            let target_user_id = auth_uid.map(|u| u.to_string()).unwrap_or_else(|| identity.id.to_string());

            // If Supabase already has profile data, don't overwrite with defaults
            if let Ok(rows) = network.api.select::<serde_json::Value>(
                "users",
                &format!("id=eq.{}", target_user_id),
                None,
                Some(1),
            ).await {
                if let Some(row) = rows.first() {
                    let remote_disp = row.get("display_name").and_then(|v| v.as_str());
                    let remote_avatar = row.get("avatar_hash").and_then(|v| v.as_str());
                    let remote_banner = row.get("banner_hash").and_then(|v| v.as_str());
                    if let Some(disp) = remote_disp {
                        if !disp.is_empty() && (identity.display_name.is_empty() || identity.display_name.starts_with("kullanici-")) {
                            let db = db_arc.read().await;
                            let _ = db.update_local_identity(&identity.id, disp, remote_avatar);
                            if let Some(b) = remote_banner {
                                let _ = db.set_local_identity_banner(&identity.id, Some(b));
                            }
                        }
                    }
                }
            }

            // Read banner_hash, bio, custom_status from local_identity to avoid
            // erasing them on every login bind.
            let (local_banner, local_bio, local_custom_status) = {
                let db = db_arc.read().await;
                let row: Option<(Option<String>, Option<String>, Option<String>)> = db.query_row(
                    "SELECT banner_hash, bio, custom_status FROM local_identity WHERE id = ?1",
                    rusqlite::params![identity.id.to_string()],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                ).ok();
                row.unwrap_or((None, None, None))
            };
            let _ = network
                .api
                .upsert(
                    "users",
                    &serde_json::json!({
                        "id": target_user_id,
                        "username": identity.username,
                        "display_name": identity.display_name,
                        "avatar_hash": identity.avatar_hash.clone().unwrap_or_default(),
                        "banner_hash": local_banner.as_deref().unwrap_or_default(),
                        "bio": local_bio.as_deref().unwrap_or_default(),
                        "custom_status": local_custom_status.as_deref().unwrap_or_default(),
                    }),
                    "id",
                )
                .await;
            let _ = network
                .api
                .upsert(
                    "devices",
                    &serde_json::json!({
                        "id": identity.device_id.to_string(),
                        "user_id": target_user_id,
                        "public_key": identity.identity_key_public,
                        "signing_public_key": identity.signing_key_public,
                        "name": host_name(),
                    }),
                    "id",
                )
                .await;
            let _ = network
                .api
                .upsert(
                    "presence",
                    &serde_json::json!({
                        "user_id": target_user_id,
                        "status": "online",
                    }),
                    "user_id",
                )
                .await;
            network.realtime.broadcast(serde_json::json!({
                "type": "presence",
                "user_id": target_user_id,
                "status": "online",
            }));
            info!("Control plane bound for {}", identity.username);
        }
        Err(e) => {
            warn!("Failed to bind control plane: {:?}", e);
        }
    }
}

pub async fn bind_control_plane(state: &AppState) {
    bind_control_plane_handles(
        state.identity.clone(),
        state.db.clone(),
        state.network.clone(),
        state.supabase_refresh_token.clone(),
    )
    .await;
}

/// Verify the user's passphrase (for sensitive operations)
#[tauri::command]
pub async fn verify_passphrase(
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<bool, VeilError> {
    check_attempts(&state)?;
    let mut verified = state.keystore.load_keys(&passphrase).is_ok();

    // Fallback: deterministic verification via username + passphrase
    if !verified {
        if let Some(identity) = state.get_or_restore_identity().await {
            let clean_un = identity.username.trim().to_lowercase();
            if let Ok((dev, db_key, rec_entropy, _)) = crate::crypto::derive_identity_bundle(&clean_un, &passphrase) {
                if let Ok(pub_id) = dev.public_identity() {
                    if pub_id.dh_public_key == identity.identity_key_public {
                        verified = true;
                        // Re-sync keystore bundle for future unlocks
                        let (dh_priv, sign_priv) = dev.export_private_bytes();
                        let bundle = DecryptedKeyBundle {
                            dh_private: dh_priv,
                            signing_private: sign_priv,
                            db_key,
                            recovery_entropy: rec_entropy,
                        };
                        let _ = state.keystore.save_keys(&passphrase, &bundle);
                    }
                }
            }
        }
    }

    if verified {
        reset_attempts(&state);
    } else {
        record_failure(&state);
        warn!("Passphrase verification failed");
    }
    Ok(verified)
}

/// Factory-reset the device identity WITHOUT requiring the passphrase.
///
/// Escape hatch for a locked device whose passphrase and recovery code are
/// both lost (e.g. a forgotten QA identity). Irreversible: keychain entries,
/// the local database and settings are wiped so onboarding starts fresh.
/// The UI must confirm twice — the command cannot be called accidentally.
#[tauri::command]
pub async fn reset_identity(state: State<'_, AppState>) -> Result<(), VeilError> {
    reset_attempts(&state);
    *state.identity.write().await = None;
    *state.device_identity.write().await = None;
    state.network.write().await.api.clear_token();
    *state.supabase_refresh_token.write().await = None;
    state.clear_db_key().await;
    state.keystore.clear_cached_secrets();
    let _ = state.keystore.clear_auto_unlock();
    let _ = state.keystore.delete_keys();

    {
        let db = state.db.read().await;
        for table in crate::db::USER_TABLES {
            let sql = format!("DELETE FROM {}", table);
            let _ = db.execute(&sql, []);
        }
    }

    info!("Identity reset — device wiped, onboarding will start fresh");
    Ok(())
}

/// Sign out (clear in-memory state, keep local DB)
#[tauri::command]
pub async fn sign_out(state: State<'_, AppState>) -> Result<(), VeilError> {
    reset_attempts(&state);
    *state.identity.write().await = None;
    *state.device_identity.write().await = None;
    state.network.write().await.api.clear_token();
    state.keystore.clear_cached_secrets();
    state.clear_db_key().await;
    info!("User signed out");
    Ok(())
}

/// Non-sensitive identity metadata for the pre-auth unlock screen.
/// Returns None fields when no local identity exists — no key material exposed.
#[tauri::command]
pub async fn get_identity_hint(state: State<'_, AppState>) -> Result<IdentityHint, VeilError> {
    if !state.keystore.has_identity() {
        return Ok(IdentityHint {
            has_identity: false,
            username: None,
            display_name: None,
            avatar_hash: None,
        });
    }

    let db = state.db.read().await;
    let row = db
        .query_row(
            "SELECT username, display_name, avatar_hash FROM local_identity LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .ok();

    match row {
        Some((username, display_name, avatar_hash)) if !username.trim().is_empty() => {
            Ok(IdentityHint {
                has_identity: true,
                username: Some(username),
                display_name: Some(display_name),
                avatar_hash,
            })
        }
        _ => {
            let _ = state.keystore.delete_keys();
            let _ = state.keystore.clear_auto_unlock();
            Ok(IdentityHint {
                has_identity: false,
                username: None,
                display_name: None,
                avatar_hash: None,
            })
        }
    }
}

/// Stable identity id derived from the DH public key, used to rebuild the
/// local_identity row when it went missing (broken local state recovery).
fn stable_identity_id(dh_public_hex: &str) -> Uuid {
    use ring::digest;
    let digest = digest::digest(&digest::SHA256, dh_public_hex.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_ref()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50; // version 5
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant
    Uuid::from_bytes(bytes)
}

/// Fetch the local identity row, rebuilding it from the recovered keys when
/// it is missing so the unlock flow never dead-ends on a half-written DB.
fn load_or_rebuild_identity_row(
    db: &crate::db::Database,
    public: &DevicePublicIdentity,
    device_id: &Uuid,
) -> VeilResult<(Uuid, String, String, Option<String>, Option<String>, Uuid)> {
    let row = db
        .query_row(
            "SELECT id, username, display_name, avatar_hash, banner_hash, device_id FROM local_identity LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .ok();

    if let Some((id, username, display_name, avatar_hash, banner_hash, stored_device)) = row {
        let parsed_id = Uuid::parse_str(&id).unwrap_or_else(|_| stable_identity_id(&public.dh_public_key));
        let _ = db.execute(
            r#"INSERT INTO user_profiles (id, username, display_name, avatar_hash, banner_hash, dh_public_key, signing_public_key, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())
               ON CONFLICT(id) DO UPDATE SET
                 username = excluded.username,
                 display_name = excluded.display_name,
                 avatar_hash = COALESCE(excluded.avatar_hash, user_profiles.avatar_hash),
                 banner_hash = COALESCE(excluded.banner_hash, user_profiles.banner_hash),
                 updated_at = unixepoch()"#,
            rusqlite::params![
                parsed_id.to_string(),
                username,
                display_name,
                avatar_hash,
                banner_hash,
                public.dh_public_key,
                public.signing_public_key,
            ],
        );
        return Ok((
            parsed_id,
            username,
            display_name,
            avatar_hash,
            banner_hash,
            Uuid::parse_str(&stored_device).unwrap_or(*device_id),
        ));
    }

    // Check user_profiles in local DB
    let prof_row = db
        .query_row(
            "SELECT id, username, display_name, avatar_hash, banner_hash FROM user_profiles WHERE dh_public_key = ?1 LIMIT 1",
            rusqlite::params![public.dh_public_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .ok();

    if let Some((id, username, display_name, avatar_hash, banner_hash)) = prof_row {
        let parsed_id = Uuid::parse_str(&id).unwrap_or_else(|_| stable_identity_id(&public.dh_public_key));
        let _ = db.execute(
            r#"INSERT INTO local_identity
               (id, username, display_name, dh_public_key, signing_public_key, device_id, avatar_hash, banner_hash)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            rusqlite::params![
                parsed_id.to_string(),
                username,
                display_name,
                public.dh_public_key,
                public.signing_public_key,
                device_id.to_string(),
                avatar_hash,
                banner_hash,
            ],
        );
        return Ok((parsed_id, username, display_name, avatar_hash, banner_hash, *device_id));
    }

    let id = stable_identity_id(&public.dh_public_key);
    let id_str = id.to_string();
    let short = id_str.split('-').next().unwrap_or("kullanici");
    let fallback_name = format!("kullanici-{short}");
    db.execute(
        r#"INSERT INTO local_identity
           (id, username, display_name, dh_public_key, signing_public_key, device_id)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        rusqlite::params![
            id.to_string(),
            fallback_name,
            fallback_name,
            public.dh_public_key,
            public.signing_public_key,
            device_id.to_string(),
        ],
    )?;
    let _ = db.execute(
        r#"INSERT INTO user_profiles (id, username, display_name, avatar_hash, banner_hash, dh_public_key, signing_public_key, updated_at)
           VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, unixepoch())
           ON CONFLICT(id) DO UPDATE SET
             username = excluded.username,
             display_name = excluded.display_name,
             updated_at = unixepoch()"#,
        rusqlite::params![
            id.to_string(),
            fallback_name,
            fallback_name,
            public.dh_public_key,
            public.signing_public_key,
        ],
    );
    info!("Local identity row rebuilt from keychain");
    Ok((id, fallback_name.clone(), fallback_name, None, None, *device_id))
}

/// Get the recovery code (requires passphrase re-verification)
#[tauri::command]
pub async fn get_recovery_code(
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    if let Ok(keys) = state.keystore.load_keys(&passphrase) {
        return Ok(format_recovery_code(&keys.recovery_entropy));
    }

    let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;
    let clean_un = identity.username.trim().to_lowercase();
    let (dev, db_key, rec_entropy, _) = crate::crypto::derive_identity_bundle(&clean_un, &passphrase)
        .map_err(|_| VeilError::InvalidPassphrase)?;
    let pub_id = dev.public_identity().map_err(|_| VeilError::InvalidPassphrase)?;
    if pub_id.dh_public_key != identity.identity_key_public {
        return Err(VeilError::InvalidPassphrase);
    }
    let (dh_priv, sign_priv) = dev.export_private_bytes();
    let bundle = DecryptedKeyBundle {
        dh_private: dh_priv,
        signing_private: sign_priv,
        db_key,
        recovery_entropy: rec_entropy,
    };
    let _ = state.keystore.save_keys(&passphrase, &bundle);
    Ok(format_recovery_code(&rec_entropy))
}

/// Verify a recovery code (constant-time comparison against stored entropy
/// when authenticated; pure format validation otherwise — recover_identity is
/// the authoritative check on the unauthenticated path).
#[tauri::command]
pub async fn verify_recovery_code(
    code: String,
    state: State<'_, AppState>,
) -> Result<bool, VeilError> {
    let supplied = parse_recovery_code(&code)?;

    if let Some(expected) = state.keystore.recovery_entropy() {
        // Task-mandated ring constant-time comparison (API deprecated upstream;
        // length is fixed at 24 bytes so timing variance is negligible anyway).
        #[allow(deprecated)]
        {
            return Ok(ring::constant_time::verify_slices_are_equal(&expected, &supplied).is_ok());
        }
    }

    // Not authenticated: cannot verify entropy — only recover_identity can
    // authoritatively check via cryptographic key derivation.
    Ok(false)
}

/// Get current device information
#[tauri::command]
pub async fn get_device_info(state: State<'_, AppState>) -> Result<DeviceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let fingerprint = fingerprint_for_keys(
        &identity.identity_key_public,
        &identity.signing_key_public,
    );

    Ok(DeviceInfo {
        id: identity.device_id.to_string(),
        name: host_name(),
        os: host_os_name(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        identity_fingerprint: fingerprint,
    })
}

/// Pick an image file and register it as the local profile avatar.
/// The image is copied into the app data dir; the returned hash is stored as
/// `avatarHash` via update_profile. Only the local filename is stored — the
fn resolve_image_bytes(path_or_data: &str) -> Result<(Vec<u8>, String), VeilError> {
    if path_or_data.starts_with("data:image/") {
        let (header, b64) = path_or_data
            .split_once(',')
            .ok_or_else(|| VeilError::InvalidInput("Geçersiz data URL".into()))?;
        let ext = if header.contains("png") {
            "png"
        } else if header.contains("webp") {
            "webp"
        } else if header.contains("gif") {
            "gif"
        } else {
            "jpg"
        };
        let bytes = B64
            .decode(b64)
            .map_err(|_| VeilError::InvalidInput("Geçersiz base64 görsel verisi".into()))?;
        Ok((bytes, ext.to_string()))
    } else {
        let ext = std::path::Path::new(path_or_data)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp"))
            .ok_or_else(|| VeilError::InvalidInput("Desteklenen formatlar: PNG, JPG, GIF, WEBP".into()))?;
        let bytes = std::fs::read(path_or_data)
            .map_err(|e| VeilError::FileError(e))?;
        Ok((bytes, ext))
    }
}

/// Pick an image file (or base64 data URL) and register it as the local user's avatar.
#[tauri::command]
pub async fn set_avatar(path: String, state: State<'_, AppState>) -> Result<String, VeilError> {
    let (identity_id, username, display_name) = {
        let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;
        (identity.id, identity.username.clone(), identity.display_name.clone())
    };

    let (bytes, ext) = resolve_image_bytes(&path)?;

    let data_dir = state
        .app
        .path()
        .app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let avatars_dir = data_dir.join("avatars");
    std::fs::create_dir_all(&avatars_dir)?;

    let hash = format!("local-{}.{}", Uuid::new_v4(), ext);
    let dest = avatars_dir.join(&hash);
    std::fs::write(&dest, &bytes)?;

    {
        let db = state.db.read().await;
        db.update_local_identity(&identity_id, &display_name, Some(&hash))?;
        let _ = db.upsert_profile(&identity_id, &username, &display_name, Some(&hash), None, None, None, None, None);
    }
    {
        let mut guard = state.identity.write().await;
        if let Some(identity) = guard.as_mut() {
            identity.avatar_hash = Some(hash.clone());
        }
    }

    // Kontrol düzlemine yansıt (best-effort).
    if config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(network) = state.network.try_read() {
            let _ = network
                .api
                .upload_blob(&format!("avatars/{}", hash), bytes.clone())
                .await;
            let _ = network
                .api
                .upsert(
                    "users",
                    &serde_json::json!({
                        "id": identity_id.to_string(),
                        "username": username,
                        "display_name": display_name,
                        "avatar_hash": hash,
                    }),
                    "id",
                )
                .await;
        }
    }

    let _ = state.app.emit("user:updated", serde_json::json!({ "userId": identity_id.to_string() }));
    info!("Avatar updated");
    Ok(hash)
}

/// Pick an image file (or base64 data URL) and register it as the local profile banner.
#[tauri::command]
pub async fn set_banner(path: String, state: State<'_, AppState>) -> Result<String, VeilError> {
    let identity_id = {
        let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;
        identity.id
    };

    let (bytes, ext) = resolve_image_bytes(&path)?;

    let data_dir = state
        .app
        .path()
        .app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let avatars_dir = data_dir.join("avatars");
    std::fs::create_dir_all(&avatars_dir)?;

    let hash = format!("local-{}.{}", Uuid::new_v4(), ext);
    let dest = avatars_dir.join(&hash);
    std::fs::write(&dest, &bytes)?;

    {
        let db = state.db.read().await;
        db.set_local_identity_banner(&identity_id, Some(&hash))?;
        let _ = db.set_user_profile_banner(&identity_id, Some(&hash));
    }
    let (username, display_name) = {
        let mut guard = state.identity.write().await;
        if let Some(identity) = guard.as_mut() {
            identity.banner_hash = Some(hash.clone());
            (identity.username.clone(), identity.display_name.clone())
        } else {
            (String::new(), String::new())
        }
    };

    // Mirror to Supabase control plane so other users see the new banner
    if config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(network) = state.network.try_read() {
            let _ = network
                .api
                .upload_blob(&format!("banners/{}", hash), bytes.clone())
                .await;
            let _ = network
                .api
                .upload_blob(&format!("avatars/{}", hash), bytes.clone())
                .await;
            let _ = network
                .api
                .upsert(
                    "users",
                    &serde_json::json!({
                        "id": identity_id.to_string(),
                        "username": username,
                        "display_name": display_name,
                        "banner_hash": hash,
                    }),
                    "id",
                )
                .await;
        }
    }

    let _ = state.app.emit("user:updated", serde_json::json!({ "userId": identity_id.to_string() }));
    info!("Banner updated");
    Ok(hash)
}

/// Read a locally stored avatar as a data URL (works for `local-*` hashes).
/// If not available locally, downloads and caches the blob from Supabase storage.
#[tauri::command]
pub async fn get_avatar(hash: String, state: State<'_, AppState>) -> Result<String, VeilError> {
    let clean = hash.trim().trim_start_matches('/').trim_start_matches('\\');
    if clean.is_empty() || clean.contains("..") {
        return Err(VeilError::InvalidInput("Invalid avatar identifier".into()));
    }
    let safe_hash = std::path::Path::new(clean)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(clean);
    let data_dir = state
        .app
        .path()
        .app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let avatars_dir = data_dir.join("avatars");
    let path = avatars_dir.join(safe_hash);

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => {
            // Not found locally — try to fetch from Supabase storage (banners / avatars / files)
            let mut fetched = None;
            if config::configured("VEILANON_SUPABASE_URL") {
                let network = state.network.read().await;
                if let Ok(remote_bytes) = network.api.download_blob(&format!("banners/{}", safe_hash)).await {
                    let _ = std::fs::create_dir_all(&avatars_dir);
                    let _ = std::fs::write(&path, &remote_bytes);
                    fetched = Some(remote_bytes);
                } else if let Ok(remote_bytes) = network.api.download_blob(&format!("avatars/{}", safe_hash)).await {
                    let _ = std::fs::create_dir_all(&avatars_dir);
                    let _ = std::fs::write(&path, &remote_bytes);
                    fetched = Some(remote_bytes);
                } else if let Ok(remote_bytes) = network.api.download_blob(&format!("files/avatars/{}", safe_hash)).await {
                    let _ = std::fs::create_dir_all(&avatars_dir);
                    let _ = std::fs::write(&path, &remote_bytes);
                    fetched = Some(remote_bytes);
                }
            }
            fetched.ok_or_else(|| VeilError::InvalidInput("Avatar not found".into()))?
        }
    };

    let mime = match std::path::Path::new(safe_hash)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
    {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png",
    };
    Ok(format!("data:{mime};base64,{}", B64.encode(&bytes)))
}

/// List the user's verified devices (from the control-plane registry).
/// Falls back to the local device when Supabase isn't configured.
#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionInfo>, VeilError> {
    let (my_id, my_device) = {
        let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;
        (identity.id, identity.device_id)
    };

    let mut sessions = Vec::new();
    if config::configured("VEILANON_SUPABASE_URL") {
        // Control-plane isteğini kısa zaman aşımıyla sar: ağ yavaşsa ya da
        // network kilidi meşgulse komut asla asılı kalmaz, yerel cihaz
        // fallback'i döner. Kilit edinimi de timeout kapsamındadır.
        let rows: Vec<serde_json::Value> = tokio::time::timeout(
            std::time::Duration::from_secs(7),
            async {
                let network = state.network.read().await;
                let filter = format!("user_id=eq.{}&select=id,name,created_at", my_id);
                network
                    .api
                    .select("devices", &filter, Some("created_at.asc"), Some(100))
                    .await
            },
        )
        .await
        .unwrap_or_else(|_| Ok(Vec::new()))
        .unwrap_or_default();
        for row in rows {
                let Some(device_id) = row.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .filter(|n| !n.is_empty() && *n != "windows" && *n != "macos" && *n != "linux")
                    .map(str::to_string)
                    .unwrap_or_else(host_name);
                let created = row
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.timestamp())
                    .unwrap_or_else(|| chrono::Utc::now().timestamp());
                sessions.push(SessionInfo {
                    device_id: device_id.to_string(),
                    name,
                    platform: host_os_name(),
                    last_active_at: created,
                    is_current: Uuid::parse_str(device_id)
                        .map(|id| id == my_device)
                        .unwrap_or(false),
                });
            }
    }

    if sessions.is_empty() {
        sessions.push(SessionInfo {
            device_id: my_device.to_string(),
            name: host_name(),
            platform: host_os_name(),
            last_active_at: chrono::Utc::now().timestamp(),
            is_current: true,
        });
    }

    Ok(sessions)
}

/// Revoke a device: deletes its row from the control-plane registry so it can
/// no longer sync or receive new message envelopes.
#[tauri::command]
pub async fn revoke_session(device_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;

    let target = Uuid::parse_str(&device_id)
        .map_err(|_| VeilError::InvalidInput("Invalid device ID".into()))?;
    if target == identity.device_id {
        return Err(VeilError::InvalidInput("Cannot revoke the current device".into()));
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(network) = state.network.try_read() {
            network
                .api
                .delete("devices", &format!("id=eq.{}", device_id))
                .await?;
        }
    }

    info!("Session revoked"); // device_id intentionally not logged
    Ok(())
}



//! MLS grup E2EE (RFC 9420) komutları.
//!
//! Akış: sahip kanalı oluştururken grup oturumu başlatır; her üye kendi
//! KeyPackage'ini üretir, sahibe gönderir; sahip Welcome üretir ve üyenin
//! X25519 ortak anahtarıyla şifreleyip saklar; üye Welcome'ı alıp oturuma
//! katılır. Anahtar malzemesi yalnızca cihazlarda — sunucu hiçbir zaman düz
//! metin veya anahtar görmez.

use tauri::State;
use serde::Deserialize;
use uuid::Uuid;
use tracing::info;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::crypto::group::MlsGroupSession;
use crate::error::{VeilError, VeilResult};
use crate::state::AppState;
use crate::config;

const MLS_CIPHERSUITE: openmls_traits::types::Ciphersuite = openmls_traits::types::Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

fn parse_channel_id(s: &str) -> VeilResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))
}

fn parse_user_id(s: &str) -> VeilResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))
}

/// Oturumu DB'den yükle (çöz) veya yoksa hata döndür.
pub(crate) async fn load_session(state: &AppState, channel_id: &Uuid) -> VeilResult<MlsGroupSession> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;
    let db = state.db.read().await;
    let blob = db
        .query_row(
            "SELECT session_blob FROM mls_sessions WHERE channel_id = ?1",
            rusqlite::params![channel_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .ok();
    drop(db);
    let Some(blob) = blob else {
        return Err(VeilError::InvalidInput(
            "Bu kanal için MLS oturumu yok (sahip oturumu başlatmamış)".into(),
        ));
    };
    let payload = B64
        .decode(blob)
        .map_err(|_| VeilError::SerializationError)?;
    if payload.len() < 12 {
        return Err(VeilError::SerializationError);
    }
    let split = payload.len() - 12;
    let plain = crate::crypto::decrypt_aes_gcm(&db_key, &payload[..split], &payload[split..])?;
    MlsGroupSession::deserialize(&plain)
}

pub(crate) async fn save_session(state: &AppState, channel_id: &Uuid, session: &MlsGroupSession) -> VeilResult<()> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;
    let plain = session.serialize()?;
    let (ct, nonce) = crate::crypto::encrypt_aes_gcm(&db_key, &plain)?;
    let mut payload = ct;
    payload.extend_from_slice(&nonce);
    let db = state.db.read().await;
    db.execute(
        "INSERT INTO mls_sessions (channel_id, session_blob, updated_at) VALUES (?1, ?2, unixepoch())
         ON CONFLICT(channel_id) DO UPDATE SET session_blob = ?2, updated_at = unixepoch()",
        rusqlite::params![channel_id.to_string(), B64.encode(&payload)],
    )?;
    Ok(())
}

/// Kanal sahibi/oluşturan tarafından çağrılır: grup oturumunu başlatır.
#[tauri::command]
pub async fn mls_init_channel(channel_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&channel_id)?;

    let session = MlsGroupSession::create_with_group_id(
        MLS_CIPHERSUITE,
        "veilanon",
        channel_id.as_bytes(),
    )?;
    save_session(&state, &channel_id, &session).await?;
    info!("MLS session initialized");
    Ok(())
}

/// Üye: kendi KeyPackage'ini üretir — (key_package_b64, signer_priv_b64, storage).
#[tauri::command]
pub async fn mls_create_key_package(_channel_id: String, state: State<'_, AppState>) -> Result<serde_json::Value, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let (kp, signer, storage) = MlsGroupSession::create_key_package(
        MLS_CIPHERSUITE,
        &identity.id.to_string(),
    )?;
    Ok(serde_json::json!({
        "keyPackage": B64.encode(&kp),
        "signerPrivate": B64.encode(&signer),
        "storage": storage,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MlsAddMemberInput {
    pub channel_id: String,
    pub user_id: String,
    pub key_package: String,
}

/// Sahip: üyenin KeyPackage'ini gruba ekler; Welcome'ı üyenin DH anahtarıyla
/// şifreleyip saklar (yerel + kontrol düzlemine best-effort).
#[tauri::command]
pub async fn mls_add_member(input: MlsAddMemberInput, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&input.channel_id)?;
    let member_id = parse_user_id(&input.user_id)?;
    if member_id == identity.id {
        return Err(VeilError::InvalidInput("Kendin eklenemezsin".into()));
    }

    let kp_bytes = B64
        .decode(&input.key_package)
        .map_err(|_| VeilError::InvalidInput("Invalid key package encoding".into()))?;

    let mut session = load_session(&state, &channel_id).await?;
    let welcome = session.add_key_package(&kp_bytes)?;
    save_session(&state, &channel_id, &session).await?;

    // Üyenin X25519 anahtarıyla şifrele (Welcome + oturum deposu üye imza anahtarı içermediğinden
    // üye kendi anahtarını zaten tutar; Welcome'ı şifrelemek yeterli).
    let member_dh = {
        let db = state.db.read().await;
        db.get_profile_dh_public(&member_id)?
    }
    .ok_or_else(|| VeilError::InvalidInput("Üyenin genel anahtarı bulunamadı".into()))?;

    let device = state
        .device_identity
        .read()
        .await
        .clone()
        .ok_or(VeilError::Unauthenticated)?;
    let shared = device.dh_agree(&member_dh)?;
    let (ct, nonce) = crate::crypto::encrypt_aes_gcm(&shared, &welcome)?;
    let mut payload = ct;
    payload.extend_from_slice(&nonce);
    let envelope = B64.encode(&payload);

    {
        let db = state.db.read().await;
        db.execute(
            "INSERT INTO mls_welcomes (channel_id, user_id, envelope, created_at) VALUES (?1, ?2, ?3, unixepoch())
             ON CONFLICT(channel_id, user_id) DO UPDATE SET envelope = ?3, created_at = unixepoch()",
            rusqlite::params![channel_id.to_string(), member_id.to_string(), envelope],
        )?;
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        if let Ok(network) = state.network.try_read() {
            let _ = network
                .api
                .upsert(
                    "mls_welcomes",
                    &serde_json::json!({
                        "channel_id": channel_id.to_string(),
                        "user_id": member_id.to_string(),
                        "envelope": envelope,
                    }),
                    "channel_id,user_id",
                )
                .await;
        }
    }

    info!("MLS member added");
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // key_package part of the IPC contract; consumed by the owner
pub struct MlsJoinInput {
    pub channel_id: String,
    pub key_package: String,
    pub signer_private: String,
    pub storage: Option<Vec<(String, String)>>,
}

/// Üye: Welcome'ı (yerel veya uzak) alır, çözer, oturuma katılır.
#[tauri::command]
pub async fn mls_consume_welcome(input: MlsJoinInput, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&input.channel_id)?;

    // 1) Yerel Welcome satırını ara; yoksa kontrol düzleminden çek.
    let envelope: Option<String> = {
        let db = state.db.read().await;
        let row = db
            .query_row(
                "SELECT envelope FROM mls_welcomes WHERE channel_id = ?1 AND user_id = ?2",
                rusqlite::params![channel_id.to_string(), identity.id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .ok();
        row
    };
    let envelope = match envelope {
        Some(e) => Some(e),
        None => {
            if config::configured("VEILANON_SUPABASE_URL") {
                let rows: Vec<serde_json::Value> = state
                    .network
                    .read()
                    .await
                    .api
                    .select(
                        "mls_welcomes",
                        &format!("channel_id=eq.{}&user_id=eq.{}", channel_id, identity.id),
                        None,
                        Some(1),
                    )
                    .await
                    .unwrap_or_default();
                rows.first().and_then(|r| r.get("envelope")).and_then(|v| v.as_str()).map(str::to_string)
            } else {
                None
            }
        }
    };
    let Some(envelope) = envelope else {
        return Err(VeilError::InvalidInput(
            "Bu kanal için Welcome bulunamadı — sahip seni E2EE kanalına eklememiş.".into(),
        ));
    };

    // 2) X25519 ile çöz: sahibin genel anahtarıyla ortak sır türet, AES-GCM çöz.
    let owner_dh = {
        let db = state.db.read().await;
        let space_id: Option<String> = db
            .query_row(
                "SELECT space_id FROM channels WHERE id = ?1",
                rusqlite::params![channel_id.to_string()],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        let Some(space_id) = space_id else {
            return Err(VeilError::InvalidInput("Kanal bulunamadı".into()));
        };
        let owner_id: Option<String> = db
            .query_row(
                "SELECT owner_id FROM spaces WHERE id = ?1",
                rusqlite::params![space_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        let Some(owner_id) = owner_id else {
            return Err(VeilError::InvalidInput("Topluluk bulunamadı".into()));
        };
        db.get_profile_dh_public(
            &Uuid::parse_str(&owner_id).unwrap_or_else(|_| Uuid::nil()),
        )?
    }
    .ok_or_else(|| VeilError::InvalidInput("Sahibin genel anahtarı bulunamadı".into()))?;

    let payload = B64
        .decode(envelope)
        .map_err(|_| VeilError::DecryptionError)?;
    if payload.len() < 12 {
        return Err(VeilError::DecryptionError);
    }
    let split = payload.len() - 12;
    let device = state
        .device_identity
        .read()
        .await
        .clone()
        .ok_or(VeilError::Unauthenticated)?;
    let shared = device.dh_agree(&owner_dh)?;
    let plain = crate::crypto::decrypt_aes_gcm(&shared, &payload[..split], &payload[split..])
        .map_err(|_| VeilError::DecryptionError)?;

    // 3) Oturuma katıl ve sakla.
    let signer_bytes = B64
        .decode(&input.signer_private)
        .map_err(|_| VeilError::InvalidInput("Invalid signer key".into()))?;
    let member_storage = input.storage.as_deref().unwrap_or(&[]);
    let session = MlsGroupSession::join_from_welcome(
        &plain,
        &signer_bytes,
        member_storage,
        &identity.display_name,
    )?;
    save_session(&state, &channel_id, &session).await?;
    {
        let db = state.db.read().await;
        let _ = db.execute(
            "DELETE FROM mls_welcomes WHERE channel_id = ?1 AND user_id = ?2",
            rusqlite::params![channel_id.to_string(), identity.id.to_string()],
        );
    }
    info!("MLS welcome consumed");
    Ok(())
}

/// Sesli kanal E2EE anahtarı: MLS export secret'inden türetilir.
#[tauri::command]
pub async fn mls_call_key(channel_id: String, state: State<'_, AppState>) -> Result<Option<String>, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&channel_id)?;
    let session = load_session(&state, &channel_id).await?;
    let secret = session.export_secret("veilanon-media-e2ee")?;
    Ok(Some(B64.encode(&secret)))
}

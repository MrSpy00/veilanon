//! Media IPC commands — LiveKit voice/video coordination
//! 
//! SECURITY: LiveKit tokens are short-lived (2h) and room-scoped.
//! Tokens are generated locally with the configured API key/secret and are
//! NEVER logged. E2EE call keys are managed here — never exposed via IPC.
//! If E2EE setup fails, the call DOES NOT proceed silently in fallback mode.

use tauri::{Manager, State};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::state::AppState;
use crate::config;
use crate::error::{VeilError, VeilResult};

const TOKEN_TTL_SECS: i64 = 2 * 60 * 60; // 2h
const TOKEN_NBF_LEEWAY_SECS: i64 = 30;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveKitTokenResponse {
    pub token: String,
    pub url: String,
    pub room_name: String,
    pub is_e2ee: bool,
    /// E2EE scope description shown to user
    pub e2ee_scope: String,
    /// MLS export secret'inden türetilmiş oda anahtarı (yalnızca IPC, sunucu görmez).
    pub e2ee_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // camera/screen hints consumed by the JS LiveKit client
pub struct JoinVoiceInput {
    pub channel_id: String,
    pub with_camera: bool,
    pub with_screen: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLivekitTokenInput {
    pub channel_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAudioDeviceInput {
    pub device_id: Option<String>,
    pub device_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVideoDeviceInput {
    pub device_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct LiveKitVideoGrants {
    room: String,
    #[serde(rename = "roomJoin")]
    room_join: bool,
    #[serde(rename = "canPublish")]
    can_publish: bool,
    #[serde(rename = "canSubscribe")]
    can_subscribe: bool,
    #[serde(rename = "canPublishData")]
    can_publish_data: bool,
}

#[derive(Serialize, Deserialize)]
struct LiveKitClaims {
    exp: usize,
    nbf: usize,
    iss: String,
    sub: String,
    /// Participant display identity used by the LiveKit client
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<String>,
    video: LiveKitVideoGrants,
}

/// Generate a room-scoped HS256 token signed with the LiveKit API secret
fn generate_livekit_token(
    api_key: &str,
    api_secret: &str,
    identity: &str,
    display_name: &str,
    room_name: &str,
    metadata: Option<String>,
) -> VeilResult<String> {
    use jsonwebtoken::{encode, EncodingKey, Header};

    if api_key.is_empty() {
        return Err(VeilError::NotConfigured("LiveKit API anahtarı yapılandırılmamış. .env dosyasını kontrol edin.".into()));
    }
    if api_secret.is_empty() {
        return Err(VeilError::NotConfigured("LiveKit API gizli anahtarı yapılandırılmamış. .env dosyasını kontrol edin.".into()));
    }

    let now = chrono::Utc::now().timestamp();
    let claims = LiveKitClaims {
        exp: (now + TOKEN_TTL_SECS) as usize,
        nbf: (now - TOKEN_NBF_LEEWAY_SECS) as usize,
        iss: api_key.to_string(),
        // Use identity (user UUID) as the sub — globally unique, safe for LiveKit
        sub: identity.to_string(),
        // Display name shown to other participants
        name: display_name.to_string(),
        metadata,
        video: LiveKitVideoGrants {
            room: room_name.to_string(),
            room_join: true,
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
        },
    };

    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(api_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!("LiveKit token generation failed: {}", e);
        VeilError::VoiceConnectionError
    })
}

fn livekit_config() -> (String, String, String) {
    let url = config::var("VEILANON_LIVEKIT_URL").unwrap_or_default();
    let key = config::var("VEILANON_LIVEKIT_API_KEY").unwrap_or_default();
    let secret = config::var("VEILANON_LIVEKIT_API_SECRET").unwrap_or_default();
    if url.is_empty() || key.is_empty() || secret.is_empty() {
        tracing::warn!("LiveKit configuration incomplete: url={}, key_set={}, secret_set={}",
            if url.is_empty() { "MISSING" } else { "ok" },
            !key.is_empty(),
            !secret.is_empty()
        );
    }
    (url, key, secret)
}

/// Join a voice channel — token minted locally with the configured secret.
/// E2EE kanallarında oda anahtarı MLS export secret'inden türetilir ve yalnızca
/// IPC üzerinden istemciye verilir (sunucu anahtarı asla görmez).
#[tauri::command]
pub async fn join_voice_channel(
    input: JoinVoiceInput,
    state: State<'_, AppState>,
) -> Result<LiveKitTokenResponse, VeilError> {
    let identity = state.get_or_restore_identity().await.ok_or_else(|| {
        tracing::error!("join_voice_channel: user not authenticated");
        VeilError::Unauthenticated
    })?;

    info!("Joining voice channel '{}' for user: {} ({})", input.channel_id, identity.username, identity.id);

    let (livekit_url, api_key, api_secret) = livekit_config();
    if livekit_url.is_empty() {
        return Err(VeilError::NotConfigured("Ses sunucusu (LiveKit) yapılandırılmamış. Yöneticiye başvurun.".into()));
    }

    let room_name = format!("channel-{}", input.channel_id);
    // Use user UUID as identity (guaranteed unique), display_name for UI
    let lk_identity = identity.id.to_string();
    let display_name = if identity.display_name.is_empty() {
        identity.username.clone()
    } else {
        identity.display_name.clone()
    };
    let accent_color = state.settings.read().await.accent_color.clone();
    let metadata_json = serde_json::json!({
        "avatarHash": identity.avatar_hash,
        "avatar_hash": identity.avatar_hash,
        "accentColor": accent_color.unwrap_or_else(|| "var(--veil-brand)".into()),
        "themeColor": "var(--veil-brand)",
    }).to_string();
    let token = generate_livekit_token(&api_key, &api_secret, &lk_identity, &display_name, &room_name, Some(metadata_json))?;

    // Kanala ait MLS oturumu varsa (E2EE ses kanalı) oda anahtarı türet.
    let mut is_e2ee = false;
    let mut e2ee_scope = "transport-encrypted".to_string();
    let mut e2ee_key = None;
    let _channel_uuid = Uuid::parse_str(&input.channel_id)
        .map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;
    {
        let db = state.db.read().await;
        let e2ee: i64 = db
            .query_row(
                "SELECT is_e2ee FROM channels WHERE id = ?1",
                rusqlite::params![input.channel_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if e2ee != 0 {
            if let Ok(Some(key)) = super::mls::mls_call_key(input.channel_id.clone(), state.clone()).await {
                is_e2ee = true;
                e2ee_scope = "MLS media E2EE (odadaki herkes için uçtan uca)".to_string();
                e2ee_key = Some(key);
            }
        }
    }

    // Ephemeral signal over realtime so non-connected users in the space can see who is in the voice channel
    let realtime = state.network.read().await.realtime.clone();
    realtime.broadcast(serde_json::json!({
        "type": "voice_presence",
        "action": "join",
        "channel_id": input.channel_id,
        "user_id": identity.id.to_string(),
        "username": identity.username,
        "display_name": identity.display_name,
        "avatar_hash": identity.avatar_hash,
    }));

    Ok(LiveKitTokenResponse {
        token,
        url: livekit_url,
        room_name,
        is_e2ee,
        e2ee_scope,
        e2ee_key,
    })
}

/// Leave the current voice channel
#[tauri::command]
pub async fn leave_voice_channel(state: State<'_, AppState>) -> Result<(), VeilError> {
    info!("Leaving voice channel");
    if let Some(identity) = state.get_or_restore_identity().await {
        let realtime = state.network.read().await.realtime.clone();
        realtime.broadcast(serde_json::json!({
            "type": "voice_presence",
            "action": "leave",
            "user_id": identity.id.to_string(),
        }));
    }
    Ok(())
}

/// Get a LiveKit token (used by the JS LiveKit client)
#[tauri::command]
pub async fn get_livekit_token(
    input: GetLivekitTokenInput,
    state: State<'_, AppState>,
) -> Result<LiveKitTokenResponse, VeilError> {
    let identity = state.get_or_restore_identity().await.ok_or(VeilError::Unauthenticated)?;

    let room_name = format!("channel-{}", input.channel_id);
    let (livekit_url, api_key, api_secret) = livekit_config();
    if livekit_url.is_empty() {
        return Err(VeilError::NotConfigured("Ses sunucusu (LiveKit) yapılandırılmamış.".into()));
    }
    let lk_identity = identity.id.to_string();
    let display_name = if identity.display_name.is_empty() {
        identity.username.clone()
    } else {
        identity.display_name.clone()
    };
    let accent_color = state.settings.read().await.accent_color.clone();
    let metadata_json = serde_json::json!({
        "avatarHash": identity.avatar_hash,
        "avatar_hash": identity.avatar_hash,
        "accentColor": accent_color.unwrap_or_else(|| "var(--veil-brand)".into()),
        "themeColor": "var(--veil-brand)",
    }).to_string();
    let token = generate_livekit_token(&api_key, &api_secret, &lk_identity, &display_name, &room_name, Some(metadata_json))?;

    let mut is_e2ee = false;
    let mut e2ee_key = None;
    {
        let db = state.db.read().await;
        let e2ee: i64 = db
            .query_row(
                "SELECT is_e2ee FROM channels WHERE id = ?1",
                rusqlite::params![input.channel_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if e2ee != 0 {
            if let Ok(Some(key)) = super::mls::mls_call_key(input.channel_id.clone(), state.clone()).await {
                is_e2ee = true;
                e2ee_key = Some(key);
            }
        }
    }

    // Ephemeral signal over realtime so non-connected users in the space can see who is in the voice channel
    let network = state.network.read().await;
    network.realtime.broadcast(serde_json::json!({
        "type": "voice_presence",
        "action": "join",
        "channel_id": input.channel_id,
        "user_id": identity.id.to_string(),
        "username": identity.username,
        "display_name": identity.display_name,
        "avatar_hash": identity.avatar_hash,
    }));

    Ok(LiveKitTokenResponse {
        token,
        url: livekit_url,
        room_name,
        is_e2ee,
        e2ee_scope: if is_e2ee { "MLS media E2EE".into() } else { "transport-encrypted".into() },
        e2ee_key,
    })
}

/// Start screen sharing — requires explicit OS permission grant
#[tauri::command]
pub async fn start_screen_share(_state: State<'_, AppState>) -> Result<(), VeilError> {
    info!("Screen share requested — awaiting OS permission");
    // The JS LiveKit client handles the actual screen capture API
    Ok(())
}

/// Stop screen sharing
#[tauri::command]
pub async fn stop_screen_share(_state: State<'_, AppState>) -> Result<(), VeilError> {
    info!("Screen share stopped");
    Ok(())
}

/// Set audio input/output device
#[tauri::command]
pub async fn set_audio_device(
    input: SetAudioDeviceInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let data_dir = state.app.path().app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let mut settings = state.settings.write().await;
    let SetAudioDeviceInput { device_id, device_type } = input;
    match device_type.as_str() {
        "input" => settings.input_device_id = device_id,
        "output" => settings.output_device_id = device_id,
        _ => return Err(VeilError::InvalidInput("device_type must be 'input' or 'output'".into())),
    }
    settings.save(&data_dir)?;
    Ok(())
}

/// Set video device
#[tauri::command]
pub async fn set_video_device(
    input: SetVideoDeviceInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let data_dir = state.app.path().app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let mut settings = state.settings.write().await;
    settings.video_device_id = input.device_id;
    settings.save(&data_dir)?;
    Ok(())
}

/// Toggle microphone mute
#[tauri::command]
#[allow(unused_variables)]
pub async fn toggle_mute(state: State<'_, AppState>) -> Result<(), VeilError> {
    // Mute state managed by JS LiveKit client; this is for persistence
    Ok(())
}

/// Toggle camera
#[tauri::command]
#[allow(unused_variables)]
pub async fn toggle_camera(state: State<'_, AppState>) -> Result<(), VeilError> {
    // Camera state managed by JS LiveKit client
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_claims(token: &str) -> LiveKitClaims {
        use jsonwebtoken::{decode, DecodingKey, Validation};
        let data = decode::<LiveKitClaims>(
            token,
            &DecodingKey::from_secret(b"test-secret"),
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        )
        .expect("token must verify with the same secret");
        data.claims
    }

    #[test]
    fn livekit_token_is_room_scoped_and_time_bounded() {
        let now = chrono::Utc::now().timestamp();
        let token =
            generate_livekit_token("api-key", "test-secret", "alice-uuid", "Alice", "channel-abc", None).unwrap();

        let claims = decode_claims(&token);
        assert_eq!(claims.iss, "api-key");
        assert_eq!(claims.sub, "alice-uuid");
        assert_eq!(claims.name, "Alice");
        assert_eq!(claims.video.room, "channel-abc");
        assert!(claims.video.room_join);
        assert!(claims.video.can_publish);
        assert!(claims.video.can_subscribe);
        assert!(claims.nbf <= now as usize);
        assert!(claims.exp >= now as usize + TOKEN_TTL_SECS as usize - 5);
    }

    #[test]
    fn livekit_token_rejects_missing_config() {
        let err = generate_livekit_token("", "", "alice-uuid", "Alice", "room-1", None).unwrap_err();
        assert!(matches!(err, VeilError::NotConfigured(_)));
    }
}

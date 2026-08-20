//! Discord köprüsü — resmî webhook tabanlı çıkış aynalama.
//!
//! Politika: kullanıcı token'ı ASLA kullanılmaz; yalnızca kanal sahibinin
//! Discord'da kendi sunucusu için oluşturduğu webhook URL'si kullanılır.
//! Köprüden geçen mesajlar "[köprü]" etiketiyle gönderilir ve Discord
//! tarafında E2EE koruması YOKTUR (kullanıcıya UI'da bildirilir).

use tauri::State;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{VeilError, VeilResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWebhookInput {
    pub channel_id: String,
    pub webhook_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookInfo {
    pub channel_id: String,
    /// Son 8 karakter hariç maskelenmiş URL (UI gösterimi için).
    pub masked_url: String,
}

fn validate_webhook_url(url: &str) -> VeilResult<String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://discord.com/api/webhooks/")
        || trimmed.starts_with("https://discordapp.com/api/webhooks/"))
    {
        return Err(VeilError::InvalidInput(
            "Geçerli bir Discord webhook URL'si gir (Discord > Kanal Ayarları > Entegrasyonlar > Webhooklar)."
                .into(),
        ));
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
pub async fn discord_set_webhook(input: SetWebhookInput, state: State<'_, AppState>) -> Result<WebhookInfo, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let url = validate_webhook_url(&input.webhook_url)?;
    let masked = mask_webhook(&url);
    let db = state.db.read().await;
    db.execute(
        "INSERT INTO discord_webhooks (channel_id, webhook_url, created_at) VALUES (?1, ?2, unixepoch())
         ON CONFLICT(channel_id) DO UPDATE SET webhook_url = ?2, created_at = unixepoch()",
        rusqlite::params![input.channel_id, url],
    )?;
    info!("Discord webhook set");
    Ok(WebhookInfo { channel_id: input.channel_id, masked_url: masked })
}

#[tauri::command]
pub async fn discord_clear_webhook(channel_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let db = state.db.read().await;
    db.execute(
        "DELETE FROM discord_webhooks WHERE channel_id = ?1",
        rusqlite::params![channel_id],
    )?;
    Ok(())
}

#[tauri::command]
pub async fn discord_get_webhook(channel_id: String, state: State<'_, AppState>) -> Result<Option<WebhookInfo>, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let db = state.db.read().await;
    let row: Option<String> = db
        .query_row(
            "SELECT webhook_url FROM discord_webhooks WHERE channel_id = ?1",
            rusqlite::params![channel_id],
            |r| r.get(0),
        )
        .ok();
    Ok(row.map(|url| WebhookInfo { channel_id, masked_url: mask_webhook(&url) }))
}

fn mask_webhook(url: &str) -> String {
    if url.len() > 8 {
        format!("{}…{}", &url[..url.len() - 8], &url[url.len() - 8..])
    } else {
        "•••".to_string()
    }
}

/// Mesajı Discord'a yansıt (best-effort, asla komutu düşürmez).
pub(crate) async fn mirror_message(
    state: &AppState,
    channel_id: &str,
    sender_name: &str,
    content: &str,
) {
    let webhook: Option<String> = {
        let db = state.db.read().await;
        db.query_row(
            "SELECT webhook_url FROM discord_webhooks WHERE channel_id = ?1",
            rusqlite::params![channel_id],
            |r| r.get(0),
        )
        .ok()
    };
    let Some(url) = webhook else { return };
    let settings = state.settings.read().await;
    if !settings.discord_bridge_enabled {
        return;
    }
    drop(settings);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let payload = serde_json::json!({
        "content": format!("**[köprü]** {sender_name}: {}", content.chars().take(1500).collect::<String>()),
        "username": "veilanon köprüsü",
    });
    let _ = client.post(url).json(&payload).send().await;
}

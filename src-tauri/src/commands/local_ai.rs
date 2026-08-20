//! Yerel yapay zeka — Ollama (http://localhost:11434) üzerinden.
//!
//! İsteğe bağlı (opt-in): hiçbir metin uygulamadan dışarı çıkmaz; model
//! kullanıcının kendi makinesinde çalışır. Ollama yoksa komut net hata döndürür.

use serde::{Deserialize, Serialize};

use crate::error::{VeilError, VeilResult};
use crate::config;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiChatInput {
    pub message: String,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiStatus {
    pub available: bool,
    pub model: Option<String>,
}

fn ollama_url() -> String {
    config::var("VEILANON_OLLAMA_URL")
        .unwrap_or_else(|| "http://localhost:11434".into())
}

async fn ollama_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok();
    let Some(client) = client else { return false };
    client
        .get(format!("{}/api/tags", ollama_url()))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn local_ai_status() -> VeilResult<LocalAiStatus> {
    if !ollama_available().await {
        return Ok(LocalAiStatus {
            available: false,
            model: None,
        });
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        ?;
    let body: serde_json::Value = client
        .get(format!("{}/api/tags", ollama_url()))
        .send()
        .await
        .map_err(VeilError::NetworkError)?
        .json()
        .await
        ?;
    let model = body
        .get("models")
        .and_then(|m| m.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string);
    Ok(LocalAiStatus {
        available: true,
        model,
    })
}

#[tauri::command]
pub async fn local_ai_chat(input: LocalAiChatInput) -> VeilResult<String> {
    let message = input.message.trim();
    if message.is_empty() {
        return Err(VeilError::InvalidInput("Mesaj boş olamaz".into()));
    }
    if message.len() > 4000 {
        return Err(VeilError::InvalidInput("Mesaj çok uzun (max 4000)".into()));
    }
    if !ollama_available().await {
        return Err(VeilError::NotConfigured(
            "Ollama çalışmıyor. Yerel yapay zekayı kullanmak için Ollama'yı \
             (ollama.com) kur ve bir model indir (örn. `ollama pull llama3.2`)."
                .into(),
        ));
    }

    let model = input.model.filter(|m| !m.trim().is_empty());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        ?;
    let payload = serde_json::json!({
        "model": model.unwrap_or_else(|| "llama3.2".into()),
        "messages": [{
            "role": "user",
            "content": message,
        }],
        "stream": false,
    });
    let body: serde_json::Value = client
        .post(format!("{}/api/chat", ollama_url()))
        .json(&payload)
        .send()
        .await
        .map_err(VeilError::NetworkError)?
        .json()
        .await
        ?;
    body.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .ok_or_else(|| VeilError::NotConfigured("Ollama yanıtı okunamadı".into()))
}

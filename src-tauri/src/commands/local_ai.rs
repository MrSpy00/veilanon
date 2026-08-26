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
    pub models: Vec<String>,
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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        ?;
    let resp = client
        .get(format!("{}/api/tags", ollama_url()))
        .send()
        .await;
    let Ok(resp) = resp else {
        return Ok(LocalAiStatus { available: false, model: None, models: vec![] });
    };
    if !resp.status().is_success() {
        return Ok(LocalAiStatus { available: false, model: None, models: vec![] });
    }
    let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    let models_arr = body.get("models").and_then(|m| m.as_array());
    let all_models: Vec<String> = models_arr
        .map(|arr| arr.iter().filter_map(|m| m.get("name").and_then(|n| n.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let primary = all_models.first().cloned()
        .or_else(|| if !all_models.is_empty() { Some("llama3.2".to_string()) } else { None });
    Ok(LocalAiStatus {
        available: true,
        model: primary.clone(),
        models: all_models,
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        ?;
    let detected_model = if let Some(m) = input.model.filter(|m| !m.trim().is_empty()) {
        m
    } else {
        let tags: serde_json::Value = client.get(format!("{}/api/tags", ollama_url())).send().await.map_err(VeilError::NetworkError)?.json().await.unwrap_or(serde_json::Value::Null);
        tags.get("models").and_then(|a| a.as_array()).and_then(|arr| arr.first()).and_then(|m| m.get("name")).and_then(|n| n.as_str()).map(|s| s.to_string()).unwrap_or_else(|| "llama3.2".into())
    };
    let payload = serde_json::json!({
        "model": detected_model,
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

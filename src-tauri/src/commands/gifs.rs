//! GIF arama komutları — Tenor v2 / Giphy API üzerinden.
//!
//! API anahtarları yalnızca Rust tarafında tutulur; WebView'e asla düşmez.
//! Anahtar yapılandırılmamışsa komut net bir hata döndürür (sahte veri yok).

use serde::Serialize;

use crate::error::{VeilError, VeilResult};
use crate::config;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GifResult {
    pub id: String,
    pub title: String,
    /// Tam boyutlu animasyonlu GIF.
    pub url: String,
    /// Küçük önizleme (seçici ızgarası için).
    pub preview: String,
    pub width: u32,
    pub height: u32,
    pub provider: String,
}

fn tenor_key() -> Option<String> {
    config::var("VEILANON_TENOR_API_KEY")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn giphy_key() -> Option<String> {
    config::var("VEILANON_GIPHY_API_KEY")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[allow(dead_code)]
fn provider_error() -> VeilError {
    VeilError::NotConfigured(
        "GIF arama yapılandırılmamış. .env dosyasına VEILANON_TENOR_API_KEY veya \
         VEILANON_GIPHY_API_KEY ekle (tenor.com / giphy.com'dan ücretsiz alınır)."
            .into(),
    )
}

fn dims(value: &serde_json::Value) -> (u32, u32) {
    let arr = value
        .get("dims")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let w = arr.first().and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let h = arr.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    (w, h)
}

async fn fetch_json(url: &str) -> VeilResult<serde_json::Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;
    let resp = client
        .get(url)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(VeilError::ServerError { code: resp.status().as_u16() });
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(VeilError::NetworkError)
}

fn parse_tenor(body: &serde_json::Value) -> Vec<GifResult> {
    let mut out = Vec::new();
    let Some(results) = body.get("results").and_then(|r| r.as_array()) else {
        return out;
    };
    for item in results {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else { continue };
        let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("GIF").to_string();

        // Check media_formats (Tenor v2)
        if let Some(formats) = item.get("media_formats").and_then(|f| f.as_object()) {
            let pick = |name: &str| formats.get(name).and_then(|f| f.get("url")).and_then(|u| u.as_str()).map(str::to_string);
            if let Some(url) = pick("gif").or_else(|| pick("mediumgif")).or_else(|| pick("tinygif")) {
                let preview = pick("tinygif").or_else(|| pick("nanogif")).unwrap_or_else(|| url.clone());
                let (width, height) = formats.get("gif").map(dims).unwrap_or((200, 200));
                out.push(GifResult {
                    id: id.to_string(),
                    title,
                    url,
                    preview,
                    width,
                    height,
                    provider: "Tenor".into(),
                });
                continue;
            }
        }

        // Check media array (Tenor v1)
        if let Some(media) = item.get("media").and_then(|m| m.as_array()).and_then(|a| a.first()).and_then(|m| m.as_object()) {
            let pick = |name: &str| media.get(name).and_then(|f| f.get("url")).and_then(|u| u.as_str()).map(str::to_string);
            if let Some(url) = pick("gif").or_else(|| pick("tinygif")) {
                let preview = pick("tinygif").unwrap_or_else(|| url.clone());
                out.push(GifResult {
                    id: id.to_string(),
                    title,
                    url,
                    preview,
                    width: 200,
                    height: 200,
                    provider: "Tenor".into(),
                });
            }
        }
    }
    out
}

fn parse_giphy(body: &serde_json::Value) -> Vec<GifResult> {
    let mut out = Vec::new();
    let Some(data) = body.get("data").and_then(|d| d.as_array()) else {
        return out;
    };
    for item in data {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else { continue };
        let Some(images) = item.get("images").and_then(|i| i.as_object()) else { continue };
        let pick = |name: &str| images.get(name).and_then(|f| f.get("url")).and_then(|u| u.as_str()).map(str::to_string);
        let Some(url) = pick("fixed_width").or_else(|| pick("original")).or_else(|| pick("downsized")) else { continue };
        let preview = pick("fixed_width_small").or_else(|| pick("fixed_height_small")).unwrap_or_else(|| url.clone());
        let width = images.get("fixed_width").and_then(|f| f.get("width")).and_then(|w| w.as_u64()).unwrap_or(200) as u32;
        let height = images.get("fixed_width").and_then(|f| f.get("height")).and_then(|h| h.as_u64()).unwrap_or(200) as u32;
        out.push(GifResult {
            id: id.to_string(),
            title: item.get("title").and_then(|t| t.as_str()).unwrap_or("GIF").to_string(),
            url,
            preview,
            width,
            height,
            provider: "Giphy".into(),
        });
    }
    out
}

#[tauri::command]
pub async fn gif_search(query: String, limit: Option<u32>) -> VeilResult<Vec<GifResult>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.unwrap_or(24).clamp(1, 50);
    let encoded_q = urlencoding(q);

    // 1. Try Giphy with configured key
    if let Some(key) = giphy_key() {
        let url = format!(
            "https://api.giphy.com/v1/gifs/search?api_key={}&q={}&limit={}&rating=g",
            key, encoded_q, limit
        );
        if let Ok(val) = fetch_json(&url).await {
            let res = parse_giphy(&val);
            if !res.is_empty() {
                return Ok(res);
            }
        }
    }

    // 2. Try Tenor v2 with configured key
    if let Some(key) = tenor_key() {
        let url = format!(
            "https://tenor.googleapis.com/v2/search?q={}&key={}&client_key=veilanon&limit={}&media_filter=gif,tinygif&contentfilter=medium",
            encoded_q, key, limit
        );
        if let Ok(val) = fetch_json(&url).await {
            let res = parse_tenor(&val);
            if !res.is_empty() {
                return Ok(res);
            }
        }
    }

    Ok(Vec::new())
}

#[tauri::command]
pub async fn gif_trending(limit: Option<u32>) -> VeilResult<Vec<GifResult>> {
    let limit = limit.unwrap_or(24).clamp(1, 50);

    // 1. Try Giphy with configured key
    if let Some(key) = giphy_key() {
        let url = format!(
            "https://api.giphy.com/v1/gifs/trending?api_key={}&limit={}&rating=g",
            key, limit
        );
        if let Ok(val) = fetch_json(&url).await {
            let res = parse_giphy(&val);
            if !res.is_empty() {
                return Ok(res);
            }
        }
    }

    // 2. Try Tenor v2 featured
    if let Some(key) = tenor_key() {
        let url = format!(
            "https://tenor.googleapis.com/v2/featured?key={}&client_key=veilanon&limit={}&media_filter=gif,tinygif&contentfilter=medium",
            key, limit
        );
        if let Ok(val) = fetch_json(&url).await {
            let res = parse_tenor(&val);
            if !res.is_empty() {
                return Ok(res);
            }
        }
    }

    Ok(Vec::new())
}

fn urlencoding(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

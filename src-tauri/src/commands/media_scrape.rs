use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub success: bool,
    pub media_urls: Vec<MediaCandidate>,
    pub title: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaCandidate {
    pub url: String,
    pub media_type: String,  // "video" or "image"
    pub source: String,      // "og:video", "og:image", "video[src]", "img[src]", etc.
}

#[command]
pub async fn scrape_url(url: String) -> Result<ScrapeResult, String> {
    // Validate URL
    let parsed = url::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;

    let lower_path = parsed.path().to_lowercase();
    if lower_path.ends_with(".mp4") || lower_path.ends_with(".webm") || lower_path.ends_with(".mov") || lower_path.ends_with(".mkv") || lower_path.ends_with(".ogv") {
        return Ok(ScrapeResult {
            success: true,
            media_urls: vec![MediaCandidate {
                url: parsed.to_string(),
                media_type: "video".into(),
                source: "direct_link".into(),
            }],
            title: Some("Doğrudan Video Linki".into()),
            error: None,
        });
    }
    if lower_path.ends_with(".png") || lower_path.ends_with(".jpg") || lower_path.ends_with(".jpeg") || lower_path.ends_with(".gif") || lower_path.ends_with(".webp") || lower_path.ends_with(".avif") || lower_path.ends_with(".bmp") || lower_path.ends_with(".svg") {
        return Ok(ScrapeResult {
            success: true,
            media_urls: vec![MediaCandidate {
                url: parsed.to_string(),
                media_type: "image".into(),
                source: "direct_link".into(),
            }],
            title: Some("Doğrudan Görsel Linki".into()),
            error: None,
        });
    }

    // Fetch the page HTML with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("Client error: {}", e))?;

    let response = client.get(parsed.as_str())
        .send()
        .await
        .map_err(|e| format!("Fetch error: {}", e))?;

    if let Some(ct) = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
        if ct.starts_with("image/") {
            return Ok(ScrapeResult {
                success: true,
                media_urls: vec![MediaCandidate {
                    url: parsed.to_string(),
                    media_type: "image".into(),
                    source: "content_type_header".into(),
                }],
                title: Some("Doğrudan Görsel".into()),
                error: None,
            });
        } else if ct.starts_with("video/") {
            return Ok(ScrapeResult {
                success: true,
                media_urls: vec![MediaCandidate {
                    url: parsed.to_string(),
                    media_type: "video".into(),
                    source: "content_type_header".into(),
                }],
                title: Some("Doğrudan Video".into()),
                error: None,
            });
        }
    }

    let html = response.text().await
        .map_err(|e| format!("Read error: {}", e))?;

    let mut media_urls = Vec::new();
    let mut title = None;

    // Extract title
    if let Some(t) = extract_meta_content(&html, "og:title") {
        title = Some(t);
    } else if let Some(t) = extract_meta_content(&html, "twitter:title") {
        title = Some(t);
    } else if let Some(t) = extract_tag_content(&html, "title") {
        title = Some(t);
    }

    // Extract Open Graph video URLs (highest priority)
    for content in extract_meta_contents(&html, "og:video") {
        media_urls.push(MediaCandidate {
            url: resolve_url(&parsed, &content),
            media_type: "video".to_string(),
            source: "og:video".to_string(),
        });
    }

    // Extract Open Graph video:secure_url
    for content in extract_meta_contents(&html, "og:video:secure_url") {
        if !media_urls.iter().any(|m| m.url == content) {
            media_urls.push(MediaCandidate {
                url: resolve_url(&parsed, &content),
                media_type: "video".to_string(),
                source: "og:video:secure_url".to_string(),
            });
        }
    }

    // Extract twitter:player:stream (Twitter card video)
    for content in extract_meta_contents(&html, "twitter:player:stream") {
        if !media_urls.iter().any(|m| m.url == content) {
            media_urls.push(MediaCandidate {
                url: resolve_url(&parsed, &content),
                media_type: "video".to_string(),
                source: "twitter:player:stream".to_string(),
            });
        }
    }

    // Extract <video> src attributes
    for src in extract_video_srcs(&html) {
        if !media_urls.iter().any(|m| m.url == src) {
            media_urls.push(MediaCandidate {
                url: resolve_url(&parsed, &src),
                media_type: "video".to_string(),
                source: "video[src]".to_string(),
            });
        }
    }

    // Extract <source> tags inside <video>
    for src in extract_source_srcs(&html) {
        if !media_urls.iter().any(|m| m.url == src) {
            media_urls.push(MediaCandidate {
                url: resolve_url(&parsed, &src),
                media_type: "video".to_string(),
                source: "video>source[src]".to_string(),
            });
        }
    }

    // Extract Open Graph image URLs (fallback if no video found)
    for content in extract_meta_contents(&html, "og:image") {
        media_urls.push(MediaCandidate {
            url: resolve_url(&parsed, &content),
            media_type: "image".to_string(),
            source: "og:image".to_string(),
        });
    }

    // Extract og:image:url
    for content in extract_meta_contents(&html, "og:image:url") {
        if !media_urls.iter().any(|m| m.url == content) {
            media_urls.push(MediaCandidate {
                url: resolve_url(&parsed, &content),
                media_type: "image".to_string(),
                source: "og:image:url".to_string(),
            });
        }
    }

    // Extract large <img> src (heuristic: skip tiny icons/logos)
    for src in extract_img_srcs(&html) {
        if !media_urls.iter().any(|m| m.url == src)
            && !src.contains("logo") && !src.contains("icon") && !src.contains("avatar")
            && !src.contains("favicon") && !src.contains("badge") {
            media_urls.push(MediaCandidate {
                url: resolve_url(&parsed, &src),
                media_type: "image".to_string(),
                source: "img[src]".to_string(),
            });
        }
    }

    Ok(ScrapeResult {
        success: true,
        media_urls,
        title,
        error: None,
    })
}

// Helper functions using regex (no HTML parser dependency needed)

fn extract_meta_content(html: &str, property: &str) -> Option<String> {
    let pattern = format!(r#"<meta[^>]*property=["']{}["'][^>]*content=["']([^"']+)["']"#, regex::escape(property));
    let pattern2 = format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*property=["']{}["']"#, regex::escape(property));

    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(caps) = re.captures(html) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(&pattern2) {
        if let Some(caps) = re.captures(html) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }
    None
}

fn extract_meta_contents(html: &str, property: &str) -> Vec<String> {
    let mut results = Vec::new();
    let pattern = format!(r#"<meta[^>]*property=["']{}["'][^>]*content=["']([^"']+)["']"#, regex::escape(property));
    let pattern2 = format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*property=["']{}["']"#, regex::escape(property));

    if let Ok(re) = regex::Regex::new(&pattern) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                results.push(m.as_str().to_string());
            }
        }
    }
    if let Ok(re) = regex::Regex::new(&pattern2) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                let val = m.as_str().to_string();
                if !results.contains(&val) {
                    results.push(val);
                }
            }
        }
    }
    results
}

fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<{}[^>]*>([^<]+)</{}>", regex::escape(tag), regex::escape(tag));
    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(caps) = re.captures(html) {
            return caps.get(1).map(|m| m.as_str().trim().to_string());
        }
    }
    None
}

fn extract_video_srcs(html: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(re) = regex::Regex::new(r#"<video[^>]*\bsrc=["']([^"']+)["']"#) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                results.push(m.as_str().to_string());
            }
        }
    }
    results
}

fn extract_source_srcs(html: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(re) = regex::Regex::new(r#"<source[^>]*\bsrc=["']([^"']+)["']"#) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                results.push(m.as_str().to_string());
            }
        }
    }
    results
}

fn extract_img_srcs(html: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(re) = regex::Regex::new(r#"<img[^>]*\bsrc=["']([^"']+)["']"#) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                let src = m.as_str().to_string();
                // Skip data URIs and tiny tracking pixels
                if !src.starts_with("data:") && !src.contains("1x1") && !src.contains("pixel") {
                    results.push(src);
                }
            }
        }
    }
    results
}

fn resolve_url(base: &url::Url, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Ok(resolved) = base.join(href) {
        resolved.to_string()
    } else {
        href.to_string()
    }
}

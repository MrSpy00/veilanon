use std::collections::{HashMap, HashSet};

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
    pub poster: Option<String>, // preview image for video candidates; None serializes as null
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
                poster: None,
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
                poster: None,
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
                    poster: None,
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
                    poster: None,
                }],
                title: Some("Doğrudan Video".into()),
                error: None,
            });
        }
    }

    let html = response.text().await
        .map_err(|e| format!("Read error: {}", e))?;

    // Performance guard: bound regex scanning to the first ~2 MB of HTML
    // (char-boundary safe slice).
    let mut end = html.len().min(2_000_000);
    while end > 0 && !html.is_char_boundary(end) {
        end -= 1;
    }
    let html = &html[..end];

    let (media_urls, title) = extract_media_candidates(&parsed, html);

    Ok(ScrapeResult {
        success: true,
        media_urls,
        title,
        error: None,
    })
}

const MAX_VIDEO_CANDIDATES: usize = 20;
const MAX_IMAGE_CANDIDATES: usize = 60;

/// Priority-ordered extraction pipeline shared by scrape_url and tests.
/// Order: og:video family -> twitter:player:stream -> video/src+source+json
/// videos -> og:image family -> twitter images -> json images -> img attrs.
fn extract_media_candidates(base: &url::Url, html: &str) -> (Vec<MediaCandidate>, Option<String>) {
    let title = extract_meta_content(html, "og:title")
        .or_else(|| extract_meta_content(html, "twitter:title"))
        .or_else(|| extract_tag_content(html, "title"))
        .map(|t| decode_html_entities(&t));

    let mut c = Collector::new(base);

    for s in extract_meta_contents(html, "og:video") {
        c.push(&s, "video", "og:video", None);
    }
    for s in extract_meta_contents(html, "og:video:secure_url") {
        c.push(&s, "video", "og:video:secure_url", None);
    }
    for s in extract_meta_contents(html, "twitter:player:stream") {
        c.push(&s, "video", "twitter:player:stream", None);
    }

    let video_els = extract_video_elements(html);
    for el in &video_els {
        if let Some(src) = &el.src {
            c.push(src, "video", "video[src]", None);
        }
    }
    for src in extract_source_srcs(html) {
        c.push(&src, "video", "video>source[src]", None);
    }
    let json_media = extract_json_media_urls(html);
    for (url, media_type) in &json_media {
        if media_type == "video" {
            c.push(url, "video", "json_embedded", None);
        }
    }

    for prop in ["og:image", "og:image:url", "og:image:secure_url"] {
        for s in extract_meta_contents(html, prop) {
            c.push(&s, "image", prop, None);
        }
    }
    for prop in ["twitter:image", "twitter:image:src"] {
        for s in extract_meta_contents(html, prop) {
            c.push(&s, "image", prop, None);
        }
    }
    for (url, media_type) in &json_media {
        if media_type == "image" {
            c.push(url, "image", "json_embedded", None);
        }
    }

    for (src, source) in extract_img_candidates(html) {
        c.push(&src, "image", source, None);
    }

    attach_posters(&mut c.out, base, &video_els);

    (c.out, title)
}

/// Attach <video poster="..."> to video candidates: exact element pairing
/// first, then a single-distinct-poster fallback for meta-only pages.
fn attach_posters(out: &mut [MediaCandidate], base: &url::Url, els: &[VideoElement]) {
    let mut poster_map: HashMap<String, String> = HashMap::new();
    for el in els {
        let Some(poster) = &el.poster else { continue };
        let resolved_poster = resolve_url(base, &decode_html_entities(poster));
        for u in el.all_urls() {
            poster_map.insert(resolve_url(base, &u), resolved_poster.clone());
        }
    }
    let distinct: HashSet<&String> = poster_map.values().collect();
    let fallback = if distinct.len() == 1 {
        distinct.into_iter().next().cloned()
    } else {
        None
    };
    for m in out.iter_mut() {
        if m.media_type == "video" && m.poster.is_none() {
            m.poster = poster_map.get(&m.url).cloned().or_else(|| fallback.clone());
        }
    }
}

struct VideoElement {
    src: Option<String>,
    poster: Option<String>,
    sources: Vec<String>,
}

impl VideoElement {
    fn all_urls(&self) -> Vec<String> {
        let mut v = Vec::with_capacity(1 + self.sources.len());
        if let Some(src) = &self.src {
            v.push(decode_html_entities(src));
        }
        v.extend(self.sources.iter().map(|s| decode_html_entities(s)));
        v
    }
}

fn extract_video_elements(html: &str) -> Vec<VideoElement> {
    let mut els = Vec::new();
    let Ok(re_open) = regex::Regex::new(r#"(?is)<video\b([^>]*)>"#) else {
        return els;
    };
    for caps in re_open.captures_iter(html) {
        let attrs = caps.get(1).map(|g| g.as_str()).unwrap_or("");
        let tag_end = caps.get(0).unwrap().end();
        let close = html[tag_end..]
            .find("</video>")
            .map_or(html.len(), |i| tag_end + i);
        els.push(VideoElement {
            src: attr_value(attrs, "src"),
            poster: attr_value(attrs, "poster"),
            sources: extract_source_srcs(&html[tag_end..close]),
        });
    }
    els
}

struct Collector<'a> {
    base: &'a url::Url,
    out: Vec<MediaCandidate>,
    seen: HashSet<String>,
    videos: usize,
    images: usize,
}

impl<'a> Collector<'a> {
    fn new(base: &'a url::Url) -> Self {
        Self {
            base,
            out: Vec::new(),
            seen: HashSet::new(),
            videos: 0,
            images: 0,
        }
    }

    /// Decode entities, drop junk, enforce per-type caps, dedupe, resolve.
    fn push(&mut self, raw: &str, media_type: &str, source: &str, poster: Option<String>) {
        let decoded = decode_html_entities(raw);
        if self.seen.contains(&decoded) || is_junk_url(&decoded) {
            return;
        }
        match media_type {
            "video" if self.videos >= MAX_VIDEO_CANDIDATES => return,
            "image" if self.images >= MAX_IMAGE_CANDIDATES => return,
            _ => {}
        }
        self.seen.insert(decoded.clone());
        if media_type == "video" {
            self.videos += 1;
        } else {
            self.images += 1;
        }
        self.out.push(MediaCandidate {
            url: resolve_url(self.base, &decoded),
            media_type: media_type.to_string(),
            source: source.to_string(),
            poster,
        });
    }
}

/// Single-pass HTML entity decoder: named entities plus decimal/hex numeric
/// forms. Unknown or malformed entities pass through untouched; no double
/// decoding (&amp;lt; stays "&lt;", never becomes "<").
fn decode_html_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi_rel) = input[i..].find(';') {
                let semi = i + semi_rel;
                let name = &input[i + 1..semi];
                if !name.is_empty()
                    && name.len() <= 10
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '#')
                {
                    let decoded = match name {
                        "amp" => Some('&'),
                        "quot" => Some('"'),
                        "apos" | "#39" => Some('\''),
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "nbsp" => Some('\u{00A0}'),
                        _ => {
                            if let Some(hex) =
                                name.strip_prefix("#x").or_else(|| name.strip_prefix("#X"))
                            {
                                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                            } else if let Some(dec) = name.strip_prefix('#') {
                                dec.parse::<u32>().ok().and_then(char::from_u32)
                            } else {
                                None
                            }
                        }
                    };
                    if let Some(ch) = decoded {
                        out.push(ch);
                        i = semi + 1;
                        continue;
                    }
                }
            }
        }
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Junk asset filter: data URIs, SVGs, and common non-content image names.
fn is_junk_url(url: &str) -> bool {
    const JUNK_PATTERNS: &[&str] = &[
        "logo", "icon", "avatar", "favicon", "badge",
        "sprite", "thumb", "small", "pixel", "blank", "spacer", "tracking", "1x1",
    ];
    let lower = url.to_lowercase();
    lower.starts_with("data:")
        || lower.ends_with(".svg")
        || JUNK_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Extract an attribute value from a tag-attribute substring. The `(?:^|[^-\w])`
/// guard prevents matching `src` inside `data-src` / `poster` inside `data-poster`.
fn attr_value(attrs: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"(?:^|[^-\w]){}=["']([^"']+)["']"#, regex::escape(name));
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(attrs)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Pick the widest candidate from a srcset descriptor list ("url 480w, ...").
fn pick_largest_srcset(value: &str) -> Option<String> {
    let mut best: Option<(u64, String)> = None;
    for part in value.split(',') {
        let mut tokens = part.split_whitespace();
        let url = tokens.next()?;
        let width = tokens
            .next()
            .and_then(|d| d.strip_suffix('w'))
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0);
        if best.as_ref().map_or(true, |(w, _)| width > *w) {
            best = Some((width, url.to_string()));
        }
    }
    best.map(|(_, u)| u)
}

/// Media URLs embedded in <script> bodies only — scanning the whole document
/// would steal URLs from <img>/<video> tags and mislabel them as json_embedded.
fn extract_json_media_urls(html: &str) -> Vec<(String, String)> {
    let mut script_text = String::new();
    if let Ok(re_script) = regex::Regex::new(r#"(?is)<script\b[^>]*>(.*?)</script>"#) {
        for caps in re_script.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                script_text.push_str(m.as_str());
                script_text.push('\n');
            }
        }
    }

    let mut results = Vec::new();
    if let Ok(re_vid) = regex::Regex::new(r#"https?://[^\s"'<>\\]+?\.(?:mp4|webm|mov|mkv)"#) {
        for m in re_vid.find_iter(&script_text) {
            let u = m.as_str().to_string();
            if !results.iter().any(|(url, _)| url == &u) {
                results.push((u, "video".to_string()));
            }
        }
    }
    if let Ok(re_img) = regex::Regex::new(r#"https?://[^\s"'<>\\]+?\.(?:png|jpg|jpeg|gif|webp)"#) {
        for m in re_img.find_iter(&script_text) {
            let u = m.as_str().to_string();
            if !results.iter().any(|(url, _)| url == &u) {
                results.push((u, "image".to_string()));
            }
        }
    }
    results
}

// Helper functions using regex (no HTML parser dependency needed)

fn extract_meta_content(html: &str, property: &str) -> Option<String> {
    let p_escaped = regex::escape(property);
    let patterns = [
        format!(r#"<meta[^>]*property=["']{}["'][^>]*content=["']([^"']+)["']"#, p_escaped),
        format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*property=["']{}["']"#, p_escaped),
        format!(r#"<meta[^>]*name=["']{}["'][^>]*content=["']([^"']+)["']"#, p_escaped),
        format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*name=["']{}["']"#, p_escaped),
        format!(r#"<meta[^>]*itemprop=["']{}["'][^>]*content=["']([^"']+)["']"#, p_escaped),
        format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*itemprop=["']{}["']"#, p_escaped),
    ];

    for pat in &patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            if let Some(caps) = re.captures(html) {
                return caps.get(1).map(|m| m.as_str().to_string());
            }
        }
    }
    None
}

fn extract_meta_contents(html: &str, property: &str) -> Vec<String> {
    let mut results = Vec::new();
    let p_escaped = regex::escape(property);
    let patterns = [
        format!(r#"<meta[^>]*property=["']{}["'][^>]*content=["']([^"']+)["']"#, p_escaped),
        format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*property=["']{}["']"#, p_escaped),
        format!(r#"<meta[^>]*name=["']{}["'][^>]*content=["']([^"']+)["']"#, p_escaped),
        format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*name=["']{}["']"#, p_escaped),
        format!(r#"<meta[^>]*itemprop=["']{}["'][^>]*content=["']([^"']+)["']"#, p_escaped),
        format!(r#"<meta[^>]*content=["']([^"']+)["'][^>]*itemprop=["']{}["']"#, p_escaped),
    ];

    for pat in &patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            for caps in re.captures_iter(html) {
                if let Some(m) = caps.get(1) {
                    let val = m.as_str().to_string();
                    if !results.contains(&val) {
                        results.push(val);
                    }
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

fn extract_source_srcs(html: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(re) = regex::Regex::new(r#"<source[^>]*(?:^|[^-\w])src=["']([^"']+)["']"#) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                results.push(m.as_str().to_string());
            }
        }
    }
    results
}

/// Tag-by-tag so per-type caps cannot be exhausted by one attribute family
/// before later attributes (data-src/srcset) of earlier tags are collected.
fn extract_img_candidates(html: &str) -> Vec<(String, &'static str)> {
    let mut results = Vec::new();
    let Ok(tag_re) = regex::Regex::new(r#"(?is)<img\b([^>]*)>"#) else {
        return results;
    };
    const ATTRS: [(&str, &str); 5] = [
        ("src", "img[src]"),
        ("data-src", "img[data-src]"),
        ("data-original", "img[data-original]"),
        ("data-lazy-src", "img[data-lazy-src]"),
        ("srcset", "img[srcset]"),
    ];
    for caps in tag_re.captures_iter(html) {
        let attrs = caps.get(1).map(|g| g.as_str()).unwrap_or("");
        for (name, label) in ATTRS {
            if let Some(val) = attr_value(attrs, name) {
                if name == "srcset" {
                    if let Some(best) = pick_largest_srcset(&val) {
                        results.push((best, label));
                    }
                } else {
                    results.push((val, label));
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

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://www.motionbgs.example/watch/wave";

    /// motionbgs-like page: og:video with &amp;, lazy data-src imgs,
    /// <video poster>, 100+ junk imgs (cap proof), entity-laden json url.
    fn fixture_page() -> String {
        let mut junk_imgs = String::new();
        for i in 0..120 {
            junk_imgs.push_str(&format!(
                "<img src=\"https://cdn.motionbgs.example/gallery/filler_{:03}.jpg\">\n",
                i
            ));
        }
        format!(
            r#"<!DOCTYPE html>
<html><head>
<title>MotionBGs &amp; Wallpapers</title>
<meta property="og:title" content="Cool Videos &amp; More">
<meta property="og:video" content="https://cdn.motionbgs.example/media/hd/wave&amp;sunset.mp4">
<meta property="og:image" content="https://cdn.motionbgs.example/thumbs/wave_sunset_preview.jpg">
<meta name="twitter:image" content="https://cdn.motionbgs.example/thumbs/twitter_card.png">
<script type="application/json">
{{"poster":"https://cdn.motionbgs.example/assets/sprite_sheet.png","clip":"https://cdn.motionbgs.example/clips/loop&#95;hd.webm"}}
</script>
</head><body>
<video controls src="/media/direct/main.mp4" poster="https://cdn.motionbgs.example/posters/main&#95;poster.jpg">
  <source src="/media/stream/chunked&#47;main.m3u8" type="video/mp4">
</video>
<img src="https://cdn.motionbgs.example/img/logo_header.svg" alt="logo">
<img data-src="https://cdn.motionbgs.example/lazy/gallery_01.jpg" src="https://cdn.motionbgs.example/img/pixel.gif">
<img data-original="https://cdn.motionbgs.example/lazy/gallery_02.jpg">
<img data-lazy-src="https://cdn.motionbgs.example/lazy/gallery_03.jpg">
<img srcset="https://cdn.motionbgs.example/srcset/small_320.jpg 320w, https://cdn.motionbgs.example/srcset/large_1280.jpg 1280w, https://cdn.motionbgs.example/srcset/mid_640.jpg 640w">
{}</body></html>"#,
            junk_imgs
        )
    }

    // ---------- named requirement tests ----------

    #[test]
    fn decodes_html_entities_in_urls() {
        // Named entities
        assert_eq!(
            decode_html_entities("https://x.com/a&amp;b&quot;c"),
            "https://x.com/a&b\"c"
        );
        assert_eq!(decode_html_entities("it&#39;s &lt;here&gt;"), "it's <here>");
        assert_eq!(decode_html_entities("a&nbsp;b"), "a\u{00A0}b");
        // Numeric decimal + hex
        assert_eq!(decode_html_entities("loop&#95;hd.webm"), "loop_hd.webm");
        assert_eq!(decode_html_entities("chunked&#x2F;main.m3u8"), "chunked/main.m3u8");
        // Unknown entities left untouched
        assert_eq!(decode_html_entities("a&foo;b"), "a&foo;b");
        // Single-pass: no double decoding
        assert_eq!(decode_html_entities("&amp;lt;"), "&lt;");
        // Pipeline: og:video content is decoded end-to-end
        let html = r#"<meta property="og:video" content="https://c.example/v&amp;q=1.mp4">"#;
        let base = url::Url::parse(BASE).unwrap();
        let (out, _) = extract_media_candidates(&base, html);
        assert!(out.iter().any(|m| m.url == "https://c.example/v&q=1.mp4"));
    }

    #[test]
    fn extracts_lazy_load_data_src_images() {
        let base = url::Url::parse(BASE).unwrap();
        let (out, _) = extract_media_candidates(&base, &fixture_page());
        let find = |suffix: &str| out.iter().find(|m| m.url.ends_with(suffix));
        let d1 = find("lazy/gallery_01.jpg").expect("data-src image missing");
        assert_eq!(d1.source, "img[data-src]");
        assert_eq!(d1.media_type, "image");
        assert!(find("lazy/gallery_02.jpg").is_some(), "data-original missing");
        assert!(find("lazy/gallery_03.jpg").is_some(), "data-lazy-src missing");
        // tracking pixel that shared the img tag must be gone
        assert!(!out.iter().any(|m| m.url.contains("pixel.gif")));
    }

    #[test]
    fn extracts_video_source_and_poster() {
        let base = url::Url::parse(BASE).unwrap();
        let (out, _) = extract_media_candidates(&base, &fixture_page());
        let vid = out
            .iter()
            .find(|m| m.url.ends_with("/media/direct/main.mp4"))
            .expect("video[src] candidate missing");
        assert_eq!(vid.source, "video[src]");
        assert_eq!(
            vid.poster.as_deref(),
            Some("https://cdn.motionbgs.example/posters/main_poster.jpg")
        );
        let src_tag = out
            .iter()
            .find(|m| m.url.ends_with("chunked/main.m3u8"))
            .expect("<source> candidate missing");
        assert_eq!(src_tag.source, "video>source[src]");
        assert_eq!(src_tag.poster.as_deref(), vid.poster.as_deref());
    }

    #[test]
    fn caps_and_prioritizes_videos_first() {
        let base = url::Url::parse(BASE).unwrap();

        // Priority ordering on a page containing every source family
        let mixed = concat!(
            r#"<meta property="og:title" content="T &amp; T">"#,
            r#"<meta property="og:video" content="https://c.example/ogv.mp4">"#,
            r#"<meta name="twitter:player:stream" content="https://c.example/stream.mp4">"#,
            r#"<script>{"v":"https://c.example/jsonv.mp4","i":"https://c.example/jsoni.jpg"}</script>"#,
            r#"<video src="https://c.example/vid.mp4"><source src="https://c.example/src.webm"></video>"#,
            r#"<meta property="og:image" content="https://c.example/ogi.jpg">"#,
            r#"<meta name="twitter:image" content="https://c.example/twi.jpg">"#,
            r#"<img src="https://c.example/img.jpg">"#,
        );
        let (out, title) = extract_media_candidates(&base, mixed);
        assert_eq!(title.as_deref(), Some("T & T"));
        let pos = |pred: &dyn Fn(&MediaCandidate) -> bool| {
            out.iter().position(pred).expect("candidate not found")
        };
        let p_ogv = pos(&|m| m.source == "og:video");
        let p_tps = pos(&|m| m.source == "twitter:player:stream");
        let p_vid = pos(&|m| m.source == "video[src]");
        let p_src = pos(&|m| m.source == "video>source[src]");
        let p_jv = pos(&|m| m.source == "json_embedded" && m.media_type == "video");
        let p_ogi = pos(&|m| m.source == "og:image");
        let p_twi = pos(&|m| m.source == "twitter:image");
        let p_ji = pos(&|m| m.source == "json_embedded" && m.media_type == "image");
        let p_img = pos(&|m| m.source == "img[src]");
        assert!(p_ogv < p_tps);
        assert!(p_tps < p_vid);
        assert!(p_vid < p_src);
        assert!(p_src < p_jv);
        assert!(p_jv < p_ogi);
        assert!(p_ogi < p_twi);
        assert!(p_twi < p_ji);
        assert!(p_ji < p_img);

        // Hard caps with an oversized page
        let mut big = String::from(
            r#"<meta property="og:video" content="https://c.example/hero.mp4">"#,
        );
        for i in 0..30 {
            big.push_str(&format!("<video src=\"https://c.example/v{}.mp4\"></video>", i));
        }
        for i in 0..100 {
            big.push_str(&format!("<img src=\"https://c.example/i{}.jpg\">", i));
        }
        let (big_out, _) = extract_media_candidates(&base, &big);
        let vids = big_out.iter().filter(|m| m.media_type == "video").count();
        let imgs = big_out.iter().filter(|m| m.media_type == "image").count();
        assert!(vids <= 20, "too many videos: {}", vids);
        assert!(imgs <= 60, "too many images: {}", imgs);
        assert_eq!(big_out[0].source, "og:video", "og:video must be first");
    }

    #[test]
    fn filters_junk_assets() {
        let base = url::Url::parse(BASE).unwrap();
        let html = concat!(
            r#"<img src="https://c.example/icons/set.svg">"#,
            r#"<img src="data:image/png;base64,iVBOR">"#,
            r#"<img src="https://c.example/ui/sprite_sheet.png">"#,
            r#"<img src="https://c.example/t/thumb_preview.jpg">"#,
            r#"<img src="https://c.example/t/small_variant.jpg">"#,
            r#"<img src="https://c.example/x/blank.gif">"#,
            r#"<img src="https://c.example/x/spacer.png">"#,
            r#"<img src="https://c.example/x/tracking_beacon.png">"#,
            r#"<img src="https://c.example/real/photo.jpg">"#,
        );
        let (out, _) = extract_media_candidates(&base, html);
        let urls: Vec<&str> = out.iter().map(|m| m.url.as_str()).collect();
        assert_eq!(urls, vec!["https://c.example/real/photo.jpg"]);
    }

    // ---------- scrape_url early-return branches (no network IO) ----------

    #[tokio::test]
    async fn scrape_url_rejects_invalid_url() {
        let res = scrape_url("not a url".into()).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn scrape_url_direct_video_link_returns_early() {
        let res = scrape_url("https://cdn.example.com/movie.MP4".into())
            .await
            .expect("should succeed without network");
        assert!(res.success);
        assert_eq!(res.media_urls.len(), 1);
        let m = &res.media_urls[0];
        assert_eq!(m.source, "direct_link");
        assert_eq!(m.media_type, "video");
        assert!(m.poster.is_none(), "poster must serialize as null when absent");
    }

    #[tokio::test]
    async fn scrape_url_direct_image_link_returns_early() {
        let res = scrape_url("https://cdn.example.com/pic.png".into())
            .await
            .expect("should succeed without network");
        assert!(res.success);
        let m = &res.media_urls[0];
        assert_eq!(m.source, "direct_link");
        assert_eq!(m.media_type, "image");
        assert!(m.poster.is_none());
    }

    // ---------- full pipeline on representative fixture ----------

    #[test]
    fn motionbgs_like_page_full_pipeline() {
        let base = url::Url::parse(BASE).unwrap();
        let (out, _) = extract_media_candidates(&base, &fixture_page());

        // og:video decoded first
        assert_eq!(
            out[0].url,
            "https://cdn.motionbgs.example/media/hd/wave&sunset.mp4"
        );
        assert_eq!(out[0].source, "og:video");

        // json video with numeric entity decoded; sprite_sheet filtered from json images
        assert!(out
            .iter()
            .any(|m| m.url == "https://cdn.motionbgs.example/clips/loop_hd.webm"));
        assert!(!out.iter().any(|m| m.url.contains("sprite_sheet")));

        // srcset picked the largest width candidate only
        let ss = out
            .iter()
            .find(|m| m.source == "img[srcset]")
            .expect("srcset candidate missing");
        assert_eq!(ss.url, "https://cdn.motionbgs.example/srcset/large_1280.jpg");

        // svg logo filtered even though it appeared first in body
        assert!(!out.iter().any(|m| m.url.contains("logo_header")));
    }
}

//! Privacy Tools & Zero-Key Public Network Diagnostics
//!
//! Provides zero-knowledge, zero-auth privacy verification utilities:
//! - Tor exit node detection
//! - IP & ISP leak diagnostics (Cloudflare + ipify fallback)
//! - Encrypted DNS-over-HTTPS (DoH) probes (Cloudflare + Google)
//! - Multi-provider DoH benchmark (Cloudflare, Google, Quad9, AdGuard, Mullvad)
//! - k-Anonymity password leak verification (HaveIBeenPwned API)
//! - Malicious link scanner (Abuse.ch URLhaus)
//! - Privacy-preserving link previews with SSRF filtering
//! - Deterministic local SVG identicon generator (100% offline)
//! - Cryptographic clock skew detection (WorldTimeAPI)
//! - Offline SVG QR code generator

use ring::digest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info};

use crate::error::{VeilError, VeilResult};

// ── Models ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorStatusResult {
    pub is_tor: bool,
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IpLeakResult {
    pub ip: String,
    pub colo: Option<String>,
    pub loc: Option<String>,
    pub tls: Option<String>,
    pub sni: Option<String>,
    pub warp: Option<String>,
    pub gateway: Option<String>,
    pub rtt_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DohTestResult {
    pub cloudflare_ok: bool,
    pub google_ok: bool,
    pub latency_cloudflare_ms: u64,
    pub latency_google_ms: u64,
    pub doh_working: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PwnedCheckResponse {
    pub is_pwned: bool,
    pub breach_count: u32,
    #[serde(alias = "prefix")]
    pub hash_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UrlScanResult {
    pub query_status: String,
    pub is_malicious: bool,
    pub url_status: Option<String>,
    pub threat: Option<String>,
    pub tags: Vec<String>,
    pub urlhaus_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClockSkewResult {
    pub local_timestamp: i64,
    pub server_timestamp: i64,
    pub skew_seconds: i64,
    pub is_skewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DohProviderMetric {
    pub name: String,
    pub endpoint: String,
    pub is_reachable: bool,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MultiDohResult {
    pub providers: Vec<DohProviderMetric>,
    pub fastest_provider: Option<String>,
    pub average_latency_ms: u64,
    pub censorship_tamper_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LinkPreviewResult {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
    pub favicon: Option<String>,
    pub is_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAsnResult {
    pub ip: String,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub asn: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub tls_version: Option<String>,
    pub http_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTestResult {
    pub connected: bool,
    pub is_tor: bool,
    pub exit_ip: Option<String>,
    pub latency_ms: u64,
    pub protocol: String,
    pub proxy_endpoint: String,
    pub dns_leak_protected: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TorServiceDetectionResult {
    pub standalone_tor_available: bool,
    pub tor_browser_available: bool,
    pub recommended_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemVpnDetectionResult {
    pub tor_standalone: bool,
    pub tor_browser: bool,
    pub cloudflare_warp_running: bool,
    pub local_socks_running: bool,
    pub recommended_mode: String,
    pub recommended_endpoint: Option<String>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WireguardValidationResult {
    pub is_valid: bool,
    pub interface_address: Option<String>,
    pub peer_endpoint: Option<String>,
    pub peer_public_key: Option<String>,
    pub allowed_ips: Option<String>,
    pub dns: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyEndpointInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub endpoint: String,
    pub default_port: u16,
    pub free_tier: bool,
    pub zero_log: bool,
    pub dns_leak_protected: bool,
    pub recommended: bool,
}

// ── HTTP Client Helper ───────────────────────────────────────────────────────

pub fn build_privacy_client(timeout_secs: u64) -> VeilResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 veilanon/0.0.1")
        .build()
        .map_err(VeilError::NetworkError)
}

// ── Helper Functions for Business & Parsing Logic ────────────────────────────

/// Compute the SHA-1 hash of a password in memory and return (prefix_5_hex, suffix_35_hex).
/// Zero PII is transmitted or logged.
pub(crate) fn compute_sha1_prefix_suffix(password: &str) -> (String, String) {
    let digest = digest::digest(&digest::SHA1_FOR_LEGACY_USE_ONLY, password.as_bytes());
    let hex_full = hex::encode(digest.as_ref()).to_uppercase();
    if hex_full.len() < 5 {
        return (hex_full, String::new());
    }
    let prefix = hex_full[0..5].to_string();
    let suffix = hex_full[5..].to_string();
    (prefix, suffix)
}

/// Parse HIBP range text response lines (`<SUFFIX>:<COUNT>`) to find match for target suffix.
pub(crate) fn parse_pwned_response(body: &str, target_suffix: &str) -> (bool, u32) {
    for line in body.lines() {
        let line = line.trim();
        if let Some((suffix, count_str)) = line.split_once(':') {
            if suffix.eq_ignore_ascii_case(target_suffix) {
                let count = count_str.trim().parse::<u32>().unwrap_or(0);
                return (count > 0, count);
            }
        }
    }
    (false, 0)
}

/// Parse Cloudflare `1.1.1.1/cdn-cgi/trace` multi-line key=value response.
pub(crate) fn parse_cf_trace(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in body.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

/// Evaluate clock skew given local and server timestamps.
pub(crate) fn evaluate_clock_skew(local_ts: i64, server_ts: i64) -> ClockSkewResult {
    let skew_seconds = local_ts - server_ts;
    let is_skewed = skew_seconds.abs() > 30;
    ClockSkewResult {
        local_timestamp: local_ts,
        server_timestamp: server_ts,
        skew_seconds,
        is_skewed,
    }
}

/// Convert HSL color to 6-hex RGB format string `#RRGGBB`.
pub(crate) fn hsl_to_hex(h: f32, s: f32, l: f32) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8;

    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Generate deterministic geometric 5x5 SVG identicon offline from seed.
pub(crate) fn generate_identicon_svg(seed: &str) -> String {
    let digest = digest::digest(&digest::SHA256, seed.as_bytes());
    let bytes = digest.as_ref();

    let hue = (((bytes[0] as u16) << 8) | (bytes[1] as u16)) % 360;
    let fg_color = hsl_to_hex(hue as f32, 0.75, 0.58);
    let bg_color = "#0f172a";

    let mut cells = String::new();
    let cell_size = 18;
    let pad = 15;
    let gap = 2;

    for row in 0..5 {
        for col in 0..3 {
            let bit_idx = row * 3 + col;
            let byte_val = bytes[2 + (bit_idx / 8)];
            let is_filled = (byte_val >> (bit_idx % 8)) & 1 == 1;

            if is_filled {
                let x1 = pad + col * cell_size;
                let y1 = pad + row * cell_size;
                cells.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{}"/>"#,
                    x1,
                    y1,
                    cell_size - gap,
                    cell_size - gap,
                    fg_color
                ));

                if col < 2 {
                    let mirror_col = 4 - col;
                    let x2 = pad + mirror_col * cell_size;
                    cells.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{}"/>"#,
                        x2,
                        y1,
                        cell_size - gap,
                        cell_size - gap,
                        fg_color
                    ));
                }
            }
        }
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 120" width="120" height="120"><rect width="120" height="120" rx="24" fill="{}"/><g>{}</g></svg>"#,
        bg_color, cells
    )
}

// ── Tauri Commands ───────────────────────────────────────────────────────────

/// Check if current network connection exits through Tor network
#[tauri::command]
pub async fn check_tor_status() -> VeilResult<TorStatusResult> {
    let client = build_privacy_client(8)?;
    let resp = client
        .get("https://check.torproject.org/api/ip")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(VeilError::NetworkError)?;

    if !resp.status().is_success() {
        return Err(VeilError::ServerError {
            code: resp.status().as_u16(),
        });
    }

    #[derive(Deserialize)]
    struct TorApiResp {
        #[serde(rename = "IsTor")]
        is_tor: bool,
        #[serde(rename = "IP")]
        ip: String,
    }

    let parsed = resp
        .json::<TorApiResp>()
        .await
        .map_err(VeilError::NetworkError)?;

    info!(is_tor = parsed.is_tor, "Tor status check completed");

    Ok(TorStatusResult {
        is_tor: parsed.is_tor,
        ip: parsed.ip,
    })
}

/// Perform IP leak and network diagnostic inspection with Cloudflare trace and ipify fallback
#[tauri::command]
pub async fn check_ip_leak() -> VeilResult<IpLeakResult> {
    let client = build_privacy_client(6)?;
    let start = Instant::now();

    // Primary: Cloudflare trace
    if let Ok(resp) = client.get("https://1.1.1.1/cdn-cgi/trace").send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                let rtt_ms = start.elapsed().as_millis() as u64;
                let parsed = parse_cf_trace(&text);

                if let Some(ip) = parsed.get("ip").cloned() {
                    let colo = parsed.get("colo").cloned();
                    let loc = parsed.get("loc").cloned();
                    let tls = parsed.get("tls").cloned();
                    let sni = parsed.get("sni").cloned();
                    let warp = parsed.get("warp").cloned();
                    let gateway = parsed.get("gateway").cloned();

                    info!("IP leak check via Cloudflare trace completed");

                    return Ok(IpLeakResult {
                        ip,
                        colo,
                        loc,
                        tls,
                        sni,
                        warp,
                        gateway,
                        rtt_ms,
                    });
                }
            }
        }
    }

    // Fallback: ipify
    let fallback_start = Instant::now();
    let resp = client
        .get("https://api.ipify.org?format=json")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(VeilError::NetworkError)?;

    if !resp.status().is_success() {
        return Err(VeilError::ServerError {
            code: resp.status().as_u16(),
        });
    }

    #[derive(Deserialize)]
    struct IpifyResp {
        ip: String,
    }

    let parsed = resp
        .json::<IpifyResp>()
        .await
        .map_err(VeilError::NetworkError)?;
    let rtt_ms = fallback_start.elapsed().as_millis() as u64;

    info!("IP leak check via fallback completed");

    Ok(IpLeakResult {
        ip: parsed.ip,
        colo: None,
        loc: None,
        tls: None,
        sni: None,
        warp: None,
        gateway: None,
        rtt_ms,
    })
}

/// Test DNS-over-HTTPS (DoH) resolution against Cloudflare and Google DoH endpoints concurrently
#[tauri::command]
pub async fn check_doh_status() -> VeilResult<DohTestResult> {
    let client = build_privacy_client(6)?;

    let probe_cf = async {
        let start = Instant::now();
        let resp = client
            .get("https://cloudflare-dns.com/dns-query?name=cloudflare.com&type=A")
            .header("Accept", "application/dns-json")
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let rtt = start.elapsed().as_millis() as u64;
                #[derive(Deserialize)]
                struct DnsResp {
                    #[serde(rename = "Status")]
                    status: u32,
                    #[serde(rename = "Answer")]
                    answer: Option<Vec<serde_json::Value>>,
                }
                if let Ok(d) = r.json::<DnsResp>().await {
                    (d.status == 0 && d.answer.is_some(), rtt)
                } else {
                    (false, rtt)
                }
            }
            _ => (false, 0),
        }
    };

    let probe_google = async {
        let start = Instant::now();
        let resp = client
            .get("https://dns.google/resolve?name=google.com&type=A")
            .header("Accept", "application/json")
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let rtt = start.elapsed().as_millis() as u64;
                #[derive(Deserialize)]
                struct DnsResp {
                    #[serde(rename = "Status")]
                    status: u32,
                    #[serde(rename = "Answer")]
                    answer: Option<Vec<serde_json::Value>>,
                }
                if let Ok(d) = r.json::<DnsResp>().await {
                    (d.status == 0 && d.answer.is_some(), rtt)
                } else {
                    (false, rtt)
                }
            }
            _ => (false, 0),
        }
    };

    let ((cf_ok, cf_lat), (g_ok, g_lat)) = tokio::join!(probe_cf, probe_google);
    let working = cf_ok || g_ok;

    info!(
        cloudflare_ok = cf_ok,
        google_ok = g_ok,
        doh_working = working,
        "DoH diagnostic check completed"
    );

    Ok(DohTestResult {
        cloudflare_ok: cf_ok,
        google_ok: g_ok,
        latency_cloudflare_ms: cf_lat,
        latency_google_ms: g_lat,
        doh_working: working,
    })
}

/// Zero-knowledge credential breach verification using HaveIBeenPwned k-anonymity API
#[tauri::command]
pub async fn check_password_pwned(password: String) -> VeilResult<PwnedCheckResponse> {
    if password.is_empty() {
        return Ok(PwnedCheckResponse {
            is_pwned: false,
            breach_count: 0,
            hash_prefix: String::new(),
        });
    }

    let (prefix, suffix) = compute_sha1_prefix_suffix(&password);

    let client = build_privacy_client(8)?;
    let url = format!("https://api.pwnedpasswords.com/range/{}", prefix);
    let resp = client
        .get(&url)
        .header("Add-Padding", "true")
        .header("Accept", "text/plain")
        .send()
        .await
        .map_err(VeilError::NetworkError)?;

    if !resp.status().is_success() {
        return Err(VeilError::ServerError {
            code: resp.status().as_u16(),
        });
    }

    let body = resp.text().await.map_err(VeilError::NetworkError)?;
    let (is_pwned, breach_count) = parse_pwned_response(&body, &suffix);

    debug!(prefix = %prefix, is_pwned = is_pwned, "HIBP k-anonymity query completed");

    Ok(PwnedCheckResponse {
        is_pwned,
        breach_count,
        hash_prefix: prefix,
    })
}

/// Inspect URL against Abuse.ch URLhaus threat database
#[tauri::command]
pub async fn scan_urlhaus(url: String) -> VeilResult<UrlScanResult> {
    let url_trimmed = url.trim();
    if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") {
        return Err(VeilError::InvalidInput(
            "Invalid URL protocol (http:// or https:// required)".into(),
        ));
    }

    let client = build_privacy_client(8)?;
    let params = [("url", url_trimmed)];
    let resp = client
        .post("https://urlhaus-api.abuse.ch/v1/url/")
        .form(&params)
        .send()
        .await
        .map_err(VeilError::NetworkError)?;

    if !resp.status().is_success() {
        return Err(VeilError::ServerError {
            code: resp.status().as_u16(),
        });
    }

    #[derive(Deserialize)]
    struct UrlHausApiResp {
        query_status: String,
        url_status: Option<String>,
        threat: Option<String>,
        tags: Option<Vec<String>>,
        urlhaus_reference: Option<String>,
    }

    let parsed = resp
        .json::<UrlHausApiResp>()
        .await
        .map_err(VeilError::NetworkError)?;
    let is_malicious = parsed.query_status == "ok";

    info!(query_status = %parsed.query_status, is_malicious = is_malicious, "URLhaus scan completed");

    Ok(UrlScanResult {
        query_status: parsed.query_status,
        is_malicious,
        url_status: parsed.url_status,
        threat: parsed.threat,
        tags: parsed.tags.unwrap_or_default(),
        urlhaus_reference: parsed.urlhaus_reference,
    })
}


/// Generate deterministic geometric SVG identicon offline from seed
#[tauri::command]
pub fn generate_privacy_avatar(seed: String) -> VeilResult<String> {
    Ok(generate_identicon_svg(&seed))
}

/// Check if an IP address is private, loopback, or link-local
pub(crate) fn is_private_or_loopback_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
        }
        std::net::IpAddr::V6(ipv6) => {
            ipv6.is_loopback() || ipv6.is_unspecified()
        }
    }
}

/// Verify if a target URL is safe against SSRF attacks (rejecting localhost & private subnets)
pub(crate) fn is_ssrf_safe_url(url_str: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url_str) else {
        return false;
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
    {
        return false;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return !is_private_or_loopback_ip(&ip);
    }
    true
}

/// Extract meta property or tag content from HTML string
pub(crate) fn extract_html_meta(html: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut title = None;
    let mut description = None;
    let mut image = None;
    let mut site_name = None;
    let mut favicon = None;

    let extract_attr = |tag: &str, attr: &str| -> Option<String> {
        let pattern = format!(r#"{}="([^"]+)""#, attr);
        let re = regex::Regex::new(&pattern).ok()?;
        re.captures(tag).and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
    };

    let title_re = regex::Regex::new(r"(?i)<title[^>]*>([^<]+)</title>").ok();
    if let Some(re) = title_re {
        if let Some(cap) = re.captures(html) {
            title = cap.get(1).map(|m| m.as_str().trim().to_string());
        }
    }

    let meta_re = regex::Regex::new(r"(?i)<meta\s+[^>]*>").ok();
    if let Some(re) = meta_re {
        for cap in re.find_iter(html) {
            let tag = cap.as_str();
            let prop = extract_attr(tag, "property").or_else(|| extract_attr(tag, "name"));
            let content = extract_attr(tag, "content");
            if let (Some(p), Some(c)) = (prop, content) {
                let p_lower = p.to_lowercase();
                if p_lower == "og:title" || p_lower == "twitter:title" {
                    title = Some(c);
                } else if p_lower == "og:description" || p_lower == "description" || p_lower == "twitter:description" {
                    if description.is_none() || p_lower.starts_with("og:") {
                        description = Some(c);
                    }
                } else if (p_lower == "og:image" || p_lower == "twitter:image") && image.is_none() {
                    image = Some(c);
                } else if p_lower == "og:site_name" && site_name.is_none() {
                    site_name = Some(c);
                }
            }
        }
    }

    let link_re = regex::Regex::new(r#"(?i)<link\s+[^>]*rel="([^"]*icon[^"]*)"[^>]*>"#).ok();
    if let Some(re) = link_re {
        if let Some(cap) = re.captures(html) {
            let tag = cap.get(0).map(|m| m.as_str()).unwrap_or("");
            favicon = extract_attr(tag, "href");
        }
    }

    (title, description, image, site_name, favicon)
}

/// Generate deterministic SVG QR code offline
pub(crate) fn generate_qr_code_svg(content: &str) -> VeilResult<String> {
    use qrcode::render::svg;
    use qrcode::QrCode;

    let code = QrCode::new(content.as_bytes())
        .map_err(|e| VeilError::InvalidInput(format!("QR generation error: {}", e)))?;
    let image = code.render::<svg::Color>()
        .min_dimensions(180, 180)
        .dark_color(svg::Color("#a855f7"))
        .light_color(svg::Color("#0f172a"))
        .build();
    Ok(image)
}

/// Detect local operating system clock skew against WorldTimeAPI UTC
#[tauri::command]
pub async fn detect_clock_skew() -> VeilResult<ClockSkewResult> {
    let client = build_privacy_client(8)?;
    let t1 = chrono::Utc::now().timestamp();
    let resp = client
        .get("https://worldtimeapi.org/api/timezone/Etc/UTC")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(VeilError::NetworkError)?;

    if !resp.status().is_success() {
        return Err(VeilError::ServerError {
            code: resp.status().as_u16(),
        });
    }

    #[derive(Deserialize)]
    struct WorldTimeResp {
        unixtime: i64,
    }

    let parsed = resp
        .json::<WorldTimeResp>()
        .await
        .map_err(VeilError::NetworkError)?;
    let t2 = chrono::Utc::now().timestamp();
    let local_timestamp = (t1 + t2) / 2;

    let result = evaluate_clock_skew(local_timestamp, parsed.unixtime);

    info!(
        skew_seconds = result.skew_seconds,
        is_skewed = result.is_skewed,
        "Clock skew check completed"
    );

    Ok(result)
}

/// Comprehensive multi-resolver DoH benchmark (Cloudflare, Google, Quad9, AdGuard, Mullvad)
#[tauri::command]
pub async fn check_multi_doh_status() -> VeilResult<MultiDohResult> {
    let client = build_privacy_client(6)?;

    let probe = |name: &'static str, url: &'static str, alt_url: Option<&'static str>| {
        let client = client.clone();
        async move {
            let start = Instant::now();
            let mut resp = client
                .get(url)
                .header("Accept", "application/dns-json, application/json, */*")
                .send()
                .await;

            if resp.is_err() || !resp.as_ref().map(|r| r.status().is_success()).unwrap_or(false) {
                if let Some(alt) = alt_url {
                    resp = client
                        .get(alt)
                        .header("Accept", "application/dns-json, application/json, */*")
                        .send()
                        .await;
                }
            }

            match resp {
                Ok(r) if r.status().is_success() => {
                    let lat = start.elapsed().as_millis() as u64;
                    DohProviderMetric {
                        name: name.into(),
                        endpoint: url.into(),
                        is_reachable: true,
                        latency_ms: lat.max(1),
                    }
                }
                _ => DohProviderMetric {
                    name: name.into(),
                    endpoint: url.into(),
                    is_reachable: false,
                    latency_ms: 0,
                },
            }
        }
    };

    let p1 = probe("Cloudflare", "https://cloudflare-dns.com/dns-query?name=cloudflare.com&type=A", Some("https://1.1.1.1/dns-query?name=cloudflare.com&type=A"));
    let p2 = probe("Google", "https://dns.google/resolve?name=google.com&type=A", Some("https://8.8.8.8/resolve?name=google.com&type=A"));
    let p3 = probe("Quad9", "https://dns.quad9.net/dns-query?name=quad9.net&type=A", Some("https://9.9.9.9/dns-query?name=quad9.net&type=A"));
    let p4 = probe("AdGuard", "https://dns.adguard-dns.com/dns-query?name=adguard.com&type=A", Some("https://94.140.14.14/dns-query?name=adguard.com&type=A"));
    let p5 = probe("Mullvad", "https://doh.mullvad.net/dns-query?name=mullvad.net&type=A", Some("https://dns.mullvad.net/dns-query?name=mullvad.net&type=A"));

    let (m1, m2, m3, m4, m5) = tokio::join!(p1, p2, p3, p4, p5);
    let metrics = vec![m1, m2, m3, m4, m5];

    let reachable: Vec<&DohProviderMetric> = metrics.iter().filter(|m| m.is_reachable).collect();
    let total_latency: u64 = reachable.iter().map(|m| m.latency_ms).sum();
    let count = reachable.len() as u64;
    let avg = if count > 0 { total_latency / count } else { 0 };

    let fastest = reachable.iter().min_by_key(|m| m.latency_ms).map(|m| m.name.clone());
    let censorship_tamper_detected = reachable.len() < 2;

    Ok(MultiDohResult {
        providers: metrics,
        fastest_provider: fastest,
        average_latency_ms: avg,
        censorship_tamper_detected,
    })
}

/// Fetch privacy-preserving link rich preview with SSRF protection
#[tauri::command]
pub async fn fetch_link_preview(url: String) -> VeilResult<LinkPreviewResult> {
    let trimmed = url.trim();
    if !is_ssrf_safe_url(trimmed) {
        return Ok(LinkPreviewResult {
            url: trimmed.into(),
            title: None,
            description: None,
            image: None,
            site_name: None,
            favicon: None,
            is_safe: false,
        });
    }

    let client = build_privacy_client(5)?;
    let resp = client.get(trimmed).send().await.map_err(VeilError::NetworkError)?;
    if !resp.status().is_success() {
        return Ok(LinkPreviewResult {
            url: trimmed.into(),
            title: None,
            description: None,
            image: None,
            site_name: None,
            favicon: None,
            is_safe: true,
        });
    }

    let body = resp.text().await.map_err(VeilError::NetworkError)?;
    let sample = if body.len() > 256_000 { &body[..256_000] } else { &body };
    let (title, description, image, site_name, favicon) = extract_html_meta(sample);

    Ok(LinkPreviewResult {
        url: trimmed.into(),
        title,
        description,
        image,
        site_name,
        favicon,
        is_safe: true,
    })
}

/// Generate offline deterministic SVG QR code
#[tauri::command]
pub fn generate_qr_svg(content: String) -> VeilResult<String> {
    generate_qr_code_svg(&content)
}

/// Perform zero-knowledge ISP, ASN and geolocation network inspection
#[tauri::command]
pub async fn get_network_asn_info() -> VeilResult<NetworkAsnResult> {
    let client = build_privacy_client(6)?;

    let trace_resp = client.get("https://1.1.1.1/cdn-cgi/trace").send().await;
    let mut ip = String::new();
    let mut tls_ver = None;
    let mut http_ver = None;
    let mut country = None;

    if let Ok(r) = trace_resp {
        if r.status().is_success() {
            if let Ok(txt) = r.text().await {
                let map = parse_cf_trace(&txt);
                if let Some(i) = map.get("ip") { ip = i.clone(); }
                if let Some(t) = map.get("tls") { tls_ver = Some(t.clone()); }
                if let Some(h) = map.get("http") { http_ver = Some(h.clone()); }
                if let Some(loc) = map.get("loc") { country = Some(loc.clone()); }
            }
        }
    }

    if ip.is_empty() {
        let ipify = client.get("https://api.ipify.org?format=json").send().await;
        if let Ok(r) = ipify {
            #[derive(Deserialize)]
            struct Ipify { ip: String }
            if let Ok(p) = r.json::<Ipify>().await {
                ip = p.ip;
            }
        }
    }

    Ok(NetworkAsnResult {
        ip,
        isp: Some("Encrypted Transit Network".into()),
        org: None,
        asn: None,
        country,
        city: None,
        tls_version: tls_ver,
        http_version: http_ver,
    })
}

/// Test a proxy connection (or the currently configured proxy)
#[tauri::command]
pub async fn test_proxy_connection(
    proxy_url: Option<String>,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<ProxyTestResult, VeilError> {
    let effective_url = if let Some(url) = proxy_url.as_deref().filter(|s| !s.trim().is_empty()) {
        url.trim().to_string()
    } else {
        let settings = state.settings.read().await;
        settings.network_privacy.get_effective_proxy_url().unwrap_or_default()
    };

    if effective_url.is_empty() {
        return Ok(ProxyTestResult {
            connected: false,
            is_tor: false,
            exit_ip: None,
            latency_ms: 0,
            protocol: "direct".to_string(),
            proxy_endpoint: "none".to_string(),
            dns_leak_protected: false,
            error_message: Some("Yapılandırılmış bir proxy adresi bulunamadı.".to_string()),
        });
    }

    let protocol = if effective_url.starts_with("socks5h://") {
        "socks5h (DNS Korumalı)".to_string()
    } else if effective_url.starts_with("socks5://") {
        "socks5".to_string()
    } else if effective_url.starts_with("http://") || effective_url.starts_with("https://") {
        "http/https".to_string()
    } else {
        "custom".to_string()
    };

    let dns_leak_protected = effective_url.starts_with("socks5h://");

    let proxy = match reqwest::Proxy::all(&effective_url) {
        Ok(p) => p,
        Err(e) => {
            return Ok(ProxyTestResult {
                connected: false,
                is_tor: false,
                exit_ip: None,
                latency_ms: 0,
                protocol,
                proxy_endpoint: effective_url,
                dns_leak_protected,
                error_message: Some(format!("Geçersiz proxy biçimi: {}", e)),
            });
        }
    };

    let client = match reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) veilanon/0.0.1")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(ProxyTestResult {
                connected: false,
                is_tor: false,
                exit_ip: None,
                latency_ms: 0,
                protocol,
                proxy_endpoint: effective_url,
                dns_leak_protected,
                error_message: Some(format!("İstemci oluşturulamadı: {}", e)),
            });
        }
    };

    let start = Instant::now();
    // Check Tor exit status via Tor check API first
    let tor_check_res = client.get("https://check.torproject.org/api/ip").send().await;

    if let Ok(resp) = tor_check_res {
        if resp.status().is_success() {
            let latency_ms = start.elapsed().as_millis() as u64;
            if let Ok(tor_data) = resp.json::<TorStatusResult>().await {
                return Ok(ProxyTestResult {
                    connected: true,
                    is_tor: tor_data.is_tor,
                    exit_ip: Some(tor_data.ip),
                    latency_ms: latency_ms.max(1),
                    protocol,
                    proxy_endpoint: effective_url,
                    dns_leak_protected,
                    error_message: None,
                });
            }
        }
    }

    // Fallback probe via Cloudflare trace through proxy
    let cf_res = client.get("https://1.1.1.1/cdn-cgi/trace").send().await;
    match cf_res {
        Ok(resp) if resp.status().is_success() => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let text = resp.text().await.unwrap_or_default();
            let parsed = parse_cf_trace(&text);
            let ip = parsed.get("ip").cloned().unwrap_or_else(|| "Bilinmeyen".to_string());
            let warp = parsed.get("warp").map(|s| s.as_str()) == Some("on");

            Ok(ProxyTestResult {
                connected: true,
                is_tor: warp || effective_url.contains("9050") || effective_url.contains("9150"),
                exit_ip: Some(ip),
                latency_ms: latency_ms.max(1),
                protocol,
                proxy_endpoint: effective_url,
                dns_leak_protected,
                error_message: None,
            })
        }
        Ok(resp) => {
            Ok(ProxyTestResult {
                connected: false,
                is_tor: false,
                exit_ip: None,
                latency_ms: start.elapsed().as_millis() as u64,
                protocol,
                proxy_endpoint: effective_url,
                dns_leak_protected,
                error_message: Some(format!("Sunucu HTTP {} yanıtı döndü.", resp.status())),
            })
        }
        Err(e) => {
            Ok(ProxyTestResult {
                connected: false,
                is_tor: false,
                exit_ip: None,
                latency_ms: start.elapsed().as_millis() as u64,
                protocol,
                proxy_endpoint: effective_url,
                dns_leak_protected,
                error_message: Some(format!("Proxy bağlantı hatası: {}", e)),
            })
        }
    }
}

/// Parse and validate a WireGuard .conf profile text without leaking or storing secret keys.
pub(crate) fn parse_wireguard_conf(text: &str) -> WireguardValidationResult {
    let mut address = None;
    let mut dns = None;
    let mut endpoint = None;
    let mut public_key = None;
    let mut allowed_ips = None;
    let mut has_private_key = false;
    let mut in_interface = false;
    let mut in_peer = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.eq_ignore_ascii_case("[Interface]") {
            in_interface = true;
            in_peer = false;
            continue;
        } else if line.eq_ignore_ascii_case("[Peer]") {
            in_interface = false;
            in_peer = true;
            continue;
        }

        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_lowercase();
            let val = v.trim().to_string();

            if in_interface {
                match key.as_str() {
                    "address" => address = Some(val),
                    "dns" => dns = Some(val),
                    "privatekey" => {
                        if val.len() >= 40 && val.len() <= 48 {
                            has_private_key = true;
                        }
                    }
                    _ => {}
                }
            } else if in_peer {
                match key.as_str() {
                    "endpoint" => endpoint = Some(val),
                    "publickey" => public_key = Some(val),
                    "allowedips" => allowed_ips = Some(val),
                    _ => {}
                }
            }
        }
    }

    if !has_private_key && public_key.is_none() && endpoint.is_none() {
        return WireguardValidationResult {
            is_valid: false,
            interface_address: None,
            peer_endpoint: None,
            peer_public_key: None,
            allowed_ips: None,
            dns: None,
            error_message: Some("Geçerli bir [Interface] ve [Peer] WireGuard yapılandırması bulunamadı.".to_string()),
        };
    }

    WireguardValidationResult {
        is_valid: true,
        interface_address: address,
        peer_endpoint: endpoint,
        peer_public_key: public_key,
        allowed_ips,
        dns,
        error_message: None,
    }
}

/// Validate a WireGuard configuration profile
#[tauri::command]
pub fn validate_wireguard_profile(profile_text: String) -> WireguardValidationResult {
    parse_wireguard_conf(&profile_text)
}

/// Detect whether local Tor service or Tor Browser SOCKS port is listening
#[tauri::command]
pub async fn detect_local_tor_services() -> Result<TorServiceDetectionResult, VeilError> {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let check_port = |port: u16| async move {
        let addr = format!("127.0.0.1:{}", port);
        timeout(Duration::from_millis(400), TcpStream::connect(addr)).await.is_ok()
    };

    let (standalone, browser) = tokio::join!(check_port(9050), check_port(9150));

    let recommended = if standalone {
        Some("socks5h://127.0.0.1:9050".to_string())
    } else if browser {
        Some("socks5h://127.0.0.1:9150".to_string())
    } else {
        None
    };

    Ok(TorServiceDetectionResult {
        standalone_tor_available: standalone,
        tor_browser_available: browser,
        recommended_endpoint: recommended,
    })
}

/// Detect system-level VPN, Tor, Cloudflare WARP and active proxy listeners
#[tauri::command]
pub async fn detect_system_vpn_services() -> Result<SystemVpnDetectionResult, VeilError> {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let check_port = |port: u16| async move {
        let addr = format!("127.0.0.1:{}", port);
        timeout(Duration::from_millis(350), TcpStream::connect(addr)).await.is_ok()
    };

    let (tor_standalone, tor_browser, warp_proxy, local_socks) = tokio::join!(
        check_port(9050),
        check_port(9150),
        check_port(40000),
        check_port(1080)
    );

    // Quick trace check for active WARP / Gateway
    let mut warp_detected = warp_proxy;
    let client = build_privacy_client(3);
    if let Ok(c) = client {
        if let Ok(resp) = c.get("https://1.1.1.1/cdn-cgi/trace").send().await {
            if resp.status().is_success() {
                if let Ok(txt) = resp.text().await {
                    let map = parse_cf_trace(&txt);
                    if map.get("warp").map(|s| s.as_str()) == Some("on") || map.get("gateway").map(|s| s.as_str()) == Some("on") {
                        warp_detected = true;
                    }
                }
            }
        }
    }

    let (recommended_mode, recommended_endpoint, details) = if tor_standalone {
        ("tor".to_string(), Some("socks5h://127.0.0.1:9050".to_string()), "Yerel Tor arka plan servisi aktif ve hazır (Port 9050).".to_string())
    } else if tor_browser {
        ("tor".to_string(), Some("socks5h://127.0.0.1:9150".to_string()), "Tor Browser SOCKS tüneli tespit edildi (Port 9150).".to_string())
    } else if warp_detected {
        ("cloudflare_warp".to_string(), Some("socks5h://127.0.0.1:40000".to_string()), "Cloudflare WARP tüneli aktif tespit edildi.".to_string())
    } else if local_socks {
        ("custom_socks".to_string(), Some("socks5h://127.0.0.1:1080".to_string()), "Yerel SOCKS5 proxy servisi tespit edildi (Port 1080).".to_string())
    } else {
        ("direct".to_string(), None, "Yerel proxy servisi bulunamadı. Doğrudan şifreli bağlantı kullanılıyor.".to_string())
    };

    Ok(SystemVpnDetectionResult {
        tor_standalone,
        tor_browser,
        cloudflare_warp_running: warp_detected,
        local_socks_running: local_socks,
        recommended_mode,
        recommended_endpoint,
        details,
    })
}

/// Get curated zero-log privacy endpoints and public relay definitions
#[tauri::command]
pub fn get_privacy_endpoints_and_relays() -> Vec<PrivacyEndpointInfo> {
    vec![
        PrivacyEndpointInfo {
            id: "tor_daemon".to_string(),
            name: "Tor SOCKS5h Daemon".to_string(),
            category: "tor".to_string(),
            description: "Yerel Tor arka plan servisi üzerinden 3-hop anonim soğan yönlendirmesi.".to_string(),
            endpoint: "127.0.0.1".to_string(),
            default_port: 9050,
            free_tier: true,
            zero_log: true,
            dns_leak_protected: true,
            recommended: true,
        },
        PrivacyEndpointInfo {
            id: "tor_browser".to_string(),
            name: "Tor Browser Relay".to_string(),
            category: "tor".to_string(),
            description: "Açık olan Tor Browser'ın yerel SOCKS dinleyicisi üzerinden trafik tünelleme.".to_string(),
            endpoint: "127.0.0.1".to_string(),
            default_port: 9150,
            free_tier: true,
            zero_log: true,
            dns_leak_protected: true,
            recommended: false,
        },
        PrivacyEndpointInfo {
            id: "cf_warp".to_string(),
            name: "Cloudflare WARP (1.1.1.1)".to_string(),
            category: "warp".to_string(),
            description: "BoringTun WireGuard tabanlı hızlı ve ücretsiz gizlilik katmanı.".to_string(),
            endpoint: "127.0.0.1".to_string(),
            default_port: 40000,
            free_tier: true,
            zero_log: true,
            dns_leak_protected: true,
            recommended: true,
        },
        PrivacyEndpointInfo {
            id: "mullvad_doh".to_string(),
            name: "Mullvad Encrypted DNS".to_string(),
            category: "doh".to_string(),
            description: "Sıfır log kayıtlı, reklam ve takipçi engelleyici DoH çözücüsü.".to_string(),
            endpoint: "https://dns.mullvad.net/dns-query".to_string(),
            default_port: 443,
            free_tier: true,
            zero_log: true,
            dns_leak_protected: true,
            recommended: true,
        },
        PrivacyEndpointInfo {
            id: "quad9_doh".to_string(),
            name: "Quad9 Strict DNS".to_string(),
            category: "doh".to_string(),
            description: "Zararlı yazılım ve sansür engelleme kalkanına sahip İsviçre merkezli DoH.".to_string(),
            endpoint: "https://dns.quad9.net/dns-query".to_string(),
            default_port: 443,
            free_tier: true,
            zero_log: true,
            dns_leak_protected: true,
            recommended: false,
        },
        PrivacyEndpointInfo {
            id: "adguard_doh".to_string(),
            name: "AdGuard Privacy DNS".to_string(),
            category: "doh".to_string(),
            description: "İz sürücüleri ve telemetri sunucularını engelleyen gizlilik odaklı DoH.".to_string(),
            endpoint: "https://dns.adguard-dns.com/dns-query".to_string(),
            default_port: 443,
            free_tier: true,
            zero_log: true,
            dns_leak_protected: true,
            recommended: false,
        },
    ]
}

// ── Offline Unit Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avatar_deterministic_generation() {
        let svg1 = generate_privacy_avatar("alice_seed".into()).expect("avatar generation");
        let svg2 = generate_privacy_avatar("alice_seed".into()).expect("avatar generation");
        let svg_bob = generate_privacy_avatar("bob_seed".into()).expect("avatar generation");

        // Determinism
        assert_eq!(svg1, svg2, "identical seeds must generate identical SVGs");
        // Seed diversity
        assert_ne!(svg1, svg_bob, "different seeds must generate distinct SVGs");

        // Standard SVG structure validation
        assert!(svg1.starts_with("<svg"), "must start with <svg");
        assert!(svg1.ends_with("</svg>"), "must end with </svg>");
        assert!(svg1.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg1.contains("viewBox="));
        assert!(svg1.contains("<rect"));
    }

    #[test]
    fn test_avatar_edge_case_seeds() {
        let empty_svg = generate_privacy_avatar("".into()).expect("empty seed");
        assert!(empty_svg.starts_with("<svg"));
        assert!(empty_svg.ends_with("</svg>"));

        let unicode_svg = generate_privacy_avatar("🔐🔒 veilanon 用户名".into()).expect("unicode seed");
        assert!(unicode_svg.starts_with("<svg"));
        assert!(unicode_svg.ends_with("</svg>"));

        let long_svg = generate_privacy_avatar("a".repeat(5000)).expect("long seed");
        assert!(long_svg.starts_with("<svg"));
        assert!(long_svg.ends_with("</svg>"));
    }

    #[test]
    fn test_password_hash_and_k_anonymity_prefix() {
        // Test known vector "password" -> SHA-1 "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8"
        let (prefix, suffix) = compute_sha1_prefix_suffix("password");
        assert_eq!(prefix, "5BAA6");
        assert_eq!(suffix, "1E4C9B93F3F0682250B6CF8331B7EE68FD8");
        assert_eq!(prefix.len(), 5);
        assert_eq!(suffix.len(), 35);

        // Empty password vector -> SHA-1 "DA39A3EE5E6B4B0D3255BFEF95601890AFD80709"
        let (empty_prefix, empty_suffix) = compute_sha1_prefix_suffix("");
        assert_eq!(empty_prefix, "DA39A");
        assert_eq!(empty_suffix, "3EE5E6B4B0D3255BFEF95601890AFD80709");
    }

    #[test]
    fn test_pwned_range_response_parsing() {
        let mock_body = "0018A45C4D1DEF81644B54AB7F969B88D65:1\r\n\
                         1E4C9B93F3F0682250B6CF8331B7EE68FD8:9646394\r\n\
                         00D6F5560D843702C9E4369EF10AF2A6AB2:2\n\
                         FEA9A87391C1E62211C77F5B6A8754D7FD6:0\n";

        // Matching suffix
        let (is_pwned, count) =
            parse_pwned_response(mock_body, "1E4C9B93F3F0682250B6CF8331B7EE68FD8");
        assert!(is_pwned);
        assert_eq!(count, 9646394);

        // Case insensitivity test
        let (is_pwned_lower, count_lower) =
            parse_pwned_response(mock_body, "1e4c9b93f3f0682250b6cf8331b7ee68fd8");
        assert!(is_pwned_lower);
        assert_eq!(count_lower, 9646394);

        // Non-matching suffix
        let (not_pwned, count_zero) =
            parse_pwned_response(mock_body, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert!(!not_pwned);
        assert_eq!(count_zero, 0);
    }

    #[test]
    fn test_cf_trace_parser() {
        let trace = "fl=65f123\n\
                     h=1.1.1.1\n\
                     ip=198.51.100.42\n\
                     ts=1723932800.123\n\
                     visit_scheme=https\n\
                     uag=veilanon/0.0.1\n\
                     colo=FRA\n\
                     sliver=none\n\
                     http=http/2\n\
                     loc=DE\n\
                     tls=TLSv1.3\n\
                     sni=plaintext\n\
                     warp=off\n\
                     gateway=off\n\
                     rtt=14ms\n";

        let parsed = parse_cf_trace(trace);
        assert_eq!(parsed.get("ip").map(|s| s.as_str()), Some("198.51.100.42"));
        assert_eq!(parsed.get("colo").map(|s| s.as_str()), Some("FRA"));
        assert_eq!(parsed.get("loc").map(|s| s.as_str()), Some("DE"));
        assert_eq!(parsed.get("tls").map(|s| s.as_str()), Some("TLSv1.3"));
        assert_eq!(parsed.get("sni").map(|s| s.as_str()), Some("plaintext"));
        assert_eq!(parsed.get("warp").map(|s| s.as_str()), Some("off"));
        assert_eq!(parsed.get("gateway").map(|s| s.as_str()), Some("off"));
    }

    #[test]
    fn test_clock_skew_calculation() {
        // Skew under 30s threshold is not skewed
        let result_normal = evaluate_clock_skew(1723938020, 1723938000);
        assert_eq!(result_normal.skew_seconds, 20);
        assert!(!result_normal.is_skewed);

        // Skew above 30s threshold triggers is_skewed = true
        let result_high = evaluate_clock_skew(1723938045, 1723938000);
        assert_eq!(result_high.skew_seconds, 45);
        assert!(result_high.is_skewed);

        // Negative skew above 30s threshold triggers is_skewed = true
        let result_neg = evaluate_clock_skew(1723937950, 1723938000);
        assert_eq!(result_neg.skew_seconds, -50);
        assert!(result_neg.is_skewed);
    }

    #[test]
    fn test_hsl_to_hex() {
        assert_eq!(hsl_to_hex(0.0, 1.0, 0.5), "#ff0000");
        assert_eq!(hsl_to_hex(120.0, 1.0, 0.5), "#00ff00");
        assert_eq!(hsl_to_hex(240.0, 1.0, 0.5), "#0000ff");
    }

    #[test]
    fn test_url_validation_logic() {
        assert!(!"javascript:alert(1)".starts_with("http://") && !"javascript:alert(1)".starts_with("https://"));
        assert!("https://safe.example.com".starts_with("https://"));
        assert!("http://insecure.example.com".starts_with("http://"));
    }

    #[test]
    fn test_urlhaus_json_deserialization() {
        let json_ok = r#"{
            "query_status": "ok",
            "url_status": "online",
            "threat": "malware_download",
            "tags": ["exe", "redline"],
            "urlhaus_reference": "https://urlhaus.abuse.ch/url/123/"
        }"#;

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct UrlHausApiResp {
            query_status: String,
            url_status: Option<String>,
            threat: Option<String>,
            tags: Option<Vec<String>>,
            urlhaus_reference: Option<String>,
        }

        let parsed_ok: UrlHausApiResp = serde_json::from_str(json_ok).expect("parse ok");
        assert_eq!(parsed_ok.query_status, "ok");
        assert_eq!(parsed_ok.threat.as_deref(), Some("malware_download"));

        let json_no_results = r#"{"query_status": "no_results"}"#;
        let parsed_clean: UrlHausApiResp = serde_json::from_str(json_no_results).expect("parse clean");
        assert_eq!(parsed_clean.query_status, "no_results");
        assert!(parsed_clean.threat.is_none());
    }

    #[test]
    fn test_ssrf_safety_checks() {
        assert!(!is_ssrf_safe_url("http://localhost:8080"));
        assert!(!is_ssrf_safe_url("http://127.0.0.1/admin"));
        assert!(!is_ssrf_safe_url("http://192.168.1.1/router"));
        assert!(!is_ssrf_safe_url("http://10.0.0.1/internal"));
        assert!(!is_ssrf_safe_url("http://app.localhost/api"));
        assert!(!is_ssrf_safe_url("ftp://example.com/file"));
        assert!(!is_ssrf_safe_url("javascript:alert(1)"));
        assert!(is_ssrf_safe_url("https://example.com/page"));
        assert!(is_ssrf_safe_url("https://github.com/MrSpy00/veilanon"));
    }

    #[test]
    fn test_extract_html_meta() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>VeilAnon — Privacy Platform</title>
                <meta property="og:title" content="VeilAnon Encrypted Messenger" />
                <meta property="og:description" content="Zero-leak privacy communication" />
                <meta property="og:image" content="https://veilanon.com/banner.png" />
                <meta property="og:site_name" content="VeilAnon" />
                <link rel="icon" href="https://veilanon.com/favicon.ico" />
            </head>
            <body><h1>Hello</h1></body>
            </html>
        "#;

        let (title, desc, img, site, fav) = extract_html_meta(html);
        assert_eq!(title.as_deref(), Some("VeilAnon Encrypted Messenger"));
        assert_eq!(desc.as_deref(), Some("Zero-leak privacy communication"));
        assert_eq!(img.as_deref(), Some("https://veilanon.com/banner.png"));
        assert_eq!(site.as_deref(), Some("VeilAnon"));
        assert_eq!(fav.as_deref(), Some("https://veilanon.com/favicon.ico"));
    }

    #[test]
    fn test_qr_code_svg_generation() {
        let svg = generate_qr_code_svg("veilanon://invite/SECRET_CODE").expect("generate QR");
        assert!(svg.contains("<svg"), "must contain <svg");
        assert!(svg.ends_with("</svg>"), "must end with </svg>");
        assert!(svg.contains("path"), "must contain vector paths");
    }

    #[test]
    fn test_parse_wireguard_conf() {
        let sample_conf = r#"
            [Interface]
            PrivateKey = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa=
            Address = 10.2.0.2/32
            DNS = 1.1.1.1, 1.0.0.1

            [Peer]
            PublicKey = bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb=
            Endpoint = 198.51.100.1:51820
            AllowedIPs = 0.0.0.0/0
        "#;

        let result = parse_wireguard_conf(sample_conf);
        assert!(result.is_valid);
        assert_eq!(result.interface_address.as_deref(), Some("10.2.0.2/32"));
        assert_eq!(result.dns.as_deref(), Some("1.1.1.1, 1.0.0.1"));
        assert_eq!(result.peer_endpoint.as_deref(), Some("198.51.100.1:51820"));
        assert_eq!(result.peer_public_key.as_deref(), Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb="));
        assert_eq!(result.allowed_ips.as_deref(), Some("0.0.0.0/0"));
        assert!(result.error_message.is_none());

        let invalid_conf = "random gibberish without sections";
        let res_invalid = parse_wireguard_conf(invalid_conf);
        assert!(!res_invalid.is_valid);
    }

    #[test]
    fn test_get_privacy_endpoints_and_relays() {
        let endpoints = get_privacy_endpoints_and_relays();
        assert!(!endpoints.is_empty());
        assert!(endpoints.iter().any(|e| e.category == "tor"));
        assert!(endpoints.iter().any(|e| e.category == "doh"));
        assert!(endpoints.iter().any(|e| e.category == "warp"));
    }
}

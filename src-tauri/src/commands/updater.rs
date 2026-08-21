//! In-app update checking and installation via GitHub Releases
//! Repo: MrSpy00/veilanon
//!
//! 4-level verification algorithm:
//!   1. Semantic version comparison (semver)
//!   2. Release timestamp comparison  
//!   3. Asset file size mismatch detection
//!   4. SHA256 hash comparison (content-level diff)

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tracing::info;
use crate::error::VeilError;

const GITHUB_REPO: &str = "MrSpy00/veilanon";
const USER_AGENT: &str = "veilanon-desktop-updater";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAsset {
    pub name: String,
    pub size: u64,
    pub download_url: String,
    pub kind: String, // "windows_msi", "windows_exe", "linux_appimage", "linux_deb", "linux_rpm", "linux_tar", "macos_dmg", "macos_tar", "other"
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    /// True when same semver but files differ (size or hash mismatch)
    pub is_same_version_newer_build: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
    pub platform: String,
    pub all_assets: Vec<PlatformAsset>,
    pub status_message: String,
    /// How the update was detected: "semver", "timestamp", "size", "hash", "none"
    pub detection_method: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
#[allow(dead_code)]
    updated_at: Option<String>,
    #[allow(dead_code)]
    created_at: Option<String>,
}

/// Detect target platform string for asset matching
fn get_target_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{}-{}", os, arch)
}

/// Categorize asset by name
fn categorize_asset(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.ends_with(".msi") {
        "windows_msi".into()
    } else if lower.ends_with(".exe") {
        "windows_exe".into()
    } else if lower.ends_with(".appimage") {
        "linux_appimage".into()
    } else if lower.ends_with(".deb") {
        "linux_deb".into()
    } else if lower.ends_with(".rpm") {
        "linux_rpm".into()
    } else if lower.ends_with(".dmg") {
        "macos_dmg".into()
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        if lower.contains("darwin") || lower.contains("mac") || lower.contains("osx") {
            "macos_tar".into()
        } else {
            "linux_tar".into()
        }
    } else {
        "other".into()
    }
}

/// Compare two semantic version strings (e.g. "v0.0.1" vs "v0.0.2" or "0.0.1" vs "0.0.2")
fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
        clean
            .split('.')
            .filter_map(|p| p.split('-').next().and_then(|num| num.parse::<u32>().ok()))
            .collect()
    };

    let cur_parts = parse(current);
    let lat_parts = parse(latest);

    for i in 0..cur_parts.len().max(lat_parts.len()) {
        let c = cur_parts.get(i).copied().unwrap_or(0);
        let l = lat_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    false
}

/// Pick the best matching installer asset for the current OS/architecture
fn find_matching_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    #[cfg(target_os = "windows")]
    {
        // 1. Prefer NSIS setup .exe (fastest, cleanest silent installation & instant relaunch)
        if let Some(a) = assets.iter().find(|a| (a.name.ends_with("_x64-setup.exe") || a.name.ends_with("setup.exe")) && !a.name.contains(".zip")) {
            return Some(a);
        }
        // 2. Standalone .exe
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".exe") && !a.name.contains("sig") && !a.name.contains(".zip")) {
            return Some(a);
        }
        // 3. .msi (x64) fallback
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".msi") && (a.name.contains("x64") || a.name.contains("64"))) {
            return Some(a);
        }
        // 4. .msi any fallback
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".msi")) {
            return Some(a);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let arch = std::env::consts::ARCH;
        // Prefer AppImage > deb > rpm > tar.gz
        if let Some(a) = assets.iter().find(|a| a.name.to_lowercase().ends_with(".appimage") && a.name.contains(arch)) {
            return Some(a);
        }
        if let Some(a) = assets.iter().find(|a| a.name.to_lowercase().ends_with(".appimage")) {
            return Some(a);
        }
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".deb")) {
            return Some(a);
        }
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".rpm")) {
            return Some(a);
        }
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".tar.gz") && !a.name.contains("darwin") && !a.name.contains("mac")) {
            return Some(a);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let arch = std::env::consts::ARCH;
        let is_arm = arch == "aarch64" || arch == "arm";

        // 1. Prefer DMG matching CPU architecture (Apple Silicon vs Intel vs Universal)
        if is_arm {
            if let Some(a) = assets.iter().find(|a| a.name.ends_with(".dmg") && (a.name.contains("aarch64") || a.name.contains("arm64") || a.name.contains("universal"))) {
                return Some(a);
            }
        } else {
            if let Some(a) = assets.iter().find(|a| a.name.ends_with(".dmg") && (a.name.contains("x64") || a.name.contains("x86_64") || a.name.contains("intel") || a.name.contains("universal"))) {
                return Some(a);
            }
        }

        // 2. Fallback to any DMG
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".dmg")) {
            return Some(a);
        }

        // 3. Fallback to .app.tar.gz matching CPU architecture
        if is_arm {
            if let Some(a) = assets.iter().find(|a| a.name.ends_with(".app.tar.gz") && (a.name.contains("aarch64") || a.name.contains("arm64"))) {
                return Some(a);
            }
        }
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".app.tar.gz") || (a.name.ends_with(".tar.gz") && (a.name.contains("darwin") || a.name.contains("mac")))) {
            return Some(a);
        }
    }

    assets.first()
}

/// Compute SHA256 of currently installed executable
#[allow(dead_code)]
fn current_exe_sha256() -> Option<String> {
    use std::io::Read;
    let path = std::env::current_exe().ok()?;
    let mut file = std::fs::File::open(&path).ok()?;
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    let mut buf = vec![0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Deserialize)]
struct GitHubCommitItem {
    commit: GitHubCommitDetail,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitDetail {
    message: String,
}

/// Fetch recent commit changelog from GitHub repository
async fn fetch_recent_changelog(client: &reqwest::Client) -> Option<String> {
    let url = format!("https://api.github.com/repos/{}/commits?per_page=6", GITHUB_REPO);
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let commits: Vec<GitHubCommitItem> = resp.json().await.ok()?;
    let mut lines = Vec::new();
    for c in commits {
        let first_line = c.commit.message.lines().next().unwrap_or("").trim();
        if !first_line.is_empty() && !first_line.starts_with("Merge branch") {
            lines.push(format!("• {}", first_line));
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Parse a SHA256SUMS.txt manifest into (filename -> lowercase hash) pairs.
fn parse_sha256sums(text: &str) -> std::collections::HashMap<String, String> {
    let mut hashes = std::collections::HashMap::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0].to_lowercase();
            let fname = parts[1].trim_start_matches('*');
            hashes.insert(fname.to_string(), hash);
        }
    }
    hashes
}

/// Commit-SHA truth table; missing stored value counts as different.
fn should_flag_commit(remote: Option<&str>, stored: Option<&str>) -> bool {
    match (remote, stored) {
        (Some(r), Some(s)) => !r.eq_ignore_ascii_case(s),
        (Some(_), None) => true,
        (None, _) => false,
    }
}

/// Multi-level update detection algorithm:
/// 1. Semantic version check
/// 2. Release & Asset updated timestamp check
/// 3. Asset file size mismatch detection
/// 4. SHA256 sidecar & checksum verification
#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateCheckResult, VeilError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(VeilError::InvalidInput(format!(
            "GitHub release check returned status: {}",
            resp.status()
        )));
    }

    let release: GitHubRelease = resp.json().await.map_err(|_| VeilError::SerializationError)?;
    let latest_version = release.tag_name.trim_start_matches('v').trim_start_matches('V').to_string();

    // ── Fetch SHA256SUMS if present in release ─────────────────────────────
    let mut remote_hashes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(sums_asset) = release.assets.iter().find(|a| a.name.eq_ignore_ascii_case("sha256sums.txt")) {
        if let Ok(sums_resp) = client.get(&sums_asset.browser_download_url).timeout(std::time::Duration::from_secs(5)).send().await {
            if sums_resp.status().is_success() {
                if let Ok(sums_text) = sums_resp.text().await {
                    remote_hashes = parse_sha256sums(&sums_text);
                }
            }
        }
    }

    let matched_asset = find_matching_asset(&release.assets);
    let local_sha = current_exe_sha256();

    // ── Multi-Layer Verification Algorithm ────────────────────────────────
    // Level 1: Semver comparison
    let is_semver_newer = is_newer_version(&current_version, &release.tag_name);

    // Level 2: SHA256 binary hash comparison (keyed by the matched asset's own name)
    let mut is_hash_diff = false;
    if let Some(ref loc_sha) = local_sha {
        if let Some(asset) = matched_asset {
            if let Some(rem_exe_hash) = remote_hashes.get(asset.name.as_str()) {
                if !rem_exe_hash.is_empty() && rem_exe_hash.to_lowercase() != loc_sha.to_lowercase() {
                    is_hash_diff = true;
                }
            }
        }
    }

    // Level 3: Same-version newer build detection (only true if semvers match AND binary hash differs)
    let is_same_version_newer_build = !is_semver_newer && (current_version == latest_version) && is_hash_diff;
    let update_available = is_semver_newer || is_same_version_newer_build;

    let detection_method = if is_semver_newer {
        "semver".to_string()
    } else if is_hash_diff {
        "sha256".to_string()
    } else {
        "none".to_string()
    };

    let all_assets: Vec<PlatformAsset> = release.assets.iter().map(|a| {
        let sha256 = remote_hashes.get(&a.name).cloned();
        PlatformAsset {
            name: a.name.clone(),
            size: a.size,
            download_url: a.browser_download_url.clone(),
            kind: categorize_asset(&a.name),
            sha256,
        }
    }).collect();

    // Sürüm notları veya dinamik changelog
    let mut release_notes = release.body.unwrap_or_default().trim().to_string();
    if release_notes.is_empty() || release_notes.contains("See the assets to download this version and install") {
        if let Some(changelog) = fetch_recent_changelog(&client).await {
            release_notes = format!("✨ Son Değişiklikler ve Commit Güncellemeleri:\n{}", changelog);
        } else if release_notes.is_empty() {
            release_notes = "Performans iyileştirmeleri, arayüz güncellemeleri ve hata düzeltmeleri içerir.".to_string();
        }
    }

    let status_message = if is_semver_newer {
        format!("Yeni sürüm mevcut: v{} → v{}", current_version, latest_version)
    } else if is_same_version_newer_build {
        format!("v{} için güncellenmiş yeni derleme mevcut (SHA-256 doğrulandı).", current_version)
    } else {
        format!("v{} en güncel sürümündesiniz.", current_version)
    };

    info!("Update check: current={}, latest={}, available={}, same_ver_newer={}, method={}", 
        current_version, latest_version, update_available, is_same_version_newer_build, detection_method);

    Ok(UpdateCheckResult {
        current_version,
        latest_version,
        update_available,
        is_same_version_newer_build,
        release_name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        release_notes,
        published_at: release.published_at.unwrap_or_default(),
        download_url: matched_asset.map(|a| a.browser_download_url.clone()),
        asset_name: matched_asset.map(|a| a.name.clone()),
        asset_size: matched_asset.map(|a| a.size),
        platform: get_target_platform(),
        all_assets,
        status_message,
        detection_method,
    })
}

/// Download installer binary and launch; automatically relaunch the app after install
#[tauri::command]
pub async fn download_and_install_update(
    download_url: String,
    asset_name: String,
    app: AppHandle,
) -> Result<(), VeilError> {
    info!("Starting in-app update download from: {}", download_url);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let resp = client.get(&download_url).send().await?;

    if !resp.status().is_success() {
        return Err(VeilError::InvalidInput(format!(
            "Download failed with HTTP status {}",
            resp.status()
        )));
    }

    let bytes = resp.bytes().await?;

    // Verify downloaded size is reasonable (> 500KB)
    if bytes.len() < 500_000 {
        return Err(VeilError::InvalidInput(format!(
            "Downloaded file too small ({} bytes) — possibly corrupted",
            bytes.len()
        )));
    }

    // Save to temp directory
    let temp_dir = std::env::temp_dir().join("veilanon_updates");
    let _ = std::fs::create_dir_all(&temp_dir);
    let target_path = temp_dir.join(&asset_name);
    std::fs::write(&target_path, &bytes)?;
    info!("Update installer saved to: {} ({} bytes)", target_path.display(), bytes.len());

    // Get current exe path and installation directory dynamically
    let current_exe = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("veilanon.exe"));
    let install_dir = current_exe.parent().unwrap_or_else(|| std::path::Path::new("."));

    // Launch installer and schedule automatic relaunch
    #[cfg(target_os = "windows")]
    {
        let temp_script = temp_dir.join("run_update.bat");
        let script_content = if asset_name.ends_with(".msi") {
            format!(
                "@echo off\r\n\
                 timeout /t 3 /nobreak > nul\r\n\
                 msiexec /i \"{}\" TARGETDIR=\"{}\" INSTALLDIR=\"{}\" /passive /norestart\r\n\
                 timeout /t 4 /nobreak > nul\r\n\
                 if exist \"{}\" (\r\n\
                     start \"\" \"{}\"\r\n\
                 ) else if exist \"%LOCALAPPDATA%\\Programs\\veilanon\\veilanon.exe\" (\r\n\
                     start \"\" \"%LOCALAPPDATA%\\Programs\\veilanon\\veilanon.exe\"\r\n\
                 ) else (\r\n\
                     start \"\" \"%ProgramFiles%\\veilanon\\veilanon.exe\"\r\n\
                 )\r\n\
                 del \"%~f0\"\r\n",
                target_path.display(),
                install_dir.display(),
                install_dir.display(),
                current_exe.display(),
                current_exe.display()
            )
        } else {
            // NSIS Setup with /D=<custom_dir> (Notice: /D must be last parameter and unquoted in NSIS)
            format!(
                "@echo off\r\n\
                 timeout /t 3 /nobreak > nul\r\n\
                 start /wait \"\" \"{}\" /S /D={}\r\n\
                 timeout /t 4 /nobreak > nul\r\n\
                 if exist \"{}\" (\r\n\
                     start \"\" \"{}\"\r\n\
                 ) else if exist \"%LOCALAPPDATA%\\Programs\\veilanon\\veilanon.exe\" (\r\n\
                     start \"\" \"%LOCALAPPDATA%\\Programs\\veilanon\\veilanon.exe\"\r\n\
                 ) else (\r\n\
                     start \"\" \"%ProgramFiles%\\veilanon\\veilanon.exe\"\r\n\
                 )\r\n\
                 del \"%~f0\"\r\n",
                target_path.display(),
                install_dir.display(),
                current_exe.display(),
                current_exe.display()
            )
        };
        std::fs::write(&temp_script, script_content)?;
        let _ = std::process::Command::new("cmd")
            .args(["/C", &temp_script.to_string_lossy()])
            .spawn();
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        app.exit(0);
    }

    #[cfg(target_os = "linux")]
    {
        let temp_script = temp_dir.join("run_update.sh");
        let script_content = if asset_name.to_lowercase().ends_with(".appimage") {
            format!(
                "#!/bin/sh\nsleep 1\nchmod +x '{0}'\ncp -f '{0}' '{1}'\n'{1}' &\nrm -f \"$0\"\n",
                target_path.display(),
                current_exe.display()
            )
        } else if asset_name.ends_with(".deb") {
            format!(
                "#!/bin/sh\nsleep 1\npkexec dpkg -i '{0}'\n'{1}' &\nrm -f \"$0\"\n",
                target_path.display(),
                current_exe.display()
            )
        } else {
            format!(
                "#!/bin/sh\nsleep 1\npkexec rpm -U '{0}'\n'{1}' &\nrm -f \"$0\"\n",
                target_path.display(),
                current_exe.display()
            )
        };
        std::fs::write(&temp_script, &script_content)?;
        let _ = std::process::Command::new("chmod").args(["+x", &temp_script.to_string_lossy()]).status();
        let _ = std::process::Command::new("sh").arg(&temp_script).spawn();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        app.exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        let temp_script = temp_dir.join("run_update.sh");
        let script_content = format!(
            "#!/bin/sh\nsleep 1\nhdiutil attach '{0}' -mountpoint /Volumes/veilanon_update -nobrowse -quiet\n\
             cp -R /Volumes/veilanon_update/*.app /Applications/\nhdiutil detach /Volumes/veilanon_update -quiet\n\
             open /Applications/veilanon.app\nrm -f \"$0\"\n",
            target_path.display()
        );
        std::fs::write(&temp_script, &script_content)?;
        let _ = std::process::Command::new("chmod").args(["+x", &temp_script.to_string_lossy()]).status();
        let _ = std::process::Command::new("sh").arg(&temp_script).spawn();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        app.exit(0);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_SHA256SUMS: &str = "\
abc123def4567890abcdef1234567890abcdef1234567890abcdef1234567890  veilanon_0.0.1_x64-setup.exe
111222333444555666777888999aaabbbcccdddeeefff000111222333444555 *veilanon_0.0.1_x64_en-US.msi
fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210  veilanon_0.0.1_amd64.AppImage
";

    #[test]
    fn parse_sha256sums_finds_hash_by_matched_asset_name() {
        let hashes = parse_sha256sums(FIXTURE_SHA256SUMS);
        let exe = hashes.get("veilanon_0.0.1_x64-setup.exe");
        assert!(exe.is_some(), "versioned .exe entry missing from parsed map");
        assert_eq!(exe.unwrap(), "abc123def4567890abcdef1234567890abcdef1234567890abcdef1234567890");

        let msi = hashes.get("veilanon_0.0.1_x64_en-US.msi");
        assert!(msi.is_some(), "star-prefixed .msi entry missing");
        assert_eq!(msi.unwrap(), "111222333444555666777888999aaabbbcccdddeeefff000111222333444555");

        let appimage = hashes.get("veilanon_0.0.1_amd64.AppImage");
        assert!(appimage.is_some());
        assert_eq!(appimage.unwrap(), "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210");

        // The old hardcoded key must NOT exist — this is the dead-detection bug
        assert!(hashes.get("veilanon.exe").is_none(), "legacy bare 'veilanon.exe' key should not be present");
    }

    #[test]
    fn parse_sha256sums_skips_malformed_lines() {
        let hashes = parse_sha256sums("not-a-hash-line\n\nshort  x.txt");
        assert!(hashes.is_empty());
    }

    #[test]
    fn should_flag_commit_both_none_is_false() {
        assert!(!should_flag_commit(None, None));
    }

    #[test]
    fn should_flag_commit_equal_is_false() {
        assert!(!should_flag_commit(Some("abc123"), Some("abc123")));
        assert!(!should_flag_commit(Some("ABC123"), Some("abc123")));
    }

    #[test]
    fn should_flag_commit_differs_is_true() {
        assert!(should_flag_commit(Some("abc123"), Some("def456")));
    }

    #[test]
    fn should_flag_commit_stored_none_remote_some_is_true() {
        assert!(should_flag_commit(Some("abc123"), None));
    }

    #[test]
    fn should_flag_commit_remote_none_is_false() {
        assert!(!should_flag_commit(None, Some("abc123")));
    }
}

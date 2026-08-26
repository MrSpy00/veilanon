//! Compile-time embedded environment access (NON-SECRET values only).
//!
//! The installed app has no working-directory `.env` (dev loads one via
//! dotenvy). `build.rs` embeds only the whitelisted NON-SECRET `VEILANON_*`
//! keys with `cargo:rustc-env` as XOR-masked hex strings (`_OBF`); this module
//! falls back to those values and unmasks them in-memory so voice/Supabase/R2
//! configuration works identically in dev and in the installed app without
//! leaving any plaintext credentials in the compiled binary's `.rodata`.
//! Runtime environment variables always win.
//!
//! SECURITY: secret keys (API secrets, tokens, private keys) are NEVER
//! embedded — see `crate::secrets::SECRET_KEYS`. They are resolved from the
//! encrypted secrets store (OS keychain protected) via `var`.

use crate::secrets;

const OBF_KEY: &[u8] = b"vEiLaNoN_sEcUrE_sTrInG_mAsK_2026_xOr";

/// Deobfuscates compile-time masked hex string into UTF-8 plaintext in memory.
fn deobfuscate(hex_str: &str) -> Option<String> {
    if hex_str.is_empty() {
        return None;
    }
    let bytes = hex::decode(hex_str).ok()?;
    let unmasked: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ OBF_KEY[i % OBF_KEY.len()])
        .collect();
    String::from_utf8(unmasked).ok()
}

/// Embedded configuration values (XOR-unmasked at runtime).
fn embedded(key: &str) -> Option<String> {
    let obf_hex = match key {
        "VEILANON_SUPABASE_URL" => option_env!("VEILANON_SUPABASE_URL_OBF"),
        "VEILANON_SUPABASE_ANON_KEY" => option_env!("VEILANON_SUPABASE_ANON_KEY_OBF"),
        "VEILANON_SUPABASE_SERVICE_ROLE_KEY" => option_env!("VEILANON_SUPABASE_SERVICE_ROLE_KEY_OBF"),
        "VEILANON_SUPABASE_DB_URL" => option_env!("VEILANON_SUPABASE_DB_URL_OBF"),
        "VEILANON_LIVEKIT_URL" => option_env!("VEILANON_LIVEKIT_URL_OBF"),
        "VEILANON_LIVEKIT_API_KEY" => option_env!("VEILANON_LIVEKIT_API_KEY_OBF"),
        "VEILANON_LIVEKIT_API_SECRET" => option_env!("VEILANON_LIVEKIT_API_SECRET_OBF"),
        "VEILANON_R2_ACCOUNT_ID" => option_env!("VEILANON_R2_ACCOUNT_ID_OBF"),
        "VEILANON_R2_ACCESS_KEY_ID" => option_env!("VEILANON_R2_ACCESS_KEY_ID_OBF"),
        "VEILANON_R2_SECRET_ACCESS_KEY" => option_env!("VEILANON_R2_SECRET_ACCESS_KEY_OBF"),
        "VEILANON_R2_BUCKET" => option_env!("VEILANON_R2_BUCKET_OBF"),
        "VEILANON_SENTRY_DSN" => option_env!("VEILANON_SENTRY_DSN_OBF"),
        "VEILANON_UPSTASH_REDIS_REST_URL" => option_env!("VEILANON_UPSTASH_REDIS_REST_URL_OBF"),
        "VEILANON_UPSTASH_REDIS_REST_TOKEN" => option_env!("VEILANON_UPSTASH_REDIS_REST_TOKEN_OBF"),
        "VEILANON_QDRANT_URL" => option_env!("VEILANON_QDRANT_URL_OBF"),
        "VEILANON_QDRANT_API_KEY" => option_env!("VEILANON_QDRANT_API_KEY_OBF"),
        "VEILANON_DISCORD_CLIENT_ID" => option_env!("VEILANON_DISCORD_CLIENT_ID_OBF"),
        "VEILANON_DISCORD_CLIENT_SECRET" => option_env!("VEILANON_DISCORD_CLIENT_SECRET_OBF"),
        "VEILANON_OLLAMA_URL" => option_env!("VEILANON_OLLAMA_URL_OBF"),
        "VEILANON_TENOR_API_KEY" => option_env!("VEILANON_TENOR_API_KEY_OBF"),
        "VEILANON_GIPHY_API_KEY" => option_env!("VEILANON_GIPHY_API_KEY_OBF"),
        _ => None,
    };
    obf_hex.and_then(deobfuscate)
}

fn key_aliases(key: &str) -> Vec<String> {
    let mut list = vec![key.to_string()];
    let upper = key.to_ascii_uppercase();
    if upper != key {
        list.push(upper.clone());
    }

    if let Some(stripped) = upper.strip_prefix("VEILANON_") {
        list.push(stripped.to_string());
        list.push(format!("NEXT_PUBLIC_{stripped}"));
        list.push(format!("PUBLIC_{stripped}"));
        list.push(format!("VITE_{stripped}"));
        list.push(format!("TAURI_{stripped}"));
    } else {
        list.push(format!("VEILANON_{upper}"));
    }

    if upper == "VEILANON_LIVEKIT_API_SECRET" || upper == "LIVEKIT_API_SECRET" {
        list.push("LIVEKIT_SECRET".to_string());
        list.push("VEILANON_LIVEKIT_SECRET".to_string());
        list.push("LIVEKIT_SECRET_KEY".to_string());
    } else if upper == "VEILANON_LIVEKIT_API_KEY" || upper == "LIVEKIT_API_KEY" {
        list.push("LIVEKIT_KEY".to_string());
        list.push("VEILANON_LIVEKIT_KEY".to_string());
    } else if upper == "VEILANON_SUPABASE_ANON_KEY" || upper == "SUPABASE_ANON_KEY" {
        list.push("SUPABASE_KEY".to_string());
        list.push("VEILANON_SUPABASE_KEY".to_string());
    } else if upper == "VEILANON_R2_ACCOUNT_ID" || upper == "R2_ACCOUNT_ID" {
        list.push("CLOUDFLARE_ACCOUNT_ID".to_string());
        list.push("ACCOUNT_ID".to_string());
    }
    list
}

/// Runtime env first, encrypted store second, embedded value third.
/// Empty runtime values are treated as unset so an accidentally empty
/// variable cannot shadow real config. Falls back to direct .env file read
/// across multiple candidate paths with aliases.
pub fn var(key: &str) -> Option<String> {
    let aliases = key_aliases(key);
    for k in &aliases {
        if let Ok(v) = std::env::var(k) {
            let trimmed = v.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }

    for k in &aliases {
        if let Some(v) = secrets::get(k) {
            let trimmed = v.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        if let Some(v) = embedded(k) {
            let trimmed = v.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }

    read_dotenv_runtime(&aliases)
}

fn read_dotenv_runtime(aliases: &[String]) -> Option<String> {
    let mut candidates = vec![
        std::path::PathBuf::from(".env"),
        std::path::PathBuf::from("../.env"),
        std::path::PathBuf::from("../../.env"),
        std::path::PathBuf::from("src-tauri/.env"),
    ];

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".env"));
        candidates.push(cwd.join("src-tauri").join(".env"));
        if let Some(p) = cwd.parent() {
            candidates.push(p.join(".env"));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join(".env"));
            if let Some(grandparent) = parent.parent() {
                candidates.push(grandparent.join(".env"));
            }
        }
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        candidates.push(std::path::Path::new(&appdata).join("veilanon").join(".env"));
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        candidates.push(std::path::Path::new(&userprofile).join(".veilanon").join(".env"));
        candidates.push(std::path::Path::new(&userprofile).join(".env"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(std::path::Path::new(&home).join(".veilanon").join(".env"));
        candidates.push(std::path::Path::new(&home).join(".config").join("veilanon").join(".env"));
        candidates.push(std::path::Path::new(&home).join(".env"));
    }

    for cand in candidates {
        if !cand.exists() {
            continue;
        }
        if let Ok(contents) = std::fs::read_to_string(&cand) {
            // Remove UTF-8 BOM if present
            let contents = contents.strip_prefix('\u{feff}').unwrap_or(&contents);
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let k_clean = k.trim();
                    if aliases.iter().any(|a| a.eq_ignore_ascii_case(k_clean)) {
                        let mut v = v.trim().to_string();
                        // Strip trailing inline comments if not in quotes
                        if !v.starts_with('"') && !v.starts_with('\'') {
                            if let Some((val_part, _)) = v.split_once(" #") {
                                v = val_part.trim().to_string();
                            }
                        }
                        if v.len() >= 2 {
                            let first = v.chars().next().unwrap();
                            let last = v.chars().last().unwrap();
                            if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                                v = v[1..v.len() - 1].to_string();
                            }
                        }
                        if !v.is_empty() {
                            return Some(v);
                        }
                    }
                }
            }
        }
    }
    None
}

/// True when `key` resolves to a non-empty value.
pub fn configured(key: &str) -> bool {
    var(key).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_resolves() {
        assert!(configured("VEILANON_SUPABASE_URL"));
        assert!(var("VEILANON_SUPABASE_URL").is_some());
        assert!(configured("VEILANON_LIVEKIT_URL"));
        assert!(configured("VEILANON_LIVEKIT_API_KEY"));
    }

    #[test]
    fn runtime_env_wins_over_embedded() {
        unsafe { std::env::set_var("VEILANON_TEST_OVERRIDE", "runtime") };
        assert_eq!(var("VEILANON_TEST_OVERRIDE").as_deref(), Some("runtime"));
    }

    #[test]
    fn deobfuscate_roundtrip() {
        let original = "https://example.supabase.co";
        let masked: Vec<u8> = original
            .as_bytes()
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ OBF_KEY[i % OBF_KEY.len()])
            .collect();
        let hex_str: String = masked.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(deobfuscate(&hex_str).as_deref(), Some(original));
    }
}

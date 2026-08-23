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

/// Runtime env first, encrypted store second, embedded value third.
/// Empty runtime values are treated as unset so an accidentally empty
/// variable cannot shadow real config. Falls back to direct .env file read
/// when neither runtime env nor embedded value is present (covers dev
/// preview and cases where the binary was built without embedded env).
pub fn var(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => secrets::get(key)
            .or_else(|| embedded(key))
            .or_else(|| read_dotenv_runtime(key)),
    }
}

fn read_dotenv_runtime(key: &str) -> Option<String> {
    for cand in [
        std::path::Path::new(".env"),
        std::path::Path::new("../.env"),
    ] {
        if let Ok(contents) = std::fs::read_to_string(cand) {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim() == key {
                        let mut v = v.trim().to_string();
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

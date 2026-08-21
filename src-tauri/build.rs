fn main() {
    // Derleme zamanı damgası: Hakkında ekranında ve güncelleme denetleyicisinde kullanılır.
    let now = chrono::Utc::now();
    let date = now.format("%Y-%m-%d %H:%M UTC").to_string();
    let timestamp = now.timestamp();
    let iso = now.to_rfc3339();
    println!("cargo:rustc-env=VEILANON_BUILD_DATE={date}");
    println!("cargo:rustc-env=VEILANON_BUILD_TIMESTAMP={timestamp}");
    println!("cargo:rustc-env=VEILANON_BUILD_ISO={iso}");

    let git_commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "release".into());
    println!("cargo:rustc-env=VEILANON_GIT_COMMIT={git_commit}");

    let rustc = std::process::Command::new(option_env!("RUSTC").unwrap_or("rustc"))
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "stable".into());
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc.trim());

    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=../.env");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [manifest_dir.join(".env"), manifest_dir.join("..").join(".env")];
    for path in &candidates {
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(contents) => {
                    let embedded_count = embed_env_file(&contents);
                    let secret_skipped = contents.lines().filter(|l| {
                        let k = l.split_once('=').map(|(k,_)| k.trim()).unwrap_or("");
                        SECRET_KEYS.contains(&k)
                    }).count();
                    if secret_skipped > 0 {
                        println!("cargo:warning=veilanon: {} secret keys SKIPPED (never embedded) from {}", secret_skipped, path.display());
                    }
                    println!(
                        "cargo:warning=veilanon: {} public environment variables embedded from {}",
                        embedded_count,
                        path.display()
                    );
                }
                Err(e) => println!("cargo:warning=veilanon: {} okunamadı: {e}", path.display()),
            };
            break;
        }
    }
    // Fallback: process environment variables.
    let mut process_env_count = 0usize;
    for &key in EMBEDDABLE_KEYS {
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                emit_obfuscated_env(key, &val);
                process_env_count += 1;
            }
        }
    }
    if process_env_count > 0 {
        println!(
            "cargo:warning=veilanon: {} environment variables embedded from process env",
            process_env_count
        );
    }

    tauri_build::build()
}

const EMBEDDABLE_KEYS: &[&str] = &[
    "VEILANON_SUPABASE_URL",
    "VEILANON_SUPABASE_ANON_KEY",
    "VEILANON_LIVEKIT_URL",
    "VEILANON_LIVEKIT_API_KEY",
    "VEILANON_R2_ACCOUNT_ID",
    "VEILANON_R2_BUCKET",
    "VEILANON_UPSTASH_REDIS_REST_URL",
    "VEILANON_QDRANT_URL",
    "VEILANON_OLLAMA_URL",
    "VEILANON_TENOR_API_KEY",
    "VEILANON_GIPHY_API_KEY",
];
const SECRET_KEYS: &[&str] = &[
    "VEILANON_SUPABASE_SERVICE_ROLE_KEY",
    "VEILANON_SUPABASE_DB_URL",
    "VEILANON_LIVEKIT_API_SECRET",
    "VEILANON_R2_ACCESS_KEY_ID",
    "VEILANON_R2_SECRET_ACCESS_KEY",
    "VEILANON_SENTRY_DSN",
    "VEILANON_UPSTASH_REDIS_REST_TOKEN",
    "VEILANON_QDRANT_API_KEY",
    "VEILANON_DISCORD_CLIENT_ID",
    "VEILANON_DISCORD_CLIENT_SECRET",
];

const OBF_KEY: &[u8] = b"vEiLaNoN_sEcUrE_sTrInG_mAsK_2026_xOr";

/// Masks a string using rolling XOR and returns a lowercase hex string.
/// This prevents plaintext URLs/keys from appearing in the compiled `.rodata`
/// and blocks simple extraction via `strings` or basic binary analysis.
fn mask_string(val: &str) -> String {
    let masked: Vec<u8> = val
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ OBF_KEY[i % OBF_KEY.len()])
        .collect();
    masked.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Emits `cargo:rustc-env=KEY_OBF=HEX_STRING`.
fn emit_obfuscated_env(key: &str, value: &str) {
    let masked_hex = mask_string(value);
    println!("cargo:rustc-env={key}_OBF={masked_hex}");
}

/// Emits obfuscated cargo env ONLY for whitelisted NON-SECRET keys.
fn embed_env_file(contents: &str) -> usize {
    let mut count = 0usize;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        let key = key.trim();
        if !EMBEDDABLE_KEYS.contains(&key) {
            continue;
        }
        let mut value = value.trim().to_string();
        if value.len() >= 2 {
            let first = value.chars().next().unwrap();
            let last = value.chars().last().unwrap();
            if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                value = value[1..value.len() - 1].to_string();
            }
        }
        if value.contains('\n') || value.contains('\r') {
            continue;
        }
        emit_obfuscated_env(key, &value);
        count += 1;
    }
    count
}

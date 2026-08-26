//! File IPC commands — client-side encryption before upload
//!
//! Flow: read → encrypt (ChaCha20-Poly1305, per-file random key) → wrap the
//! content key with the DB key (AES-256-GCM) → store metadata locally →
//! upload ciphertext to Supabase Storage as `files/{channel}/{id}`.
//! The server only ever sees an opaque blob; plaintext filenames never leave
//! the device.

use tauri::{Manager, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, warn};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::state::AppState;
use crate::error::VeilError;
use crate::crypto::{decrypt_aes_gcm, encrypt_aes_gcm};
use crate::crypto::file_enc;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub file_id: String,
    pub size_bytes: u64,
    pub upload_url: Option<String>,
    pub is_encrypted: bool,
    /// Opaque storage path — the attachment reference stored in messages.
    pub r2_key: Option<String>,
    /// Base64-encoded content key + nonce for recipient decryption (channel/message E2EE protected)
    pub content_key_ciphertext: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadFileInput {
    pub path: String,
    pub channel_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFileInput {
    pub file_id: String,
    pub destination_path: String,
    pub r2_key: Option<String>,
    pub content_key_ciphertext: Option<String>,
    pub mime_type_hint: Option<String>,
    #[allow(dead_code)] // legacy fields kept for IPC contract compatibility
    pub content_key_b64: Option<String>,
    #[allow(dead_code)]
    pub nonce_b64: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFileInput {
    pub file_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadBytesInput {
    pub bytes: Vec<u8>,
    pub channel_id: String,
}

const MAX_FILE_SIZE: u64 = 25 * 1024 * 1024; // 25 MB

/// Helper function to encrypt and persist file bytes client-side
pub async fn encrypt_and_store_bytes(
    content: &[u8],
    channel_id: &str,
    state: &AppState,
) -> Result<FileInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let _identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;

    let size = content.len() as u64;
    if size > MAX_FILE_SIZE {
        return Err(VeilError::FileTooLarge);
    }

    // Client-side encryption: random per-file key + ChaCha20-Poly1305.
    let encrypted = file_enc::encrypt_file(content)?;

    // Wrap the content key with the DB key for local metadata storage.
    let (key_cipher, key_iv) = encrypt_aes_gcm(&db_key, &encrypted.content_key)?;

    let file_id = Uuid::new_v4();
    let storage_path = format!("files/{}/{}.bin", channel_id, file_id);

    // Persist metadata locally first (ciphertext-only).
    {
        let db = state.db.read().await;
        db.execute(
            r#"INSERT INTO file_metadata
               (id, channel_id, storage_path, size_bytes, content_key_cipher,
                content_key_iv, nonce)
               VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
            rusqlite::params![
                file_id.to_string(),
                channel_id,
                storage_path,
                size as i64,
                B64.encode(&key_cipher),
                B64.encode(&key_iv),
                B64.encode(&encrypted.nonce),
            ],
        )?;
    }

    // Persist ciphertext locally in blob cache on disk for instant preview
    if let Ok(data_dir) = state.app.path().app_data_dir() {
        let blob_path = data_dir.join("blobs").join(&storage_path);
        if let Some(parent) = blob_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&blob_path, &encrypted.ciphertext);
    }

    // Best-effort upload to Supabase Storage; local metadata survives retries.
    let uploaded = {
        let network = state.network.read().await;
        let ciphertext = encrypted.ciphertext.clone();
        network.api.upload_blob(&storage_path, ciphertext).await.is_ok()
    };
    if !uploaded {
        warn!("Blob upload deferred (offline) — metadata kept locally");
    }

    let key_bundle_str = format!("{}:{}", B64.encode(&encrypted.content_key), B64.encode(&encrypted.nonce));

    info!("File encrypted and stored (id={})", file_id);
    Ok(FileInfo {
        file_id: file_id.to_string(),
        size_bytes: size,
        upload_url: None,
        is_encrypted: true,
        r2_key: Some(storage_path),
        content_key_ciphertext: Some(key_bundle_str),
    })
}

/// Helper function to resolve file ciphertext, content key, nonce, and mime type
/// whether uploaded locally or received as an attachment in a message.
async fn resolve_file_metadata_and_ciphertext(
    file_id: &str,
    fallback_r2_key: Option<&str>,
    fallback_key_cipher: Option<&str>,
    fallback_mime: Option<&str>,
    state: &AppState,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>, Option<String>), VeilError> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;

    // 0. Direct fallback from message attachment metadata if provided
    let mut direct_found: Option<(String, i64, Vec<u8>, Vec<u8>, Option<String>)> = None;
    if let Some(cipher_str) = fallback_key_cipher {
        let parts: Vec<&str> = cipher_str.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(k), Ok(n)) = (B64.decode(parts[0]), B64.decode(parts[1])) {
                let sp = fallback_r2_key
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(file_id)
                    .to_string();
                let mh = fallback_mime.map(|s| s.to_string());
                direct_found = Some((sp, 0, k, n, mh));
            }
        }
    }

    // 1. Check local file_metadata table
    let local_res: Option<(String, i64, String, String, String)> = {
        let db = state.db.read().await;
        db.query_row(
            r#"SELECT storage_path, size_bytes, content_key_cipher, content_key_iv, nonce
               FROM file_metadata WHERE id = ?1 AND deleted_at IS NULL"#,
            rusqlite::params![file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).ok()
    };

    let (storage_path, size_bytes, content_key, nonce_bytes, mime_hint) = if let Some(df) = direct_found {
        df
    } else if let Some((sp, sb, kc, ki, n)) = local_res {
        let key = decrypt_aes_gcm(
            &db_key,
            &B64.decode(&kc).map_err(|_| VeilError::DecryptionError)?,
            &B64.decode(&ki).map_err(|_| VeilError::DecryptionError)?,
        )?;
        let nonce = B64.decode(&n).map_err(|_| VeilError::DecryptionError)?;

        let db = state.db.read().await;
        let mut mh = None;
        if let Ok(conn) = db.conn() {
            if let Ok(mut stmt) = conn.prepare("SELECT attachments FROM messages WHERE attachments LIKE ?1 LIMIT 1") {
                if let Ok(mut rows) = stmt.query(rusqlite::params![format!("%{}%", file_id)]) {
                    if let Ok(Some(row)) = rows.next() {
                        if let Ok(json_str) = row.get::<_, String>(0) {
                            if let Ok(atts) = serde_json::from_str::<Vec<crate::models::message::AttachmentRef>>(&json_str) {
                                if let Some(att) = atts.into_iter().find(|a| a.file_id.to_string() == file_id) {
                                    mh = att.mime_type_hint;
                                }
                            }
                        }
                    }
                }
            }
        }
        drop(db);
        (sp, sb, key, nonce, mh)
    } else {
        // 2. Search local messages table for attachment matching file_id
        let db = state.db.read().await;
        let mut found: Option<(String, i64, Vec<u8>, Vec<u8>, Option<String>)> = None;

        if let Ok(conn) = db.conn() {
            if let Ok(mut stmt) = conn.prepare("SELECT attachments FROM messages WHERE attachments LIKE ?1") {
                if let Ok(rows) = stmt.query_map(rusqlite::params![format!("%{}%", file_id)], |row| {
                    row.get::<_, String>(0)
                }) {
                    for json_str_res in rows {
                        if let Ok(json_str) = json_str_res {
                            if let Ok(atts) = serde_json::from_str::<Vec<crate::models::message::AttachmentRef>>(&json_str) {
                                if let Some(att) = atts.into_iter().find(|a| a.file_id.to_string() == file_id) {
                                    let parts: Vec<&str> = att.content_key_ciphertext.split(':').collect();
                                    if parts.len() == 2 {
                                        if let (Ok(k), Ok(n)) = (B64.decode(parts[0]), B64.decode(parts[1])) {
                                            found = Some((att.r2_key, att.size_bytes as i64, k, n, att.mime_type_hint));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        drop(db);

        if found.is_none() && crate::config::configured("VEILANON_SUPABASE_URL") {
            let network = state.network.read().await;
            let filter = format!("attachments=like.*{}*", file_id);
            if let Ok(rows) = network.api.select::<serde_json::Value>("messages", &filter, None, Some(5)).await {
                for r in rows {
                    if let Some(atts_val) = r.get("attachments") {
                        if let Ok(atts) = serde_json::from_value::<Vec<crate::models::message::AttachmentRef>>(atts_val.clone()) {
                            if let Some(att) = atts.into_iter().find(|a| a.file_id.to_string() == file_id) {
                                let parts: Vec<&str> = att.content_key_ciphertext.split(':').collect();
                                if parts.len() == 2 {
                                    if let (Ok(k), Ok(n)) = (B64.decode(parts[0]), B64.decode(parts[1])) {
                                        found = Some((att.r2_key, att.size_bytes as i64, k, n, att.mime_type_hint));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((sp, sb, k, n, mh)) = found {
            if let Ok((key_cipher, key_iv)) = encrypt_aes_gcm(&db_key, &k) {
                let db = state.db.read().await;
                let _ = db.execute(
                    r#"INSERT INTO file_metadata
                       (id, channel_id, storage_path, size_bytes, content_key_cipher, content_key_iv, nonce)
                       VALUES (?1, '', ?2, ?3, ?4, ?5, ?6)
                       ON CONFLICT(id) DO UPDATE SET
                           content_key_cipher = excluded.content_key_cipher,
                           content_key_iv = excluded.content_key_iv,
                           nonce = excluded.nonce"#,
                    rusqlite::params![
                        file_id,
                        sp,
                        sb,
                        B64.encode(&key_cipher),
                        B64.encode(&key_iv),
                        B64.encode(&n),
                    ],
                );
            }
            (sp, sb, k, n, mh)
        } else {
            return Err(VeilError::InvalidInput("file not found".into()));
        }
    };

    // 3. Check local disk blob cache or download from Supabase Storage / R2
    let data_dir = state.app.path().app_data_dir().ok();
    let local_blob_path = data_dir.as_ref().map(|d| d.join("blobs").join(&storage_path));

    let ciphertext = if let Some(local_path) = local_blob_path.as_ref().filter(|p| p.exists()) {
        std::fs::read(local_path).map_err(VeilError::FileError)?
    } else {
        let network = state.network.read().await;
        let downloaded = network.api.download_blob(&storage_path).await?;
        if let Some(local_path) = local_blob_path.as_ref() {
            if let Some(parent) = local_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(local_path, &downloaded);
        }
        downloaded
    };

    if size_bytes > 0 && (ciphertext.len() as i64) < size_bytes {
        return Err(VeilError::InvalidInput("incomplete file on server".into()));
    }

    Ok((ciphertext, content_key, nonce_bytes, mime_hint))
}

/// Upload a file from a disk path — encrypted client-side.
#[tauri::command]
pub async fn upload_file(
    input: UploadFileInput,
    state: State<'_, AppState>,
) -> Result<FileInfo, VeilError> {
    let content = tokio::fs::read(&input.path).await?;
    encrypt_and_store_bytes(&content, &input.channel_id, &state).await
}

/// Upload in-memory bytes (e.g. pasted clipboard image) — encrypted client-side.
#[tauri::command]
pub async fn upload_bytes(
    input: UploadBytesInput,
    state: State<'_, AppState>,
) -> Result<FileInfo, VeilError> {
    encrypt_and_store_bytes(&input.bytes, &input.channel_id, &state).await
}

fn validate_download_path(input_path: &str, data_dir: &std::path::Path) -> Result<std::path::PathBuf, VeilError> {
    let p = std::path::Path::new(input_path);
    if input_path.contains("..") { return Err(VeilError::InvalidInput("Geçersiz dosya yolu".into())); }
    if !p.is_absolute() { return Err(VeilError::InvalidInput("Dosya yolu mutlak olmalıdır".into())); }
    let s = p.to_string_lossy();
    if s.contains('\0') { return Err(VeilError::InvalidInput("Geçersiz dosya yolu".into())); }
    let allowed = {
        let downloads = dirs_next();
        p.starts_with(data_dir)
            || downloads.iter().any(|d| p.starts_with(d))
            || p.extension().is_some()
    };
    if !allowed { return Err(VeilError::InvalidInput("Dosya yalnızca güvenli klasörlere indirilebilir".into())); }
    Ok(p.to_path_buf())
}

fn dirs_next() -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let h = std::path::PathBuf::from(home);
        v.push(h.join("Downloads"));
        v.push(h.join("Desktop"));
        v.push(h.join("Documents"));
        v.push(h.join("Belgeler"));
    }
    if let Ok(dl) = std::env::var("USERPROFILE") { v.push(std::path::PathBuf::from(dl).join("Downloads")); }
    v
}

/// Download and decrypt a file to the destination path.
#[tauri::command]
pub async fn download_file(
    input: DownloadFileInput,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let data_dir = state.app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let dest = validate_download_path(&input.destination_path, &data_dir)?;
    if dest.to_string_lossy().contains("..") { return Err(VeilError::InvalidInput("Geçersiz dosya yolu".into())); }
    let (ciphertext, content_key, nonce_bytes, _) = resolve_file_metadata_and_ciphertext(
        &input.file_id,
        input.r2_key.as_deref(),
        input.content_key_ciphertext.as_deref(),
        input.mime_type_hint.as_deref(),
        &state,
    )
    .await?;

    let plaintext = file_enc::decrypt_file(&ciphertext, &content_key, &nonce_bytes)?;
    if plaintext.len() as u64 > MAX_FILE_SIZE { return Err(VeilError::FileTooLarge); }
    if let Some(parent) = dest.parent() { let _ = tokio::fs::create_dir_all(parent).await; }
    tokio::fs::write(&dest, plaintext).await?;

    info!("File downloaded and decrypted (id={})", input.file_id);
    Ok(dest.to_string_lossy().to_string())
}

/// Delete a file — removes the blob from storage and tombstones metadata.
#[tauri::command]
pub async fn delete_file(
    input: DeleteFileInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let storage_path: Option<String> = {
        let db = state.db.read().await;
        db.query_row(
            "SELECT storage_path FROM file_metadata WHERE id = ?1",
            rusqlite::params![input.file_id],
            |row| row.get(0),
        )
        .ok()
    };

    if let Some(path) = &storage_path {
        let network = state.network.read().await;
        let _ = network.api.delete_blob(path).await;
    }

    {
        let db = state.db.read().await;
        db.execute(
            "UPDATE file_metadata SET deleted_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().timestamp(), input.file_id],
        )?;
    }

    info!("File deleted (id={})", input.file_id);
    Ok(())
}

/// Get file metadata without downloading content.
#[tauri::command]
pub async fn get_file_info(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<FileInfo, VeilError> {
    let (id, size_bytes, is_encrypted, storage_path): (String, i64, i64, String) = {
        let db = state.db.read().await;
        db.query_row(
            r#"SELECT id, size_bytes,
                      CASE WHEN content_key_cipher <> '' THEN 1 ELSE 0 END,
                      storage_path
               FROM file_metadata WHERE id = ?1 AND deleted_at IS NULL"#,
            rusqlite::params![file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| VeilError::InvalidInput("file not found".into()))?
    };

    Ok(FileInfo {
        file_id: id,
        size_bytes: size_bytes as u64,
        upload_url: None,
        is_encrypted: is_encrypted != 0,
        r2_key: Some(storage_path),
        content_key_ciphertext: None,
    })
}

/// Fetch decrypted file content as a data URL for inline chat preview (images, videos, audio).
#[tauri::command]
pub async fn get_file_data_url(
    file_id: String,
    r2_key: Option<String>,
    content_key_ciphertext: Option<String>,
    mime_type_hint: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let (ciphertext, content_key, nonce_bytes, mime_hint) = resolve_file_metadata_and_ciphertext(
        &file_id,
        r2_key.as_deref(),
        content_key_ciphertext.as_deref(),
        mime_type_hint.as_deref(),
        &state,
    )
    .await?;

    let plaintext = file_enc::decrypt_file(&ciphertext, &content_key, &nonce_bytes)?;

    let mime = if plaintext.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if plaintext.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if plaintext.starts_with(b"GIF87a") || plaintext.starts_with(b"GIF89a") {
        "image/gif"
    } else if plaintext.len() > 12 && &plaintext[0..4] == b"RIFF" && &plaintext[8..12] == b"WEBP" {
        "image/webp"
    } else if plaintext.starts_with(b"BM") {
        "image/bmp"
    } else if plaintext.starts_with(b"<svg") || (plaintext.starts_with(b"<?xml") && plaintext.windows(4).any(|w| w == b"<svg")) {
        "image/svg+xml"
    } else if plaintext.len() > 12 && &plaintext[4..8] == b"ftyp" && &plaintext[8..12] == b"avif" {
        "image/avif"
    } else if plaintext.len() > 8 && (&plaintext[4..8] == b"ftyp" || &plaintext[4..8] == b"moov") {
        "video/mp4"
    } else if plaintext.starts_with(b"\x1A\x45\xDF\xA3") {
        if let Some(ref hint) = mime_hint {
            if hint.starts_with("video/") {
                "video/webm"
            } else if hint.starts_with("audio/") || hint.contains("opus") || hint.contains("audio") || hint.contains("ses") {
                "audio/webm"
            } else {
                // Inspect header tracks for video vs audio codec
                if plaintext.windows(5).any(|w| w == b"V_VP8" || w == b"V_VP9" || w == b"V_AV1") {
                    "video/webm"
                } else {
                    "audio/webm"
                }
            }
        } else {
            // Inspect header tracks for video vs audio codec
            if plaintext.windows(5).any(|w| w == b"V_VP8" || w == b"V_VP9" || w == b"V_AV1") {
                "video/webm"
            } else {
                "audio/webm"
            }
        }
    } else if plaintext.starts_with(b"fLaC") {
        "audio/flac"
    } else if plaintext.starts_with(b"ID3") || (plaintext.len() > 2 && plaintext[0] == 0xFF && (plaintext[1] & 0xE0) == 0xE0) {
        "audio/mpeg"
    } else if plaintext.starts_with(b"OggS") {
        if let Some(ref hint) = mime_hint {
            if hint.starts_with("audio/") {
                hint.as_str()
            } else {
                "audio/ogg"
            }
        } else {
            "audio/ogg"
        }
    } else if plaintext.len() > 12 && &plaintext[0..4] == b"RIFF" && &plaintext[8..12] == b"WAVE" {
        "audio/wav"
    } else if plaintext.starts_with(b"%PDF") {
        "application/pdf"
    } else if let Some(ref hint) = mime_hint {
        hint.as_str()
    } else {
        "application/octet-stream"
    };

    Ok(format!("data:{mime};base64,{}", B64.encode(&plaintext)))
}

/// Write UTF-8 text contents to a user-chosen path (.json exports only).
#[tauri::command]
pub async fn write_text_file_user(path: String, contents: String) -> Result<(), VeilError> {
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    if ext.as_deref() != Some("json") {
        return Err(VeilError::InvalidInput("Yalnızca .json dosyaları kaydedilebilir.".into()));
    }

    let dest = std::path::PathBuf::from(&path);
    if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&dest, contents.as_bytes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_files_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("veil-files-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir must be creatable");
        dir
    }

    #[tokio::test]
    async fn write_text_file_user_writes_json_contents() {
        let dir = temp_files_dir();
        let path = dir.join("nested").join("export.json");
        let contents = r#"{"hello":"dünya"}"#.to_string();

        write_text_file_user(path.to_string_lossy().to_string(), contents.clone())
            .await
            .expect("json export must succeed");

        let written = std::fs::read_to_string(&path).expect("file must exist after write");
        assert_eq!(written, contents);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn write_text_file_user_rejects_non_json_extension() {
        let dir = temp_files_dir();
        let path = dir.join("export.txt");

        let err =
            write_text_file_user(path.to_string_lossy().to_string(), "{}".to_string())
                .await
                .unwrap_err();
        assert!(matches!(err, VeilError::InvalidInput(_)));
        assert!(!path.exists(), "rejected path must not be written");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

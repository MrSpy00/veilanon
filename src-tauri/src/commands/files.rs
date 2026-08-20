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

    info!("File encrypted and stored (id={})", file_id);
    Ok(FileInfo {
        file_id: file_id.to_string(),
        size_bytes: size,
        upload_url: None,
        is_encrypted: true,
        r2_key: Some(storage_path),
    })
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

/// Download and decrypt a file to the destination path.
#[tauri::command]
pub async fn download_file(
    input: DownloadFileInput,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;

    // Local metadata: storage path + wrapped content key + file nonce.
    let (storage_path, size_bytes, key_cipher, key_iv, nonce): (String, i64, String, String, String) = {
        let db = state.db.read().await;
        db.query_row(
            r#"SELECT storage_path, size_bytes, content_key_cipher, content_key_iv, nonce
               FROM file_metadata WHERE id = ?1 AND deleted_at IS NULL"#,
            rusqlite::params![input.file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(|_| VeilError::InvalidInput("file not found".into()))?
    };

    // Check local disk blob cache first, otherwise download from network
    let data_dir = state.app.path().app_data_dir().ok();
    let local_blob_path = data_dir.as_ref().map(|d| d.join("blobs").join(&storage_path));

    let ciphertext = if let Some(local_path) = local_blob_path.as_ref().filter(|p| p.exists()) {
        std::fs::read(local_path).map_err(|e| VeilError::FileError(e))?
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

    if (ciphertext.len() as i64) < size_bytes {
        return Err(VeilError::InvalidInput("incomplete file on server".into()));
    }

    // Unwrap the content key, then decrypt.
    let content_key = decrypt_aes_gcm(&db_key, &B64.decode(&key_cipher).map_err(|_| VeilError::DecryptionError)?, &B64.decode(&key_iv).map_err(|_| VeilError::DecryptionError)?)?;
    let nonce_bytes = B64.decode(&nonce).map_err(|_| VeilError::DecryptionError)?;
    let plaintext = file_enc::decrypt_file(&ciphertext, &content_key, &nonce_bytes)?;

    tokio::fs::write(&input.destination_path, plaintext).await?;

    info!("File downloaded and decrypted (id={})", input.file_id);
    Ok(input.destination_path)
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
    })
}

/// Fetch decrypted file content as a data URL for inline chat preview (images, videos, audio).
#[tauri::command]
pub async fn get_file_data_url(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let db_key = state.get_db_key().await.ok_or(VeilError::Unauthenticated)?;

    let (storage_path, size_bytes, key_cipher, key_iv, nonce): (String, i64, String, String, String) = {
        let db = state.db.read().await;
        db.query_row(
            r#"SELECT storage_path, size_bytes, content_key_cipher, content_key_iv, nonce
               FROM file_metadata WHERE id = ?1 AND deleted_at IS NULL"#,
            rusqlite::params![file_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(|_| VeilError::InvalidInput("file not found".into()))?
    };

    // Check local disk blob cache first, otherwise download from network
    let data_dir = state.app.path().app_data_dir().ok();
    let local_blob_path = data_dir.as_ref().map(|d| d.join("blobs").join(&storage_path));

    let ciphertext = if let Some(local_path) = local_blob_path.as_ref().filter(|p| p.exists()) {
        std::fs::read(local_path).map_err(|e| VeilError::FileError(e))?
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

    if (ciphertext.len() as i64) < size_bytes {
        return Err(VeilError::InvalidInput("incomplete file on server".into()));
    }

    let content_key = decrypt_aes_gcm(
        &db_key,
        &B64.decode(&key_cipher).map_err(|_| VeilError::DecryptionError)?,
        &B64.decode(&key_iv).map_err(|_| VeilError::DecryptionError)?,
    )?;
    let nonce_bytes = B64.decode(&nonce).map_err(|_| VeilError::DecryptionError)?;
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
    } else if plaintext.starts_with(b"<svg") || plaintext.starts_with(b"<?xml") && plaintext.windows(4).any(|w| w == b"<svg") {
        "image/svg+xml"
    } else if plaintext.len() > 8 && (&plaintext[4..8] == b"ftyp" || &plaintext[4..8] == b"moov") {
        "video/mp4"
    } else if plaintext.starts_with(b"\x1A\x45\xDF\xA3") {
        "video/webm"
    } else if plaintext.starts_with(b"fLaC") {
        "audio/flac"
    } else if plaintext.starts_with(b"ID3") || (plaintext.len() > 2 && plaintext[0] == 0xFF && (plaintext[1] & 0xE0) == 0xE0) {
        "audio/mpeg"
    } else if plaintext.starts_with(b"OggS") {
        "audio/ogg"
    } else if plaintext.len() > 12 && &plaintext[0..4] == b"RIFF" && &plaintext[8..12] == b"WAVE" {
        "audio/wav"
    } else {
        "application/octet-stream"
    };

    Ok(format!("data:{mime};base64,{}", B64.encode(&plaintext)))
}

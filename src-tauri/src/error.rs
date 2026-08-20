//! veilanon error types
//! All errors are carefully worded to avoid leaking sensitive information
//! in log output or IPC responses.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VeilError {
    // ── Crypto ───────────────────────────────────────────────────────────────
    #[error("cryptographic operation failed")]
    CryptoError,

    #[error("key derivation failed")]
    KeyDerivationError,

    #[error("signature verification failed")]
    SignatureError,

    #[error("decryption failed")]
    DecryptionError,

    #[error("encryption failed")]
    EncryptionError,

    #[error("Kayıtlı bir kimlik bulunamadı — lütfen önce kimlik oluşturun")]
    IdentityNotFound,

    #[allow(dead_code)]
    #[error("Bu cihazda zaten kayıtlı bir kimlik bulunmaktadır")]
    IdentityExists,

    #[error("Parola hatalı veya geçersiz")]
    InvalidPassphrase,

    #[error("Kurtarma kodu geçersiz veya biçimi hatalı")]
    InvalidRecoveryCode,

    // ── Database ─────────────────────────────────────────────────────────────
    #[error("local database error")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("database migration failed")]
    #[allow(dead_code)]
    MigrationError,

    #[error("data serialization error")]
    SerializationError,

    // ── Network ──────────────────────────────────────────────────────────────
    #[error("network request failed")]
    NetworkError(#[from] reqwest::Error),

    #[error("websocket connection error")]
    #[allow(dead_code)]
    WebSocketError,

    #[error("server returned an error: {code}")]
    ServerError { code: u16 },

    #[error("rate limit exceeded — please wait before retrying")]
    RateLimitError,

    #[error("not authenticated")]
    Unauthenticated,

    #[error("permission denied")]
    PermissionDenied,

    // ── File ─────────────────────────────────────────────────────────────────
    #[error("file operation failed")]
    FileError(#[from] std::io::Error),

    #[error("file too large")]
    FileTooLarge,

    #[error("unsupported file type")]
    #[allow(dead_code)]
    UnsupportedFileType,

    // ── Media ─────────────────────────────────────────────────────────────────
    #[error("media device access denied")]
    #[allow(dead_code)]
    MediaAccessDenied,

    #[error("voice channel connection failed")]
    VoiceConnectionError,

    #[error("e2ee key exchange failed for call")]
    #[allow(dead_code)]
    CallKeyExchangeError,

    // ── General ──────────────────────────────────────────────────────────────
    #[error("peer's public key is not yet registered — message queued for later delivery")]
    PeerKeyMissing,

    #[error("{0}")]
    InvalidInput(String),

    #[allow(dead_code)] // reserved for future feature gates
    #[error("Bu özellik henüz kullanılamıyor")]
    NotYetAvailable,

    #[error("{0}")]
    NotConfigured(String),

    #[error("Dahili sistem hatası")]
    Internal(#[from] anyhow::Error),
}

// Serialize for IPC — never include raw key material or internal details
impl serde::Serialize for VeilError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type VeilResult<T> = Result<T, VeilError>;


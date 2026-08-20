//! veilanon data models
//! All plaintext content fields MUST be encrypted before transmission/storage.
//! The server NEVER receives plaintext message content, file content, or display names in plaintext.

pub mod channel;
pub mod event;
pub mod message;
pub mod space;
pub mod user;

// Re-exports for convenience
pub use user::Identity;

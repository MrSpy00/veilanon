//! Supabase REST API client
//! 
//! SECURITY INVARIANTS:
//! - Authorization header uses anon key + user JWT — never exposes service role key
//! - Request bodies contain only ciphertext, never plaintext message content
//! - Logs NEVER include request bodies, tokens, or any ciphertext

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use crate::error::{VeilError, VeilResult};
use crate::config;

pub struct ApiClient {
    client: Client,
    base_url: String,
    anon_key: String,
    access_token: Option<String>,
    active_proxy: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: SupabaseUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseUser {
    pub id: String,
    pub email: Option<String>,
    pub created_at: String,
}

impl ApiClient {
    pub fn build_http_client(proxy_url: Option<&str>) -> VeilResult<Client> {
        let mut builder = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connection_verbose(false); // No verbose logging — prevents token leakage

        if let Some(proxy_str) = proxy_url {
            let trimmed = proxy_str.trim();
            if !trimmed.is_empty() {
                let proxy = reqwest::Proxy::all(trimmed).map_err(VeilError::NetworkError)?;
                builder = builder.proxy(proxy);
            }
        }

        builder.build().map_err(VeilError::NetworkError)
    }

    pub fn new() -> Self {
        let base_url = config::var("VEILANON_SUPABASE_URL")
            .unwrap_or_else(|| "https://your-project.supabase.co".to_string());
        let anon_key = config::var("VEILANON_SUPABASE_ANON_KEY")
            .unwrap_or_default();

        let client = Self::build_http_client(None)
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url,
            anon_key,
            access_token: None,
            active_proxy: None,
        }
    }

    pub fn apply_proxy(&mut self, proxy_url: Option<&str>) -> VeilResult<()> {
        let client = Self::build_http_client(proxy_url)?;
        self.client = client;
        self.active_proxy = proxy_url.map(|s| s.to_string());
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_active_proxy(&self) -> Option<&str> {
        self.active_proxy.as_deref()
    }

    pub fn set_access_token(&mut self, token: String) {
        self.access_token = Some(token);
    }

    pub fn clear_token(&mut self) {
        self.access_token = None;
    }

    fn auth_header(&self) -> String {
        if let Some(token) = &self.access_token {
            format!("Bearer {}", token)
        } else {
            format!("Bearer {}", self.anon_key)
        }
    }

    /// Authenticate anonymously (username + passphrase, no email required)
    #[allow(dead_code)]
    pub async fn sign_in_anonymous(&self) -> VeilResult<AuthResponse> {
        let url = format!("{}/auth/v1/signup", self.base_url);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Content-Type", "application/json")
            // GoTrue anonymous sign-up: no email/password, empty data payload.
            .json(&serde_json::json!({
                "data": {},
                "gotrue_meta_security": {}
            }))
            .send()
            .await?;

        self.check_status(resp.status())?;
        let auth: AuthResponse = resp.json().await?;
        debug!("Authentication successful"); // No token in log
        Ok(auth)
    }

    /// Sign in with deterministic internal email + password
    pub async fn sign_in_with_password(&self, email: &str, password: &str) -> VeilResult<AuthResponse> {
        let url = format!("{}/auth/v1/token?grant_type=password", self.base_url);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "gotrue_meta_security": {}
            }))
            .send()
            .await?;

        self.check_status(resp.status())?;
        let auth: AuthResponse = resp.json().await?;
        debug!("GoTrue password sign-in successful");
        Ok(auth)
    }

    /// Sign up with deterministic internal email + password
    pub async fn sign_up_with_password(&self, email: &str, password: &str) -> VeilResult<AuthResponse> {
        let url = format!("{}/auth/v1/signup", self.base_url);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "data": {},
                "gotrue_meta_security": {}
            }))
            .send()
            .await?;

        self.check_status(resp.status())?;
        let auth: AuthResponse = resp.json().await?;
        debug!("GoTrue password sign-up successful");
        Ok(auth)
    }

    /// Sign in with password or fall back to sign up if account doesn't exist yet
    #[allow(dead_code)]
    pub async fn sign_in_or_sign_up(&self, email: &str, password: &str) -> VeilResult<AuthResponse> {
        match self.sign_in_with_password(email, password).await {
            Ok(auth) => Ok(auth),
            Err(_) => self.sign_up_with_password(email, password).await,
        }
    }

    /// Exchange a stored refresh token for a fresh access token — reuses the
    /// SAME anonymous Supabase user instead of creating a new one on every
    /// app start (prevents MAU bloat and orphaned control-plane rows).
    pub async fn refresh_access_token(&self, refresh_token: &str) -> VeilResult<AuthResponse> {
        let url = format!("{}/auth/v1/token?grant_type=refresh_token", self.base_url);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "refresh_token": refresh_token,
                "gotrue_meta_security": {}
            }))
            .send()
            .await?;

        self.check_status(resp.status())?;
        let auth: AuthResponse = resp.json().await?;
        debug!("Token refreshed"); // No token in log
        Ok(auth)
    }

    /// Update user password in Supabase GoTrue (authenticated session required)
    pub async fn update_user_password(&self, new_password: &str) -> VeilResult<()> {
        let url = format!("{}/auth/v1/user", self.base_url);
        let resp = self.client
            .put(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "password": new_password
            }))
            .send()
            .await?;

        self.check_status(resp.status())?;
        debug!("GoTrue user password updated");
        Ok(())
    }

    /// Insert a row (ciphertext message, etc.)
    pub async fn insert<T: Serialize>(&self, table: &str, row: &T) -> VeilResult<()> {
        let url = format!("{}/rest/v1/{}", self.base_url, table);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            .json(row)
            .send()
            .await?;

        self.check_status(resp.status())?;
        Ok(())
    }

    /// Upsert a row (merge-duplicates on conflict) — used for profiles,
    /// presence and memberships where the id is the natural key.
    pub async fn upsert<T: Serialize>(&self, table: &str, row: &T, on_conflict: &str) -> VeilResult<()> {
        let url = format!("{}/rest/v1/{}?on_conflict={}", self.base_url, table, on_conflict);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates,return=minimal")
            .json(row)
            .send()
            .await?;

        self.check_status(resp.status())?;
        Ok(())
    }

    /// Query rows with filter
    pub async fn select<T: for<'de> Deserialize<'de>>(
        &self,
        table: &str,
        filter: &str,
        order: Option<&str>,
        limit: Option<u32>,
    ) -> VeilResult<Vec<T>> {
        let mut url = format!("{}/rest/v1/{}?{}", self.base_url, table, filter);
        if let Some(ord) = order {
            url.push_str(&format!("&order={}", ord));
        }
        if let Some(lim) = limit {
            url.push_str(&format!("&limit={}", lim.min(500)));
        }

        let resp = self.client
            .get(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .send()
            .await?;

        self.check_status(resp.status())?;
        let items: Vec<T> = resp.json().await?;
        Ok(items)
    }

    /// Soft delete — sets deleted_at
    #[allow(dead_code)] // delete sync lands next
    pub async fn soft_delete(&self, table: &str, id: &str) -> VeilResult<()> {
        let url = format!("{}/rest/v1/{}?id=eq.{}", self.base_url, table, id);
        let resp = self.client
            .patch(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "deleted_at": chrono::Utc::now() }))
            .send()
            .await?;

        self.check_status(resp.status())?;
        Ok(())
    }

    /// Update rows matching a filter (tombstones, statuses).
    #[allow(dead_code)] // tombstone sync is wired via delete_message when the UI calls it
    pub async fn update(
        &self,
        table: &str,
        filter: &str,
        patch: &serde_json::Value,
    ) -> VeilResult<()> {
        let url = format!("{}/rest/v1/{}?{}", self.base_url, table, filter);
        let resp = self.client
            .patch(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(patch)
            .send()
            .await?;
        self.check_status(resp.status())?;
        Ok(())
    }

    /// Upload an opaque (already encrypted) blob to Supabase Storage.
    /// Paths are namespaced: `files/{channel}/{fileId}` — plaintext filenames
    /// never reach the server.
    pub async fn upload_blob(&self, path: &str, bytes: Vec<u8>) -> VeilResult<()> {
        let url = format!("{}/storage/v1/object/{}", self.base_url, path);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/octet-stream")
            .header("x-upsert", "true")
            .body(bytes.clone())
            .send()
            .await?;
        if resp.status().is_client_error() {
            let put_resp = self.client
                .put(&url)
                .header("apikey", &self.anon_key)
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/octet-stream")
                .header("x-upsert", "true")
                .body(bytes)
                .send()
                .await?;
            self.check_status(put_resp.status())?;
            return Ok(());
        }
        self.check_status(resp.status())?;
        Ok(())
    }

    /// Download an opaque blob from Supabase Storage.
    /// Supports public buckets (avatars, banners, space icons) and authenticated buckets.
    pub async fn download_blob(&self, path: &str) -> VeilResult<Vec<u8>> {
        let clean_path = path.trim_start_matches('/');
        
        // 1. Try public storage endpoint (standard for avatars, banners, attachments)
        let public_url = format!("{}/storage/v1/object/public/{}", self.base_url, clean_path);
        let resp = self.client
            .get(&public_url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .send()
            .await;

        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(bytes) = r.bytes().await {
                    return Ok(bytes.to_vec());
                }
            }
        }

        // 2. Try authenticated storage endpoint fallback
        let auth_url = format!("{}/storage/v1/object/authenticated/{}", self.base_url, clean_path);
        let resp = self.client
            .get(&auth_url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .send()
            .await;

        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(bytes) = r.bytes().await {
                    return Ok(bytes.to_vec());
                }
            }
        }

        // 3. Direct object path fallback
        let direct_url = format!("{}/storage/v1/object/{}", self.base_url, clean_path);
        let resp = self.client
            .get(&direct_url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        self.check_status(resp.status())?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// Delete a blob from Supabase Storage.
    pub async fn delete_blob(&self, path: &str) -> VeilResult<()> {
        let clean_path = path.trim_start_matches('/');
        let url = format!("{}/storage/v1/object/{}", self.base_url, clean_path);
        let resp = self.client
            .delete(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        self.check_status(resp.status())?;
        Ok(())
    }

    /// Call a Supabase Edge Function / PostgRPC function (RPC).
    /// Used for SECURITY DEFINER functions that bypass RLS (e.g. create_dm_channel).
    #[allow(dead_code)]
    pub async fn rpc<T: for<'de> Deserialize<'de>>(
        &self,
        function_name: &str,
        params: &serde_json::Value,
    ) -> VeilResult<T> {
        let url = format!("{}/rest/v1/rpc/{}", self.base_url, function_name);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(params)
            .send()
            .await?;

        self.check_status(resp.status())?;
        let result: T = resp.json().await?;
        Ok(result)
    }

    /// Call a Supabase RPC that returns no value (void).
    pub async fn rpc_void(
        &self,
        function_name: &str,
        params: &serde_json::Value,
    ) -> VeilResult<()> {
        let url = format!("{}/rest/v1/rpc/{}", self.base_url, function_name);
        let resp = self.client
            .post(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            .json(params)
            .send()
            .await?;

        self.check_status(resp.status())?;
        Ok(())
    }

    /// Delete rows matching a filter (device revocation, account cleanup).
    pub async fn delete(&self, table: &str, filter: &str) -> VeilResult<()> {
        let url = format!("{}/rest/v1/{}?{}", self.base_url, table, filter);
        let resp = self.client
            .delete(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        self.check_status(resp.status())?;
        Ok(())
    }

    fn check_status(&self, status: StatusCode) -> VeilResult<()> {
        match status.as_u16() {
            200..=299 => Ok(()),
            400 => Err(VeilError::InvalidInput("Geçersiz istek veya hatalı giriş bilgileri.".into())),
            401 => Err(VeilError::Unauthenticated),
            403 => Err(VeilError::PermissionDenied),
            429 => Err(VeilError::RateLimitError),
            s => {
                error!("Server returned status {}", s);
                Err(VeilError::ServerError { code: s })
            }
        }
    }
}

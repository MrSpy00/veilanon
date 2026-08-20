//! Friends, DM and presence IPC commands
//! 
//! Local-first: friend graph is persisted locally; Supabase sync is
//! best-effort and never fails a command. Typing/presence are ephemeral.

use tauri::{Emitter, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, debug};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::db::channels::FriendRow;
use crate::error::{VeilError, VeilResult};
use crate::models::channel::{Channel, ChannelType};
use crate::state::AppState;
use crate::config;

use super::spaces::{to_channel_info, ChannelInfo};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FriendInfo {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub status: String,
    pub online_status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsAddInput {
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDmCreateInput {
    pub name: Option<String>,
    pub member_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // is_typing part of IPC contract; emitted over realtime
pub struct TypingSetInput {
    pub channel_id: String,
    pub is_typing: bool,
}

fn to_friend_info(row: &FriendRow) -> FriendInfo {
    let mapped_status = match row.status.as_str() {
        "accepted" | "friends" => "friends",
        "pending" | "pending_outgoing" | "outgoing" => "pending_outgoing",
        "pending_incoming" | "incoming" => "pending_incoming",
        "blocked" => "blocked",
        other => other,
    };
    FriendInfo {
        user_id: row.user_id.to_string(),
        username: row.username.clone(),
        display_name: row.display_name.clone(),
        avatar_hash: row.avatar_hash.clone(),
        status: mapped_status.to_string(),
        online_status: row.online_status.clone(),
    }
}

fn parse_user_id(s: &str) -> VeilResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))
}

fn dm_channel_type_string(stored: &str) -> String {
    match stored {
        "direct_message" => "dm".to_string(),
        "group_direct_message" => "group_dm".to_string(),
        other => other.to_string(),
    }
}

/// Best-effort remote profile lookup with multi-level fallback strategies
async fn fetch_profile_remotely(
    state: &AppState,
    username_or_id: &str,
) -> Option<(Uuid, String, String, Option<String>, Option<String>, Option<String>)> {
    if !config::configured("VEILANON_SUPABASE_URL") {
        return None;
    }
    let network = state.network.read().await;
    let clean = username_or_id.trim().trim_start_matches('@');
    if clean.is_empty() {
        return None;
    }

    let mut found_item: Option<serde_json::Value> = None;

    // 1. Direct UUID match if input is a valid UUID
    if let Ok(uid) = Uuid::parse_str(clean) {
        if let Ok(rows) = network.api.select::<serde_json::Value>("users", &format!("id=eq.{}", uid), None, Some(1)).await {
            if let Some(item) = rows.into_iter().next() {
                found_item = Some(item);
            }
        }
    }

    // URL-encoded clean query for PostgREST
    let clean_enc = clean.replace('%', "%25").replace('&', "%26").replace('=', "%3D").replace('+', "%2B").replace(' ', "%20");

    // 2. Exact username match (case-insensitive)
    if found_item.is_none() {
        if let Ok(rows) = network.api.select::<serde_json::Value>("users", &format!("username=ilike.{}", clean_enc), None, Some(1)).await {
            if let Some(item) = rows.into_iter().next() {
                found_item = Some(item);
            }
        }
    }

    // 3. Exact display_name match (case-insensitive)
    if found_item.is_none() {
        if let Ok(rows) = network.api.select::<serde_json::Value>("users", &format!("display_name=ilike.{}", clean_enc), None, Some(1)).await {
            if let Some(item) = rows.into_iter().next() {
                found_item = Some(item);
            }
        }
    }

    // 4. Substring / wildcard search in username or display_name
    if found_item.is_none() {
        let filter = format!("or=(username.ilike.%25{}%25,display_name.ilike.%25{}%25)", clean_enc, clean_enc);
        if let Ok(rows) = network.api.select::<serde_json::Value>("users", &filter, None, Some(1)).await {
            if let Some(item) = rows.into_iter().next() {
                found_item = Some(item);
            }
        }
    }

    let item = found_item?;

    let uid_val = item.get("id").or_else(|| item.get("user_id"));
    let uid_str = match uid_val {
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return None,
    };
    let uid = Uuid::parse_str(&uid_str).ok()?;

    let uname = item.get("username")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(clean)
        .to_string();

    let display = item.get("display_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&uname)
        .to_string();

    let avatar = item.get("avatar_hash")
        .or_else(|| item.get("avatarHash"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut dh = item.get("dh_public_key").and_then(|v| v.as_str()).map(str::to_string);
    let mut signing = item.get("signing_public_key").and_then(|v| v.as_str()).map(str::to_string);

    if dh.is_none() || signing.is_none() {
        let dev_filter = format!("user_id=eq.{}&select=public_key,signing_public_key", uid);
        if let Ok(dev_rows) = network.api.select::<serde_json::Value>("devices", &dev_filter, None, Some(1)).await {
            if let Some(dev) = dev_rows.first() {
                dh = dev.get("public_key").and_then(|v| v.as_str()).map(str::to_string);
                signing = dev.get("signing_public_key").and_then(|v| v.as_str()).map(str::to_string);
            }
        }
    }

    Some((uid, uname, display, avatar, dh, signing))
}

// ── Friends ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn friends_add(
    input: FriendsAddInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let clean_username = input.username.trim().trim_start_matches('@').to_string();

    if clean_username.is_empty() || clean_username.len() > 64 {
        return Err(VeilError::InvalidInput("Geçerli bir kullanıcı adı girin".into()));
    }
    if clean_username.eq_ignore_ascii_case(&identity.username) || clean_username == identity.id.to_string() {
        return Err(VeilError::InvalidInput("Kendine arkadaşlık isteği gönderemezsin".into()));
    }

    let db = state.db.read().await;
    let profile = match db.get_profile_by_username(&clean_username)? {
        Some(profile) => Some(profile),
        None => fetch_profile_remotely(&state, &clean_username).await.map(
            |(id, uname, display, avatar, dh, signing)| {
                let _ = db.upsert_profile(
                    &id,
                    &uname,
                    &display,
                    avatar.as_deref(),
                    dh.as_deref(),
                    signing.as_deref(),
                    None,
                    None,
                    None,
                );
                (id, uname, display, avatar)
            },
        ),
    };

    let (friend_id, friend_uname, _friend_disp, _) = profile.ok_or(VeilError::InvalidInput("Kullanıcı bulunamadı. Lütfen kullanıcı adını kontrol edin.".into()))?;
    if friend_id == identity.id {
        return Err(VeilError::InvalidInput("Kendine arkadaşlık isteği gönderemezsin".into()));
    }

    db.upsert_friend(&identity.id, &friend_id, "pending_outgoing")?;
    drop(db);

    // Register outgoing request on Supabase friendships table
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .upsert(
                "friendships",
                &serde_json::json!({
                    "user_id": identity.id.to_string(),
                    "friend_id": friend_id.to_string(),
                    "status": "pending",
                }),
                "user_id,friend_id",
            )
            .await;

        // Broadcast friend request signal over realtime so recipient is notified instantly
        network.realtime.broadcast(serde_json::json!({
            "type": "friend_request",
            "action": "incoming",
            "sender_id": identity.id.to_string(),
            "sender_username": identity.username,
            "sender_display_name": identity.display_name,
            "sender_avatar_hash": identity.avatar_hash,
            "target_id": friend_id.to_string(),
        }));
    }

    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    info!("Friend request sent to {} ({})", friend_uname, friend_id);
    Ok(())
}

#[tauri::command]
pub async fn friends_accept(user_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let friend_id = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    db.upsert_friend(&identity.id, &friend_id, "friends")?;
    db.upsert_friend(&friend_id, &identity.id, "friends")?;
    drop(db);

    // Best-effort control-plane sync (both directions)
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .upsert(
                "friendships",
                &serde_json::json!({
                    "user_id": friend_id.to_string(),
                    "friend_id": identity.id.to_string(),
                    "status": "accepted",
                }),
                "user_id,friend_id",
            )
            .await;
        let _ = network
            .api
            .upsert(
                "friendships",
                &serde_json::json!({
                    "user_id": identity.id.to_string(),
                    "friend_id": friend_id.to_string(),
                    "status": "accepted",
                }),
                "user_id,friend_id",
            )
            .await;
    }
    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    info!("Friend accepted: {}", friend_id);
    Ok(())
}

#[tauri::command]
pub async fn friends_reject(user_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let friend_id = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    db.remove_friend(&identity.id, &friend_id)?;
    db.remove_friend(&friend_id, &identity.id)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .delete(
                "friendships",
                &format!("user_id=eq.{}&friend_id=eq.{}", identity.id, friend_id),
            )
            .await;
        let _ = network
            .api
            .delete(
                "friendships",
                &format!("user_id=eq.{}&friend_id=eq.{}", friend_id, identity.id),
            )
            .await;
    }
    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    info!("Friend request rejected: {}", friend_id);
    Ok(())
}

#[tauri::command]
pub async fn friends_cancel(user_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let friend_id = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    db.remove_friend(&identity.id, &friend_id)?;
    db.remove_friend(&friend_id, &identity.id)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .delete(
                "friendships",
                &format!("user_id=eq.{}&friend_id=eq.{}", identity.id, friend_id),
            )
            .await;
        let _ = network
            .api
            .delete(
                "friendships",
                &format!("user_id=eq.{}&friend_id=eq.{}", friend_id, identity.id),
            )
            .await;
    }
    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    info!("Friend request cancelled: {}", friend_id);
    Ok(())
}

#[tauri::command]
pub async fn friends_remove(user_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let friend_id = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    db.remove_friend(&identity.id, &friend_id)?;
    db.remove_friend(&friend_id, &identity.id)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .delete(
                "friendships",
                &format!("user_id=eq.{}&friend_id=eq.{}", identity.id, friend_id),
            )
            .await;
        let _ = network
            .api
            .delete(
                "friendships",
                &format!("user_id=eq.{}&friend_id=eq.{}", friend_id, identity.id),
            )
            .await;
    }
    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    info!("Friend removed: {}", friend_id);
    Ok(())
}

#[tauri::command]
pub async fn friends_block(user_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let friend_id = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    db.upsert_friend(&identity.id, &friend_id, "blocked")?;
    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    Ok(())
}

#[tauri::command]
pub async fn friends_unblock(user_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let friend_id = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    db.remove_friend(&identity.id, &friend_id)?;
    let _ = state.app.emit("friends:changed", serde_json::json!({ "userId": friend_id.to_string() }));
    Ok(())
}

#[tauri::command]
pub async fn friends_list(state: State<'_, AppState>) -> Result<Vec<FriendInfo>, VeilError> {
    let identity = {
        let guard = state.get_or_restore_identity().await;
        guard.as_ref().ok_or(VeilError::Unauthenticated)?.clone()
    };

    // 1. Remote sync: pull incoming & outgoing friendships from Supabase
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("or=(user_id.eq.{},friend_id.eq.{})", identity.id, identity.id);
        if let Ok(remote_rows) = network.api.select::<serde_json::Value>("friendships", &filter, None, Some(200)).await {
            let mut peer_ids: Vec<String> = Vec::new();
            let mut active_remote_pairs: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

            for r in &remote_rows {
                let mut peer_uuid: Option<Uuid> = None;
                if let Some(uid) = r.get("user_id").and_then(|v| v.as_str()) {
                    if uid != identity.id.to_string() {
                        peer_ids.push(uid.to_string());
                        if let Ok(u) = Uuid::parse_str(uid) { peer_uuid = Some(u); }
                    }
                }
                if let Some(fid) = r.get("friend_id").and_then(|v| v.as_str()) {
                    if fid != identity.id.to_string() {
                        peer_ids.push(fid.to_string());
                        if let Ok(u) = Uuid::parse_str(fid) { peer_uuid = Some(u); }
                    }
                }
                if let Some(p) = peer_uuid {
                    active_remote_pairs.insert(p);
                }
            }

            if !peer_ids.is_empty() {
                peer_ids.sort();
                peer_ids.dedup();
                let users_filter = format!("id=in.({})&select=id,username,display_name,avatar_hash", peer_ids.join(","));
                if let Ok(users) = network.api.select::<serde_json::Value>("users", &users_filter, None, Some(200)).await {
                    let db = state.db.read().await;
                    for u in users {
                        if let (Some(uid_str), Some(uname), Some(disp)) = (
                            u.get("id").and_then(|v| v.as_str()),
                            u.get("username").and_then(|v| v.as_str()),
                            u.get("display_name").and_then(|v| v.as_str()),
                        ) {
                            if let Ok(uid) = Uuid::parse_str(uid_str) {
                                let av = u.get("avatar_hash").and_then(|v| v.as_str());
                                let _ = db.upsert_profile(&uid, uname, disp, av, None, None, None, None, None);
                            }
                        }
                    }
                }

                let dev_filter = format!("user_id=in.({})&select=user_id,public_key,signing_public_key", peer_ids.join(","));
                if let Ok(devices) = network.api.select::<serde_json::Value>("devices", &dev_filter, None, Some(200)).await {
                    let db = state.db.read().await;
                    for d in devices {
                        if let (Some(uid_str), Some(pk), Some(spk)) = (
                            d.get("user_id").and_then(|v| v.as_str()),
                            d.get("public_key").and_then(|v| v.as_str()),
                            d.get("signing_public_key").and_then(|v| v.as_str()),
                        ) {
                            if let Ok(uid) = Uuid::parse_str(uid_str) {
                                let _ = db.upsert_profile(&uid, "", "", None, Some(pk), Some(spk), None, None, None);
                            }
                        }
                    }
                }

                let db = state.db.read().await;
                for r in &remote_rows {
                    let r_user = r.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
                    let r_friend = r.get("friend_id").and_then(|v| v.as_str()).unwrap_or("");
                    let r_status = r.get("status").and_then(|v| v.as_str()).unwrap_or("pending");

                    if let (Ok(u1), Ok(u2)) = (Uuid::parse_str(r_user), Uuid::parse_str(r_friend)) {
                        if u1 == identity.id {
                            let local_status = if r_status == "accepted" { "friends" } else if r_status == "blocked" { "blocked" } else { "pending_outgoing" };
                            let _ = db.upsert_friend(&identity.id, &u2, local_status);
                        } else if u2 == identity.id {
                            let local_status = if r_status == "accepted" { "friends" } else if r_status == "blocked" { "blocked" } else { "pending_incoming" };
                            let _ = db.upsert_friend(&identity.id, &u1, local_status);
                        }
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    let rows = db.list_friends(&identity.id)?;
    Ok(rows.iter().map(to_friend_info).collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmOpenInput {
    pub user_id: String,
}

/// Helper to resolve display name and fetch profile for a peer
async fn resolve_peer_name(state: &AppState, peer_id: &Uuid) -> String {
    let db = state.db.read().await;
    if let Ok(Some((uname, disp, _, _, _))) = db.get_profile_by_id(peer_id) {
        if !disp.is_empty() {
            return disp;
        } else if !uname.is_empty() {
            return format!("@{uname}");
        }
    }
    drop(db);

    // Try fetching remotely from Supabase
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("id=eq.{}&select=id,username,display_name,avatar_hash", peer_id);
        if let Ok(users) = network.api.select::<serde_json::Value>("users", &filter, None, Some(1)).await {
            if let Some(u) = users.first() {
                let disp = u.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
                let uname = u.get("username").and_then(|v| v.as_str()).unwrap_or("");
                let av = u.get("avatar_hash").and_then(|v| v.as_str());

                // Also fetch device keys
                let dev_filter = format!("user_id=eq.{}&select=public_key,signing_public_key", peer_id);
                let mut dh_pk: Option<String> = None;
                let mut sig_pk: Option<String> = None;
                if let Ok(devs) = network.api.select::<serde_json::Value>("devices", &dev_filter, None, Some(1)).await {
                    if let Some(d) = devs.first() {
                        dh_pk = d.get("public_key").and_then(|v| v.as_str()).map(str::to_string);
                        sig_pk = d.get("signing_public_key").and_then(|v| v.as_str()).map(str::to_string);
                    }
                }

                let db = state.db.read().await;
                let _ = db.upsert_profile(peer_id, uname, disp, av, dh_pk.as_deref(), sig_pk.as_deref(), None, None, None);
                if !disp.is_empty() { return disp.to_string(); }
                if !uname.is_empty() { return format!("@{uname}"); }
            }
        }
    }

    "Direkt Mesaj".to_string()
}

/// Open (or create) a 1:1 DM channel with a user. Idempotent: an existing
/// DM between the two users is returned as-is.
/// Herhangi bir kullanıcıyla DM açılabilir (arkadaş olma şartı yoktur).
#[tauri::command]
pub async fn dm_open(
    input: DmOpenInput,
    state: State<'_, AppState>,
) -> Result<ChannelInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let peer_id = parse_user_id(&input.user_id)?;
    if peer_id == identity.id {
        return Err(VeilError::InvalidInput("Cannot DM yourself".into()));
    }

    let db = state.db.read().await;
    let is_friend = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM friends WHERE user_id = ?1 AND friend_id = ?2 AND status IN ('friends', 'accepted'))",
        rusqlite::params![identity.id.to_string(), peer_id.to_string()],
        |r| r.get::<_, bool>(0),
    ).unwrap_or(false);
    drop(db);

    if !is_friend {
        let my_dm_privacy = {
            let settings = state.settings.read().await;
            settings.dm_privacy.clone()
        };
        if my_dm_privacy == "nobody" {
            return Err(VeilError::InvalidInput("Direkt mesaj gönderiminiz gizlilik ayarlarınızda kapalıdır. Ayarlar > Gizlilik bölümünden değiştirebilirsiniz.".into()));
        }

        let mut target_dm_privacy = "everyone".to_string();
        if config::configured("VEILANON_SUPABASE_URL") {
            let network = state.network.read().await;
            let filter = format!("id=eq.{}&select=dm_privacy", peer_id);
            if let Ok(rows) = network.api.select::<serde_json::Value>("users", &filter, None, Some(1)).await {
                if let Some(first) = rows.first() {
                    if let Some(priv_val) = first.get("dm_privacy").and_then(|v| v.as_str()) {
                        target_dm_privacy = priv_val.to_string();
                    }
                }
            }
        }

        if target_dm_privacy == "nobody" {
            return Err(VeilError::InvalidInput("Bu kullanıcı direkt mesaj alımını kapatmıştır.".into()));
        }
        if target_dm_privacy == "friends" {
            return Err(VeilError::InvalidInput("Bu kullanıcı yalnızca arkadaşlarından direkt mesaj kabul etmektedir.".into()));
        }
        if target_dm_privacy == "same_server" {
            let db = state.db.read().await;
            let same_server = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM space_members sm1 JOIN space_members sm2 ON sm1.space_id = sm2.space_id WHERE sm1.user_id = ?1 AND sm2.user_id = ?2)",
                rusqlite::params![identity.id.to_string(), peer_id.to_string()],
                |r| r.get::<_, bool>(0),
            ).unwrap_or(false);
            drop(db);
            if !same_server {
                return Err(VeilError::InvalidInput("Bu kullanıcı yalnızca ortak sunucudaki kişilerden direkt mesaj kabul etmektedir.".into()));
            }
        }
    }

    let peer_name = resolve_peer_name(&state, &peer_id).await;

    let db = state.db.read().await;

    // Existing DM channel in local SQLite?
    if let Some(existing) = db.find_dm_with(&identity.id, &peer_id)? {
        let rows = db.list_dm_channels()?;
        let row = rows
            .iter()
            .find(|r| r.id == existing)
            .ok_or(VeilError::InvalidInput("DM channel not found".into()))?;
        let mut info = to_channel_info(row);
        info.channel_type = dm_channel_type_string(&info.channel_type);
        info.name = peer_name;
        info.peer_id = Some(peer_id.to_string());
        let profile = db.get_profile_by_id(&peer_id).ok().flatten();
        info.avatar_hash = profile.as_ref().and_then(|p| p.2.clone());
        info.online_status = profile.map(|p| p.4);
        return Ok(info);
    }
    drop(db);

    // Existing DM channel in Supabase?
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let my_cm_filter = format!("user_id=eq.{}", identity.id);
        if let Ok(my_memberships) = network.api.select::<serde_json::Value>("channel_members", &my_cm_filter, None, Some(100)).await {
            let my_cids: Vec<String> = my_memberships.iter().filter_map(|m| m.get("channel_id").and_then(|v| v.as_str()).map(str::to_string)).collect();
            if !my_cids.is_empty() {
                let peer_cm_filter = format!("user_id=eq.{}&channel_id=in.({})", peer_id, my_cids.join(","));
                if let Ok(peer_memberships) = network.api.select::<serde_json::Value>("channel_members", &peer_cm_filter, None, Some(1)).await {
                    if let Some(first_match) = peer_memberships.first() {
                        if let Some(matched_cid_str) = first_match.get("channel_id").and_then(|v| v.as_str()) {
                            if let Ok(matched_cid) = Uuid::parse_str(matched_cid_str) {
                                let db = state.db.read().await;
                                let db_key = state.get_db_key().await;
                                let channel = Channel {
                                    id: matched_cid,
                                    space_id: None,
                                    name: peer_name.clone(),
                                    channel_type: ChannelType::DirectMessage,
                                    position: db.next_channel_position(None)?,
                                    topic: None,
                                    is_nsfw: false,
                                    is_e2ee: true,
                                    slow_mode_seconds: 0,
                                    permission_overrides: Vec::new(),
                                    created_at: chrono::Utc::now(),
                                    last_message_id: None,
                                    unread_count: 0,
                                    mentioned: false,
                                };
                                let _ = db.upsert_channel(&channel, db_key.as_ref());
                                let _ = db.add_channel_member(&matched_cid, &identity.id);
                                let _ = db.add_channel_member(&matched_cid, &peer_id);
                                let profile = db.get_profile_by_id(&peer_id).ok().flatten();
                                let avatar_hash = profile.as_ref().and_then(|p| p.2.clone());
                                let online_status = profile.map(|p| p.4);
                                
                                return Ok(ChannelInfo {
                                    id: matched_cid.to_string(),
                                    space_id: None,
                                    name: peer_name,
                                    channel_type: "dm".into(),
                                    position: channel.position,
                                    is_nsfw: false,
                                    is_e2ee: true,
                                    unread_count: 0,
                                    mentioned: false,
                                    last_message_id: None,
                                    avatar_hash,
                                    peer_id: Some(peer_id.to_string()),
                                    online_status,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    // Create a new DM channel + membership rows for both sides.
    let channel_id = Uuid::new_v4();
    let channel = Channel {
        id: channel_id,
        space_id: None,
        name: peer_name.clone(),
        channel_type: ChannelType::DirectMessage,
        position: db.next_channel_position(None)?,
        topic: None,
        is_nsfw: false,
        is_e2ee: true,
        slow_mode_seconds: 0,
        permission_overrides: Vec::new(),
        created_at: chrono::Utc::now(),
        last_message_id: None,
        unread_count: 0,
        mentioned: false,
    };
    let db_key = state.get_db_key().await;
    db.upsert_channel(&channel, db_key.as_ref())?;
    db.add_channel_member(&channel_id, &identity.id)?;
    db.add_channel_member(&channel_id, &peer_id)?;
    let profile = db.get_profile_by_id(&peer_id).ok().flatten();
    let avatar_hash = profile.as_ref().and_then(|p| p.2.clone());
    let online_status = profile.map(|p| p.4);
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let ch_result = network
            .api
            .upsert(
                "channels",
                &serde_json::json!({
                    "id": channel_id.to_string(),
                    "name": channel.name,
                    "channel_type": "dm",
                    "position": channel.position,
                    "is_e2ee": true,
                }),
                "id",
            )
            .await;
        if let Err(e) = &ch_result {
            debug!("DM channel sync to Supabase failed: {}", e);
        }

        let cm_result = network
            .api
            .rpc_void(
                "create_dm_channel",
                &serde_json::json!({
                    "p_channel_id": channel_id.to_string(),
                    "p_peer_user_id": peer_id.to_string(),
                }),
            )
            .await;
        if let Err(e) = &cm_result {
            debug!("DM channel_members RPC failed ({}), falling back to direct upsert", e);
            let _ = network.api.upsert(
                "channel_members",
                &serde_json::json!({
                    "channel_id": channel_id.to_string(),
                    "user_id": identity.id.to_string(),
                }),
                "channel_id,user_id",
            ).await;
            let _ = network.api.upsert(
                "channel_members",
                &serde_json::json!({
                    "channel_id": channel_id.to_string(),
                    "user_id": peer_id.to_string(),
                }),
                "channel_id,user_id",
            ).await;
        }
    }

    let _ = state.app.emit("channels:changed", ());
    info!("DM channel opened");
    Ok(ChannelInfo {
        id: channel_id.to_string(),
        space_id: None,
        name: channel.name,
        channel_type: "dm".into(),
        position: channel.position,
        is_nsfw: false,
        is_e2ee: true,
        unread_count: 0,
        mentioned: false,
        last_message_id: None,
        avatar_hash,
        peer_id: Some(peer_id.to_string()),
        online_status,
    })
}

#[tauri::command]
pub async fn dm_list(state: State<'_, AppState>) -> Result<Vec<ChannelInfo>, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("user_id=eq.{}", identity.id);
        if let Ok(memberships) = network.api.select::<serde_json::Value>("channel_members", &filter, None, Some(100)).await {
            let mut remote_cids: Vec<String> = Vec::new();
            for m in &memberships {
                if let Some(cid) = m.get("channel_id").and_then(|v| v.as_str()) {
                    remote_cids.push(cid.to_string());
                }
            }

            // Çift yönlü mutabakat: Sunucudan silinen DM sohbetlerini yerel SQLite'tan da sil
            {
                let db = state.db.read().await;
                if let Ok(local_dms) = db.list_dm_channels() {
                    for ldm in local_dms {
                        if !remote_cids.contains(&ldm.id.to_string()) {
                            let _ = db.delete_channel(&ldm.id);
                        }
                    }
                }
            }

            if !remote_cids.is_empty() {
                remote_cids.sort();
                remote_cids.dedup();
                let ch_filter = format!("id=in.({})&space_id=is.null", remote_cids.join(","));
                if let Ok(channels) = network.api.select::<serde_json::Value>("channels", &ch_filter, None, Some(100)).await {
                    let db = state.db.read().await;
                    let db_key = state.get_db_key().await;
                    for ch in channels {
                        if let (Some(cid_str), Some(cname)) = (
                            ch.get("id").and_then(|v| v.as_str()),
                            ch.get("name").and_then(|v| v.as_str()),
                        ) {
                            if let Ok(cid) = Uuid::parse_str(cid_str) {
                                let ctype_str = ch.get("channel_type").and_then(|v| v.as_str()).unwrap_or("direct_message");
                                let ctype = if ctype_str == "group_dm" || ctype_str == "group_direct_message" {
                                    ChannelType::GroupDirectMessage
                                } else {
                                    ChannelType::DirectMessage
                                };
                                let channel = Channel {
                                    id: cid,
                                    space_id: None,
                                    name: cname.to_string(),
                                    channel_type: ctype,
                                    position: ch.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                    topic: None,
                                    is_nsfw: false,
                                    is_e2ee: true,
                                    slow_mode_seconds: 0,
                                    permission_overrides: Vec::new(),
                                    created_at: chrono::Utc::now(),
                                    last_message_id: None,
                                    unread_count: 0,
                                    mentioned: false,
                                };
                                let _ = db.upsert_channel(&channel, db_key.as_ref());
                                let _ = db.add_channel_member(&cid, &identity.id);

                                // Pull other members for this channel
                                let cm_filter = format!("channel_id=eq.{}", cid);
                                if let Ok(all_cm) = network.api.select::<serde_json::Value>("channel_members", &cm_filter, None, Some(50)).await {
                                    for row in all_cm {
                                        if let Some(uid_str) = row.get("user_id").and_then(|v| v.as_str()) {
                                            if let Ok(uid) = Uuid::parse_str(uid_str) {
                                                let _ = db.add_channel_member(&cid, &uid);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    let rows = db.list_dm_channels()?;
    let mut missing_peers: Vec<Uuid> = Vec::new();

    for row in &rows {
        let stored_type = row.channel_type.as_str();
        if stored_type == "direct_message" || stored_type == "dm" {
            if let Ok(members) = db.list_channel_members(&row.id) {
                if let Some(peer) = members.iter().find(|m| **m != identity.id) {
                    if let Ok(prof) = db.get_profile_by_id(peer) {
                        if prof.is_none() {
                            missing_peers.push(*peer);
                        }
                    }
                }
            }
        }
    }
    drop(db);

    if !missing_peers.is_empty() && config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        missing_peers.sort();
        missing_peers.dedup();
        let p_ids: Vec<String> = missing_peers.iter().map(|u| u.to_string()).collect();
        let u_filter = format!("id=in.({})&select=id,username,display_name,avatar_hash,banner_hash", p_ids.join(","));
        if let Ok(u_rows) = network.api.select::<serde_json::Value>("users", &u_filter, None, Some(100)).await {
            let db = state.db.read().await;
            for u in u_rows {
                if let (Some(uid_str), Some(uname)) = (u.get("id").and_then(|v| v.as_str()), u.get("username").and_then(|v| v.as_str())) {
                    if let Ok(uid) = Uuid::parse_str(uid_str) {
                        let disp = u.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
                        let av = u.get("avatar_hash").and_then(|v| v.as_str());
                        let _ = db.upsert_profile(&uid, uname, disp, av, None, None, None, None, None);
                    }
                }
            }
        }
    }

    // Sync presence for all DM peers from Supabase (90s heartbeat TTL)
    {
        let mut peer_ids: Vec<Uuid> = Vec::new();
        {
            let db = state.db.read().await;
            for row in &rows {
                let stored_type = row.channel_type.as_str();
                if stored_type == "direct_message" || stored_type == "dm" {
                    if let Ok(members) = db.list_channel_members(&row.id) {
                        for m in &members {
                            if *m != identity.id && !peer_ids.contains(m) {
                                peer_ids.push(*m);
                            }
                        }
                    }
                }
            }
        }
        if !peer_ids.is_empty() && config::configured("VEILANON_SUPABASE_URL") {
            let network = state.network.read().await;
            peer_ids.sort();
            peer_ids.dedup();
            let p_ids: Vec<String> = peer_ids.iter().map(|u| u.to_string()).collect();
            let presence_filter = format!("user_id=in.({})&select=user_id,status,heartbeat_at,last_seen", p_ids.join(","));
            if let Ok(presences) = network.api.select::<serde_json::Value>("presence", &presence_filter, None, Some(200)).await {
                let db = state.db.read().await;
                let now = chrono::Utc::now();
                for p in presences {
                    if let Some(uid_str) = p.get("user_id").and_then(|v| v.as_str()) {
                        if let Ok(uid) = Uuid::parse_str(uid_str) {
                            let raw_status = p.get("status").and_then(|v| v.as_str()).unwrap_or("offline");
                            let status_str = if raw_status != "offline" && raw_status != "invisible" {
                                let hb_str = p.get("heartbeat_at").or_else(|| p.get("last_seen")).and_then(|v| v.as_str()).unwrap_or("");
                                if let Ok(hb_time) = chrono::DateTime::parse_from_rfc3339(hb_str) {
                                    if (now - hb_time.with_timezone(&chrono::Utc)).num_seconds() <= 90 {
                                        raw_status
                                    } else {
                                        "offline"
                                    }
                                } else {
                                    "offline"
                                }
                            } else {
                                "offline"
                            };
                            let _ = db.update_presence(&uid, status_str);
                        }
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let mut info = to_channel_info(&row);
        let stored_type = info.channel_type.clone();
        info.channel_type = dm_channel_type_string(&stored_type);
        // 1:1 DMs render as the peer's name and avatar instead of a generic label.
        if stored_type == "direct_message" || stored_type == "dm" {
            let members = db.list_channel_members(&row.id)?;
            if let Some(peer) = members.iter().find(|m| **m != identity.id) {
                info.peer_id = Some(peer.to_string());
                if let Ok(Some((uname, disp, av, _bio, status))) = db.get_profile_by_id(peer) {
                    info.avatar_hash = av;
                    info.online_status = Some(status);
                    if !disp.trim().is_empty() {
                        info.name = disp;
                    } else if !uname.trim().is_empty() {
                        info.name = format!("@{uname}");
                    }
                }
            }
        }
        result.push(info);
    }
    Ok(result)
}

#[tauri::command]
pub async fn group_dm_create(
    input: GroupDmCreateInput,
    state: State<'_, AppState>,
) -> Result<ChannelInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let _identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;

    if input.member_ids.is_empty() {
        return Err(VeilError::InvalidInput("Group DM needs at least one member".into()));
    }
    for member in &input.member_ids {
        parse_user_id(member)?;
    }

    let name = match input.name.as_deref() {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => "Group DM".to_string(),
    };

    let db = state.db.read().await;
    let position = db.next_channel_position(None)?;
    let channel = Channel {
        id: Uuid::new_v4(),
        space_id: None,
        name,
        channel_type: ChannelType::GroupDirectMessage,
        position,
        topic: None,
        is_nsfw: false,
        is_e2ee: true,
        slow_mode_seconds: 0,
        permission_overrides: Vec::new(),
        created_at: chrono::Utc::now(),
        last_message_id: None,
        unread_count: 0,
        mentioned: false,
    };
    let db_key = state.get_db_key().await;
    db.upsert_channel(&channel, db_key.as_ref())?;
    for member in &input.member_ids {
        if let Ok(uid) = Uuid::parse_str(member) {
            db.add_channel_member(&channel.id, &uid)?;
        }
    }
    db.add_channel_member(&channel.id, &_identity.id)?;

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .insert(
                "channels",
                &serde_json::json!({
                    "id": channel.id.to_string(),
                    "name": channel.name,
                    "channel_type": "group_dm",
                    "position": channel.position,
                    "is_e2ee": true,
                }),
            )
            .await;
        let mut all_members: Vec<uuid::Uuid> = vec![_identity.id];
        for member in &input.member_ids {
            if let Ok(uid) = Uuid::parse_str(member) {
                all_members.push(uid);
            }
        }
        let member_strs: Vec<String> = all_members.iter().map(|u| u.to_string()).collect();
        let _ = network
            .api
            .rpc_void(
                "add_channel_members",
                &serde_json::json!({
                    "p_channel_id": channel.id.to_string(),
                    "p_user_ids": member_strs,
                }),
            )
            .await;
    }

    let _ = state.app.emit("channels:changed", serde_json::json!({ "channelId": channel.id.to_string() }));
    info!("Group DM created"); // member list intentionally not logged
    Ok(ChannelInfo {
        id: channel.id.to_string(),
        space_id: None,
        name: channel.name,
        channel_type: "group_dm".into(),
        position,
        is_nsfw: false,
        is_e2ee: true,
        unread_count: 0,
        mentioned: false,
        last_message_id: None,
        avatar_hash: None,
        peer_id: None,
        online_status: None,
    })
}

// ── Presence / typing ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn presence_update(status: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;

    let normalized = match status.as_str() {
        "online" | "away" | "dnd" | "offline" | "invisible" => status,
        _ => return Err(VeilError::InvalidInput("Invalid presence status".into())),
    };

    let db = state.db.read().await;
    db.upsert_profile(
        &identity.id,
        &identity.username,
        &identity.display_name,
        None,
        Some(identity.identity_key_public.as_str()),
        Some(identity.signing_key_public.as_str()),
        None,
        None,
        None,
    )?;
    db.update_presence(&identity.id, &normalized)?;
    let profile_full = db.get_profile_full_by_id(&identity.id).ok().flatten();
    let avatar_str = profile_full.as_ref().and_then(|p| p.2.as_deref()).unwrap_or("");
    let banner_str = profile_full.as_ref().and_then(|p| p.3.as_deref());
    let custom_status_str = profile_full.as_ref().and_then(|p| p.6.as_deref());
    drop(db);

    // Best-effort: mirror user + presence to the control plane.
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let mut user_payload = serde_json::json!({
            "id": identity.id.to_string(),
            "username": identity.username,
            "display_name": identity.display_name,
            "avatar_hash": avatar_str,
            "banner_hash": banner_str,
        });
        if let Some(cs) = custom_status_str {
            user_payload["custom_status"] = serde_json::json!(cs);
        }
        let _ = network.api.upsert("users", &user_payload, "id").await;

        let mut pres_payload = serde_json::json!({
            "user_id": identity.id.to_string(),
            "status": normalized,
            "heartbeat_at": chrono::Utc::now().to_rfc3339(),
            "last_seen": chrono::Utc::now().to_rfc3339(),
        });
        if let Some(cs) = custom_status_str {
            pres_payload["custom_status"] = serde_json::json!(cs);
        }
        let _ = network.api.upsert("presence", &pres_payload, "user_id").await;
    }

    // Ephemeral signal over realtime for immediate presence sync
    let realtime = state.network.read().await.realtime.clone();
    let mut broadcast_payload = serde_json::json!({
        "type": "presence",
        "user_id": identity.id.to_string(),
        "status": normalized,
    });
    if let Some(cs) = custom_status_str {
        broadcast_payload["custom_status"] = serde_json::json!(cs);
    }
    realtime.broadcast(broadcast_payload);

    let _ = state.app.emit("presence:changed", serde_json::json!({
        "userId": identity.id.to_string(),
        "status": normalized,
        "customStatus": custom_status_str,
    }));

    Ok(())
}

#[tauri::command]
pub async fn typing_set(input: TypingSetInput, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    Uuid::parse_str(&input.channel_id).map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))?;

    // Ephemeral signal over the realtime broadcast channel — never persisted.
    let realtime = state.network.read().await.realtime.clone();
    realtime.broadcast(serde_json::json!({
        "type": "typing",
        "channel_id": input.channel_id,
        "user_id": identity.id.to_string(),
        "username": identity.username,
        "display_name": identity.display_name,
        "is_typing": input.is_typing,
    }));
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponse {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub banner_hash: Option<String>,
    pub bio: Option<String>,
    pub custom_status: Option<String>,
    pub online_status: String,
    pub friend_status: String,
    /// Kayıt/hesap açma tarihi (unix saniye). `showJoinDate` kapalıyken
    /// yalnızca kullanıcının kendi profili için doldurulur.
    pub created_at: Option<i64>,
}

/// Kapsamlı profil görünümü: yerel önbellekten okur, bio'yu DB anahtarıyla
/// çözer, arkadaşlık durumunu ekler. Uzak kullanıcı için de çalışır.
/// Query the Supabase `presence` table for a user's real online status,
/// applying the 90-second heartbeat TTL. Falls back to the locally cached
/// value when Supabase is unreachable or the row doesn't exist.
async fn fetch_real_presence(state: &AppState, user_id: &Uuid, local_fallback: &str) -> String {
    if !config::configured("VEILANON_SUPABASE_URL") {
        return local_fallback.to_string();
    }
    let network = state.network.read().await;
    let filter = format!("user_id=eq.{}&select=status,heartbeat_at,last_seen", user_id);
    let Ok(rows) = network.api.select::<serde_json::Value>("presence", &filter, None, Some(1)).await else {
        return local_fallback.to_string();
    };
    let Some(first) = rows.first() else {
        return local_fallback.to_string();
    };
    let raw_status = first.get("status").and_then(|v| v.as_str()).unwrap_or("offline");
    if raw_status == "offline" || raw_status == "invisible" {
        return "offline".to_string();
    }
    let hb_str = first.get("heartbeat_at")
        .or_else(|| first.get("last_seen"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if let Ok(hb_time) = chrono::DateTime::parse_from_rfc3339(hb_str) {
        let now = chrono::Utc::now();
        if (now - hb_time.with_timezone(&chrono::Utc)).num_seconds() <= 90 {
            return raw_status.to_string();
        }
    }
    "offline".to_string()
}

#[tauri::command]
pub async fn get_user_profile(user_id: String, state: State<'_, AppState>) -> Result<UserProfileResponse, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let target = parse_user_id(&user_id)?;
    let is_self = target == identity.id;

    let show_join_date = {
        let settings = state.settings.read().await;
        settings.show_join_date || is_self
    };

    // Remote sync for profile if needed
    if !is_self && config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let u_filter = format!("id=eq.{}&select=id,username,display_name,avatar_hash,banner_hash,bio,custom_status", target);
        if let Ok(u_rows) = network.api.select::<serde_json::Value>("users", &u_filter, None, Some(1)).await {
            if let Some(u) = u_rows.first() {
                let uname = u.get("username").and_then(|v| v.as_str()).unwrap_or("");
                let disp = u.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
                let av = u.get("avatar_hash").and_then(|v| v.as_str());
                let ban = u.get("banner_hash").and_then(|v| v.as_str());
                let cs = u.get("custom_status").and_then(|v| v.as_str());
                let db = state.db.read().await;
                let _ = db.upsert_profile(&target, uname, disp, av, None, None, None, None, None);
                if let Some(b) = ban {
                    let _ = db.set_user_profile_banner(&target, Some(b));
                }
                if let Some(c) = cs {
                    let _ = db.update_custom_status(&target, Some(c));
                }
            }
        }
    }

    let db_key = state.get_db_key().await;
    let db = state.db.read().await;
    let profile = db.get_profile_full_by_id(&target)?;
    let Some((username, display_name, avatar_hash, banner_hash, bio_ciphertext, online_status, custom_status)) = profile else {
        // Kendi profili henüz önbelleğe yazılmadıysa (ilk açılış) satırı oluştur.
        if is_self {
            let _ = db.upsert_profile(
                &identity.id,
                &identity.username,
                &identity.display_name,
                identity.avatar_hash.as_deref(),
                Some(identity.identity_key_public.as_str()),
                Some(identity.signing_key_public.as_str()),
                None,
                None,
                None,
            );
            if let Some(ref b) = identity.banner_hash {
                let _ = db.set_user_profile_banner(&identity.id, Some(b));
            }
            let real_status = fetch_real_presence(&state, &identity.id, "online").await;
            return Ok(UserProfileResponse {
                user_id: identity.id.to_string(),
                username: identity.username.clone(),
                display_name: identity.display_name.clone(),
                avatar_hash: identity.avatar_hash.clone(),
                banner_hash: identity.banner_hash.clone(),
                bio: None,
                custom_status: None,
                online_status: real_status,
                friend_status: "friends".into(),
                created_at: if show_join_date {
                    db.get_profile_created_at(&identity.id).ok().flatten()
                        .or_else(|| Some(chrono::Utc::now().timestamp()))
                } else {
                    None
                },
            });
        }
        return Err(VeilError::InvalidInput("User not found".into()));
    };

    let bio = match (bio_ciphertext, db_key) {
        (Some(encoded), Some(key)) => {
            let payload = B64.decode(&encoded).ok();
            payload
                .and_then(|p| {
                    if p.len() < 12 {
                        return None;
                    }
                    let split = p.len() - 12;
                    crate::crypto::decrypt_aes_gcm(&key, &p[..split], &p[split..]).ok()
                })
                .and_then(|plain| String::from_utf8(plain).ok())
                .or_else(|| Some(encoded.clone()))
        }
        (Some(plain), None) => Some(plain),
        _ => None,
    };
    let friend_status = if is_self {
        "friends".to_string()
    } else {
        let row = db.query_row(
            "SELECT status FROM friends WHERE user_id = ?1 AND friend_id = ?2",
            rusqlite::params![identity.id.to_string(), target.to_string()],
            |r| r.get::<_, String>(0),
        );
        row.unwrap_or_else(|_| "none".to_string())
    };

    let created_at = if show_join_date {
        db.get_profile_created_at(&target).ok().flatten()
    } else {
        None
    };
    drop(db);

    let real_online_status = fetch_real_presence(&state, &target, &online_status).await;

    Ok(UserProfileResponse {
        user_id: target.to_string(),
        username,
        display_name,
        avatar_hash,
        banner_hash,
        bio,
        custom_status,
        online_status: real_online_status,
        friend_status,
        created_at,
    })
}

/// Kullanıcı adından profil çözümleme — `veilanon://u/<username>` derin
/// bağlantıları ve https://veilanon.com/u/<username> paylaşım linkleri için.
#[tauri::command]
pub async fn resolve_username(username: String, state: State<'_, AppState>) -> Result<UserProfileResponse, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let username = username.trim().trim_start_matches('@');
    if username.is_empty() || username.len() > 32 {
        return Err(VeilError::InvalidInput("Invalid username".into()));
    }

    let db = state.db.read().await;
    let profile = db.get_profile_by_username(username)?;
    let Some((target, username, display_name, avatar_hash)) = profile else {
        if username.eq_ignore_ascii_case(&identity.username) {
            let full_self = db.get_profile_full_by_id(&identity.id).ok().flatten();
            let local_status = full_self.as_ref().map(|p| p.5.as_str()).unwrap_or("online");
            let real_status = fetch_real_presence(&state, &identity.id, local_status).await;
            return Ok(UserProfileResponse {
                user_id: identity.id.to_string(),
                username: identity.username.clone(),
                display_name: identity.display_name.clone(),
                avatar_hash: identity.avatar_hash.clone(),
                banner_hash: identity.banner_hash.clone(),
                bio: full_self.as_ref().and_then(|p| p.4.clone()),
                custom_status: full_self.as_ref().and_then(|p| p.6.clone()),
                online_status: real_status,
                friend_status: "friends".into(),
                created_at: None,
            });
        }
        return Err(VeilError::InvalidInput("User not found".into()));
    };

    let friend_status = if target == identity.id {
        "friends".to_string()
    } else {
        let row = db.query_row(
            "SELECT status FROM friends WHERE user_id = ?1 AND friend_id = ?2",
            rusqlite::params![identity.id.to_string(), target.to_string()],
            |r| r.get::<_, String>(0),
        );
        row.unwrap_or_else(|_| "none".to_string())
    };

    let full_target = db.get_profile_full_by_id(&target).ok().flatten();
    let banner_hash = full_target.as_ref().and_then(|p| p.3.clone());
    let custom_status = full_target.as_ref().and_then(|p| p.6.clone());
    let bio = full_target.as_ref().and_then(|p| p.4.clone());
    let local_status = full_target.as_ref().map(|p| p.5.as_str()).unwrap_or("offline");
    drop(db);

    let real_online_status = fetch_real_presence(&state, &target, local_status).await;

    Ok(UserProfileResponse {
        user_id: target.to_string(),
        username,
        display_name,
        avatar_hash,
        banner_hash,
        bio,
        custom_status,
        online_status: real_online_status,
        friend_status,
        created_at: None,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutualSpaceInfo {
    pub id: String,
    pub name: String,
    pub icon_hash: Option<String>,
    pub member_count: u32,
    pub banner_hash: Option<String>,
    pub description: Option<String>,
    pub custom_link: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutualFriendInfo {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub online_status: String,
}

#[tauri::command]
pub async fn get_mutual_spaces(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MutualSpaceInfo>, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let target = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    let spaces = db.list_spaces()?;
    let mut mutual = Vec::new();

    for space in spaces {
        if space.id == Uuid::nil() {
            continue;
        }
        let is_me_member = db.is_space_member(&space.id, &identity.id)?;
        let is_target_member = db.is_space_member(&space.id, &target)?;
        if is_me_member && is_target_member {
            mutual.push(MutualSpaceInfo {
                id: space.id.to_string(),
                name: space.name,
                icon_hash: space.icon_hash,
                member_count: space.member_count as u32,
                banner_hash: space.banner_hash,
                description: space.description,
                custom_link: space.custom_link,
            });
        }
    }

    Ok(mutual)
}

#[tauri::command]
pub async fn get_mutual_friends(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MutualFriendInfo>, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let target = parse_user_id(&user_id)?;

    let db = state.db.read().await;
    let my_friends = db.list_friends(&identity.id)?;
    let target_friends = db.list_friends(&target)?;

    let target_friend_ids: std::collections::HashSet<Uuid> = target_friends
        .iter()
        .filter(|f| f.status == "accepted" || f.status == "friends")
        .map(|f| f.user_id)
        .collect();

    let mut mutual = Vec::new();
    for mf in my_friends {
        if (mf.status == "accepted" || mf.status == "friends") && target_friend_ids.contains(&mf.user_id) && mf.user_id != identity.id && mf.user_id != target {
            let avatar_hash = db.get_profile_by_id(&mf.user_id).ok().flatten().and_then(|p| p.2);
            mutual.push(MutualFriendInfo {
                user_id: mf.user_id.to_string(),
                username: mf.username,
                display_name: mf.display_name,
                avatar_hash,
                online_status: mf.online_status,
            });
        }
    }

    Ok(mutual)
}

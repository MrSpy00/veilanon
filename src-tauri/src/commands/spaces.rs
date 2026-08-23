//! Space (community), channel, role, invite and member IPC commands
//! 
//! Local-first: every mutation is persisted locally before any network sync.
//! Supabase sync is best-effort — network failures never fail the command.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::Instant;
use tracing::{info, warn};

use crate::db::channels::{BanRow, ChannelRow, InviteRow, MemberRow, RoleRow, SpaceRow};
use crate::error::{VeilError, VeilResult};
use crate::models::channel::ChannelType;
use crate::models::space::Permissions;
use crate::state::AppState;
use crate::config;
use tauri::{Emitter, Manager, State};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

// ── IPC shapes ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInfo {
    pub id: String,
    pub name: String,
    pub icon_hash: Option<String>,
    pub owner_id: String,
    pub member_count: u32,
    pub is_owner: bool,
    pub my_roles: Vec<String>,
    pub banner_hash: Option<String>,
    pub description: Option<String>,
    pub custom_link: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub id: String,
    pub space_id: Option<String>,
    pub name: String,
    pub channel_type: String,
    pub position: i32,
    pub is_nsfw: bool,
    pub is_e2ee: bool,
    pub unread_count: u32,
    pub mentioned: bool,
    pub last_message_id: Option<String>,
    pub avatar_hash: Option<String>,
    pub peer_id: Option<String>,
    pub online_status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoleInfo {
    pub id: String,
    pub space_id: String,
    pub name: String,
    pub color: Option<String>,
    pub permissions: Vec<String>,
    pub position: i32,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteInfo {
    pub id: String,
    pub code: String,
    pub space_id: String,
    pub max_uses: Option<u32>,
    pub used_count: u32,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberInfo {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub role_ids: Vec<String>,
    pub online_status: String,
}

// ── Inputs (Tauri maps camelCase JS keys onto these snake_case fields) ───────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceCreateInput {
    pub name: String,
    pub icon_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceUpdateInput {
    pub id: String,
    pub name: Option<String>,
    pub icon_hash: Option<String>,
    pub banner_hash: Option<String>,
    pub description: Option<String>,
    pub clear_icon: Option<bool>,
    pub clear_banner: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceTransferOwnershipInput {
    pub space_id: String,
    pub new_owner_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelCreateInput {
    pub space_id: String,
    pub name: String,
    pub channel_type: String,
    pub position: Option<i32>,
    /// Metin kanalları için MLS grup E2EE (yalnızca sahip açabilir).
    pub e2ee: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelUpdateInput {
    pub id: String,
    pub name: Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleCreateInput {
    pub space_id: String,
    pub name: String,
    pub color: Option<String>,
    pub permissions: Vec<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleUpdateInput {
    pub id: String,
    pub name: Option<String>,
    pub color: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolesReorderInput {
    pub space_id: String,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOverrideItem {
    pub target_id: String,
    pub target_type: String, // "role" | "member"
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelOverridesInput {
    pub channel_id: String,
    pub overrides: Vec<ChannelOverrideItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteCreateInput {
    pub space_id: String,
    pub max_uses: Option<u32>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberUpdateInput {
    pub space_id: String,
    pub user_id: String,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanInput {
    pub space_id: String,
    pub user_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeoutInput {
    pub space_id: String,
    pub user_id: String,
    /// Unix saniye; 0 veya geçmiş bir değer susturmayı kaldırır.
    pub until: Option<i64>,
}

// ── Mapping helpers ─────────────────────────────────────────────────────────

fn to_space_info(row: &SpaceRow) -> SpaceInfo {
    SpaceInfo {
        id: row.id.to_string(),
        name: row.name.clone(),
        icon_hash: row.icon_hash.clone(),
        owner_id: row.owner_id.to_string(),
        member_count: row.member_count,
        is_owner: row.is_owner,
        my_roles: row.my_roles.iter().map(|r| r.to_string()).collect(),
        banner_hash: row.banner_hash.clone(),
        description: row.description.clone(),
        custom_link: row.custom_link.clone(),
    }
}

pub(crate) fn to_channel_info(row: &ChannelRow) -> ChannelInfo {
    ChannelInfo {
        id: row.id.to_string(),
        space_id: row.space_id.map(|s| s.to_string()),
        name: row.name.clone(),
        channel_type: row.channel_type.clone(),
        position: row.position,
        is_nsfw: row.is_nsfw,
        is_e2ee: row.is_e2ee,
        unread_count: row.unread_count,
        mentioned: row.mentioned,
        last_message_id: row.last_message_id.map(|s| s.to_string()),
        avatar_hash: None,
        peer_id: None,
        online_status: None,
    }
}

fn to_role_info(row: &RoleRow) -> RoleInfo {
    RoleInfo {
        id: row.id.to_string(),
        space_id: row.space_id.to_string(),
        name: row.name.clone(),
        color: row.color.clone(),
        permissions: row.permissions.enabled_ids(),
        position: row.position,
        is_default: row.is_default,
    }
}

fn to_invite_info(row: &InviteRow) -> InviteInfo {
    InviteInfo {
        id: row.id.to_string(),
        code: row.code.clone(),
        space_id: row.space_id.to_string(),
        max_uses: row.max_uses,
        used_count: row.used_count,
        expires_at: row.expires_at.map(|dt| dt.timestamp()),
    }
}

fn to_member_info(row: &MemberRow) -> MemberInfo {
    MemberInfo {
        user_id: row.user_id.to_string(),
        username: row.username.clone(),
        display_name: row.display_name.clone(),
        avatar_hash: row.avatar_hash.clone(),
        role_ids: row.role_ids.iter().map(|r| r.to_string()).collect(),
        online_status: row.online_status.clone(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BanInfo {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub created_at: i64,
}

fn to_ban_info(row: &BanRow) -> BanInfo {
    BanInfo {
        user_id: row.user_id.to_string(),
        username: row.username.clone(),
        display_name: row.display_name.clone(),
        banned_by: row.banned_by.to_string(),
        reason: row.reason.clone(),
        created_at: row.created_at.timestamp(),
    }
}

/// Moderasyon yetkisi: sahip veya ilgili izne sahip bir rol üyesi (Administrator dahil).
fn can_moderate(
    db: &crate::db::Database,
    space_id: &Uuid,
    user_id: &Uuid,
    permission: &str,
) -> VeilResult<bool> {
    let space = db.get_space(space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id == *user_id {
        return Ok(true);
    }
    let member = db.list_space_members(space_id)?
        .into_iter()
        .find(|m| m.user_id == *user_id);
    let Some(member) = member else { return Ok(false) };
    let roles = db.list_roles(space_id)?;
    for role in roles {
        if member.role_ids.contains(&role.id) && (role.permissions.administrator || role.permissions.enabled_ids().iter().any(|p| p == permission)) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Kullanıcının sunucudaki en yüksek rol seviyesini döndürür.
/// Sunucu sahibi için i32::MAX döner.
fn get_user_highest_role_position(
    db: &crate::db::Database,
    space_id: &Uuid,
    user_id: &Uuid,
) -> VeilResult<i32> {
    let space = db.get_space(space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id == *user_id {
        return Ok(i32::MAX);
    }
    let member = db.list_space_members(space_id)?
        .into_iter()
        .find(|m| m.user_id == *user_id);
    let Some(member) = member else { return Ok(0) };
    let roles = db.list_roles(space_id)?;
    let mut highest = 0;
    for role in roles {
        if member.role_ids.contains(&role.id) && role.position > highest {
            highest = role.position;
        }
    }
    Ok(highest)
}

/// Hiyerarşik moderasyon kontrolü:
/// Çağıran üye hedef üyeyi atabilir / susturabilir / yasaklayabilir / rolünü değiştirebilir mi?
fn can_moderate_target(
    db: &crate::db::Database,
    space_id: &Uuid,
    caller_id: &Uuid,
    target_id: &Uuid,
    required_permission: &str,
) -> VeilResult<bool> {
    let space = db.get_space(space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if *caller_id == *target_id {
        return Ok(false); // Kendi kendini atamaz/yasaklayamaz
    }
    if space.owner_id == *target_id {
        return Ok(false); // Sahibe hiçbir işlem yapılamaz
    }
    if space.owner_id == *caller_id {
        return Ok(true); // Sahip herkesi yönetebilir
    }
    if !can_moderate(db, space_id, caller_id, required_permission)? {
        return Ok(false);
    }
    let caller_rank = get_user_highest_role_position(db, space_id, caller_id)?;
    let target_rank = get_user_highest_role_position(db, space_id, target_id)?;
    Ok(caller_rank > target_rank)
}

/// Rol yönetimi hiyerarşi kontrolü:
/// Çağıran kişi belirli pozisyondaki bir rolü düzenleyebilir / silebilir / oluşturabilir mi?
fn can_manage_role_position(
    db: &crate::db::Database,
    space_id: &Uuid,
    caller_id: &Uuid,
    target_role_position: i32,
) -> VeilResult<bool> {
    let space = db.get_space(space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id == *caller_id || space.is_owner {
        return Ok(true);
    }
    if !can_moderate(db, space_id, caller_id, "manage_roles")? {
        return Ok(false);
    }
    let caller_rank = get_user_highest_role_position(db, space_id, caller_id)?;
    Ok(caller_rank > target_role_position)
}

fn parse_space_id(s: &str) -> VeilResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| VeilError::InvalidInput("Invalid space ID".into()))
}

fn parse_channel_id(s: &str) -> VeilResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| VeilError::InvalidInput("Invalid channel ID".into()))
}

fn parse_channel_type(s: &str) -> VeilResult<ChannelType> {
    match s {
        "text" => Ok(ChannelType::Text),
        "voice" => Ok(ChannelType::Voice),
        "announcement" => Ok(ChannelType::Announcement),
        "forum" => Ok(ChannelType::Forum),
        "category" => Ok(ChannelType::Category),
        _ => Err(VeilError::InvalidInput("Invalid channel type".into())),
    }
}

/// Best-effort Supabase push — never fails the command; skipped entirely when
/// the Supabase URL env var is not configured.
async fn best_effort_insert(state: &AppState, table: &str, payload: serde_json::Value) {
    if !config::configured("VEILANON_SUPABASE_URL") {
        return;
    }
    // Supabase columns are NOT NULL with defaults; a JSON null from an
    // Option<T> would be rejected with 400. Drop nulls so the DB default
    // ('' / 0 / false) applies instead.
    let payload = match payload {
        serde_json::Value::Object(map) => {
            let cleaned: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .filter(|(_, v)| !v.is_null())
                .collect();
            serde_json::Value::Object(cleaned)
        }
        other => other,
    };
    let network = state.network.read().await;
    // Plain insert, not upsert: PostgREST `on_conflict` + RLS rejects
    // rows where the conflict column differs from auth.uid() (403) —
    // e.g. spaces.id vs owner_id. Clients always mint fresh UUIDs, so
    // the 409 collision window is effectively zero.
    let _ = network.api.insert(table, &payload).await;
}

/// Best-effort Supabase update — never fails the command.
async fn best_effort_update(state: &AppState, table: &str, payload: &serde_json::Value, filter: &str) {
    if !config::configured("VEILANON_SUPABASE_URL") {
        return;
    }
    let network = state.network.read().await;
    let _ = network.api.update(table, filter, payload).await;
}

/// Best-effort Supabase delete — never fails the command.
async fn best_effort_delete(state: &AppState, table: &str, filter: &str) {
    if !config::configured("VEILANON_SUPABASE_URL") {
        return;
    }
    let network = state.network.read().await;
    let _ = network.api.delete(table, filter).await;
}

// ── Spaces ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn spaces_list(state: State<'_, AppState>) -> Result<Vec<SpaceInfo>, VeilError> {
    let user_id = {
        let identity = state.get_or_restore_identity().await;
        let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
        identity.id
    };

    // Remote sync: üye olunan veya sahip olunan sunucuları Supabase'den çek
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("user_id=eq.{}&select=space_id", user_id);
            let owner_filter = format!("owner_id=eq.{}&select=id", user_id);

            let mut all_space_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

            let mut sync_performed = false;
            if let Ok(memberships) = network.api.select::<serde_json::Value>("memberships", &filter, None, Some(100)).await {
                sync_performed = true;
                for m in memberships {
                    if let Some(sid) = m.get("space_id").and_then(|v| v.as_str()) {
                        all_space_ids.insert(sid.to_string());
                    }
                }
            }

            if let Ok(owned) = network.api.select::<serde_json::Value>("spaces", &owner_filter, None, Some(100)).await {
                sync_performed = true;
                for o in owned {
                    if let Some(sid) = o.get("id").and_then(|v| v.as_str()) {
                        all_space_ids.insert(sid.to_string());
                    }
                }
            }

            // Çift yönlü mutabakat: Sunucudan silinen sunucuları yerel SQLite'tan da sil
            if sync_performed {
                let db = state.db.read().await;
                if let Ok(local_spaces) = db.list_spaces() {
                    for ls in local_spaces {
                        if ls.id != Uuid::nil() && !all_space_ids.contains(&ls.id.to_string()) {
                            let _ = db.delete_space(&ls.id);
                        }
                    }
                }
            }

            let space_ids: Vec<String> = all_space_ids.into_iter().collect();
            
            if !space_ids.is_empty() {
                let spaces_filter = format!("id=in.({})&select=id,name,icon_hash,owner_id,custom_link,banner_hash,description", space_ids.join(","));
                if let Ok(remote_spaces) = network.api.select::<serde_json::Value>("spaces", &spaces_filter, None, Some(100)).await {
                    let db = state.db.read().await;
                    let db_key = state.get_db_key().await;
                    let mut fetched_ids = std::collections::HashSet::new();

                    for s in remote_spaces {
                        if let (Some(id_str), Some(name)) = (s.get("id").and_then(|v| v.as_str()), s.get("name").and_then(|v| v.as_str())) {
                            if let Ok(id) = Uuid::parse_str(id_str) {
                                fetched_ids.insert(id);
                                let icon = s.get("icon_hash").and_then(|v| v.as_str());
                                let link = s.get("custom_link").and_then(|v| v.as_str());
                                let banner = s.get("banner_hash").and_then(|v| v.as_str());
                                let desc = s.get("description").and_then(|v| v.as_str());
                                let remote_owner_str = s.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
                                let remote_owner_id = Uuid::parse_str(remote_owner_str).unwrap_or(user_id);
                                let modified = state.spaces_modified.read().await;
                                let skip_owner = if let Some(last_mod) = modified.get(&id.to_string()) {
                                    last_mod.elapsed().as_secs() < 10
                                } else {
                                    false
                                };
                                drop(modified);
                                let effective_owner = if skip_owner {
                                    // Keep local owner_id to avoid overwriting a just-completed transfer
                                    let db = state.db.read().await;
                                    if let Ok(Some(local)) = db.get_space(&id) {
                                        local.owner_id
                                    } else {
                                        remote_owner_id
                                    }
                                } else {
                                    remote_owner_id
                                };
                                let _ = db.insert_space_full(&id, name, icon, &effective_owner, banner, desc, link);
                                let _ = db.add_space_member(&id, &user_id);
                                let is_current_user_owner = effective_owner == user_id;
                                let _ = db.set_space_owner(&id, &user_id, is_current_user_owner);

                                // 1. Channels sync
                                let ch_filter = format!("space_id=eq.{}&select=id,name,channel_type,position,is_nsfw,is_e2ee", id);
                                if let Ok(ch_rows) = network.api.select::<serde_json::Value>("channels", &ch_filter, Some("position.asc"), Some(100)).await {
                                    for ch in ch_rows {
                                        if let (Some(cid_str), Some(cname), Some(ctype_str)) = (
                                             ch.get("id").and_then(|v| v.as_str()),
                                             ch.get("name").and_then(|v| v.as_str()),
                                             ch.get("channel_type").and_then(|v| v.as_str()),
                                         ) {
                                             if let Ok(cid) = Uuid::parse_str(cid_str) {
                                                 let pos = ch.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                                 let is_nsfw = ch.get("is_nsfw").and_then(|v| v.as_bool()).unwrap_or(false);
                                                 let is_e2ee = ch.get("is_e2ee").and_then(|v| v.as_bool()).unwrap_or(false);
                                                 let ctype = match ctype_str {
                                                     "voice" => ChannelType::Voice,
                                                     "announcement" => ChannelType::Announcement,
                                                     "forum" => ChannelType::Forum,
                                                     _ => ChannelType::Text,
                                                 };
                                                 let channel = crate::models::channel::Channel {
                                                     id: cid,
                                                     space_id: Some(id),
                                                     name: cname.to_string(),
                                                     channel_type: ctype,
                                                     position: pos,
                                                     topic: None,
                                                     is_nsfw,
                                                     is_e2ee,
                                                     slow_mode_seconds: 0,
                                                     permission_overrides: Vec::new(),
                                                     created_at: chrono::Utc::now(),
                                                     last_message_id: None,
                                                     unread_count: 0,
                                                     mentioned: false,
                                                 };
                                                 let _ = db.upsert_channel(&channel, db_key.as_ref());
                                             }
                                         }
                                    }
                                }

                                // 2. Roles sync
                                let r_filter = format!("space_id=eq.{}&select=id,name,color,permissions,position", id);
                                if let Ok(r_rows) = network.api.select::<serde_json::Value>("roles", &r_filter, Some("position.asc"), Some(100)).await {
                                    for r in r_rows {
                                        if let (Some(rid_str), Some(rname)) = (
                                             r.get("id").and_then(|v| v.as_str()),
                                             r.get("name").and_then(|v| v.as_str()),
                                         ) {
                                             if let Ok(rid) = Uuid::parse_str(rid_str) {
                                                 let color = r.get("color").and_then(|v| v.as_str());
                                                 let pos = r.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                                 let perms: Permissions = r.get("permissions")
                                                     .and_then(|v| serde_json::from_value(v.clone()).ok())
                                                     .unwrap_or_default();
                                                 let _ = db.upsert_role(&rid, &id, rname, color, &perms, pos);
                                             }
                                         }
                                    }
                                }

                                // 3. Space members sync
                                let mem_filter = format!("space_id=eq.{}&select=user_id", id);
                                if let Ok(mem_rows) = network.api.select::<serde_json::Value>("memberships", &mem_filter, None, Some(100)).await {
                                    for m in mem_rows {
                                        if let Some(uid_str) = m.get("user_id").and_then(|v| v.as_str()) {
                                             if let Ok(uid) = Uuid::parse_str(uid_str) {
                                                 let _ = db.add_space_member(&id, &uid);
                                             }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 4. Role members for current user sync
                    let rm_filter = format!("user_id=eq.{}&select=role_id", user_id);
                    if let Ok(rm_rows) = network.api.select::<serde_json::Value>("role_members", &rm_filter, None, Some(100)).await {
                        let mut my_rids = Vec::new();
                        for rm in rm_rows {
                            if let Some(rid_str) = rm.get("role_id").and_then(|v| v.as_str()) {
                                if let Ok(rid) = Uuid::parse_str(rid_str) {
                                     my_rids.push(rid);
                                }
                            }
                        }
                        for space_id in &fetched_ids {
                            let _ = db.update_space_member_roles(space_id, &user_id, &my_rids);
                        }
                    }
                }
            }
        }

    let db = state.db.read().await;
    let rows = db.list_spaces()?;
    Ok(rows.iter().map(to_space_info).collect())
}

#[tauri::command]
pub async fn spaces_create(
    input: SpaceCreateInput,
    state: State<'_, AppState>,
) -> Result<SpaceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;

    if input.name.trim().is_empty() || input.name.len() > 64 {
        return Err(VeilError::InvalidInput("Space name must be 1-64 characters".into()));
    }

    let id = Uuid::new_v4();
    {
        let db = state.db.read().await;
        db.insert_space(&id, input.name.trim(), input.icon_hash.as_deref(), &identity.id)?;
        db.set_space_owner(&id, &identity.id, true)?;
        let _ = db.add_space_member(&id, &identity.id);
        let _ = db.upsert_profile(
            &identity.id,
            &identity.username,
            &identity.display_name,
            identity.avatar_hash.as_deref(),
            Some(&identity.identity_key_public),
            Some(&identity.signing_key_public),
            None,
            None,
            None,
        );
        let row = db.get_space(&id)?.ok_or(VeilError::DatabaseError(rusqlite::Error::QueryReturnedNoRows))?;
        let info = to_space_info(&row);
        drop(db);

        // Varsayılan kanallar: her topluluk #genel metin + Genel ses kanalıyla
        // açılır (Discord konvansiyonu). Sahibi isteyen silebilir.
        let db_key = state.get_db_key().await;
        let default_channels = [
            (Uuid::new_v4(), "genel".to_string(), ChannelType::Text),
            (Uuid::new_v4(), "Genel".to_string(), ChannelType::Voice),
        ];
        let mut payloads = Vec::with_capacity(2);
        for (idx, (chan_id, chan_name, chan_type)) in default_channels.iter().enumerate() {
            let channel = crate::models::channel::Channel {
                id: *chan_id,
                space_id: Some(id),
                name: chan_name.clone(),
                channel_type: chan_type.clone(),
                position: idx as i32,
                topic: None,
                is_nsfw: false,
                is_e2ee: false,
                slow_mode_seconds: 0,
                permission_overrides: Vec::new(),
                created_at: chrono::Utc::now(),
                last_message_id: None,
                unread_count: 0,
                mentioned: false,
            };
            {
                let db = state.db.read().await;
                db.upsert_channel(&channel, db_key.as_ref())?;
            }
            payloads.push(serde_json::json!({
                "id": chan_id.to_string(),
                "space_id": id.to_string(),
                "name": chan_name,
                "channel_type": format!("{:?}", chan_type).to_lowercase(),
                "position": idx,
            }));
        }

        best_effort_insert(
            &state,
            "spaces",
            serde_json::json!({
                "id": id.to_string(),
                "name": input.name.trim(),
                "icon_hash": input.icon_hash,
                "owner_id": identity.id.to_string(),
            }),
        )
        .await;
        best_effort_insert(
            &state,
            "memberships",
            serde_json::json!({
                "user_id": identity.id.to_string(),
                "space_id": id.to_string(),
            }),
        )
        .await;
        for payload in payloads {
            best_effort_insert(&state, "channels", payload).await;
        }
        info!("Space created");
        Ok(info)
    }
}

#[tauri::command]
pub async fn spaces_update(
    input: SpaceUpdateInput,
    state: State<'_, AppState>,
) -> Result<SpaceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.id)?;

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if row.owner_id != identity.id {
        return Err(VeilError::PermissionDenied);
    }

    if let Some(name) = &input.name {
        if name.trim().is_empty() || name.len() > 64 {
            return Err(VeilError::InvalidInput("Space name must be 1-64 characters".into()));
        }
    }
    if let Some(desc) = &input.description {
        if desc.chars().count() > 300 {
            return Err(VeilError::InvalidInput("Space description must be at most 300 characters".into()));
        }
    }

    let icon_opt = if input.clear_icon.unwrap_or(false) {
        Some(None)
    } else if let Some(i) = &input.icon_hash {
        Some(Some(i.as_str()))
    } else {
        None
    };

    let banner_opt = if input.clear_banner.unwrap_or(false) {
        Some(None)
    } else if let Some(b) = &input.banner_hash {
        Some(Some(b.as_str()))
    } else {
        None
    };

    let desc_opt = input.description.as_deref().map(Some);

    db.update_space(
        &space_id,
        input.name.as_deref(),
        icon_opt,
        banner_opt,
        desc_opt,
    )?;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    drop(db);

    let mut remote_fields = serde_json::Map::new();
    if let Some(name) = &input.name {
        remote_fields.insert("name".into(), serde_json::json!(name));
    }
    if input.clear_icon.unwrap_or(false) {
        remote_fields.insert("icon_hash".into(), serde_json::Value::Null);
    } else if let Some(icon) = &input.icon_hash {
        remote_fields.insert("icon_hash".into(), serde_json::json!(icon));
    }
    if input.clear_banner.unwrap_or(false) {
        remote_fields.insert("banner_hash".into(), serde_json::Value::Null);
    } else if let Some(banner) = &input.banner_hash {
        remote_fields.insert("banner_hash".into(), serde_json::json!(banner));
    }
    if let Some(desc) = &input.description {
        remote_fields.insert("description".into(), serde_json::json!(desc));
    }
    if !remote_fields.is_empty() {
        best_effort_update(
            &state,
            "spaces",
            &serde_json::Value::Object(remote_fields),
            &format!("id=eq.{}", space_id),
        )
        .await;
    }

    let _ = state.app.emit("space:updated", serde_json::json!({ "spaceId": space_id.to_string() }));
    Ok(to_space_info(&row))
}

#[tauri::command]
pub async fn spaces_transfer_ownership(
    input: SpaceTransferOwnershipInput,
    state: State<'_, AppState>,
) -> Result<SpaceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let new_owner_id = Uuid::parse_str(&input.new_owner_id)
        .map_err(|_| VeilError::InvalidInput("Geçersiz hedef kullanıcı ID".into()))?;

    if new_owner_id == identity.id {
        return Err(VeilError::InvalidInput("Zaten bu topluluğun kurucu sahibisiniz".into()));
    }

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Topluluk bulunamadı".into()))?;
    if row.owner_id != identity.id {
        return Err(VeilError::PermissionDenied);
    }
    let mut is_member = db.is_space_member(&space_id, &new_owner_id)?;
    drop(db);

    if !is_member && config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("space_id=eq.{}&user_id=eq.{}", space_id, new_owner_id);
        if let Ok(memberships) = network.api.select::<serde_json::Value>("memberships", &filter, None, Some(1)).await {
            if !memberships.is_empty() {
                let db = state.db.read().await;
                let _ = db.add_space_member(&space_id, &new_owner_id);
                is_member = true;
            }
        }
    }

    if !is_member {
        return Err(VeilError::InvalidInput("Yeni sahip bu topluluğun bir üyesi olmalıdır".into()));
    }

    let db = state.db.read().await;
    db.transfer_space_ownership(&space_id, &new_owner_id)?;
    drop(db);

    // Record timestamp so spaces_list skips Supabase owner_id overwrite for 10s
    {
        let mut modified = state.spaces_modified.write().await;
        modified.insert(space_id.to_string(), Instant::now());
    }

    best_effort_update(
        &state,
        "spaces",
        &serde_json::json!({ "owner_id": new_owner_id.to_string() }),
        &format!("id=eq.{}", space_id),
    )
    .await;

    let db = state.db.read().await;
    let updated = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    let info = to_space_info(&updated);
    let _ = state.app.emit("space:updated", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("spaces:changed", ());
    let _ = state.app.emit("members:changed", serde_json::json!({ "spaceId": space_id.to_string() }));
    info!("Transferred ownership of space {} to {}", space_id, new_owner_id);
    Ok(info)
}

fn resolve_image_bytes(path_or_data: &str) -> Result<(Vec<u8>, String), VeilError> {
    if path_or_data.starts_with("data:image/") {
        let (header, b64) = path_or_data
            .split_once(',')
            .ok_or_else(|| VeilError::InvalidInput("Geçersiz data URL".into()))?;
        let ext = if header.contains("png") {
            "png"
        } else if header.contains("webp") {
            "webp"
        } else if header.contains("gif") {
            "gif"
        } else {
            "jpg"
        };
        let bytes = B64
            .decode(b64)
            .map_err(|_| VeilError::InvalidInput("Geçersiz base64 görsel verisi".into()))?;
        Ok((bytes, ext.to_string()))
    } else {
        let ext = std::path::Path::new(path_or_data)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp"))
            .ok_or_else(|| VeilError::InvalidInput("Desteklenen formatlar: PNG, JPG, GIF, WEBP".into()))?;
        let bytes = std::fs::read(path_or_data)
            .map_err(|e| VeilError::FileError(e))?;
        Ok((bytes, ext))
    }
}

/// Store a banner image for a space (owner only). The image is copied into
/// the app data dir as a `local-*` hash; the banner never leaves the device.
#[tauri::command]
pub async fn spaces_set_banner(
    space_id: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if row.owner_id != identity.id {
        return Err(VeilError::PermissionDenied);
    }

    let (bytes, ext) = resolve_image_bytes(&path)?;

    let data_dir = state
        .app
        .path()
        .app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let avatars_dir = data_dir.join("avatars");
    std::fs::create_dir_all(&avatars_dir)?;

    let hash = format!("local-{}.{}", Uuid::new_v4(), ext);
    let dest = avatars_dir.join(&hash);
    std::fs::write(&dest, &bytes)?;

    db.update_space(&space_id, None, None, Some(Some(&hash)), None)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .upload_blob(&format!("files/banners/{}", hash), bytes.clone())
            .await;
        let _ = network
            .api
            .upload_blob(&format!("files/avatars/{}", hash), bytes.clone())
            .await;
    }

    best_effort_update(
        &state,
        "spaces",
        &serde_json::json!({ "banner_hash": hash }),
        &format!("id=eq.{}", space_id),
    )
    .await;

    let _ = state.app.emit("space:updated", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("spaces:changed", ());
    info!("Space banner updated");
    Ok(hash)
}

/// Store an icon image for a space (owner only). Same storage as banners.
#[tauri::command]
pub async fn spaces_set_icon(
    space_id: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if row.owner_id != identity.id {
        return Err(VeilError::PermissionDenied);
    }

    let (bytes, ext) = resolve_image_bytes(&path)?;

    let data_dir = state
        .app
        .path()
        .app_data_dir()
        .map_err(|_| VeilError::FileError(std::io::Error::new(std::io::ErrorKind::NotFound, "app data dir")))?;
    let avatars_dir = data_dir.join("avatars");
    std::fs::create_dir_all(&avatars_dir)?;

    let hash = format!("local-{}.{}", Uuid::new_v4(), ext);
    let dest = avatars_dir.join(&hash);
    std::fs::write(&dest, &bytes)?;

    db.update_space(&space_id, None, Some(Some(&hash)), None, None)?;
    drop(db);

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .upload_blob(&format!("files/avatars/{}", hash), bytes.clone())
            .await;
    }

    best_effort_update(
        &state,
        "spaces",
        &serde_json::json!({ "icon_hash": hash }),
        &format!("id=eq.{}", space_id),
    )
    .await;

    let _ = state.app.emit("space:updated", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("spaces:changed", ());
    info!("Space icon updated");
    Ok(hash)
}

#[tauri::command]
pub async fn spaces_delete(space_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if row.owner_id != identity.id {
        return Err(VeilError::PermissionDenied);
    }
    db.delete_space(&space_id)?;
    drop(db);

    // Supabase: space'i sil, ilişkili kanalları, üyeleri ve davetleri temizle
    best_effort_delete(&state, "memberships", &format!("space_id=eq.{}", space_id)).await;
    best_effort_delete(&state, "channels", &format!("space_id=eq.{}", space_id)).await;
    let space_roles = {
        let db = state.db.read().await;
        db.list_roles(&space_id).unwrap_or_default()
    };
    let space_role_ids: Vec<String> = space_roles.iter().map(|r| r.id.to_string()).collect();
    if !space_role_ids.is_empty() {
        best_effort_delete(&state, "role_members", &format!("role_id=in.({})", space_role_ids.join(","))).await;
    }
    best_effort_delete(&state, "roles", &format!("space_id=eq.{}", space_id)).await;
    best_effort_delete(&state, "spaces", &format!("id=eq.{}", space_id)).await;

    let _ = state.app.emit("space:deleted", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("spaces:changed", ());
    info!("Space deleted");
    Ok(())
}

/// Topluluktan ayrıl (sahip olmayan üyeler için)
#[tauri::command]
pub async fn spaces_leave(space_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    {
        let db = state.db.read().await;
        if let Ok(Some(sp)) = db.get_space(&space_id) {
            if sp.owner_id == identity.id {
                return Err(VeilError::InvalidInput("Topluluk sahibi sunucudan ayrılamaz. Önce sahipliği devredin veya sunucuyu silin.".into()));
            }
        }
        let _ = db.remove_space_member(&space_id, &identity.id);
        let _ = db.delete_space(&space_id);
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("space_id=eq.{}&user_id=eq.{}", space_id, identity.id);
        let _ = network.api.delete("memberships", &filter).await;
    }

    let _ = state.app.emit("space:deleted", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("spaces:changed", ());
    info!("Left space: {}", space_id);
    Ok(())
}

// ── Channels ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn channels_list(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChannelInfo>, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;
    let db = state.db.read().await;
    let rows = db.get_channels_for_space(&space_id)?;
    Ok(rows.iter().map(to_channel_info).collect())
}

#[tauri::command]
pub async fn channels_create(
    input: ChannelCreateInput,
    state: State<'_, AppState>,
) -> Result<ChannelInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let _identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let channel_type = parse_channel_type(&input.channel_type)?;

    if input.name.trim().is_empty() || input.name.len() > 64 {
        return Err(VeilError::InvalidInput("Channel name must be 1-64 characters".into()));
    }

    let db = state.db.read().await;
    let position = input.position.unwrap_or(db.next_channel_position(Some(&space_id))?);
    let want_e2ee = input.e2ee.unwrap_or(false)
        && matches!(channel_type, ChannelType::Text | ChannelType::Voice);
    let channel = crate::models::channel::Channel {
        id: Uuid::new_v4(),
        space_id: Some(space_id),
        name: input.name.trim().to_string(),
        channel_type: channel_type.clone(),
        position,
        topic: None,
        is_nsfw: false,
        is_e2ee: want_e2ee,
        slow_mode_seconds: 0,
        permission_overrides: Vec::new(),
        created_at: chrono::Utc::now(),
        last_message_id: None,
        unread_count: 0,
        mentioned: false,
    };
    let db_key = state.get_db_key().await;
    db.upsert_channel(&channel, db_key.as_ref())?;
    let row = db
        .get_channel(&channel.id, db_key.as_ref())?
        .ok_or(VeilError::InvalidInput("Channel not found".into()))?;

    let info = ChannelInfo {
        id: channel.id.to_string(),
        space_id: Some(space_id.to_string()),
        name: channel.name,
        channel_type: format!("{:?}", channel_type).to_lowercase(),
        position,
        is_nsfw: false,
        is_e2ee: want_e2ee,
        unread_count: 0,
        mentioned: false,
        last_message_id: None,
        avatar_hash: None,
        peer_id: None,
        online_status: None,
    };
    let _ = row; // Full model available for future topic support
    drop(db);

    // E2EE kanalı: MLS grup oturumunu hemen başlat (sahip üyedir).
    if want_e2ee {
        super::mls::mls_init_channel(info.id.clone(), state.clone()).await?;
    }

    best_effort_insert(
        &state,
        "channels",
        serde_json::json!({
            "id": info.id,
            "space_id": info.space_id,
            "name": info.name,
            "channel_type": info.channel_type,
            "position": info.position,
            "is_e2ee": want_e2ee,
        }),
    )
    .await;
    info!("Channel created");
    Ok(info)
}

#[tauri::command]
pub async fn channels_update(
    input: ChannelUpdateInput,
    state: State<'_, AppState>,
) -> Result<ChannelInfo, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&input.id)?;
    let db = state.db.read().await;
    let db_key = state.get_db_key().await;
    let mut channel = db
        .get_channel(&channel_id, db_key.as_ref())?
        .ok_or(VeilError::InvalidInput("Channel not found".into()))?;

    if let Some(name) = &input.name {
        if name.trim().is_empty() || name.len() > 64 {
            return Err(VeilError::InvalidInput("Channel name must be 1-64 characters".into()));
        }
        channel.name = name.trim().to_string();
    }
    if let Some(position) = input.position {
        channel.position = position;
    }
    db.upsert_channel(&channel, db_key.as_ref())?;

    let rows = db.get_channels_for_space(&channel.space_id.ok_or(VeilError::InvalidInput("DM channels are not editable here".into()))?)?;
    let row = rows
        .iter()
        .find(|r| r.id == channel_id)
        .ok_or(VeilError::InvalidInput("Channel not found".into()))?;
    Ok(to_channel_info(row))
}

#[tauri::command]
pub async fn channels_delete(channel_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&channel_id)?;

    let db = state.db.read().await;
    let db_key = state.get_db_key().await;
    let channel = db
        .get_channel(&channel_id, db_key.as_ref())?
        .ok_or(VeilError::InvalidInput("Channel not found".into()))?;

    if let Some(space_id) = channel.space_id {
        let space = db
            .get_space(&space_id)?
            .ok_or(VeilError::InvalidInput("Space not found".into()))?;
        if space.owner_id != identity.id {
            return Err(VeilError::PermissionDenied);
        }
    }

    db.delete_channel(&channel_id)?;
    drop(db);

    best_effort_delete(&state, "messages", &format!("channel_id=eq.{}", channel_id)).await;
    best_effort_delete(&state, "channel_members", &format!("channel_id=eq.{}", channel_id)).await;
    best_effort_delete(&state, "channels", &format!("id=eq.{}", channel_id)).await;

    let _ = state.app.emit("channel:deleted", serde_json::json!({ "channelId": channel_id.to_string() }));
    info!("Channel deleted");
    Ok(())
}

// ── Roles ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn roles_list(space_id: String, state: State<'_, AppState>) -> Result<Vec<RoleInfo>, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("space_id=eq.{}&select=id,space_id,name,color,permissions,position", space_id);
        if let Ok(remote_roles) = network.api.select::<serde_json::Value>("roles", &filter, Some("position.asc"), Some(100)).await {
            let db = state.db.read().await;
            for r in remote_roles {
                if let (Some(id_str), Some(name)) = (r.get("id").and_then(|v| v.as_str()), r.get("name").and_then(|v| v.as_str())) {
                    if let Ok(id) = Uuid::parse_str(id_str) {
                        let color = r.get("color").and_then(|v| v.as_str());
                        let position = r.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let perms: Permissions = r.get("permissions")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default();
                        let _ = db.upsert_role(&id, &space_id, name, color, &perms, position);
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    let rows = db.list_roles(&space_id)?;
    Ok(rows.iter().map(to_role_info).collect())
}

#[tauri::command]
pub async fn roles_create(
    input: RoleCreateInput,
    state: State<'_, AppState>,
) -> Result<RoleInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;

    if input.name.trim().is_empty() || input.name.len() > 64 {
        return Err(VeilError::InvalidInput("Role name must be 1-64 characters".into()));
    }

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id != identity.id && !space.is_owner && !can_moderate(&db, &space_id, &identity.id, "manage_roles")? {
        return Err(VeilError::PermissionDenied);
    }

    let position = match input.position {
        Some(pos) => {
            if !can_manage_role_position(&db, &space_id, &identity.id, pos)? {
                return Err(VeilError::PermissionDenied);
            }
            pos
        }
        None => db.next_role_position(&space_id)?,
    };

    let id = Uuid::new_v4();
    let mut permissions = Permissions::default();
    permissions.apply_ids(&input.permissions);
    db.insert_role(&id, &space_id, input.name.trim(), input.color.as_deref(), &permissions, position)?;
    let row = db.get_role(&id)?.ok_or(VeilError::InvalidInput("Role not found".into()))?;

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .insert(
                "roles",
                &serde_json::json!({
                    "id": id.to_string(),
                    "space_id": space_id.to_string(),
                    "name": input.name.trim(),
                    "color": input.color.as_deref().unwrap_or(""),
                    "position": position,
                    "permissions": serde_json::to_value(&permissions.enabled_ids()).unwrap_or_default(),
                }),
            )
            .await;
    }

    let _ = state.app.emit("roles:changed", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("members:changed", serde_json::json!({ "spaceId": space_id.to_string() }));
    Ok(to_role_info(&row))
}

#[tauri::command]
pub async fn roles_update(
    input: RoleUpdateInput,
    state: State<'_, AppState>,
) -> Result<RoleInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let role_id = Uuid::parse_str(&input.id).map_err(|_| VeilError::InvalidInput("Invalid role ID".into()))?;
    let db = state.db.read().await;

    let role = db.get_role(&role_id)?.ok_or(VeilError::InvalidInput("Role not found".into()))?;
    let space = db.get_space(&role.space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id != identity.id && !space.is_owner && !can_moderate(&db, &role.space_id, &identity.id, "manage_roles")? {
        return Err(VeilError::PermissionDenied);
    }

    if !can_manage_role_position(&db, &role.space_id, &identity.id, role.position)? {
        return Err(VeilError::PermissionDenied);
    }

    if let Some(new_pos) = input.position {
        if !can_manage_role_position(&db, &role.space_id, &identity.id, new_pos)? {
            return Err(VeilError::PermissionDenied);
        }
    }

    let permissions = input.permissions.as_ref().map(|ids| {
        let mut p = Permissions::default();
        p.apply_ids(ids);
        p
    });
    db.update_role(
        &role_id,
        input.name.as_deref(),
        Some(input.color.as_deref()),
        permissions.as_ref(),
        input.position,
    )?;
    let row = db.get_role(&role_id)?.ok_or(VeilError::InvalidInput("Role not found".into()))?;

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let mut patch = serde_json::Map::new();
        if let Some(n) = &input.name {
            patch.insert("name".into(), serde_json::Value::String(n.clone()));
        }
        if let Some(c) = &input.color {
            patch.insert("color".into(), serde_json::Value::String(c.clone()));
        }
        if let Some(p) = input.position {
            patch.insert("position".into(), serde_json::Value::Number(p.into()));
        }
        if let Some(perm) = &permissions {
            patch.insert("permissions".into(), serde_json::to_value(&perm.enabled_ids()).unwrap_or_default());
        }
        let _ = network
            .api
            .update("roles", &format!("id=eq.{}", role_id), &serde_json::Value::Object(patch))
            .await;
    }

    let _ = state.app.emit("roles:changed", serde_json::json!({ "spaceId": role.space_id.to_string() }));
    let _ = state.app.emit("members:changed", serde_json::json!({ "spaceId": role.space_id.to_string() }));
    Ok(to_role_info(&row))
}

#[tauri::command]
pub async fn roles_reorder(
    input: RolesReorderInput,
    state: State<'_, AppState>,
) -> Result<Vec<RoleInfo>, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let db = state.db.read().await;

    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id != identity.id && !space.is_owner && !can_moderate(&db, &space_id, &identity.id, "manage_roles")? {
        return Err(VeilError::PermissionDenied);
    }

    let role_uuids: Vec<Uuid> = input
        .role_ids
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    db.reorder_roles(&space_id, &role_uuids)?;
    let _ = state.app.emit("roles:changed", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("members:changed", serde_json::json!({ "spaceId": space_id.to_string() }));
    let updated_rows = db.list_roles(&space_id)?;
    Ok(updated_rows.iter().map(to_role_info).collect())
}

#[tauri::command]
pub async fn roles_delete(role_id: String, state: State<'_, AppState>) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let role_id = Uuid::parse_str(&role_id).map_err(|_| VeilError::InvalidInput("Invalid role ID".into()))?;
    let db = state.db.read().await;

    let role = db.get_role(&role_id)?.ok_or(VeilError::InvalidInput("Role not found".into()))?;
    let space = db.get_space(&role.space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id != identity.id && !space.is_owner && !can_moderate(&db, &role.space_id, &identity.id, "manage_roles")? {
        return Err(VeilError::PermissionDenied);
    }

    if !can_manage_role_position(&db, &role.space_id, &identity.id, role.position)? {
        return Err(VeilError::PermissionDenied);
    }

    db.delete_role(&role_id)?;

    // Rolü taşıyan üyelerin atamalarını temizle (JSON dizisinden çıkar).
    let members = db.list_space_members(&role.space_id)?;
    for member in members {
        if member.role_ids.contains(&role_id) {
            let remaining: Vec<Uuid> = member
                .role_ids
                .iter()
                .filter(|r| **r != role_id)
                .copied()
                .collect();
            let _ = db.update_space_member_roles(&role.space_id, &member.user_id, &remaining);
        }
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let _ = network
            .api
            .delete("roles", &format!("id=eq.{}", role_id))
            .await;
    }

    let _ = state.app.emit("roles:changed", serde_json::json!({ "spaceId": role.space_id.to_string() }));
    let _ = state.app.emit("members:changed", serde_json::json!({ "spaceId": role.space_id.to_string() }));
    info!("Role deleted");
    Ok(())
}

// ── Channel Overrides ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn channels_update_overrides(
    input: ChannelOverridesInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&input.channel_id)?;
    let db = state.db.read().await;
    let db_key = state.get_db_key().await;
    let channel = db
        .get_channel(&channel_id, db_key.as_ref())?
        .ok_or(VeilError::InvalidInput("Channel not found".into()))?;
    let space_id = channel.space_id.ok_or(VeilError::InvalidInput("Only space channels have overrides".into()))?;

    if !can_moderate(&db, &space_id, &identity.id, "manage_channels")? {
        return Err(VeilError::PermissionDenied);
    }

    let overrides: Vec<crate::models::channel::PermissionOverride> = input
        .overrides
        .into_iter()
        .map(|item| {
            let mut allow = Permissions::default();
            allow.apply_ids(&item.allow);
            let mut deny = Permissions::default();
            deny.apply_ids(&item.deny);
            crate::models::channel::PermissionOverride {
                target_id: Uuid::parse_str(&item.target_id).unwrap_or_else(|_| Uuid::nil()),
                target_type: if item.target_type == "member" {
                    crate::models::channel::OverrideTarget::Member
                } else {
                    crate::models::channel::OverrideTarget::Role
                },
                allow,
                deny,
            }
        })
        .collect();

    db.update_channel_overrides(&channel_id, &overrides)?;
    let _ = state.app.emit(
        "channels:changed",
        serde_json::json!({ "spaceId": space_id.to_string(), "channelId": channel_id.to_string() }),
    );
    Ok(())
}

#[tauri::command]
pub async fn channels_get_overrides(
    channel_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChannelOverrideItem>, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let channel_id = parse_channel_id(&channel_id)?;
    let db = state.db.read().await;
    let db_key = state.get_db_key().await;
    let channel = db
        .get_channel(&channel_id, db_key.as_ref())?
        .ok_or(VeilError::InvalidInput("Channel not found".into()))?;

    let items = channel
        .permission_overrides
        .into_iter()
        .map(|ov| ChannelOverrideItem {
            target_id: ov.target_id.to_string(),
            target_type: match ov.target_type {
                crate::models::channel::OverrideTarget::Role => "role".to_string(),
                crate::models::channel::OverrideTarget::Member => "member".to_string(),
            },
            allow: ov.allow.enabled_ids(),
            deny: ov.deny.enabled_ids(),
        })
        .collect();
    Ok(items)
}

// ── Invites ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn invites_create(
    input: InviteCreateInput,
    state: State<'_, AppState>,
) -> Result<InviteInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;

    // 8 bytes of entropy → 16 hex chars, uppercase, no ambiguous characters
    let code = hex::encode_upper(crate::crypto::random_bytes(8)?);

    let id = Uuid::new_v4();
    let expires_at = input.expires_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
    let db = state.db.read().await;
    db.insert_invite(&id, &code, &space_id, &identity.id, input.max_uses, expires_at)?;
    let row = db.get_invite_by_code(&code)?.ok_or(VeilError::InvalidInput("Invite not found".into()))?;
    drop(db);

    // Sync invite to Supabase
    if config::configured("VEILANON_SUPABASE_URL") {
        best_effort_insert(
            &state,
            "invites",
            serde_json::json!({
                "id": id.to_string(),
                "space_id": space_id.to_string(),
                "code": code,
                "creator_id": identity.id.to_string(),
                "max_uses": input.max_uses,
                "expires_at": expires_at.map(|dt| dt.to_rfc3339()),
            }),
        )
        .await;
    }

    Ok(to_invite_info(&row))
}

#[tauri::command]
pub async fn invites_redeem(code: String, state: State<'_, AppState>) -> Result<SpaceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    
    let mut clean = code.trim().trim_start_matches('@');
    if let Some(pos) = clean.find("code=") {
        let after = &clean[pos + 5..];
        let end = after.find('&').unwrap_or(after.len());
        clean = &after[..end];
    }
    if let Some(stripped) = clean.strip_prefix("https://") {
        clean = stripped;
    } else if let Some(stripped) = clean.strip_prefix("http://") {
        clean = stripped;
    } else if let Some(stripped) = clean.strip_prefix("veilanon://") {
        clean = stripped;
    }
    let domains = [
        "veilanon.com/",
        "www.veilanon.com/",
        "veilanon.com.tr/",
        "www.veilanon.com.tr/",
        "veilanon.online/",
        "www.veilanon.online/",
        "veilanon.info/",
        "www.veilanon.info/",
        "localhost/",
        "127.0.0.1/",
    ];
    for d in domains {
        if let Some(stripped) = clean.strip_prefix(d) {
            clean = stripped;
            break;
        }
    }
    let prefixes = ["invite/", "join/", "c/", "server/", "space/"];
    for p in prefixes {
        if let Some(stripped) = clean.strip_prefix(p) {
            clean = stripped;
            break;
        }
    }
    if let Some(pos) = clean.find('?') {
        clean = &clean[..pos];
    }
    if let Some(pos) = clean.find('#') {
        clean = &clean[..pos];
    }
    let clean_code = clean.trim_end_matches('/').trim();

    let mut space_id_opt: Option<Uuid> = None;

    // 1. Önce yerel veritabanını kontrol et
    {
        let db = state.db.read().await;
        if let Ok(Some(inv)) = db.get_invite_by_code(clean_code) {
            space_id_opt = Some(inv.space_id);
        } else if let Ok(Some(sp)) = db.get_space_by_custom_link(clean_code) {
            space_id_opt = Some(sp.id);
        } else if let Ok(sp_uuid) = Uuid::parse_str(clean_code) {
            if let Ok(Some(sp)) = db.get_space(&sp_uuid) {
                space_id_opt = Some(sp.id);
            }
        }
    }

    // 2. Yerelde bulunamadıysa Supabase üzerinden sorgula
    if space_id_opt.is_none() && config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        // A. invites tablosunda ara
        let inv_filter = format!("code=eq.{}&select=space_id", clean_code.to_uppercase());
        if let Ok(rows) = network.api.select::<serde_json::Value>("invites", &inv_filter, None, Some(1)).await {
            if let Some(row) = rows.first() {
                if let Some(sid_str) = row.get("space_id").and_then(|v| v.as_str()) {
                    if let Ok(sid) = Uuid::parse_str(sid_str) {
                        space_id_opt = Some(sid);
                    }
                }
            }
        }

        // B. spaces custom_link veya id ara
        if space_id_opt.is_none() {
            let sp_filter = if let Ok(uuid) = Uuid::parse_str(clean_code) {
                format!("id=eq.{}&select=id", uuid)
            } else {
                format!("custom_link=eq.{}&select=id", clean_code.to_lowercase())
            };
            if let Ok(rows) = network.api.select::<serde_json::Value>("spaces", &sp_filter, None, Some(1)).await {
                if let Some(row) = rows.first() {
                    if let Some(sid_str) = row.get("id").and_then(|v| v.as_str()) {
                        if let Ok(sid) = Uuid::parse_str(sid_str) {
                            space_id_opt = Some(sid);
                        }
                    }
                }
            }
        }
    }

    let space_id = space_id_opt.ok_or_else(|| VeilError::InvalidInput("Geçersiz veya süresi dolmuş davet kodu".into()))?;

    // 3. Uzak sunucudan topluluk ve kanal bilgilerini indir
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let sp_filter = format!("id=eq.{}&select=id,name,icon_hash,owner_id,custom_link,banner_hash,description", space_id);
        if let Ok(rows) = network.api.select::<serde_json::Value>("spaces", &sp_filter, None, Some(1)).await {
            if let Some(s) = rows.first() {
                let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("Topluluk");
                let icon = s.get("icon_hash").and_then(|v| v.as_str());
                let owner_str = s.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
                let owner_id = Uuid::parse_str(owner_str).unwrap_or(identity.id);
                let db = state.db.read().await;
                let _ = db.insert_space(&space_id, name, icon, &owner_id);
                let _ = db.set_space_owner(&space_id, &identity.id, owner_id == identity.id);
                if let Some(link) = s.get("custom_link").and_then(|v| v.as_str()) {
                    let _ = db.set_custom_link(&space_id, link);
                }
                if let Some(banner) = s.get("banner_hash").and_then(|v| v.as_str()) {
                    let _ = db.update_space(&space_id, None, None, Some(Some(banner)), None);
                }
                if let Some(desc) = s.get("description").and_then(|v| v.as_str()) {
                    let _ = db.update_space(&space_id, None, None, None, Some(Some(desc)));
                }
            }
        }

        let ch_filter = format!("space_id=eq.{}&select=id,name,channel_type,position,is_nsfw,is_e2ee", space_id);
        if let Ok(ch_rows) = network.api.select::<serde_json::Value>("channels", &ch_filter, Some("position.asc"), Some(100)).await {
            let db = state.db.read().await;
            let db_key = state.get_db_key().await;
            for ch in ch_rows {
                if let (Some(cid_str), Some(cname), Some(ctype_str)) = (
                    ch.get("id").and_then(|v| v.as_str()),
                    ch.get("name").and_then(|v| v.as_str()),
                    ch.get("channel_type").and_then(|v| v.as_str()),
                ) {
                    if let Ok(cid) = Uuid::parse_str(cid_str) {
                        let pos = ch.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let is_nsfw = ch.get("is_nsfw").and_then(|v| v.as_bool()).unwrap_or(false);
                        let is_e2ee = ch.get("is_e2ee").and_then(|v| v.as_bool()).unwrap_or(false);
                        let ctype = match ctype_str {
                            "voice" => ChannelType::Voice,
                            "announcement" => ChannelType::Announcement,
                            "forum" => ChannelType::Forum,
                            _ => ChannelType::Text,
                        };
                        let channel = crate::models::channel::Channel {
                            id: cid,
                            space_id: Some(space_id),
                            name: cname.to_string(),
                            channel_type: ctype,
                            position: pos,
                            topic: None,
                            is_nsfw,
                            is_e2ee,
                            slow_mode_seconds: 0,
                            permission_overrides: Vec::new(),
                            created_at: chrono::Utc::now(),
                            last_message_id: None,
                            unread_count: 0,
                            mentioned: false,
                        };
                        let _ = db.upsert_channel(&channel, db_key.as_ref());
                    }
                }
            }
        }
    }

    // 4. Yerel üyeliği ekle
    {
        let db = state.db.read().await;
        if db.is_banned(&space_id, &identity.id)? {
            return Err(VeilError::InvalidInput("Bu topluluktan yasaklandınız".into()));
        }
        db.add_space_member(&space_id, &identity.id)?;
    }

    // 5. Supabase membership kaydını ekle
    if config::configured("VEILANON_SUPABASE_URL") {
        best_effort_insert(
            &state,
            "memberships",
            serde_json::json!({
                "user_id": identity.id.to_string(),
                "space_id": space_id.to_string(),
            }),
        )
        .await;
    }

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    let _ = state.app.emit("spaces:changed", ());
    info!("Joined space {}", space_id);
    Ok(to_space_info(&row))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCustomLinkInput {
    pub space_id: String,
    pub link: String,
}

/// Topluluğa özel, bir kez alınabilen kısa bağlantı (sahip). `veilanon.com/join/<link>`
/// ve `veilanon://join/<link>` biçimleriyle davet yerine kullanılabilir.
/// Bağlantı zaten alınmışsa değiştirilemez; boşaltılamaz — bu bilinçli bir
/// tasarım kararıdır (bağlantıyı paylaşanların erişiminin sürpriz biçimde
/// kesilmemesi için).
#[tauri::command]
pub async fn spaces_set_custom_link(
    input: SetCustomLinkInput,
    state: State<'_, AppState>,
) -> Result<SpaceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;

    let link = input.link.trim().to_lowercase();
    if link.is_empty() || link.len() < 2 || link.len() > 32 {
        return Err(VeilError::InvalidInput(
            "Bağlantı 2-32 karakter olmalı (küçük harf, rakam, tire)".into(),
        ));
    }
    if !link
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(VeilError::InvalidInput(
            "Bağlantı yalnızca küçük harf, rakam ve tire içerebilir".into(),
        ));
    }

    let db = state.db.read().await;
    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if row.owner_id != identity.id {
        return Err(VeilError::PermissionDenied);
    }

    // Aynı link başka bir toplulukta kullanılmış olmasın (local unique + Supabase unique).
    if let Some(other) = db.get_space_by_custom_link(&link)? {
        if other.id != space_id {
            return Err(VeilError::InvalidInput("Bu bağlantı zaten alınmış".into()));
        }
    }

    if !db.set_custom_link(&space_id, &link)? {
        return Err(VeilError::InvalidInput(
            "Bu topluluğun özel bağlantısı zaten var ve bir kez alınabilir".into(),
        ));
    }

    // Kontrol düzlemine yansıt (best-effort; unique index orada da korur).
    best_effort_update(
        &state,
        "spaces",
        &serde_json::json!({ "custom_link": link }),
        &format!("id=eq.{}", space_id),
    )
    .await;

    let row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    Ok(to_space_info(&row))
}

// ── Members ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn members_list(space_id: String, state: State<'_, AppState>) -> Result<Vec<MemberInfo>, VeilError> {
    state.get_or_restore_identity().await.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = format!("space_id=eq.{}&select=user_id", space_id);
        if let Ok(memberships) = network.api.select::<serde_json::Value>("memberships", &filter, None, Some(200)).await {
            let mut user_ids: Vec<String> = memberships
                .into_iter()
                .filter_map(|m| m.get("user_id").and_then(|v| v.as_str()).map(str::to_string))
                .collect();
            user_ids.sort();
            user_ids.dedup();

            if !user_ids.is_empty() {
                let users_filter = format!("id=in.({})&select=id,username,display_name,avatar_hash,banner_hash,bio,custom_status", user_ids.join(","));
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
                                let banner = u.get("banner_hash").and_then(|v| v.as_str());
                                let bio = u.get("bio").and_then(|v| v.as_str());
                                let status = u.get("custom_status").and_then(|v| v.as_str());
                                let _ = db.upsert_profile(&uid, uname, disp, av, None, None, banner, bio, status);
                                let _ = db.add_space_member(&space_id, &uid);
                            }
                        }
                    }
                }

                // 1. First sync roles for this space so space_role_ids is populated
                let roles_filter = format!("space_id=eq.{}&select=id,space_id,name,color,permissions,position", space_id);
                if let Ok(remote_roles) = network.api.select::<serde_json::Value>("roles", &roles_filter, Some("position.asc"), Some(100)).await {
                    let db = state.db.read().await;
                    for r in remote_roles {
                        if let (Some(id_str), Some(name)) = (r.get("id").and_then(|v| v.as_str()), r.get("name").and_then(|v| v.as_str())) {
                            if let Ok(id) = Uuid::parse_str(id_str) {
                                let color = r.get("color").and_then(|v| v.as_str());
                                let position = r.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                let perms: Permissions = r.get("permissions")
                                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                                    .unwrap_or_default();
                                let _ = db.upsert_role(&id, &space_id, name, color, &perms, position);
                            }
                        }
                    }
                }

                let space_roles = {
                    let db = state.db.read().await;
                    db.list_roles(&space_id).unwrap_or_default()
                };
                let space_role_ids: Vec<String> = space_roles.iter().map(|r| r.id.to_string()).collect();
                if !space_role_ids.is_empty() {
                    let role_members_filter = format!("role_id=in.({})", space_role_ids.join(","));
                    if let Ok(rm_rows) = network.api.select::<serde_json::Value>("role_members", &role_members_filter, None, Some(500)).await {
                        let mut user_roles_map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
                        for uid_str in &user_ids {
                            if let Ok(uid) = Uuid::parse_str(uid_str) {
                                user_roles_map.insert(uid, Vec::new());
                            }
                        }
                        for rm in rm_rows {
                            if let (Some(rid_str), Some(uid_str)) = (
                                rm.get("role_id").and_then(|v| v.as_str()),
                                rm.get("user_id").and_then(|v| v.as_str()),
                            ) {
                                if let (Ok(rid), Ok(uid)) = (Uuid::parse_str(rid_str), Uuid::parse_str(uid_str)) {
                                    user_roles_map.entry(uid).or_default().push(rid);
                                }
                            }
                        }
                        let db = state.db.read().await;
                        let modified = state.role_members_modified.read().await;
                        for (uid, rids) in user_roles_map {
                            let key = format!("{}:{}", space_id, uid);
                            if let Some(last_mod) = modified.get(&key) {
                                if last_mod.elapsed().as_secs() < 5 {
                                    continue;
                                }
                            }
                            let _ = db.update_space_member_roles(&space_id, &uid, &rids);
                        }
                    }
                }

                let presence_filter = format!("user_id=in.({})&select=user_id,status,heartbeat_at,last_seen", user_ids.join(","));
                if let Ok(presences) = network.api.select::<serde_json::Value>("presence", &presence_filter, None, Some(200)).await {
                    let db = state.db.read().await;
                    let now = chrono::Utc::now();
                    for p in presences {
                        if let Some(uid_str) = p.get("user_id").and_then(|v| v.as_str()) {
                            if let Ok(uid) = Uuid::parse_str(uid_str) {
                                let mut status_str = p.get("status").and_then(|v| v.as_str()).unwrap_or("offline");
                                if status_str != "offline" && status_str != "invisible" {
                                    let hb_str = p.get("heartbeat_at").or_else(|| p.get("last_seen")).and_then(|v| v.as_str()).unwrap_or("");
                                    if !hb_str.is_empty() {
                                        if let Ok(hb_time) = chrono::DateTime::parse_from_rfc3339(hb_str) {
                                            if (now - hb_time.with_timezone(&chrono::Utc)).num_seconds() > 90 {
                                                status_str = "offline";
                                            }
                                        } else {
                                            status_str = "offline";
                                        }
                                    } else {
                                        status_str = "offline";
                                    }
                                }
                                if status_str == "invisible" {
                                    status_str = "offline";
                                }
                                let _ = db.update_presence(&uid, status_str);
                            }
                        }
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    let rows = db.list_space_members(&space_id)?;
    Ok(rows.iter().map(to_member_info).collect())
}

#[tauri::command]
pub async fn members_update(
    input: MemberUpdateInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let user_id = Uuid::parse_str(&input.user_id).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))?;

    let role_ids: Vec<Uuid> = input
        .role_ids
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    
    // Moderasyon ve hiyerarşi kontrolü
    let is_owner = space.owner_id == identity.id || space.is_owner;
    if !is_owner {
        if !can_moderate_target(&db, &space_id, &identity.id, &user_id, "manage_roles")? {
            return Err(VeilError::PermissionDenied);
        }

        // Çağıran kişi kendi en yüksek rolünden eşit veya daha yüksek bir rolü veremez/alamaz (sahip hariç)
        let caller_rank = get_user_highest_role_position(&db, &space_id, &identity.id)?;
        let all_roles = db.list_roles(&space_id)?;
        for role in &all_roles {
            if role_ids.contains(&role.id) && role.position >= caller_rank {
                return Err(VeilError::PermissionDenied);
            }
        }
    }

    db.update_space_member_roles(&space_id, &user_id, &role_ids)?;

    // Record timestamp so members_list skips Supabase overwrite for 5s
    {
        let mut modified = state.role_members_modified.write().await;
        modified.insert(format!("{}:{}", space_id, user_id), Instant::now());
    }

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let space_roles = {
            let db = state.db.read().await;
            db.list_roles(&space_id).unwrap_or_default()
        };
        let space_role_ids: Vec<String> = space_roles.iter().map(|r| r.id.to_string()).collect();
        if !space_role_ids.is_empty() {
            if let Err(e) = network
                .api
                .delete("role_members", &format!("user_id=eq.{}&role_id=in.({})", user_id, space_role_ids.join(",")))
                .await
            {
                warn!("Supabase role_members delete failed for user {}: {}", user_id, e);
            }
        }
        for r_id in &role_ids {
            if let Err(e) = network
                .api
                .upsert(
                    "role_members",
                    &serde_json::json!({
                        "role_id": r_id.to_string(),
                        "user_id": user_id.to_string(),
                    }),
                    "role_id,user_id",
                )
                .await
            {
                warn!("Supabase role_members upsert failed for role {}: {}", r_id, e);
            }
        }
    }

    let _ = state.app.emit("roles:changed", serde_json::json!({ "spaceId": space_id.to_string() }));
    let _ = state.app.emit("members:changed", serde_json::json!({ "spaceId": space_id.to_string(), "userId": user_id.to_string() }));
    Ok(())
}

// ── Moderation (kick / ban / timeout) ────────────────────────────────────────

/// Üyeyi topluluktan çıkarır (sahip veya kick_members izni + hiyerarşi üstünlüğü).
#[tauri::command]
pub async fn spaces_kick_member(
    input: BanInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let user_id = Uuid::parse_str(&input.user_id).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))?;
    if user_id == identity.id {
        return Err(VeilError::InvalidInput("Kendi kendini atamazsın".into()));
    }

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id == user_id {
        return Err(VeilError::InvalidInput("Sahip atılamaz".into()));
    }
    if !can_moderate_target(&db, &space_id, &identity.id, &user_id, "kick_members")? {
        return Err(VeilError::PermissionDenied);
    }
    if !db.is_space_member(&space_id, &user_id)? {
        return Err(VeilError::InvalidInput("Bu kullanıcı üye değil".into()));
    }
    db.remove_space_member(&space_id, &user_id)?;
    drop(db);

    best_effort_delete(&state, "memberships", &format!("user_id=eq.{}&space_id=eq.{}", user_id, space_id)).await;
    info!("Member kicked");
    Ok(())
}

/// Üyeyi yasaklar: üyelik silinir + ban listesine eklenir (davetle bile dönemez).
#[tauri::command]
pub async fn spaces_ban_member(
    input: BanInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let user_id = Uuid::parse_str(&input.user_id).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))?;
    if user_id == identity.id {
        return Err(VeilError::InvalidInput("Kendi kendini yasaklayamazsın".into()));
    }

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id == user_id {
        return Err(VeilError::InvalidInput("Sahip yasaklanamaz".into()));
    }
    if !can_moderate_target(&db, &space_id, &identity.id, &user_id, "ban_members")? {
        return Err(VeilError::PermissionDenied);
    }
    db.ban_member(&space_id, &user_id, &identity.id, input.reason.as_deref())?;
    if db.is_space_member(&space_id, &user_id)? {
        db.remove_space_member(&space_id, &user_id)?;
    }
    drop(db);

    best_effort_insert(
        &state,
        "bans",
        serde_json::json!({
            "space_id": space_id.to_string(),
            "user_id": user_id.to_string(),
            "banned_by": identity.id.to_string(),
            "reason": input.reason,
        }),
    )
    .await;
    best_effort_delete(&state, "memberships", &format!("user_id=eq.{}&space_id=eq.{}", user_id, space_id)).await;
    info!("Member banned");
    Ok(())
}

/// Banı kaldırır (sahip veya ban_members izni).
#[tauri::command]
pub async fn spaces_unban_member(
    input: BanInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let user_id = Uuid::parse_str(&input.user_id).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))?;

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id != identity.id && !space.is_owner && !can_moderate(&db, &space_id, &identity.id, "ban_members")? {
        return Err(VeilError::PermissionDenied);
    }
    db.unban_member(&space_id, &user_id)?;
    drop(db);

    best_effort_delete(&state, "bans", &format!("user_id=eq.{}&space_id=eq.{}", user_id, space_id)).await;
    info!("Member unbanned");
    Ok(())
}

/// Geçici susturma: belirtilen süreye kadar mesaj gönderimi engellenir.
/// `until` = 0 veya geçmiş bir değer susturmayı kaldırır.
#[tauri::command]
pub async fn spaces_timeout_member(
    input: TimeoutInput,
    state: State<'_, AppState>,
) -> Result<(), VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&input.space_id)?;
    let user_id = Uuid::parse_str(&input.user_id).map_err(|_| VeilError::InvalidInput("Invalid user ID".into()))?;
    if user_id == identity.id {
        return Err(VeilError::InvalidInput("Kendi kendine süre veremezsin".into()));
    }

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id == user_id {
        return Err(VeilError::InvalidInput("Sahibe süre verilemez".into()));
    }
    if !can_moderate_target(&db, &space_id, &identity.id, &user_id, "timeout_members")? {
        return Err(VeilError::PermissionDenied);
    }
    let now = chrono::Utc::now().timestamp();
    let until = input.until.filter(|u| *u > now);
    db.set_member_timeout(&space_id, &user_id, until)?;
    drop(db);

    if let Some(until) = until {
        best_effort_update(
            &state,
            "memberships",
            &serde_json::json!({ "timeout_until": until }),
            &format!("user_id=eq.{}&space_id=eq.{}", user_id, space_id),
        )
        .await;
    }
    info!("Member timed out");
    Ok(())
}

/// Yasaklı üyelerin listesi (sahip veya ban_members izni).
#[tauri::command]
pub async fn spaces_bans_list(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<BanInfo>, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let space_id = parse_space_id(&space_id)?;

    let db = state.db.read().await;
    let space = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Space not found".into()))?;
    if space.owner_id != identity.id && !space.is_owner && !can_moderate(&db, &space_id, &identity.id, "ban_members")? {
        return Ok(Vec::new());
    }
    let rows = db.list_bans(&space_id)?;
    Ok(rows.iter().map(to_ban_info).collect())
}

/// Keşfet & Arama: Toplulukları arama ve listeleme (Yerel + Supabase)
#[tauri::command]
pub async fn spaces_search_public(
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SpaceInfo>, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let my_user_id = identity.as_ref().map(|id| id.id.to_string()).unwrap_or_default();

    let q = query.unwrap_or_default().trim().to_lowercase();
    let mut discovered: std::collections::HashMap<Uuid, SpaceInfo> = std::collections::HashMap::new();

    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = if q.is_empty() {
            "select=id,name,icon_hash,owner_id,custom_link,banner_hash,description".to_string()
        } else {
            format!("or=(name.ilike.%25{}%25,custom_link.ilike.%25{}%25)&select=id,name,icon_hash,owner_id,custom_link,banner_hash,description", q, q)
        };
        if let Ok(remote_spaces) = network.api.select::<serde_json::Value>("spaces", &filter, None, Some(50)).await {
            let space_ids: Vec<String> = remote_spaces.iter().filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(str::to_string)).collect();
            let mut member_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            if !space_ids.is_empty() {
                let m_filter = format!("space_id=in.({})&select=space_id", space_ids.join(","));
                if let Ok(memberships) = network.api.select::<serde_json::Value>("memberships", &m_filter, None, Some(500)).await {
                    for m in &memberships {
                        if let Some(sid) = m.get("space_id").and_then(|v| v.as_str()) {
                            *member_counts.entry(sid.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }

            for s in remote_spaces {
                if let (Some(id_str), Some(name)) = (s.get("id").and_then(|v| v.as_str()), s.get("name").and_then(|v| v.as_str())) {
                    if let Ok(id) = Uuid::parse_str(id_str) {
                        let icon = s.get("icon_hash").and_then(|v| v.as_str()).map(str::to_string);
                        let owner_str = s.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
                        let is_owner = !my_user_id.is_empty() && owner_str == my_user_id;
                        let link = s.get("custom_link").and_then(|v| v.as_str()).map(str::to_string);
                        let banner = s.get("banner_hash").and_then(|v| v.as_str()).map(str::to_string);
                        let desc = s.get("description").and_then(|v| v.as_str()).map(str::to_string);
                        let count = member_counts.get(id_str).copied().unwrap_or(1);

                        discovered.insert(id, SpaceInfo {
                            id: id.to_string(),
                            name: name.to_string(),
                            icon_hash: icon,
                            owner_id: owner_str.to_string(),
                            member_count: count,
                            is_owner,
                            my_roles: if is_owner { vec!["Owner".into()] } else { Vec::new() },
                            banner_hash: banner,
                            description: desc,
                            custom_link: link,
                        });
                    }
                }
            }
        }
    }

    let db = state.db.read().await;
    if let Ok(spaces) = db.list_spaces() {
        for s in spaces {
            if s.id == Uuid::nil() { continue; }
            if q.is_empty()
                || s.name.to_lowercase().contains(&q)
                || s.description.as_ref().map(|d| d.to_lowercase().contains(&q)).unwrap_or(false)
                || s.custom_link.as_ref().map(|l| l.to_lowercase().contains(&q)).unwrap_or(false)
            {
                discovered.insert(s.id, to_space_info(&s));
            }
        }
    }

    let mut result: Vec<SpaceInfo> = discovered.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

/// Açık / Özel bağlantı ile topluluğa katıl
#[tauri::command]
pub async fn spaces_join_public(
    space_id_or_link: String,
    state: State<'_, AppState>,
) -> Result<SpaceInfo, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    let trimmed = space_id_or_link.trim().trim_start_matches('@');
    let trimmed = if let Some(stripped) = trimmed.strip_prefix("https://veilanon.com/join/") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("veilanon://join/") {
        stripped
    } else {
        trimmed
    };

    let mut space_id_opt: Option<Uuid> = None;

    // 1. Önce yerel veritabanında ara
    {
        let db = state.db.read().await;
        if let Ok(uuid) = Uuid::parse_str(trimmed) {
            if let Ok(Some(sp)) = db.get_space(&uuid) {
                space_id_opt = Some(sp.id);
            }
        } else if let Ok(Some(sp)) = db.get_space_by_custom_link(trimmed) {
            space_id_opt = Some(sp.id);
        }
    }

    // 2. Yerelde bulunamazsa Supabase üzerinden sorgula
    if space_id_opt.is_none() && config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let filter = if let Ok(uuid) = Uuid::parse_str(trimmed) {
            format!("id=eq.{}&select=id,name,icon_hash,owner_id,custom_link,banner_hash,description", uuid)
        } else {
            format!("custom_link=eq.{}&select=id,name,icon_hash,owner_id,custom_link,banner_hash,description", trimmed.to_lowercase())
        };
        if let Ok(rows) = network.api.select::<serde_json::Value>("spaces", &filter, None, Some(1)).await {
            if let Some(s) = rows.first() {
                if let Some(sid_str) = s.get("id").and_then(|v| v.as_str()) {
                    if let Ok(sid) = Uuid::parse_str(sid_str) {
                        space_id_opt = Some(sid);
                        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("Topluluk");
                        let icon = s.get("icon_hash").and_then(|v| v.as_str());
                        let owner_str = s.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
                        let owner_id = Uuid::parse_str(owner_str).unwrap_or(identity.id);
                        let db = state.db.read().await;
                        let _ = db.insert_space(&sid, name, icon, &owner_id);
                        let _ = db.set_space_owner(&sid, &identity.id, owner_id == identity.id);
                        if let Some(link) = s.get("custom_link").and_then(|v| v.as_str()) {
                            let _ = db.set_custom_link(&sid, link);
                        }
                        if let Some(banner) = s.get("banner_hash").and_then(|v| v.as_str()) {
                            let _ = db.update_space(&sid, None, None, Some(Some(banner)), None);
                        }
                        if let Some(desc) = s.get("description").and_then(|v| v.as_str()) {
                            let _ = db.update_space(&sid, None, None, None, Some(Some(desc)));
                        }
                    }
                }
            }
        }
    }

    let space_id = space_id_opt.ok_or_else(|| VeilError::InvalidInput("Topluluk bulunamadı veya bağlantı geçersiz".into()))?;

    // 3. Kanalları Supabase'den indir
    if config::configured("VEILANON_SUPABASE_URL") {
        let network = state.network.read().await;
        let ch_filter = format!("space_id=eq.{}&select=id,name,channel_type,position,is_nsfw,is_e2ee", space_id);
        if let Ok(ch_rows) = network.api.select::<serde_json::Value>("channels", &ch_filter, Some("position.asc"), Some(100)).await {
            let db = state.db.read().await;
            let db_key = state.get_db_key().await;
            for ch in ch_rows {
                if let (Some(cid_str), Some(cname), Some(ctype_str)) = (
                    ch.get("id").and_then(|v| v.as_str()),
                    ch.get("name").and_then(|v| v.as_str()),
                    ch.get("channel_type").and_then(|v| v.as_str()),
                ) {
                    if let Ok(cid) = Uuid::parse_str(cid_str) {
                        let pos = ch.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                        let is_nsfw = ch.get("is_nsfw").and_then(|v| v.as_bool()).unwrap_or(false);
                        let is_e2ee = ch.get("is_e2ee").and_then(|v| v.as_bool()).unwrap_or(false);
                        let ctype = match ctype_str {
                            "voice" => ChannelType::Voice,
                            "announcement" => ChannelType::Announcement,
                            "forum" => ChannelType::Forum,
                            _ => ChannelType::Text,
                        };
                        let channel = crate::models::channel::Channel {
                            id: cid,
                            space_id: Some(space_id),
                            name: cname.to_string(),
                            channel_type: ctype,
                            position: pos,
                            topic: None,
                            is_nsfw,
                            is_e2ee,
                            slow_mode_seconds: 0,
                            permission_overrides: Vec::new(),
                            created_at: chrono::Utc::now(),
                            last_message_id: None,
                            unread_count: 0,
                            mentioned: false,
                        };
                        let _ = db.upsert_channel(&channel, db_key.as_ref());
                    }
                }
            }
        }

            // 3b. Rolleri Supabase'den indir
            let r_filter = format!("space_id=eq.{}&select=id,name,color,permissions,position", space_id);
            if let Ok(r_rows) = network.api.select::<serde_json::Value>("roles", &r_filter, Some("position.asc"), Some(100)).await {
                let db = state.db.read().await;
                for r in r_rows {
                    if let (Some(rid_str), Some(rname)) = (
                        r.get("id").and_then(|v| v.as_str()),
                        r.get("name").and_then(|v| v.as_str()),
                    ) {
                        if let Ok(rid) = Uuid::parse_str(rid_str) {
                            let color = r.get("color").and_then(|v| v.as_str());
                            let pos = r.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let perms: Permissions = r.get("permissions")
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .unwrap_or_default();
                            let _ = db.upsert_role(&rid, &space_id, rname, color, &perms, pos);
                        }
                    }
                }
            }
        }

    // 4. Yerel üyelik ve yasak kontrolü
    {
        let db = state.db.read().await;
        if db.is_banned(&space_id, &identity.id)? {
            return Err(VeilError::PermissionDenied);
        }
        db.add_space_member(&space_id, &identity.id)?;
    }

    // 5. Supabase membership kaydı
    if config::configured("VEILANON_SUPABASE_URL") {
        best_effort_insert(
            &state,
            "memberships",
            serde_json::json!({
                "user_id": identity.id.to_string(),
                "space_id": space_id.to_string(),
            }),
        )
        .await;
    }

    let db = state.db.read().await;
    let updated_row = db.get_space(&space_id)?.ok_or(VeilError::InvalidInput("Topluluk bulunamadı".into()))?;
    let _ = state.app.emit("spaces:changed", ());
    info!("Joined space {}", space_id);
    Ok(to_space_info(&updated_row))
}


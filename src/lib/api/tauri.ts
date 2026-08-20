/**
 * veilanon — typed Tauri IPC wrappers
 * Single source of truth for all `invoke` calls.
 * Tauri 2 maps camelCase JS keys to snake_case Rust params automatically.
 */
import { invoke } from '@tauri-apps/api/core';

// ── Domain types (mirrors Rust structs) ─────────────────────────

export type ChannelType = 'text' | 'voice' | 'category' | 'announcement' | 'forum' | 'dm' | 'group_dm';
export type PresenceStatus = 'online' | 'away' | 'dnd' | 'offline' | 'invisible';
export type FriendStatus = 'friends' | 'pending_incoming' | 'pending_outgoing' | 'blocked' | 'none';
/** Mirrors Rust `NotificationPreview` (snake_case serde). */
export type NotificationPreview = 'full' | 'sender' | 'none';
/** Mirrors Rust `PresenceVisibility` (snake_case serde). */
export type PresenceVisibility = 'everyone' | 'contacts_only' | 'nobody';

export interface SpaceInfo {
  id: string;
  name: string;
  iconHash: string | null;
  ownerId: string;
  memberCount: number;
  isOwner: boolean;
  myRoles: string[];
  bannerHash?: string | null;
  description?: string | null;
  /** Bir kez alınabilen özel kısa bağlantı (sahip belirler). */
  customLink?: string | null;
}

export interface ChannelInfo {
  id: string;
  spaceId: string | null;
  name: string;
  channelType: ChannelType;
  position: number;
  isNsfw: boolean;
  isE2ee: boolean;
  unreadCount: number;
  mentioned: boolean;
  lastMessageId: string | null;
  topic?: string | null;
  slowModeSeconds?: number;
  avatarHash?: string | null;
  peerId?: string | null;
  onlineStatus?: string | null;
}

export interface RoleInfo {
  id: string;
  spaceId: string;
  name: string;
  color: string | null;
  permissions: string[];
  position: number;
  isDefault: boolean;
}

export interface ChannelOverrideItem {
  targetId: string;
  targetType: 'role' | 'member';
  allow: string[];
  deny: string[];
}

export interface InviteInfo {
  id: string;
  code: string;
  spaceId: string;
  maxUses: number | null;
  usedCount: number;
  expiresAt: number | null;
}

export interface MemberInfo {
  userId: string;
  username: string;
  displayName: string;
  avatarHash: string | null;
  roleIds: string[];
  onlineStatus: PresenceStatus;
}

export interface BanInfo {
  userId: string;
  username: string;
  displayName: string;
  bannedBy: string;
  reason: string | null;
  createdAt: number;
}

export interface FriendInfo {
  userId: string;
  username: string;
  displayName: string;
  avatarHash?: string | null;
  status: FriendStatus;
  onlineStatus: PresenceStatus;
}

export interface IdentityInfo {
  id: string;
  username: string;
  displayName: string;
  avatarHash?: string | null;
  bannerHash?: string | null;
  bio?: string | null;
  customStatus?: string | null;
  deviceId: string;
  /** DevicePublicIdentity — Rust serializes with snake_case fields (no serde rename). */
  publicKey: {
    dh_public_key: string;
    signing_public_key: string;
    fingerprint: string;
  };
  /** Only returned once at identity creation. */
  recoveryCode?: string | null;
}

export interface IdentityHint {
  hasIdentity: boolean;
  username: string | null;
  displayName: string | null;
  avatarHash: string | null;
  bannerHash?: string | null;
}

export interface MessageInfo {
  id: string;
  channelId: string;
  senderId: string;
  senderName: string | null;
  senderAvatarHash: string | null;
  senderRoleColor: string | null;
  content: string | null;
  messageType: string;
  status: string;
  replyToId: string | null;
  pinned: boolean;
  reactions: Array<{ emoji: string; userIds: string[]; count: number }>;
  attachments: unknown[];
  editedAt: number | null;
  createdAt: number;
  disappearsAt: number | null;
}

export interface VoiceJoinResponse {
  token: string;
  url: string;
  roomName: string;
  isE2ee: boolean;
  e2eeScope: string;
  e2eeKey?: string | null;
}

export interface DeviceInfo {
  id: string;
  name: string;
  os: string;
}

export interface SessionInfo {
  deviceId: string;
  name: string;
  platform?: string;
  lastActiveAt: number;
  isCurrent: boolean;
}

export interface AboutInfo {
  appName: string;
  version: string;
  description: string;
  developer: string;
  developerUrl: string;
  developerGithub: string;
  projectGithub: string;
  supportUrl: string;
  license: string;
  buildDate: string;
  rustVersion: string;
  platform: string;
}

export interface FileInfo {
  fileId: string;
  sizeBytes: number;
  uploadUrl: string | null;
  isEncrypted: boolean;
  r2Key: string | null;
  contentKeyCiphertext?: string | null;
}

export type NetworkProxyMode =
  | 'direct'
  | 'tor'
  | 'custom_socks'
  | 'custom_http'
  | 'wireguard'
  | 'cloudflare_warp';

export interface NetworkPrivacySettings {
  mode: NetworkProxyMode;
  proxyHost: string;
  proxyPort: number;
  strictMode: boolean;
  routeAppOnly: boolean;
  customProxyUrl?: string | null;
  wireguardProfile?: string | null;
  autoStartTor?: boolean;
  verifyExitNode?: boolean;
  torBridgeType?: string | null;
  activePreset?: string | null;
  wireguardEndpoint?: string | null;
  wireguardPublicKey?: string | null;
  wireguardAllowedIps?: string | null;
}

/**
 * App settings — mirrors `AppSettings` in Rust (`src-tauri/src/state.rs`).
 * Rust uses `#[serde(rename_all = "camelCase", default)]`, so these keys match
 * the wire format exactly and missing fields are defaulted server-side.
 */
export interface AppSettings {
  // Privacy
  presenceVisibility: PresenceVisibility;
  showReadReceipts: boolean;
  showTypingIndicator: boolean;
  autoDownloadMedia: boolean;
  linkPreviews: boolean;
  notificationPreview: NotificationPreview;
  telemetryEnabled: boolean;
  localAiEnabled: boolean;
  discordBridgeEnabled: boolean;
  /** Kayıt tarihi profilde görünsün mü? Açıkken herkes görür; kapalıyken yalnızca kendin. */
  showJoinDate: boolean;
  /** Ağ & Bağlantı Gizliliği (Tor / SOCKS5 / WireGuard) */
  networkPrivacy?: NetworkPrivacySettings;
  // Appearance
  theme?: string;
  fontSize?: number;
  reduceMotion?: boolean;
  compactMode?: boolean;
  accentColor?: string | null;
  amoledMode?: boolean;
  presetThemeId?: string;
  customThemeName?: string;
  customCss?: string;
  customCssEnabled?: boolean;
  customBgImage?: string;
  customBgVideo?: string;
  customBgOpacity?: number;
  savedThemes?: string;
  // Notifications
  desktopNotifications?: boolean;
  notificationSound?: boolean;
  mentionOnly?: boolean;
  notificationVolume?: number;
  soundMessages?: boolean;
  soundMentions?: boolean;
  soundFriends?: boolean;
  soundCalls?: boolean;
  dndSuppressNotifications?: boolean;
  // Audio/Video
  inputDeviceId?: string | null;
  outputDeviceId?: string | null;
  videoDeviceId?: string | null;
  inputVolume?: number;
  outputVolume?: number;
  noiseSuppression?: boolean;
  echoCancellation?: boolean;
  pushToTalk?: boolean;
  pushToTalkKey?: string | null;
  mirrorCamera?: boolean;
  // System
  startOnLogin?: boolean;
  minimizeToTray?: boolean;
  closeToTray?: boolean;
  hardwareAcceleration?: boolean;
  language?: string;
  /** Açılışta GitHub'dan sürüm kontrolü. */
  autoUpdateCheck?: boolean;
  /** Bu cihazda parola hatırlanıp açılışta sorulmasın. */
  autoUnlock?: boolean;
  /** DM gizlilik ayarı: herkes, arkadaşlar, aynı sunucu, hiçbiri */
  dmPrivacy?: 'everyone' | 'friends' | 'same_server' | 'nobody';
}

export interface KeyPairResponse {
  publicKey: string;
  keyType: string;
}

// ── Identity & device ────────────────────────────────────────────

const avatarCache = new Map<string, string>();
const avatarInFlight = new Map<string, Promise<string>>();

export const identityApi = {
  create(input: { username: string; displayName: string; passphrase: string }) {
    return invoke<IdentityInfo>('create_identity', { input });
  },
  load(passphrase: string) {
    return invoke<IdentityInfo>('load_identity', { passphrase });
  },
  tryAutoUnlock() {
    return invoke<IdentityInfo | null>('try_auto_unlock');
  },
  setAutoUnlock(enabled: boolean, passphrase?: string) {
    return invoke<void>('set_auto_unlock', { enabled, passphrase });
  },
  hasAutoUnlock() {
    return invoke<boolean>('has_auto_unlock');
  },
  recover(recoveryCode: string, newPassphrase: string) {
    return invoke<IdentityInfo>('recover_identity', { recoveryCode, newPassphrase });
  },
  getIdentityHint() {
    return invoke<IdentityHint>('get_identity_hint');
  },
  updateProfile(input: {
    displayName: string;
    username?: string;
    avatarHash?: string | null;
    bio?: string | null;
    bannerHash?: string | null;
    customStatus?: string | null;
    clearAvatar?: boolean;
    clearBanner?: boolean;
  }) {
    const payload = {
      ...input,
      clearAvatar: input.clearAvatar ?? (input.avatarHash === null ? true : undefined),
      clearBanner: input.clearBanner ?? (input.bannerHash === null ? true : undefined),
    };
    return invoke<void>('update_profile', { input: payload });
  },
  checkUsernameAvailable(username: string) {
    return invoke<boolean>('check_username_available', { username });
  },
  setAvatar(path: string) {
    return invoke<string>('set_avatar', { path });
  },
  setBanner(path: string) {
    return invoke<string>('set_banner', { path });
  },
  getAvatar(hash: string): Promise<string> {
    if (!hash || typeof hash !== 'string' || !hash.trim()) {
      return Promise.reject(new Error('Empty avatar hash'));
    }
    const cached = avatarCache.get(hash);
    if (cached) return Promise.resolve(cached);
    const existing = avatarInFlight.get(hash);
    if (existing) return existing;
    const promise = invoke<string>('get_avatar', { hash })
      .then((data) => {
        avatarCache.set(hash, data);
        avatarInFlight.delete(hash);
        return data;
      })
      .catch((err) => {
        avatarInFlight.delete(hash);
        throw err;
      });
    avatarInFlight.set(hash, promise);
    return promise;
  },
  signOut() {
    return invoke<void>('sign_out');
  },
  resetIdentity() {
    return invoke<void>('reset_identity');
  },
  verifyPassphrase(passphrase: string) {
    return invoke<boolean>('verify_passphrase', { passphrase });
  },
  getRecoveryCode(passphrase: string) {
    return invoke<string>('get_recovery_code', { passphrase });
  },
  verifyRecoveryCode(code: string) {
    return invoke<boolean>('verify_recovery_code', { code });
  },
  getDeviceInfo() {
    return invoke<DeviceInfo>('get_device_info');
  },
  listSessions() {
    return invoke<SessionInfo[]>('list_sessions');
  },
  revokeSession(deviceId: string) {
    return invoke<void>('revoke_session', { deviceId });
  },
};

// ── Crypto & verification ────────────────────────────────────────

export const cryptoApi = {
  generateKeypair(keyType?: string) {
    return invoke<KeyPairResponse>('generate_keypair', keyType !== undefined ? { keyType } : {});
  },
  signMessage(input: { message: string }) {
    return invoke<string>('sign_message', { input });
  },
  verifySignature(input: { message: string; signature: string; publicKey: string }) {
    return invoke<boolean>('verify_signature', { input });
  },
  getPublicKey() {
    return invoke<string>('get_public_key');
  },
  fingerprint() {
    return invoke<string>('fingerprint');
  },
};

// ── Messages ─────────────────────────────────────────────────────

export const messageApi = {
  send(input: {
    channelId: string;
    content: string;
    messageType?: string | null;
    replyToId?: string | null;
    disappearSeconds?: number | null;
    attachments?: Array<{
      fileId: string;
      r2Key: string;
      sizeBytes: number;
      contentKeyCiphertext?: string | null;
      mimeTypeHint?: string | null;
    }>;
  }) {
    return invoke<MessageInfo>('send_message', { input });
  },
  load(channelId: string, beforeId?: string, limit = 50) {
    return invoke<MessageInfo[]>('load_messages', { channelId, beforeId, limit });
  },
  edit(messageId: string, content: string) {
    return invoke<MessageInfo>('edit_message', { messageId, newContent: content });
  },
  delete(messageId: string) {
    return invoke<void>('delete_message', { messageId });
  },
  clearChannel(channelId: string) {
    return invoke<void>('clear_channel_messages', { channelId });
  },
  addReaction(messageId: string, emoji: string) {
    return invoke<void>('add_reaction', { messageId, emoji });
  },
  removeReaction(messageId: string, emoji: string) {
    return invoke<void>('remove_reaction', { messageId, emoji });
  },
  pin(messageId: string) {
    return invoke<void>('pin_message', { messageId });
  },
  unpin(messageId: string) {
    return invoke<void>('unpin_message', { messageId });
  },
  markAsRead(channelId: string) {
    return invoke<void>('mark_as_read', { channelId });
  },
  getPinned(channelId: string) {
    return invoke<MessageInfo[]>('get_pinned_messages', { channelId });
  },
  search(query: string, channelId?: string | null, limit?: number | null) {
    return invoke<MessageInfo[]>('search_messages', {
      channelId: channelId ?? null,
      query,
      limit: limit ?? null,
    });
  },
};

// ── Voice / video (LiveKit) ──────────────────────────────────────

export const voiceApi = {
  join(input: { channelId: string; withCamera?: boolean; withScreen?: boolean }) {
    return invoke<VoiceJoinResponse>('join_voice_channel', {
      input: {
        channelId: input.channelId,
        withCamera: input.withCamera ?? false,
        withScreen: input.withScreen ?? false,
      },
    });
  },
  leave() {
    return invoke<void>('leave_voice_channel');
  },
  getLivekitToken(input: { channelId: string }) {
    return invoke<VoiceJoinResponse>('get_livekit_token', { input });
  },
  startScreenShare() {
    return invoke<void>('start_screen_share');
  },
  stopScreenShare() {
    return invoke<void>('stop_screen_share');
  },
  setAudioDevice(input: { deviceId: string | null; deviceType?: 'input' | 'output' }) {
    return invoke<void>('set_audio_device', {
      input: { deviceId: input.deviceId, deviceType: input.deviceType ?? 'input' },
    });
  },
  setVideoDevice(input: { deviceId: string | null }) {
    return invoke<void>('set_video_device', { input });
  },
  toggleMute() {
    return invoke<void>('toggle_mute');
  },
  toggleCamera() {
    return invoke<void>('toggle_camera');
  },
};

// ── Files ────────────────────────────────────────────────────────

export const fileApi = {
  upload(input: { path: string; channelId: string }) {
    return invoke<FileInfo>('upload_file', { input });
  },
  uploadBytes(input: { bytes: number[] | Uint8Array; channelId: string }) {
    const bytesArray = input.bytes instanceof Uint8Array ? Array.from(input.bytes) : input.bytes;
    return invoke<FileInfo>('upload_bytes', { input: { bytes: bytesArray, channelId: input.channelId } });
  },
  download(input: { fileId: string; destinationPath: string }) {
    return invoke<string>('download_file', { input });
  },
  delete(fileId: string) {
    return invoke<void>('delete_file', { input: { fileId } });
  },
  getInfo(fileId: string) {
    return invoke<FileInfo>('get_file_info', { fileId });
  },
  getDataUrl(fileId: string) {
    return invoke<string>('get_file_data_url', { fileId });
  },
};

// ── Settings ─────────────────────────────────────────────────────

export const settingsApi = {
  get() {
    return invoke<AppSettings>('get_settings');
  },
  update(newSettings: Partial<AppSettings>) {
    return invoke<AppSettings>('update_settings', { newSettings });
  },
  getAboutInfo() {
    return invoke<AboutInfo>('get_about_info');
  },
  setStartupBehavior(startOnLogin: boolean, minimizeToTray: boolean) {
    return invoke<void>('set_startup_behavior', { startOnLogin, minimizeToTray });
  },
};

export interface Diagnostics {
  appVersion: string;
  platform: string;
  supabaseConfigured: boolean;
  supabaseReachable: boolean;
  livekitConfigured: boolean;
  r2Configured: boolean;
  realtimeConnected: boolean;
  messageCount: number;
  friendCount: number;
  spaceCount: number;
  queuedCount: number;
  fileCount: number;
  databaseSizeBytes: number;
  logDirectory: string;
}

export const diagnosticsApi = {
  get() {
    return invoke<Diagnostics>('get_diagnostics');
  },
  getLogDirectory() {
    return invoke<string>('get_log_directory');
  },
  openLogFolder() {
    return invoke<void>('open_log_folder');
  },
};

// ── Data management ──────────────────────────────────────────────

export const dataApi = {
  exportData(outputPath: string) {
    return invoke<string>('export_data', { outputPath });
  },
  importData(input: { archivePath: string; passphrase: string }) {
    return invoke<void>('import_data', { archivePath: input.archivePath, passphrase: input.passphrase });
  },
  clearLocalData(passphrase: string) {
    return invoke<void>('clear_local_data', { passphrase });
  },
};

// ── Spaces (NEW) ─────────────────────────────────────────────────

export const spaceApi = {
  list() {
    return invoke<SpaceInfo[]>('spaces_list');
  },
  create(input: { name: string; iconHash?: string | null }) {
    return invoke<SpaceInfo>('spaces_create', { input });
  },
  update(input: {
    id: string;
    name?: string;
    iconHash?: string | null;
    bannerHash?: string | null;
    clearIcon?: boolean;
    clearBanner?: boolean;
    description?: string | null;
  }) {
    const payload = {
      ...input,
      clearIcon: input.clearIcon ?? (input.iconHash === null ? true : undefined),
      clearBanner: input.clearBanner ?? (input.bannerHash === null ? true : undefined),
    };
    return invoke<SpaceInfo>('spaces_update', { input: payload });
  },
  setBanner(spaceId: string, path: string) {
    return invoke<string>('spaces_set_banner', { spaceId, path });
  },
  setIcon(spaceId: string, path: string) {
    return invoke<string>('spaces_set_icon', { spaceId, path });
  },
  setCustomLink(spaceId: string, link: string) {
    return invoke<SpaceInfo>('spaces_set_custom_link', { input: { spaceId, link } });
  },
  transferOwnership(spaceId: string, newOwnerId: string) {
    return invoke<SpaceInfo>('spaces_transfer_ownership', { input: { spaceId, newOwnerId } });
  },
  searchPublic(query?: string) {
    return invoke<SpaceInfo[]>('spaces_search_public', { query: query ?? null });
  },
  joinPublic(spaceIdOrLink: string) {
    return invoke<SpaceInfo>('spaces_join_public', { spaceIdOrLink });
  },
  delete(spaceId: string) {
    return invoke<void>('spaces_delete', { spaceId });
  },
  leave(spaceId: string) {
    return invoke<void>('spaces_leave', { spaceId });
  },
};

export const channelApi = {
  list(spaceId: string) {
    return invoke<ChannelInfo[]>('channels_list', { spaceId });
  },
  create(input: { spaceId: string; name: string; channelType: ChannelType; position?: number; e2ee?: boolean }) {
    return invoke<ChannelInfo>('channels_create', { input });
  },
  update(input: { id: string; name?: string; position?: number }) {
    return invoke<ChannelInfo>('channels_update', { input });
  },
  delete(channelId: string) {
    return invoke<void>('channels_delete', { channelId });
  },
  getOverrides(channelId: string) {
    return invoke<ChannelOverrideItem[]>('channels_get_overrides', { channelId });
  },
  updateOverrides(input: { channelId: string; overrides: ChannelOverrideItem[] }) {
    return invoke<void>('channels_update_overrides', { input });
  },
};

export const roleApi = {
  list(spaceId: string) {
    return invoke<RoleInfo[]>('roles_list', { spaceId });
  },
  create(input: { spaceId: string; name: string; color?: string | null; permissions: string[]; position?: number }) {
    return invoke<RoleInfo>('roles_create', { input });
  },
  update(input: { id: string; name?: string; color?: string | null; permissions?: string[]; position?: number }) {
    return invoke<RoleInfo>('roles_update', { input });
  },
  reorder(input: { spaceId: string; roleIds: string[] }) {
    return invoke<RoleInfo[]>('roles_reorder', { input });
  },
  deleteRole(roleId: string) {
    return invoke<void>('roles_delete', { roleId });
  },
};

export const inviteApi = {
  create(input: { spaceId: string; maxUses?: number | null; expiresAt?: number | null }) {
    return invoke<InviteInfo>('invites_create', { input });
  },
  redeem(code: string) {
    return invoke<SpaceInfo>('invites_redeem', { code });
  },
};

export const memberApi = {
  list(spaceId: string) {
    return invoke<MemberInfo[]>('members_list', { spaceId });
  },
  update(input: { spaceId: string; userId: string; roleIds: string[] }) {
    return invoke<void>('members_update', { input });
  },
  kick(input: { spaceId: string; userId: string; reason?: string | null }) {
    return invoke<void>('spaces_kick_member', { input });
  },
  ban(input: { spaceId: string; userId: string; reason?: string | null }) {
    return invoke<void>('spaces_ban_member', { input });
  },
  unban(input: { spaceId: string; userId: string }) {
    return invoke<void>('spaces_unban_member', { input: { spaceId: input.spaceId, userId: input.userId } });
  },
  timeout(input: { spaceId: string; userId: string; until: number | null }) {
    return invoke<void>('spaces_timeout_member', { input });
  },
  listBans(spaceId: string) {
    return invoke<BanInfo[]>('spaces_bans_list', { spaceId });
  },
};

// ── Friends & DMs (NEW) ──────────────────────────────────────────

export const friendApi = {
  add(input: { username: string }) {
    return invoke<void>('friends_add', { input });
  },
  accept(userId: string) {
    return invoke<void>('friends_accept', { userId });
  },
  reject(userId: string) {
    return invoke<void>('friends_reject', { userId });
  },
  cancel(userId: string) {
    return invoke<void>('friends_cancel', { userId });
  },
  remove(userId: string) {
    return invoke<void>('friends_remove', { userId });
  },
  block(userId: string) {
    return invoke<void>('friends_block', { userId });
  },
  unblock(userId: string) {
    return invoke<void>('friends_unblock', { userId });
  },
  list() {
    return invoke<FriendInfo[]>('friends_list');
  },
};

export const dmApi = {
  list() {
    return invoke<ChannelInfo[]>('dm_list');
  },
  open(userId: string) {
    return invoke<ChannelInfo>('dm_open', { input: { userId } });
  },
  createGroup(input: { name?: string; memberIds: string[] }) {
    return invoke<ChannelInfo>('group_dm_create', { input });
  },
};

// ── Presence & typing (NEW) ──────────────────────────────────────

export const presenceApi = {
  update(status: PresenceStatus) {
    return invoke<void>('presence_update', { status });
  },
  setTyping(input: { channelId: string; isTyping: boolean }) {
    return invoke<void>('typing_set', { input });
  },
};

export interface UserProfileInfo {
  userId: string;
  username: string;
  displayName: string;
  avatarHash: string | null;
  bio: string | null;
  customStatus?: string | null;
  onlineStatus: PresenceStatus;
  friendStatus: FriendStatus;
  bannerHash?: string | null;
  /** Kayıt/hesap açma tarihi (unix saniye); showJoinDate kapalıysa yalnızca kendi profilinde. */
  createdAt?: number | null;
  showJoinDate?: boolean;
}

export const socialApi = {
  getUserProfile(userId: string) {
    return invoke<UserProfileInfo>('get_user_profile', { userId });
  },
  resolveUsername(username: string) {
    return invoke<UserProfileInfo>('resolve_username', { username });
  },
};

// ── GIF search (Tenor / Giphy, keys stay server-side) ───────────────────────

export interface GifResult {
  id: string;
  title: string;
  url: string;
  preview: string;
  width: number;
  height: number;
  provider: string;
}

export const gifApi = {
  search(query: string, limit = 24) {
    return invoke<GifResult[]>('gif_search', { query, limit });
  },
  trending(limit = 24) {
    return invoke<GifResult[]>('gif_trending', { limit });
  },
};

// ── Local AI (Ollama, opt-in) ────────────────────────────────────────────────

export const localAiApi = {
  chat(input: { message: string; model?: string | null }) {
    return invoke<string>('local_ai_chat', { input });
  },
  status() {
    return invoke<{ available: boolean; model: string | null }>('local_ai_status');
  },
};

// ── MLS group E2EE ───────────────────────────────────────────────────────────

export interface MlsKeyPackage {
  keyPackage: string;
  signerPrivate: string;
}

export const mlsApi = {
  initChannel(channelId: string) {
    return invoke<void>('mls_init_channel', { channelId });
  },
  createKeyPackage(channelId: string) {
    return invoke<MlsKeyPackage>('mls_create_key_package', { channelId });
  },
  addMember(input: { channelId: string; userId: string; keyPackage: string }) {
    return invoke<void>('mls_add_member', { input });
  },
  consumeWelcome(input: { channelId: string; keyPackage: string; signerPrivate: string }) {
    return invoke<void>('mls_consume_welcome', { input });
  },
  callKey(channelId: string) {
    return invoke<string | null>('mls_call_key', { channelId });
  },
};

// ── Discord bridge (webhook, policy-compliant) ───────────────────────────────

export interface WebhookInfo {
  channelId: string;
  maskedUrl: string;
}

export const discordApi = {
  setWebhook(input: { channelId: string; webhookUrl: string }) {
    return invoke<WebhookInfo>('discord_set_webhook', { input });
  },
  clearWebhook(channelId: string) {
    return invoke<void>('discord_clear_webhook', { channelId });
  },
  getWebhook(channelId: string) {
    return invoke<WebhookInfo | null>('discord_get_webhook', { channelId });
  },
};

// ── In-app Updater ──────────────────────────────────────────────────────────

export interface PlatformAsset {
  name: string;
  size: number;
  downloadUrl: string;
  kind: string;
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  isSameVersionNewerBuild: boolean;
  releaseName: string;
  releaseNotes: string;
  publishedAt: string;
  downloadUrl?: string | null;
  assetName?: string | null;
  assetSize?: number | null;
  platform: string;
  allAssets: PlatformAsset[];
  statusMessage: string;
  detectionMethod: string;
}

export const updaterApi = {
  check() {
    return invoke<UpdateCheckResult>('check_for_updates');
  },
  downloadAndInstall(downloadUrl: string, assetName: string) {
    return invoke<void>('download_and_install_update', { downloadUrl, assetName });
  },
};

// ── Privacy Tools & Diagnostics ─────────────────────────────────────────────

export interface TorStatusResult {
  isTor: boolean;
  ip: string;
}

export interface IpLeakResult {
  ip: string;
  colo?: string | null;
  loc?: string | null;
  tls?: string | null;
  sni?: string | null;
  warp?: string | null;
  gateway?: string | null;
  rttMs: number;
}

export interface DohTestResult {
  cloudflareOk: boolean;
  googleOk: boolean;
  latencyCloudflareMs: number;
  latencyGoogleMs: number;
  dohWorking: boolean;
}

export interface DohProviderMetric {
  name: string;
  endpoint: string;
  isReachable: boolean;
  latencyMs: number;
}

export interface MultiDohResult {
  providers: DohProviderMetric[];
  fastestProvider: string | null;
  averageLatencyMs: number;
  censorshipTamperDetected: boolean;
}

export interface PwnedCheckResponse {
  isPwned: boolean;
  breachCount: number;
  hashPrefix: string;
}

export interface UrlScanResult {
  queryStatus: string;
  isMalicious: boolean;
  urlStatus?: string | null;
  threat?: string | null;
  tags: string[];
  urlhausReference?: string | null;
}

export interface LinkPreviewResult {
  url: string;
  title: string | null;
  description: string | null;
  image: string | null;
  siteName: string | null;
  favicon: string | null;
  isSafe: boolean;
}

export interface NetworkAsnResult {
  ip: string;
  isp?: string | null;
  org?: string | null;
  asn?: string | null;
  country?: string | null;
  city?: string | null;
  tlsVersion?: string | null;
  httpVersion?: string | null;
}

export interface ClockSkewResult {
  localTimestamp: number;
  serverTimestamp: number;
  skewSeconds: number;
  isSkewed: boolean;
}

export interface ProxyTestResult {
  connected: boolean;
  isTor: boolean;
  exitIp: string | null;
  latencyMs: number;
  protocol: string;
  proxyEndpoint: string;
  dnsLeakProtected: boolean;
  errorMessage: string | null;
}

export interface TorServiceDetectionResult {
  standaloneTorAvailable: boolean;
  torBrowserAvailable: boolean;
  recommendedEndpoint: string | null;
}

export interface SystemVpnDetectionResult {
  torStandalone: boolean;
  torBrowser: boolean;
  cloudflareWarpRunning: boolean;
  localSocksRunning: boolean;
  recommendedMode: string;
  recommendedEndpoint: string | null;
  details: string;
}

export interface WireguardValidationResult {
  isValid: boolean;
  interfaceAddress: string | null;
  peerEndpoint: string | null;
  peerPublicKey: string | null;
  allowedIps: string | null;
  dns: string | null;
  errorMessage: string | null;
}

export interface PrivacyEndpointInfo {
  id: string;
  name: string;
  category: string;
  description: string;
  endpoint: string;
  defaultPort: number;
  freeTier: boolean;
  zeroLog: boolean;
  dnsLeakProtected: boolean;
  recommended: boolean;
}

export const privacyToolsApi = {
  checkTorStatus() {
    return invoke<TorStatusResult>('check_tor_status');
  },
  checkIpLeak() {
    return invoke<IpLeakResult>('check_ip_leak');
  },
  checkDohStatus() {
    return invoke<DohTestResult>('check_doh_status');
  },
  checkMultiDohStatus() {
    return invoke<MultiDohResult>('check_multi_doh_status');
  },
  checkPasswordPwned(password: string) {
    return invoke<PwnedCheckResponse>('check_password_pwned', { password });
  },
  scanUrl(url: string) {
    return invoke<UrlScanResult>('scan_urlhaus', { url });
  },
  fetchLinkPreview(url: string) {
    return invoke<LinkPreviewResult>('fetch_link_preview', { url });
  },
  generateQrSvg(content: string) {
    return invoke<string>('generate_qr_svg', { content });
  },
  getNetworkAsnInfo() {
    return invoke<NetworkAsnResult>('get_network_asn_info');
  },
  generateAvatar(seed: string) {
    return invoke<string>('generate_privacy_avatar', { seed });
  },
  detectClockSkew() {
    return invoke<ClockSkewResult>('detect_clock_skew');
  },
  testProxyConnection(proxyUrl?: string | null) {
    return invoke<ProxyTestResult>('test_proxy_connection', { proxyUrl: proxyUrl ?? null });
  },
  detectLocalTorServices() {
    return invoke<TorServiceDetectionResult>('detect_local_tor_services');
  },
  detectSystemVpnServices() {
    return invoke<SystemVpnDetectionResult>('detect_system_vpn_services');
  },
  validateWireguardProfile(profileText: string) {
    return invoke<WireguardValidationResult>('validate_wireguard_profile', { profileText });
  },
  getPrivacyEndpointsAndRelays() {
    return invoke<PrivacyEndpointInfo[]>('get_privacy_endpoints_and_relays');
  },
};


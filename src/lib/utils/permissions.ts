/**
 * veilanon — Comprehensive Granular Permission & Hierarchy Engine
 *
 * Implements Discord-grade permission resolution:
 * 1. Space Owner: Infinite rank, bypasses all permission checks and channel overrides.
 * 2. Administrator Role: Bypasses all channel overrides and checks.
 * 3. Base Permissions: Union of all assigned server roles.
 * 4. Channel Overrides:
 *    a. Apply @everyone role override (Deny removes, Allow adds).
 *    b. Apply member's specific roles' overrides (Union of Denies removes, Union of Allows adds).
 *    c. Apply member-specific override (Deny removes, Allow adds).
 * 5. Role Hierarchy:
 *    - Highest role position determines power level.
 *    - Cannot moderate, kick, ban, timeout, or manage members with >= highest role rank.
 *    - Cannot manage, assign, or delete roles with >= highest role rank.
 */

import type { RoleInfo, MemberInfo, ChannelOverrideItem } from '$lib/api/tauri';
import type { IconName } from '$lib/types/icon';

export type PermissionCategory = 'general' | 'moderation' | 'text' | 'voice';

export interface PermissionDef {
  id: string;
  label: string;
  desc: string;
  category: PermissionCategory;
  danger?: boolean;
  icon?: IconName;
}

export const ALL_PERMISSIONS: PermissionDef[] = [
  // ── Genel & Yönetim ────────────────────────────────────────────────────────
  {
    id: 'administrator',
    label: 'Yönetici (Administrator)',
    desc: 'Topluluktaki tüm izinleri koşulsuz sağlar ve kanal kısıtlamalarını atlar. Bu yetkiye sahip üyeler tüm kanallara erişebilir ve yönetebilir.',
    category: 'general',
    danger: true,
    icon: 'shield',
  },
  {
    id: 'manage_space',
    label: 'Topluluğu Yönet',
    desc: 'Topluluk adını, simgesini, bannerını, açıklamasını ve özel bağlantısını düzenleyebilir.',
    category: 'general',
    icon: 'settings',
  },
  {
    id: 'view_audit_log',
    label: 'Denetim Kaydını Görüntüle',
    desc: 'Toplulukta gerçekleştirilen tüm moderasyon, rol ve kanal değişiklik kayıtlarını inceleyebilir.',
    category: 'general',
    icon: 'info',
  },
  {
    id: 'manage_roles',
    label: 'Rolleri Yönet',
    desc: 'Kendi rolünden daha düşük seviyedeki rolleri oluşturabilir, düzenleyebilir, sıralayabilir ve üyelere atayabilir.',
    category: 'general',
    icon: 'shield',
  },
  {
    id: 'manage_channels',
    label: 'Kanalları Yönet',
    desc: 'Yeni metin, ses ve forum kanalları oluşturabilir, kanal ayarlarını, izinlerini ve sıralamasını düzenleyebilir.',
    category: 'general',
    icon: 'hash',
  },
  {
    id: 'manage_invites',
    label: 'Davetleri Yönet',
    desc: 'Topluluk için davet bağlantıları oluşturabilir, aktif davetleri listeleyebilir ve süresi dolmamış davetleri iptal edebilir.',
    category: 'general',
    icon: 'link',
  },
  {
    id: 'manage_webhooks',
    label: 'Webhook & Entegrasyonları Yönet',
    desc: 'Discord köprüsü ve harici bot webhook entegrasyonlarını ekleyebilir, güncelleyebilir veya kaldırabilir.',
    category: 'general',
    icon: 'megaphone',
  },

  // ── Moderasyon & Üyeler ───────────────────────────────────────────────────
  {
    id: 'kick_members',
    label: 'Üyeleri At (Kick)',
    desc: 'Kendinden alt seviyedeki üyeleri topluluktan çıkarabilir. Atılan üyeler geçerli bir davetle tekrar katılabilir.',
    category: 'moderation',
    icon: 'logout',
  },
  {
    id: 'ban_members',
    label: 'Üyeleri Yasakla (Ban)',
    desc: 'Kendinden alt seviyedeki üyeleri topluluktan kalıcı olarak yasaklar ve davetle bile geri dönmelerini engeller.',
    category: 'moderation',
    danger: true,
    icon: 'x',
  },
  {
    id: 'timeout_members',
    label: 'Üyeleri Sustur (Timeout)',
    desc: 'Belirli bir süre boyunca üyelerin mesaj göndermesini, tepki vermesini ve ses kanallarında konuşmasını engeller.',
    category: 'moderation',
    icon: 'moon',
  },

  // ── Metin & Sohbet ────────────────────────────────────────────────────────
  {
    id: 'send_messages',
    label: 'Mesaj Gönder',
    desc: 'Metin, duyuru ve forum kanallarına mesaj yazabilir.',
    category: 'text',
    icon: 'chat',
  },
  {
    id: 'read_messages',
    label: 'Mesaj Geçmişini Oku',
    desc: 'Kanalın geçmiş mesajlarını ve arşivini okuyabilir.',
    category: 'text',
    icon: 'chat',
  },
  {
    id: 'manage_messages',
    label: 'Mesajları Yönet',
    desc: 'Diğer kullanıcıların gönderdiği mesajları silebilir ve sabitleme durumlarını değiştirebilir.',
    category: 'text',
    danger: true,
    icon: 'trash',
  },
  {
    id: 'embed_links',
    label: 'Bağlantı Önizlemesi Ekle',
    desc: 'Mesajlarda paylaşılan web bağlantılarının zengin kart önizlemelerini gösterir.',
    category: 'text',
    icon: 'link',
  },
  {
    id: 'attach_files',
    label: 'Dosya & Medya Yükle',
    desc: 'Resim, video, belge, ses kaydı ve dosya ekleri yükleyebilir.',
    category: 'text',
    icon: 'upload',
  },
  {
    id: 'add_reactions',
    label: 'Tepki Ekle',
    desc: 'Mesajlara yeni emoji tepkileri ekleyebilir.',
    category: 'text',
    icon: 'sparkle',
  },
  {
    id: 'use_slash_commands',
    label: 'Eğik Çizgi (/) Komutlarını Kullan',
    desc: 'Uygulama ve bot eğik çizgi komutlarını çalıştırabilir.',
    category: 'text',
    icon: 'sparkle',
  },
  {
    id: 'mention_everyone',
    label: '@everyone & @here Etiketle',
    desc: 'Kanal veya sunucudaki tüm üyelere anlık bildirim gönderebilir.',
    category: 'text',
    danger: true,
    icon: 'bell',
  },
  {
    id: 'pin_messages',
    label: 'Mesajları Sabitle',
    desc: 'Önemli mesajları kanalın sabitlenenler paneline ekleyebilir veya kaldırabilir.',
    category: 'text',
    icon: 'pin',
  },

  // ── Ses & Görüntü ─────────────────────────────────────────────────────────
  {
    id: 'connect_voice',
    label: 'Sese Bağlan',
    desc: 'Ses ve görüntülü arama kanallarına katılabilir.',
    category: 'voice',
    icon: 'volume',
  },
  {
    id: 'speak',
    label: 'Konuş',
    desc: 'Ses kanallarında mikrofonunu kullanarak konuşabilir.',
    category: 'voice',
    icon: 'mic',
  },
  {
    id: 'stream_video',
    label: 'Kamera Aç (Video)',
    desc: 'Ses kanallarında kamerasını canlı yayına aktarabilir.',
    category: 'voice',
    icon: 'camera',
  },
  {
    id: 'share_screen',
    label: 'Ekran Paylaş',
    desc: 'Masaüstünü veya uygulama penceresini ses kanalında canlı paylaşabilir.',
    category: 'voice',
    icon: 'screen',
  },
  {
    id: 'mute_members',
    label: 'Üyeleri Sustur (Server Mute)',
    desc: 'Ses kanalındaki diğer üyelerin mikrofonunu topluluk genelinde kapatabilir.',
    category: 'voice',
    icon: 'mic-off',
  },
  {
    id: 'deafen_members',
    label: 'Üyeleri Sağırlaştır (Server Deafen)',
    desc: 'Ses kanalındaki diğer üyelerin ses duymasını engelleyebilir.',
    category: 'voice',
    icon: 'volume-x',
  },
  {
    id: 'move_members',
    label: 'Üyeleri Taşı',
    desc: 'Üyeleri bir ses kanalından diğerine aktarabilir veya bağlantısını kesebilir.',
    category: 'voice',
    icon: 'phone-off',
  },
  {
    id: 'use_voice_activity',
    label: 'Ses Etkinliği Kullan',
    desc: 'Bas-konuş zorunluluğu olmadan otomatik ses algılamasıyla konuşabilir.',
    category: 'voice',
    icon: 'mic',
  },
  {
    id: 'priority_speaker',
    label: 'Öncelikli Konuşmacı',
    desc: 'Konuştuğunda kanaldaki diğer üyelerin ses seviyesi otomatik olarak kısılır.',
    category: 'voice',
    icon: 'volume',
  },
];

export const PERMISSION_MAP = new Map<string, PermissionDef>(
  ALL_PERMISSIONS.map((p) => [p.id, p])
);

export const PERMISSION_CATEGORIES: Array<{ id: PermissionCategory; label: string; icon: IconName }> = [
  { id: 'general', label: 'Genel & Yönetim', icon: 'shield' },
  { id: 'moderation', label: 'Moderasyon', icon: 'settings' },
  { id: 'text', label: 'Metin & Sohbet', icon: 'chat' },
  { id: 'voice', label: 'Ses & Video', icon: 'volume' },
];

/**
 * Calculates a member's highest role rank in the space.
 * Returns Infinity if owner, 0 if no roles, or the maximum role position.
 */
export function getMemberHighestRolePosition(
  member: MemberInfo | null | undefined,
  roles: RoleInfo[],
  isOwner = false
): number {
  if (isOwner) return Infinity;
  if (!member || !member.roleIds || member.roleIds.length === 0) return 0;
  let highest = 0;
  for (const rid of member.roleIds) {
    const role = roles.find((r) => r.id === rid);
    if (role && role.position > highest) {
      highest = role.position;
    }
  }
  return highest;
}

/**
 * Checks whether an actor can moderate or perform actions on a target member.
 */
export function canActorManageTarget(
  actorUserId: string,
  targetUserId: string,
  roles: RoleInfo[],
  members: MemberInfo[],
  isActorOwner = false,
  targetIsOwner = false,
  requiredPermission?: string
): boolean {
  if (targetIsOwner && !isActorOwner) return false;
  if (isActorOwner) return true;
  if (actorUserId === targetUserId) return false;

  const actorMember = members.find((m) => m.userId === actorUserId);
  const targetMember = members.find((m) => m.userId === targetUserId);
  if (!actorMember) return false;

  // Check if actor has the required permission
  if (requiredPermission) {
    const actorEffective = computeEffectivePermissions({
      isOwner: isActorOwner,
      userRoles: roles.filter((r) => actorMember.roleIds.includes(r.id)),
      allRoles: roles,
    });
    if (!actorEffective.has(requiredPermission)) {
      return false;
    }
  }

  const actorRank = getMemberHighestRolePosition(actorMember, roles, isActorOwner);
  const targetRank = getMemberHighestRolePosition(targetMember, roles, targetIsOwner);
  return actorRank > targetRank;
}

/**
 * Checks whether an actor can edit, delete, or create a role with a given position.
 */
export function canActorManageRole(
  actorUserId: string,
  targetRole: RoleInfo | { position?: number; permissions?: string[] } | null,
  roles: RoleInfo[],
  members: MemberInfo[],
  isActorOwner = false
): boolean {
  if (isActorOwner) return true;
  const actorMember = members.find((m) => m.userId === actorUserId);
  if (!actorMember) return false;

  const actorEffective = computeEffectivePermissions({
    isOwner: isActorOwner,
    userRoles: roles.filter((r) => actorMember.roleIds.includes(r.id)),
    allRoles: roles,
  });

  if (!actorEffective.has('manage_roles')) {
    return false;
  }

  const actorRank = getMemberHighestRolePosition(actorMember, roles, isActorOwner);
  const targetPos = targetRole?.position ?? 0;

  // If target role is admin, actor must also be admin or owner
  if (targetRole?.permissions?.includes('administrator') && !actorEffective.has('administrator')) {
    return false;
  }

  return actorRank > targetPos;
}

export interface EffectivePermissionParams {
  isOwner?: boolean;
  userId?: string;
  userRoles: RoleInfo[];
  allRoles: RoleInfo[];
  channelOverrides?: ChannelOverrideItem[];
  timeoutUntil?: number | null;
}

export interface EffectivePermissions {
  has: (permissionId: string) => boolean;
  isOwner: boolean;
  isAdmin: boolean;
  isTimedOut: boolean;
  timeoutRemainingSeconds: number;
  allEnabledIds: string[];
}

/**
 * Computes exact effective permissions for a user in a given space and optional channel.
 */
export function computeEffectivePermissions(
  params: EffectivePermissionParams
): EffectivePermissions {
  const {
    isOwner = false,
    userId,
    userRoles = [],
    allRoles = [],
    channelOverrides = [],
    timeoutUntil = null,
  } = params;

  const nowSec = Math.floor(Date.now() / 1000);
  const isTimedOut = timeoutUntil !== null && timeoutUntil > nowSec;
  const timeoutRemainingSeconds = isTimedOut ? timeoutUntil! - nowSec : 0;

  // 1. Owner bypass
  if (isOwner) {
    return {
      has: () => true,
      isOwner: true,
      isAdmin: true,
      isTimedOut: false,
      timeoutRemainingSeconds: 0,
      allEnabledIds: ALL_PERMISSIONS.map((p) => p.id),
    };
  }

  // 2. Check if user has Administrator role
  const isAdmin = userRoles.some((r) => r.permissions.includes('administrator'));
  if (isAdmin) {
    return {
      has: () => true,
      isOwner: false,
      isAdmin: true,
      isTimedOut: false,
      timeoutRemainingSeconds: 0,
      allEnabledIds: ALL_PERMISSIONS.map((p) => p.id),
    };
  }

  // 3. Base permissions (union of user's server roles + default role)
  const perms = new Set<string>();
  const defaultRole = allRoles.find((r) => r.isDefault || r.name === '@everyone');
  if (defaultRole) {
    for (const p of defaultRole.permissions) {
      perms.add(p);
    }
  }

  for (const role of userRoles) {
    for (const p of role.permissions) {
      perms.add(p);
    }
  }

  // Fallback baseline for any member in the space if no restrictive role explicitly exists
  if (perms.size === 0) {
    perms.add('read_messages');
    perms.add('send_messages');
    perms.add('connect_voice');
    perms.add('speak');
    perms.add('stream_video');
    perms.add('share_screen');
    perms.add('add_reactions');
    perms.add('attach_files');
    perms.add('embed_links');
    perms.add('use_slash_commands');
  }

  // 4. If channel overrides exist, apply hierarchy
  if (channelOverrides.length > 0) {
    // 4a. @everyone override (targetType === 'role', targetId matches default role or is named @everyone)
    const everyoneOverride = channelOverrides.find(
      (ov) =>
        ov.targetType === 'role' &&
        (allRoles.find((r) => r.id === ov.targetId)?.isDefault ||
          allRoles.find((r) => r.id === ov.targetId)?.name === '@everyone')
    );
    if (everyoneOverride) {
      for (const deny of everyoneOverride.deny) {
        perms.delete(deny);
      }
      for (const allow of everyoneOverride.allow) {
        perms.add(allow);
      }
    }

    // 4b. Role overrides for member's assigned roles
    const userRoleIds = new Set(userRoles.map((r) => r.id));
    const relevantRoleOverrides = channelOverrides.filter(
      (ov) => ov.targetType === 'role' && userRoleIds.has(ov.targetId)
    );

    // Apply all denies of matching roles
    for (const ov of relevantRoleOverrides) {
      for (const deny of ov.deny) {
        perms.delete(deny);
      }
    }
    // Apply all allows of matching roles
    for (const ov of relevantRoleOverrides) {
      for (const allow of ov.allow) {
        perms.add(allow);
      }
    }

    // 4c. Member-specific override
    if (userId) {
      const memberOverride = channelOverrides.find(
        (ov) => ov.targetType === 'member' && ov.targetId === userId
      );
      if (memberOverride) {
        for (const deny of memberOverride.deny) {
          perms.delete(deny);
        }
        for (const allow of memberOverride.allow) {
          perms.add(allow);
        }
      }
    }
  }

  // 5. If member is currently timed out, revoke interactive permissions
  if (isTimedOut) {
    perms.delete('send_messages');
    perms.delete('add_reactions');
    perms.delete('attach_files');
    perms.delete('speak');
    perms.delete('stream_video');
    perms.delete('share_screen');
  }

  return {
    has: (permissionId: string) => perms.has(permissionId),
    isOwner: false,
    isAdmin: false,
    isTimedOut,
    timeoutRemainingSeconds,
    allEnabledIds: Array.from(perms),
  };
}

/**
 * Format slowmode seconds to a human-friendly string.
 */
export function formatSlowmode(seconds: number): string {
  if (seconds <= 0) return 'Kapalı';
  if (seconds < 60) return `${seconds} saniye`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} dakika`;
  return `${Math.floor(seconds / 3600)} saat`;
}

/**
 * Format timeout remaining seconds to a countdown string.
 */
export function formatTimeoutRemaining(seconds: number): string {
  if (seconds <= 0) return 'Sona erdi';
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  if (m === 0) return `${s}sn`;
  const h = Math.floor(m / 60);
  if (h === 0) return `${m}dk ${s}sn`;
  const d = Math.floor(h / 24);
  if (d === 0) return `${h}sa ${m % 60}dk`;
  return `${d}g ${h % 24}sa`;
}

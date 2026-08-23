/**
 * veilanon — Comprehensive Notification & Toast Store
 *
 * Manages in-app toasts, notification inbox/center history, desktop notifications,
 * and crystal-clear Web Audio cues with strict respect to user privacy & DND settings.
 */
import { writable, derived, get } from 'svelte/store';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { settingsApi, type NotificationPreview, type AppSettings } from '$lib/api/tauri';
import { uiStore } from '$lib/stores/ui';
import { streamerMode } from '$lib/stores/streamerMode';
import {
  playMessageSound,
  playMentionSound,
  playFriendRequestSound,
  playCallJoinSound,
  playCallLeaveSound,
} from '$lib/utils/sound';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
  action?: ToastAction;
}

export type NotificationCategory =
  | 'message'
  | 'mention'
  | 'friend_request'
  | 'space_invite'
  | 'call'
  | 'system';

export interface NotificationItem {
  id: string;
  type: NotificationCategory;
  title: string;
  body: string;
  timestamp: number; // Unix timestamp in seconds
  read: boolean;
  avatarHash?: string | null;
  username?: string;
  channelId?: string | null;
  spaceId?: string | null;
  data?: Record<string, any>;
  actionLabel?: string;
}

export interface MessageNotificationParams {
  senderName: string;
  content: string;
  channelName?: string;
  isMention?: boolean;
  isDm?: boolean;
  channelId?: string;
  spaceId?: string | null;
  avatarHash?: string | null;
  username?: string;
}

export interface FriendRequestNotificationParams {
  username: string;
  displayName?: string;
  avatarHash?: string | null;
  userId?: string;
}

export interface SpaceInviteNotificationParams {
  spaceName: string;
  inviteCode: string;
  spaceId?: string;
  inviterName?: string;
}

const TOASTS_STORE = writable<Toast[]>([]);
const NOTIFICATIONS_KEY = 'veilanon_notifications_history_v1';

function loadStoredNotifications(): NotificationItem[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = localStorage.getItem(NOTIFICATIONS_KEY);
    if (!raw) return [];
    const items = JSON.parse(raw);
    return Array.isArray(items) ? items.slice(0, 100) : [];
  } catch {
    return [];
  }
}

function saveStoredNotifications(items: NotificationItem[]) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(NOTIFICATIONS_KEY, JSON.stringify(items.slice(0, 100)));
  } catch {
    // LocalStorage error fallback
  }
}

const NOTIFICATIONS_STORE = writable<NotificationItem[]>(loadStoredNotifications());

// Sync back to localStorage whenever notification items change
NOTIFICATIONS_STORE.subscribe((items) => {
  saveStoredNotifications(items);
});

export const unreadNotificationCount = derived(NOTIFICATIONS_STORE, ($items) => {
  return $items.filter((i) => !i.read).length;
});

let cachedSettings: AppSettings | null = null;
let settingsFetchTimer: ReturnType<typeof setTimeout> | null = null;

async function getOrFetchSettings(): Promise<AppSettings | null> {
  if (cachedSettings) return cachedSettings;
  try {
    cachedSettings = await settingsApi.get();
    if (settingsFetchTimer) clearTimeout(settingsFetchTimer);
    settingsFetchTimer = setTimeout(() => {
      cachedSettings = null;
    }, 8000); // 8s cache
    return cachedSettings;
  } catch {
    return null;
  }
}

const recentToastMessages = new Map<string, number>();
let lastAnyToastAt = 0;

function normalizeToastKey(msg: string): string {
  return msg
    .trim()
    .toLowerCase()
    .replace(/@\w+/g, '@user')
    .replace(/https?:\/\/\S+/g, 'url')
    .replace(/[\s.,!?;:()"'`]+/g, ' ')
    .replace(/\d+/g, '#')
    .replace(/\s+/g, ' ')
    .trim();
}

function addToast(type: ToastType, message: string, duration = 4000, action?: ToastAction) {
  if (!message || !message.trim()) return '';
  const cleanMsg = message.trim();
  const key = normalizeToastKey(cleanMsg);
  const now = Date.now();
  // Global throttle: no toast within 900ms of any previous toast
  if (now - lastAnyToastAt < 900) return '';
  // Per-key dedup: same semantic toast within 10s is suppressed
  const lastTime = recentToastMessages.get(key);
  if (lastTime !== undefined && now - lastTime < 10000) return '';
  // Concurrent duplicate check
  const currentToasts = get(TOASTS_STORE);
  if (currentToasts.some((t) => normalizeToastKey(t.message) === key)) return '';
  // Hard cap: max 2 concurrent toasts
  if (currentToasts.length >= 2) return '';
  recentToastMessages.set(key, now);
  lastAnyToastAt = now;
  setTimeout(() => recentToastMessages.delete(key), 10500);

  const id = crypto.randomUUID();
  TOASTS_STORE.update((toasts) => {
    const trimmed = toasts.slice(-1);
    return [...trimmed, { id, type, message: cleanMsg, duration, action }];
  });
  if (duration > 0) {
    setTimeout(() => removeToast(id), duration);
  }
  return id;
}

function removeToast(id: string) {
  TOASTS_STORE.update((toasts) => toasts.filter((t) => t.id !== id));
}

let notificationPermissionChecked = false;
let notificationPermissionGranted = false;

async function checkDesktopPermission(): Promise<boolean> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return false;
  if (notificationPermissionChecked) return notificationPermissionGranted;
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const perm = await requestPermission();
      granted = perm === 'granted';
    }
    notificationPermissionChecked = true;
    notificationPermissionGranted = granted;
    return granted;
  } catch {
    return false;
  }
}

function isStreamerPopupSuppressed(): boolean {
  try {
    const s = get(streamerMode);
    return s.enabled && s.suppressNotificationPopups;
  } catch {
    return false;
  }
}

function computeVolume(settings: AppSettings | null, baseVolume: number): number {
  const userVolPercent = settings?.notificationVolume ?? 80;
  const multiplier = Math.max(0, Math.min(100, userVolPercent)) / 100;
  return baseVolume * multiplier;
}

function isDndActive(settings: AppSettings | null): boolean {
  const currentPresence = get(uiStore).presence;
  const dndSuppressed = settings?.dndSuppressNotifications ?? true;
  return dndSuppressed && (currentPresence === 'dnd' || currentPresence === 'invisible');
}

export const notificationStore = {
  subscribe: NOTIFICATIONS_STORE.subscribe,
  unreadCount: unreadNotificationCount,

  add(item: Omit<NotificationItem, 'id' | 'timestamp' | 'read'>) {
    const now = Math.floor(Date.now() / 1000);
    const existing = get(NOTIFICATIONS_STORE);
    const isDuplicate = existing.some(
      (n) =>
        n.channelId &&
        n.channelId === item.channelId &&
        n.type === item.type &&
        n.title === item.title &&
        Math.abs(n.timestamp - now) < 5
    );
    if (isDuplicate) return null;

    const newItem: NotificationItem = {
      ...item,
      id: crypto.randomUUID(),
      timestamp: now,
      read: false,
    };
    NOTIFICATIONS_STORE.update((list) => [newItem, ...list].slice(0, 100));
    return newItem.id;
  },

  markAsRead(id: string) {
    NOTIFICATIONS_STORE.update((list) =>
      list.map((item) => (item.id === id ? { ...item, read: true } : item))
    );
  },

  markAllAsRead() {
    NOTIFICATIONS_STORE.update((list) => list.map((item) => ({ ...item, read: true })));
  },

  clearAll() {
    NOTIFICATIONS_STORE.set([]);
  },

  remove(id: string) {
    NOTIFICATIONS_STORE.update((list) => list.filter((item) => item.id !== id));
  },
};

export const toastStore = {
  subscribe: TOASTS_STORE.subscribe,
  success: (msg: string, dur?: number, act?: ToastAction) => addToast('success', msg, dur, act),
  error: (msg: string, dur?: number, act?: ToastAction) => addToast('error', msg, dur, act),
  warning: (msg: string, dur?: number, act?: ToastAction) => addToast('warning', msg, dur, act),
  warn: (msg: string, dur?: number, act?: ToastAction) => addToast('warning', msg, dur, act),
  info: (msg: string, dur?: number, act?: ToastAction) => addToast('info', msg, dur, act),
  remove: removeToast,

  /**
   * Handle incoming message notification: plays sound & emits desktop notification & records to history
   */
  async notifyMessage(params: MessageNotificationParams) {
    const settings = await getOrFetchSettings();
    const soundMasterEnabled = settings?.notificationSound ?? true;
    const desktopEnabled = settings?.desktopNotifications ?? true;
    const mentionOnly = settings?.mentionOnly ?? false;
    const preview: NotificationPreview = settings?.notificationPreview ?? 'full';
    const isDnd = isDndActive(settings);

    const isMention = !!params.isMention;
    const isDm = !!params.isDm;

    // Check specific sound categories
    const soundAllowed = isMention
      ? (settings?.soundMentions ?? true)
      : (settings?.soundMessages ?? true);

    // 1. Audio cue (if not DND and sound enabled)
    if (soundMasterEnabled && soundAllowed && !isDnd) {
      if (isMention || isDm) {
        playMentionSound(computeVolume(settings, 0.5));
      } else {
        playMessageSound(computeVolume(settings, 0.4));
      }
    }

    // 2. Add to Notification Center history
    const historyTitle = isMention
      ? `📢 @${params.senderName} senden bahsetti`
      : isDm
      ? `💬 ${params.senderName}`
      : `#${params.channelName || 'kanal'} · ${params.senderName}`;

    notificationStore.add({
      type: isMention ? 'mention' : 'message',
      title: historyTitle,
      body: params.content || 'Yeni bir ek veya mesaj paylaştı.',
      avatarHash: params.avatarHash,
      username: params.username || params.senderName,
      channelId: params.channelId,
      spaceId: params.spaceId,
    });

    // 3. Desktop notification
    if (desktopEnabled && !isDnd && !isStreamerPopupSuppressed()) {
      if (mentionOnly && !isMention && !isDm) {
        return;
      }

      const hasPerm = await checkDesktopPermission();
      if (!hasPerm) return;

      let title = isDm
        ? `Direkt Mesaj — ${params.senderName}`
        : params.channelName
        ? `#${params.channelName} — ${params.senderName}`
        : params.senderName;

      if (isMention) {
        title = `📢 Bahsedildin! (${title})`;
      }

      let body = '';
      if (preview === 'full') {
        body = params.content || 'Yeni bir ek veya mesaj paylaştı.';
      } else if (preview === 'sender') {
        body = `${params.senderName} sana bir mesaj gönderdi.`;
      } else {
        body = 'Yeni mesajınız var.';
      }

      try {
        sendNotification({
          title,
          body,
        });
      } catch {
        // Desktop notification is best-effort
      }
    }
  },

  /**
   * Handle incoming friend request notification
   */
  async notifyFriendRequest(params: FriendRequestNotificationParams) {
    const settings = await getOrFetchSettings();
    const soundMasterEnabled = settings?.notificationSound ?? true;
    const friendSoundEnabled = settings?.soundFriends ?? true;
    const desktopEnabled = settings?.desktopNotifications ?? true;
    const isDnd = isDndActive(settings);

    if (soundMasterEnabled && friendSoundEnabled && !isDnd) {
      playFriendRequestSound(computeVolume(settings, 0.45));
    }

    const name = params.displayName || params.username;
    addToast('info', `@${params.username} sana arkadaşlık isteği gönderdi! 🎉`, 6000);

    notificationStore.add({
      type: 'friend_request',
      title: 'Arkadaşlık İsteği',
      body: `${name} (@${params.username}) sana arkadaşlık isteği gönderdi.`,
      avatarHash: params.avatarHash,
      username: params.username,
      actionLabel: 'Görüntüle',
    });

    if (desktopEnabled && !isDnd) {
      const hasPerm = await checkDesktopPermission();
      if (hasPerm) {
        try {
          sendNotification({
            title: 'Yeni Arkadaşlık İsteği',
            body: `${name} (@${params.username}) sana arkadaşlık isteği gönderdi.`,
          });
        } catch {
          // best-effort
        }
      }
    }
  },

  /**
   * Handle incoming space/community invite notification
   */
  async notifySpaceInvite(params: SpaceInviteNotificationParams) {
    const settings = await getOrFetchSettings();
    const soundMasterEnabled = settings?.notificationSound ?? true;
    const isDnd = isDndActive(settings);

    if (soundMasterEnabled && !isDnd) {
      playMentionSound(computeVolume(settings, 0.4));
    }

    const title = params.inviterName
      ? `${params.inviterName} seni davet etti`
      : 'Topluluk Daveti';

    addToast('info', `"${params.spaceName}" topluluğuna davet edildin! 🚀`, 6000);

    notificationStore.add({
      type: 'space_invite',
      title,
      body: `"${params.spaceName}" topluluğuna katılmak için davet edildin.`,
      spaceId: params.spaceId,
      actionLabel: 'Katıl',
      data: { inviteCode: params.inviteCode },
    });
  },

  /**
   * Sound cues for voice call actions
   */
  async notifyCallJoin() {
    const settings = await getOrFetchSettings();
    const soundMasterEnabled = settings?.notificationSound ?? true;
    const callSoundEnabled = settings?.soundCalls ?? true;
    if (soundMasterEnabled && callSoundEnabled) {
      playCallJoinSound(computeVolume(settings, 0.35));
    }
  },

  async notifyCallLeave() {
    const settings = await getOrFetchSettings();
    const soundMasterEnabled = settings?.notificationSound ?? true;
    const callSoundEnabled = settings?.soundCalls ?? true;
    if (soundMasterEnabled && callSoundEnabled) {
      playCallLeaveSound(computeVolume(settings, 0.3));
    }
  },

  /**
   * System alert notification
   */
  notifySystem(title: string, body: string) {
    addToast('info', `${title}: ${body}`, 5000);
    notificationStore.add({
      type: 'system',
      title,
      body,
    });
  },
};

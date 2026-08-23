/**
 * Message store — manages channel message state
 */
import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import { toastStore } from './notifications';
import { spaceStore } from './spaces';
import { uiStore } from './ui';
import { authStore } from './auth';

export interface Message {
  id: string;
  channelId: string;
  senderId: string;
  content: string | null;
  messageType: string;
  status: 'sending' | 'sent' | 'delivered' | 'read' | 'failed' | 'queued';
  replyToId: string | null;
  pinned: boolean;
  reactions: Array<{ emoji: string; userIds: string[]; count: number }>;
  attachments: unknown[];
  editedAt: number | null;
  createdAt: number;
  deletedAt?: number | null;
  disappearsAt: number | null;
  // Local ephemeral state
  isOwn?: boolean;
  senderName?: string;
  senderAvatarHash?: string | null;
  senderRoleColor?: string | null;
}

export interface TypingUser {
  userId: string;
  name: string;
}

export interface MessageState {
  byChannel: Record<string, Message[]>;
  loading: Record<string, boolean>;
  hasMore: Record<string, boolean>;
  typingUsers: Record<string, TypingUser[]>;
}

const CACHE_KEY_PREFIX = 'veilanon_msg_cache_';
const MAX_CACHED_MESSAGES_PER_CHANNEL = 100;

function saveChannelCache(channelId: string, msgs: Message[]) {
  if (typeof window === 'undefined' || !channelId) return;
  try {
    const toSave = msgs.slice(-MAX_CACHED_MESSAGES_PER_CHANNEL);
    localStorage.setItem(`${CACHE_KEY_PREFIX}${channelId}`, JSON.stringify(toSave));
  } catch { /* quota full or private mode */ }
}

function loadChannelCache(channelId: string): Message[] {
  if (typeof window === 'undefined' || !channelId) return [];
  try {
    const raw = localStorage.getItem(`${CACHE_KEY_PREFIX}${channelId}`);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed;
    }
  } catch { /* ignore */ }
  return [];
}

function createMessageStore() {
  const { subscribe, update } = writable<MessageState>({
    byChannel: {},
    loading: {},
    hasMore: {},
    typingUsers: {},
  });

  let realtimeUnlisten: (() => void) | null = null;
  let broadcastUnlisten: (() => void) | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let dmSyncTimer: ReturnType<typeof setInterval> | null = null;
  const typingTimers: Record<string, ReturnType<typeof setTimeout>> = {};

  function handleBroadcast(payload: Record<string, unknown>) {
    const actual = ((payload?.payload || payload) as Record<string, unknown>) || {};
    if (actual.type !== 'typing') return;
    const channelId = actual.channel_id as string;
    const userId = actual.user_id as string;
    const isTyping = actual.is_typing as boolean;
    let name = (actual.display_name as string) || (actual.username as string) || '';
    if (!channelId || !userId) return;

    // Filter out own typing signal
    try {
      const auth = get(authStore);
      if (auth.identity?.id === userId) return;
    } catch { /* ignored */ }

    if (!name || (name.length === 36 && name.includes('-'))) {
      try {
        const spaces = get(spaceStore);
        const foundDm = spaces.dmChannels.find((d) => d.peerId === userId || d.id === channelId);
        if (foundDm?.name && !foundDm.name.includes('-')) {
          name = foundDm.name;
        }
      } catch { /* ignored */ }
    }
    if (!name) name = 'Kullanıcı';

    const timerKey = `${channelId}:${userId}`;
    const state = get({ subscribe });
    const current = state.typingUsers[channelId] ?? [];
    const without = current.filter((u: TypingUser) => u.userId !== userId);
    const next = isTyping ? [...without, { userId, name }] : without;
    update(s => ({
      ...s,
      typingUsers: { ...s.typingUsers, [channelId]: next },
    }));

    clearTimeout(typingTimers[timerKey]);
    if (isTyping) {
      typingTimers[timerKey] = setTimeout(() => {
        update(s => ({
          ...s,
          typingUsers: {
            ...s.typingUsers,
            [channelId]: (s.typingUsers[channelId] ?? []).filter((u: TypingUser) => u.userId !== userId),
          },
        }));
        delete typingTimers[timerKey];
      }, 4000);
    } else {
      delete typingTimers[timerKey];
    }
  }

  return {
    subscribe,

    /** Wire the Supabase realtime bridge + polling fallback (Tauri only).
     *  `activeChannel` returns the currently open channel (if any). */
    async initRealtime(activeChannel: () => string | null) {
      if (realtimeUnlisten || !('__TAURI_INTERNALS__' in window)) return;
      try {
        realtimeUnlisten = await listen<{ channel_id?: string; id?: string }>(
          'veilanon:realtime-message',
          (event) => {
            const channelId = event.payload?.channel_id;
            if (!channelId) return;
            void this.syncChannel(channelId);
          }
        );
      } catch {
        // Realtime unavailable (browser preview / no backend) — degrade to polling.
      }
      try {
        broadcastUnlisten = await listen<Record<string, unknown>>(
          'veilanon:broadcast',
          (event) => handleBroadcast(event.payload)
        );
      } catch {
        // Broadcasts are ephemeral; skipping them is safe.
      }
      if (!pollTimer) {
        pollTimer = setInterval(() => {
          try {
            const channelId = activeChannel();
            if (channelId) void this.syncChannel(channelId);
          } catch {
            // activeChannel() threw — safe to ignore, next tick retries.
          }
        }, 5000);
      }
      if (!dmSyncTimer) {
        dmSyncTimer = setInterval(() => {
          try {
            spaceStore.refreshDms();
          } catch {
            // DM refresh failed — next tick retries.
          }
        }, 8000);
      }
    },

    /** Merge remote rows for a channel into the local store. */
    async syncChannel(channelId: string) {
      try {
        const fresh = await invoke<Message[]>('sync_messages', { channelId });
        if (!fresh.length) return;
        const state = get({ subscribe });
        const existing = state.byChannel[channelId] ?? [];
        const known = new Map<string, Message>(existing.map((m: Message) => [m.id, m]));

        let currentUserId: string | null = null;
        let currentUsername: string | null = null;
        try {
          const auth = get(authStore);
          currentUserId = auth.identity?.id ?? null;
          currentUsername = auth.identity?.username?.toLowerCase() ?? null;
        } catch { /* ignored */ }

        let hasChanges = false;
        const merged = [...existing];
        const incomingForNotify: Message[] = [];

        for (const m of fresh) {
          const prev = known.get(m.id);
          if (!prev) {
            // Check if there is an optimistic sending message with the same content and sender from the last 15 seconds
            const optIdx = merged.findIndex(
              (x) =>
                x.status === 'sending' &&
                x.senderId === (currentUserId || 'self') &&
                x.content === m.content &&
                Math.abs((x.createdAt || 0) - (m.createdAt || 0)) < 15
            );
            if (optIdx !== -1) {
              // Reconcile optimistic item with confirmed remote message
              merged[optIdx] = {
                ...m,
                attachments: (m.attachments && m.attachments.length > 0) ? m.attachments : merged[optIdx].attachments,
                isOwn: true,
              };
              known.set(m.id, merged[optIdx]);
              hasChanges = true;
            } else {
              merged.push(m);
              known.set(m.id, m);
              hasChanges = true;
              if (m.senderId !== 'self' && (!currentUserId || m.senderId !== currentUserId)) {
                incomingForNotify.push(m);
              }
            }
          } else {
            const hasContentUpdate = m.content !== null && m.content !== undefined && m.content !== prev.content;
            const hasOtherUpdate =
              prev.reactions?.length !== m.reactions?.length ||
              prev.pinned !== m.pinned ||
              prev.status !== m.status ||
              prev.editedAt !== m.editedAt ||
              prev.deletedAt !== m.deletedAt;

            if (hasContentUpdate || hasOtherUpdate) {
              const idx = merged.findIndex(x => x.id === m.id);
              if (idx !== -1) {
                merged[idx] = {
                  ...prev,
                  ...m,
                  // Preserve existing decrypted content if remote row returned null
                  content: (m.content !== null && m.content !== undefined) ? m.content : prev.content,
                  attachments: (m.attachments && m.attachments.length > 0) ? m.attachments : prev.attachments,
                  isOwn: prev.isOwn ?? (m.senderId === currentUserId || m.senderId === 'self'),
                };
                hasChanges = true;
              }
            }
          }
        }

        if (!hasChanges) return;

        merged.sort((a, b) => (a.createdAt || 0) - (b.createdAt || 0));

        saveChannelCache(channelId, merged);

        update(s => ({
          ...s,
          byChannel: {
            ...s.byChannel,
            [channelId]: merged,
          },
        }));

        // Notification and unread badge dispatch
        const ui = get(uiStore);
        const isCurrentActiveChannel = (ui.activeChannelId === channelId || ui.activeDmId === channelId);
        const isDocumentFocused = typeof document !== 'undefined' ? document.hasFocus() : true;

        const spaces = get(spaceStore);
        let channelName = '';
        let isDm = false;
        const foundDm = spaces.dmChannels.find(d => d.id === channelId);
        if (foundDm) {
          isDm = true;
          channelName = foundDm.name;
        } else {
          for (const spaceId in spaces.channelsBySpace) {
            const ch = spaces.channelsBySpace[spaceId].find(c => c.id === channelId);
            if (ch) {
              channelName = ch.name;
              break;
            }
          }
        }
        if (!channelName) {
          void spaceStore.refreshDms();
        }

        for (const msg of incomingForNotify) {
          const text = msg.content ?? '';
          const isMention = currentUsername
            ? text.toLowerCase().includes(`@${currentUsername}`) || text.includes('@everyone') || text.includes('@here')
            : text.includes('@everyone') || text.includes('@here');

          if (!isCurrentActiveChannel) {
            spaceStore.incrementUnread(channelId, isMention);
          }

          void toastStore.notifyMessage({
            senderName: msg.senderName || 'Kullanıcı',
            content: text,
            channelName,
            isMention,
            isDm,
            channelId,
          });
        }
      } catch {
        // sync failed, next interval will retry
      }
    },

    async sendMessage(channelId: string, content: string, replyToId?: string, attachments: Message['attachments'] = []) {
      // Optimistic update
      let currentUser: { id: string; displayName: string; username: string; avatarHash: string | null } | null = null;
      try {
        const { authStore } = await import('$lib/stores/auth');
        const auth = get(authStore);
        if (auth.identity) {
          currentUser = {
            id: auth.identity.id,
            displayName: auth.identity.displayName,
            username: auth.identity.username,
            avatarHash: auth.identity.avatarHash ?? null,
          };
        }
      } catch { /* ignored */ }

      const tempId = crypto.randomUUID();
      const tempMsg: Message = {
        id: tempId,
        channelId,
        senderId: currentUser?.id ?? 'self',
        senderName: currentUser?.displayName || currentUser?.username || 'Sen',
        senderAvatarHash: currentUser?.avatarHash ?? null,
        content,
        messageType: 'text',
        status: 'sending',
        replyToId: replyToId ?? null,
        pinned: false,
        reactions: [],
        attachments,
        editedAt: null,
        createdAt: Date.now() / 1000,
        disappearsAt: null,
        isOwn: true,
      };

      update(s => {
        const nextList = [...(s.byChannel[channelId] ?? []), tempMsg];
        saveChannelCache(channelId, nextList);
        return {
          ...s,
          byChannel: {
            ...s.byChannel,
            [channelId]: nextList,
          },
        };
      });

      try {
        const sent = await invoke<Message>('send_message', {
          input: { channelId, content, replyToId, attachments }
        });

        // Replace temp with confirmed
        update(s => {
          const list = s.byChannel[channelId] ?? [];
          const alreadyHasSent = list.some((m: Message) => m.id === sent.id);
          let nextList: Message[];
          if (alreadyHasSent) {
            // Already reconciled by syncChannel — just remove temp message and ensure attachments
            nextList = list
              .filter((m: Message) => m.id !== tempId)
              .map((m: Message) => m.id === sent.id ? {
                ...m,
                ...sent,
                attachments: (sent.attachments && sent.attachments.length > 0) ? sent.attachments : attachments,
                isOwn: true,
              } : m);
          } else {
            nextList = list.map((m: Message) =>
              m.id === tempId ? {
                ...sent,
                attachments: (sent.attachments && sent.attachments.length > 0) ? sent.attachments : attachments,
                isOwn: true
              } : m
            );
          }
          saveChannelCache(channelId, nextList);
          return {
            ...s,
            byChannel: {
              ...s.byChannel,
              [channelId]: nextList,
            },
          };
        });
      } catch (err) {
        // Mark as failed
        update(s => {
          const nextList: Message[] = (s.byChannel[channelId] ?? []).map((m: Message) =>
            m.id === tempId ? { ...m, status: 'failed' as const } : m
          );
          saveChannelCache(channelId, nextList);
          return {
            ...s,
            byChannel: {
              ...s.byChannel,
              [channelId]: nextList,
            },
          };
        });
        throw err;
      }
    },

    async loadMessages(channelId: string, beforeId?: string) {
      // Optimistic cache load: if channel is empty in memory, immediately load from local cache
      const curr = get({ subscribe });
      if (!beforeId && (!curr.byChannel[channelId] || curr.byChannel[channelId].length === 0)) {
        const cached = loadChannelCache(channelId);
        if (cached.length > 0) {
          update(s => ({
            ...s,
            byChannel: {
              ...s.byChannel,
              [channelId]: cached,
            },
          }));
        }
      }

      update(s => ({ ...s, loading: { ...s.loading, [channelId]: true } }));
      try {
        const messages = await invoke<Message[]>('load_messages', {
          channelId,
          beforeId,
          limit: 50,
        });

        update(s => {
          const currentList = s.byChannel[channelId] ?? [];
          const sendingMsgs = currentList.filter(m => m.status === 'sending' || m.status === 'queued');
          
          let finalMessages: Message[];
          if (beforeId) {
            finalMessages = [...messages, ...currentList];
          } else if (messages.length > 0) {
            // Keep any in-flight sending messages that haven't confirmed yet
            const confirmedIds = new Set(messages.map(m => m.id));
            const stillSending = sendingMsgs.filter(m => !confirmedIds.has(m.id));
            finalMessages = [...messages, ...stillSending];
          } else {
            finalMessages = currentList.length > 0 ? currentList : messages;
          }
          
          if (finalMessages.length > 0) {
            saveChannelCache(channelId, finalMessages);
          }

          return {
            ...s,
            byChannel: {
              ...s.byChannel,
              [channelId]: finalMessages,
            },
            hasMore: { ...s.hasMore, [channelId]: messages.length === 50 },
            loading: { ...s.loading, [channelId]: false },
          };
        });

        // Trigger background sync to guarantee latest messages from Supabase
        if (!beforeId) {
          void this.syncChannel(channelId);
        }
      } catch {
        update(s => ({ ...s, loading: { ...s.loading, [channelId]: false } }));
      }
    },

    appendMessage(channelId: string, message: Message) {
      update(s => {
        const nextList = [...(s.byChannel[channelId] ?? []), message];
        saveChannelCache(channelId, nextList);
        return {
          ...s,
          byChannel: {
            ...s.byChannel,
            [channelId]: nextList,
          },
        };
      });
    },

    setTyping(channelId: string, users: Array<TypingUser | string>) {
      const typingList: TypingUser[] = users.map((u) =>
        typeof u === 'string' ? { userId: u, name: u } : u
      );
      update((s) => ({
        ...s,
        typingUsers: { ...s.typingUsers, [channelId]: typingList },
      }));
    },

    async deleteMessage(channelId: string, messageId: string) {
      await invoke('delete_message', { messageId });
      update(s => {
        const nextList = (s.byChannel[channelId] ?? []).filter((m: Message) => m.id !== messageId);
        saveChannelCache(channelId, nextList);
        return {
          ...s,
          byChannel: {
            ...s.byChannel,
            [channelId]: nextList,
          },
        };
      });
    },

    async clearChannel(channelId: string) {
      await invoke('clear_channel_messages', { channelId });
      saveChannelCache(channelId, []);
      update(s => ({
        ...s,
        byChannel: {
          ...s.byChannel,
          [channelId]: [],
        },
      }));
    },

    /** Local patch after edit/pin — server state already updated via API. */
    patchMessage(channelId: string, messageId: string, patch: Partial<Message>) {
      update(s => ({
        ...s,
        byChannel: {
          ...s.byChannel,
          [channelId]: (s.byChannel[channelId] ?? []).map((m: Message) =>
            m.id === messageId ? { ...m, ...patch } : m
          ),
        },
      }));
    },

    /** Local reaction update (server round-trip already done). */
    patchReaction(channelId: string, messageId: string, emoji: string, add: boolean) {
      update(s => ({
        ...s,
        byChannel: {
          ...s.byChannel,
          [channelId]: (s.byChannel[channelId] ?? []).map((m: Message) => {
            if (m.id !== messageId) return m;
            const reactions = [...m.reactions];
            const idx = reactions.findIndex(r => r.emoji === emoji);
            if (add) {
              if (idx >= 0) {
                const r = reactions[idx];
                if (!r.userIds.includes('self')) r.userIds.push('self');
                r.count = r.userIds.length;
              } else {
                reactions.push({ emoji, userIds: ['self'], count: 1 });
              }
            } else if (idx >= 0) {
              const r = reactions[idx];
              r.userIds = r.userIds.filter((u: string) => u !== 'self');
              r.count = r.userIds.length;
              if (r.userIds.length === 0) reactions.splice(idx, 1);
            }
            return { ...m, reactions };
          }),
        },
      }));
    },

    getForChannel: (channelId: string) =>
      derived({ subscribe }, s => s.byChannel[channelId] ?? []),

    reset() {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
      if (dmSyncTimer) {
        clearInterval(dmSyncTimer);
        dmSyncTimer = null;
      }
      if (typeof window !== 'undefined') {
        try {
          const keysToRemove: string[] = [];
          for (let i = 0; i < localStorage.length; i++) {
            const k = localStorage.key(i);
            if (k && k.startsWith(CACHE_KEY_PREFIX)) {
              keysToRemove.push(k);
            }
          }
          for (const k of keysToRemove) {
            localStorage.removeItem(k);
          }
        } catch { /* ignore */ }
      }
      update(() => ({
        byChannel: {},
        loading: {},
        hasMore: {},
        typingUsers: {},
      }));
    },
  };
}

export const messageStore = createMessageStore();

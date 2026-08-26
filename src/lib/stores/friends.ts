import { writable, get } from 'svelte/store';
import { friendApi, type FriendInfo, type FriendStatus } from '$lib/api/tauri';
import { toastStore } from './notifications';

interface FriendsState {
  friends: FriendInfo[];
  loading: boolean;
  error: string | null;
}

function createFriendsStore() {
  const { subscribe, update, set } = writable<FriendsState>({
    friends: [],
    loading: false,
    error: null,
  });

  const knownIncomingIds = new Set<string>();
  let hasInitialized = false;
  let loadTimer: ReturnType<typeof setTimeout> | null = null;

  function removeFriend(userId: string) {
    update(s => ({ ...s, friends: s.friends.filter(f => f.userId !== userId) }));
    knownIncomingIds.delete(userId);
  }

  async function doLoad() {
    const state = get({ subscribe });
    const hadData = state.friends.length > 0;
    if (!hadData && !state.error) update(s => ({ ...s, loading: true, error: null }));
    try {
      const friends = await Promise.race([
        friendApi.list(),
        new Promise<never>((_, rej) => setTimeout(() => rej(new Error('timeout')), 12000))
      ]) as FriendInfo[];
      if (hasInitialized) {
        for (const f of friends) {
          if (f.status === 'pending_incoming' && !knownIncomingIds.has(f.userId)) {
            knownIncomingIds.add(f.userId);
            void toastStore.notifyFriendRequest({ username: f.username, displayName: f.displayName });
          }
        }
      } else {
        for (const f of friends) if (f.status === 'pending_incoming') knownIncomingIds.add(f.userId);
        hasInitialized = true;
      }
      set({ friends, loading: false, error: null });
    } catch (err) {
      const msg = String(err);
      if (msg.includes('timeout')) update(s => ({ ...s, loading: false, error: 'Zaman aşımı, yeniden deneniyor...' }));
      else update(s => ({ ...s, loading: false, error: msg }));
    }
  }

  let presencePoll: ReturnType<typeof setInterval> | null = null;
  let loadVersion = 0;
  let pendingReload = false;
  let isReloading = false;

  async function safeDoLoad() {
    if (isReloading) {
      pendingReload = true;
      return;
    }
    isReloading = true;
    const v = ++loadVersion;
    try {
      await doLoad();
    } finally {
      isReloading = false;
    }
    if (pendingReload && v === loadVersion) {
      pendingReload = false;
      isReloading = true;
      const v2 = ++loadVersion;
      try {
        await doLoad();
      } finally {
        isReloading = false;
      }
      void v2;
    } else if (pendingReload && v !== loadVersion) {
      pendingReload = false;
      await safeDoLoad();
    }
  }

  function startPresencePoll() {
    if (presencePoll) return;
    if (typeof window === 'undefined') return;
    presencePoll = setInterval(() => { void safeDoLoad(); }, 15000);
    window.addEventListener('focus', () => { void safeDoLoad(); });
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') void safeDoLoad();
    });
  }

  function stopPresencePoll() {
    if (presencePoll) { clearInterval(presencePoll); presencePoll = null; }
  }
  if (typeof window !== 'undefined') startPresencePoll();

  return {
    subscribe,

    async load(): Promise<FriendInfo[]> {
      if (loadTimer !== null) {
        clearTimeout(loadTimer);
        loadTimer = null;
      }
      await safeDoLoad();
      startPresencePoll();
      return get({ subscribe }).friends;
    },

    async add(username: string): Promise<void> {
      let clean = username.trim();
      if (clean.includes('/u/')) {
        clean = clean.split('/u/')[1] || clean;
      } else if (clean.includes('/user/')) {
        clean = clean.split('/user/')[1] || clean;
      }
      clean = clean.split('?')[0].split('#')[0].split('/')[0].trim().replace(/^@/, '').trim();
      if (!clean) throw new Error('Geçerli bir kullanıcı adı girin');
      await friendApi.add({ username: clean });
      await this.load();
    },

    async accept(userId: string): Promise<void> {
      await friendApi.accept(userId);
      await this.load();
    },

    async reject(userId: string): Promise<void> {
      await friendApi.reject(userId);
      removeFriend(userId);
    },

    async cancel(userId: string): Promise<void> {
      await friendApi.cancel(userId);
      removeFriend(userId);
    },

    async remove(userId: string): Promise<void> {
      await friendApi.remove(userId);
      removeFriend(userId);
    },

    async block(userId: string) {
      await friendApi.block(userId);
      removeFriend(userId);
    },

    async unblock(userId: string) {
      await friendApi.unblock(userId);
      await this.load();
    },

    setStatus(userId: string, status: FriendStatus) {
      update(s => ({
        ...s,
        friends: s.friends.map(f => (f.userId === userId ? { ...f, status } : f)),
      }));
    },

    reset() {
      if (loadTimer !== null) {
        clearTimeout(loadTimer);
        loadTimer = null;
      }
      stopPresencePoll();
      knownIncomingIds.clear();
      hasInitialized = false;
      set({ friends: [], loading: false, error: null });
    },
  };
}

export const friendsStore = createFriendsStore();

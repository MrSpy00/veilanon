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
    update(s => ({ ...s, loading: true, error: null }));
    try {
      const friends = await friendApi.list();
      
      // Detect new incoming requests after initialization
      if (hasInitialized) {
        for (const f of friends) {
          if (f.status === 'pending_incoming' && !knownIncomingIds.has(f.userId)) {
            knownIncomingIds.add(f.userId);
            void toastStore.notifyFriendRequest({
              username: f.username,
              displayName: f.displayName,
            });
          }
        }
      } else {
        for (const f of friends) {
          if (f.status === 'pending_incoming') {
            knownIncomingIds.add(f.userId);
          }
        }
        hasInitialized = true;
      }

      set({ friends, loading: false, error: null });
    } catch (err) {
      update(s => ({ ...s, loading: false, error: String(err) }));
    }
  }

  return {
    subscribe,

    load() {
      if (loadTimer !== null) {
        clearTimeout(loadTimer);
      }
      loadTimer = setTimeout(() => {
        loadTimer = null;
        doLoad();
      }, 100);
    },

    async add(username: string): Promise<void> {
      await friendApi.add({ username });
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
      knownIncomingIds.clear();
      hasInitialized = false;
      set({ friends: [], loading: false, error: null });
    },
  };
}

export const friendsStore = createFriendsStore();

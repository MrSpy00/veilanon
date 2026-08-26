import { writable, get } from 'svelte/store';
import { identityApi } from '$lib/api/tauri';
import { authStore } from '$lib/stores/auth';
import { cacheAvatar } from '$lib/components/ui/Avatar.svelte';
import { cacheBanner } from '$lib/components/ui/BannerImage.svelte';
import { toastStore } from '$lib/stores/notifications';

export interface PlaylistItem {
  id: string;
  url: string;
  dataUrl?: string;
  name: string;
  createdAt: number;
}

export interface PlaylistConfig {
  enabled: boolean;
  intervalSeconds: number;
  mode: 'sequential' | 'shuffle';
  currentIndex: number;
}

export interface ProfilePlaylistState {
  avatarItems: PlaylistItem[];
  bannerItems: PlaylistItem[];
  avatarConfig: PlaylistConfig;
  bannerConfig: PlaylistConfig;
}

const STORAGE_KEY = 'veil_profile_playlist';

const DEFAULT_CONFIG: PlaylistConfig = {
  enabled: false,
  intervalSeconds: 60,
  mode: 'sequential',
  currentIndex: 0,
};

const DEFAULT_STATE: ProfilePlaylistState = {
  avatarItems: [],
  bannerItems: [],
  avatarConfig: { ...DEFAULT_CONFIG },
  bannerConfig: { ...DEFAULT_CONFIG },
};

function loadState(): ProfilePlaylistState {
  if (typeof window === 'undefined') return DEFAULT_STATE;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_STATE;
    const parsed = JSON.parse(raw);
    return {
      avatarItems: Array.isArray(parsed.avatarItems) ? parsed.avatarItems : [],
      bannerItems: Array.isArray(parsed.bannerItems) ? parsed.bannerItems : [],
      avatarConfig: { ...DEFAULT_CONFIG, ...(parsed.avatarConfig || {}) },
      bannerConfig: { ...DEFAULT_CONFIG, ...(parsed.bannerConfig || {}) },
    };
  } catch {
    return DEFAULT_STATE;
  }
}

function saveState(state: ProfilePlaylistState) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // best effort
  }
}

function createProfilePlaylistStore() {
  const { subscribe, update, set } = writable<ProfilePlaylistState>(loadState());

  let lastAvatarSwitch = Date.now();
  let lastBannerSwitch = Date.now();

  async function applyMediaDirectly(type: 'avatar' | 'banner', mediaData: string) {
    try {
      if (type === 'avatar') {
        const hash = await identityApi.setAvatar(mediaData);
        if (hash) {
          cacheAvatar(hash, mediaData);
          authStore.updateIdentity({ avatarHash: hash });
        }
      } else {
        const hash = await identityApi.setBanner(mediaData);
        if (hash) {
          cacheBanner(hash, mediaData);
          authStore.updateIdentity({ bannerHash: hash });
        }
      }
    } catch (err) {
      console.warn(`Failed to auto-rotate ${type}:`, err);
    }
  }

  function advancePlaylist(type: 'avatar' | 'banner') {
    const st = get({ subscribe });
    const items = type === 'avatar' ? st.avatarItems : st.bannerItems;
    const cfg = type === 'avatar' ? st.avatarConfig : st.bannerConfig;

    if (!cfg.enabled || items.length < 2) return;

    let nextIdx = cfg.currentIndex;
    if (cfg.mode === 'shuffle') {
      let rand = Math.floor(Math.random() * items.length);
      if (rand === nextIdx && items.length > 1) {
        rand = (rand + 1) % items.length;
      }
      nextIdx = rand;
    } else {
      nextIdx = (nextIdx + 1) % items.length;
    }

    const nextItem = items[nextIdx];
    if (!nextItem) return;

    update(state => {
      const updated = {
        ...state,
        [type === 'avatar' ? 'avatarConfig' : 'bannerConfig']: {
          ...cfg,
          currentIndex: nextIdx,
        },
      };
      saveState(updated);
      return updated;
    });

    const mediaData = nextItem.dataUrl || nextItem.url;
    void applyMediaDirectly(type, mediaData);
  }

  function checkTimers() {
    const now = Date.now();
    const st = get({ subscribe });

    if (st.avatarConfig.enabled && st.avatarItems.length >= 2) {
      const intervalMs = Math.max(5, st.avatarConfig.intervalSeconds) * 1000;
      if (now - lastAvatarSwitch >= intervalMs) {
        lastAvatarSwitch = now;
        advancePlaylist('avatar');
      }
    }

    if (st.bannerConfig.enabled && st.bannerItems.length >= 2) {
      const intervalMs = Math.max(5, st.bannerConfig.intervalSeconds) * 1000;
      if (now - lastBannerSwitch >= intervalMs) {
        lastBannerSwitch = now;
        advancePlaylist('banner');
      }
    }
  }

  // Global background ticker
  if (typeof window !== 'undefined') {
    setInterval(checkTimers, 2000);
  }

  return {
    subscribe,

    addItem(type: 'avatar' | 'banner', item: { url: string; dataUrl?: string; name?: string }) {
      const newItem: PlaylistItem = {
        id: crypto.randomUUID(),
        url: item.url,
        dataUrl: item.dataUrl,
        name: item.name || (type === 'avatar' ? 'Avatar Görseli' : 'Banner Görseli'),
        createdAt: Date.now(),
      };

      update(state => {
        const key = type === 'avatar' ? 'avatarItems' : 'bannerItems';
        const list = [...state[key], newItem];
        const updated = { ...state, [key]: list };
        saveState(updated);
        return updated;
      });

      toastStore.success(`${type === 'avatar' ? 'Avatar' : 'Banner'} listesine eklendi.`);
      return newItem;
    },

    removeItem(type: 'avatar' | 'banner', id: string) {
      update(state => {
        const key = type === 'avatar' ? 'avatarItems' : 'bannerItems';
        const list = state[key].filter(x => x.id !== id);
        const updated = { ...state, [key]: list };
        saveState(updated);
        return updated;
      });
    },

    updateConfig(type: 'avatar' | 'banner', config: Partial<PlaylistConfig>) {
      update(state => {
        const key = type === 'avatar' ? 'avatarConfig' : 'bannerConfig';
        const updated = {
          ...state,
          [key]: { ...state[key], ...config },
        };
        saveState(updated);
        return updated;
      });
    },

    async applyItem(type: 'avatar' | 'banner', item: PlaylistItem) {
      const mediaData = item.dataUrl || item.url;
      await applyMediaDirectly(type, mediaData);
      toastStore.success(`${type === 'avatar' ? 'Avatar' : 'Banner'} güncellendi.`);
    },

    addAvatarItem(item: { url: string; dataUrl?: string; name?: string }) {
      return this.addItem('avatar', item);
    },

    addBannerItem(item: { url: string; dataUrl?: string; name?: string }) {
      return this.addItem('banner', item);
    },

    removeAvatarItem(id: string) {
      this.removeItem('avatar', id);
    },

    removeBannerItem(id: string) {
      this.removeItem('banner', id);
    },

    updateAvatarConfig(config: Partial<PlaylistConfig>) {
      this.updateConfig('avatar', config);
    },

    updateBannerConfig(config: Partial<PlaylistConfig>) {
      this.updateConfig('banner', config);
    },

    async applyAvatarItemNow(item: PlaylistItem) {
      await this.applyItem('avatar', item);
    },

    async applyBannerItemNow(item: PlaylistItem) {
      await this.applyItem('banner', item);
    },

    exportJson(): string {
      const st = get({ subscribe });
      return JSON.stringify(st, null, 2);
    },

    importJson(jsonStr: string): boolean {
      try {
        const parsed = JSON.parse(jsonStr);
        if (typeof parsed !== 'object' || !parsed) return false;
        const validState: ProfilePlaylistState = {
          avatarItems: Array.isArray(parsed.avatarItems) ? parsed.avatarItems : [],
          bannerItems: Array.isArray(parsed.bannerItems) ? parsed.bannerItems : [],
          avatarConfig: { ...DEFAULT_CONFIG, ...(parsed.avatarConfig || {}) },
          bannerConfig: { ...DEFAULT_CONFIG, ...(parsed.bannerConfig || {}) },
        };
        set(validState);
        saveState(validState);
        toastStore.success('Oynatma listeleri başarıyla içe aktarıldı.');
        return true;
      } catch (err) {
        toastStore.error('Geçersiz JSON formatı.');
        return false;
      }
    },
  };
}

export const profilePlaylistStore = createProfilePlaylistStore();


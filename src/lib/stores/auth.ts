/**
 * veilanon auth store
 * Manages identity state without exposing private keys to the store.
 */
import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { IdentityInfo, IdentityHint } from '$lib/api/tauri';
import { spaceStore } from './spaces';
import { messageStore } from './messages';
import { friendsStore } from './friends';
import { uiStore } from './ui';

export type { IdentityInfo };

interface AuthState {
  isAuthenticated: boolean;
  identity: IdentityInfo | null;
  identityHint: IdentityHint | null;
  loading: boolean;
  error: string | null;
  dmPrivacy: 'everyone' | 'friends' | 'same_server' | 'nobody';
}

const initialState: AuthState = {
  isAuthenticated: false,
  identity: null,
  identityHint: null,
  loading: false,
  error: null,
  dmPrivacy: 'everyone',
};

function resetAllStores() {
  spaceStore.reset();
  messageStore.reset();
  friendsStore.reset();
  uiStore.navigate(null, null);
  uiStore.closeModal();
  uiStore.resetThemeToDefault();
}

function createAuthStore() {
  const { subscribe, set, update } = writable<AuthState>(initialState);

  if (typeof window !== 'undefined') {
    listen('auth:unauthenticated', () => {
      set(initialState);
      resetAllStores();
      void loadIdentityHint();
    }).catch(() => {});
  }

  async function loadIdentityHint() {
    try {
      const hint = await invoke<IdentityHint>('get_identity_hint');
      update(s => ({ ...s, identityHint: hint }));
      return hint;
    } catch {
      update(s => ({ ...s, identityHint: null }));
      return null;
    }
  }

  return {
    subscribe,

    async initialize() {
      update(s => ({ ...s, loading: true }));
      try {
        const auto = await invoke<IdentityInfo | null>('try_auto_unlock');
        if (auto) {
          uiStore.loadUserTheme(auto.id);
          update(s => ({
            ...s,
            isAuthenticated: true,
            identity: auto,
            loading: false,
            error: null,
          }));
          void this.refreshRemoteProfile();
          return;
        }
      } catch { /* proceed to manual login */ }
      resetAllStores();
      await loadIdentityHint();
      update(s => ({ ...s, loading: false }));
    },

    async getIdentityHint() {
      return loadIdentityHint();
    },

    /** Kendi profilini Supabase'ten yeniden çeker ve kimlik durumuna uygular. */
    async refreshRemoteProfile() {
      try {
        const { socialApi } = await import('$lib/api/tauri');
        const p = await socialApi.refreshProfile();
        update(s => {
          if (!s.identity) return s;
          const next = { ...s.identity };
          if (p.displayName) next.displayName = p.displayName;
          if (p.username) next.username = p.username;
          if (p.avatarHash) next.avatarHash = p.avatarHash;
          if (p.bannerHash) next.bannerHash = p.bannerHash;
          return { ...s, identity: next };
        });
      } catch { /* best-effort: yerel durum korunur */ }
    },

    async createIdentity(username: string, displayName: string, passphrase: string, rememberMe = false) {
      update(s => ({ ...s, loading: true, error: null }));
      try {
        const response = await invoke<IdentityInfo>('create_identity', {
          input: { username, displayName, passphrase }
        });
        if (rememberMe) {
          await invoke('set_auto_unlock', { enabled: true, passphrase }).catch(() => {});
        }
        uiStore.loadUserTheme(response.id);
        update(s => ({
          ...s,
          isAuthenticated: true,
          identity: response,
          loading: false,
          error: null,
        }));
        void this.refreshRemoteProfile();
        return response;
      } catch (err) {
        const error = String(err);
        update(s => ({ ...s, loading: false, error }));
        throw new Error(error);
      }
    },

    async loginWithCredentials(username: string, passphrase: string, rememberMe = false) {
      update(s => ({ ...s, loading: true, error: null }));
      try {
        const response = await invoke<IdentityInfo>('login_with_credentials', {
          input: { username, passphrase }
        });
        if (rememberMe) {
          await invoke('set_auto_unlock', { enabled: true, passphrase }).catch(() => {});
        }
        uiStore.loadUserTheme(response.id);
        update(s => ({
          ...s,
          isAuthenticated: true,
          identity: response,
          loading: false,
          error: null,
        }));
        void this.refreshRemoteProfile();
        return response;
      } catch (err) {
        const error = String(err);
        update(s => ({ ...s, loading: false, error }));
        throw new Error(error);
      }
    },

    async loadIdentity(passphrase: string, rememberMe = false) {
      update(s => ({ ...s, loading: true, error: null }));
      try {
        const response = await invoke<IdentityInfo>('load_identity', { passphrase });
        if (rememberMe) {
          await invoke('set_auto_unlock', { enabled: true, passphrase }).catch(() => {});
        }
        uiStore.loadUserTheme(response.id);
        update(s => ({
          ...s,
          isAuthenticated: true,
          identity: response,
          loading: false,
          error: null,
        }));
        return response;
      } catch (err) {
        const error = String(err);
        update(s => ({ ...s, loading: false, error }));
        throw new Error(error);
      }
    },

    async recoverIdentity(recoveryCode: string, newPassphrase: string, username?: string) {
      update(s => ({ ...s, loading: true, error: null }));
      try {
        const response = await invoke<IdentityInfo>('recover_identity', {
          recoveryCode,
          newPassphrase,
          username: username || null,
        });
        uiStore.loadUserTheme(response.id);
        update(s => ({
          ...s,
          isAuthenticated: true,
          identity: response,
          loading: false,
          error: null,
        }));
        void this.refreshRemoteProfile();
        return response;
      } catch (err) {
        const error = String(err);
        update(s => ({ ...s, loading: false, error }));
        throw new Error(error);
      }
    },

    async signOut() {
      await invoke('set_auto_unlock', { enabled: false }).catch(() => {});
      await invoke('sign_out');
      set(initialState);
      resetAllStores();
      // Re-read the hint so the onboarding screen doesn't offer "create a new
      // identity" for a device that already has one.
      await loadIdentityHint();
    },

    /**
     * Factory-reset the device: wipes the local identity, keychain and all
     * local data WITHOUT requiring the passphrase. Irreversible — the UI must
     * show a double confirmation before calling this.
     */
    async resetIdentity() {
      await invoke('reset_identity');
      set(initialState);
      resetAllStores();
      await loadIdentityHint();
    },

    updateIdentity(partial: Partial<IdentityInfo>) {
      update(s => {
        const identity = s.identity ? { ...s.identity, ...partial } : s.identity;
        const hint = s.identityHint
          ? {
              ...s.identityHint,
              ...(partial.displayName !== undefined ? { displayName: partial.displayName } : {}),
              ...(partial.username !== undefined ? { username: partial.username } : {}),
              ...(partial.avatarHash !== undefined ? { avatarHash: partial.avatarHash } : {}),
              ...(partial.bannerHash !== undefined ? { bannerHash: partial.bannerHash } : {}),
            }
          : s.identityHint;
        return { ...s, identity, identityHint: hint };
      });
    },

    setDmPrivacy(privacy: AuthState['dmPrivacy']) {
      update(s => ({ ...s, dmPrivacy: privacy }));
      try {
        localStorage.setItem('veilanon_dm_privacy', privacy);
      } catch { /* ignored */ }
      import('$lib/api/tauri').then(({ settingsApi }) =>
        settingsApi.get().then(s => settingsApi.update({ ...s, dmPrivacy: privacy }))
      ).catch(() => {});
    },

    clearError() {
      update(s => ({ ...s, error: null }));
    },
  };
}

export const authStore = createAuthStore();

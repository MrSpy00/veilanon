/**
 * veilanon — Zero-Leak Auto-Privacy & Screen Share Shield Store
 *
 * Automatically protects and censors emails, passwords, recovery keys, tokens,
 * device fingerprints, and private credentials during screen shares, broadcasts,
 * and recordings — even when Streamer Mode is off.
 *
 * DEFAULT: Enabled (autoShieldEnabled: true) for maximum privacy.
 * Users can freely toggle it OFF in Settings -> Privacy.
 */

import { writable, derived, get } from 'svelte/store';
import { mediaStore } from './media';
import { streamerMode } from './streamerMode';

export interface PrivacyShieldConfig {
  /** Master toggle: Auto-protect all sensitive data when screen sharing or recording is detected (Default: true) */
  autoShieldEnabled: boolean;
  /** Protect emails, device IDs and user identifiers (Default: true) */
  protectEmailsAndIds: boolean;
  /** Protect passwords, passphrases and recovery secrets (Default: true) */
  protectPasswordsAndKeys: boolean;
  /** Protect invite codes and vanity links (Default: true) */
  protectInvites: boolean;
  /** Protect bridge webhooks and integration tokens (Default: true) */
  protectWebhooks: boolean;
  /** Auto-hide revealed passwords and secrets after seconds (0 = disabled, default = 5s) */
  autoHideTimeoutSeconds: number;
  /** Mask style for auto-shielded secrets */
  maskStyle: 'asterisks' | 'bullets' | 'blur' | 'hidden';
  /** Blur media and files during screen share */
  blurMediaOnShare: boolean;
  /** Suppress clipboard previews containing tokens or keys */
  protectClipboard: boolean;
}

const STORAGE_KEY = 'veilanon_privacy_shield_config_v2';

const DEFAULT_CONFIG: PrivacyShieldConfig = {
  autoShieldEnabled: true,
  protectEmailsAndIds: true,
  protectPasswordsAndKeys: true,
  protectInvites: true,
  protectWebhooks: true,
  autoHideTimeoutSeconds: 5,
  maskStyle: 'bullets',
  blurMediaOnShare: true,
  protectClipboard: true,
};

function loadStoredConfig(): PrivacyShieldConfig {
  if (typeof window === 'undefined') {
    return { ...DEFAULT_CONFIG };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_CONFIG };
    return {
      ...DEFAULT_CONFIG,
      ...JSON.parse(raw),
    };
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

function saveStoredConfig(cfg: PrivacyShieldConfig) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg));
  } catch {
    // ignore
  }
}

const configStore = writable<PrivacyShieldConfig>(loadStoredConfig());

configStore.subscribe((cfg) => {
  saveStoredConfig(cfg);
});

// Temporarily revealed secrets map with active timers
export const revealedSecrets = writable<Record<string, number>>({});
const activeTimers = new Map<string, ReturnType<typeof setTimeout>>();

/**
 * Derived screen share state from media store (LiveKit calls or local share)
 */
export const isScreenShareActive = derived(mediaStore, ($media) => {
  return $media.isScreenSharing || $media.participants.some((p) => p.isScreenSharing);
});

/**
 * Derived master shield status:
 * Active if Streamer Mode is ON, OR if user has autoShieldEnabled ON and screen sharing is active.
 */
export const isShieldActive = derived(
  [configStore, isScreenShareActive, streamerMode],
  ([$cfg, $screenShare, $streamer]) => {
    return $streamer.enabled || ($cfg.autoShieldEnabled && $screenShare);
  }
);

/**
 * Reactive derived store for revealed secrets - triggers UI updates
 */
export const revealedMap = derived(revealedSecrets, ($map) => $map);

export const privacyShield = {
  subscribe: configStore.subscribe,
  isShieldActive,
  isScreenShareActive,
  revealed: { subscribe: revealedSecrets.subscribe },
  revealedMap,

  updateConfig<K extends keyof PrivacyShieldConfig>(key: K, value: PrivacyShieldConfig[K]) {
    configStore.update((c) => ({ ...c, [key]: value }));
  },

  setMasterToggle(enabled: boolean) {
    configStore.update((c) => ({ ...c, autoShieldEnabled: enabled }));
  },

  revealSecret(key: string, durationSeconds?: number) {
    const cfg = get(configStore);
    const dur = durationSeconds ?? cfg.autoHideTimeoutSeconds;
    if (dur <= 0) {
      revealedSecrets.update((map) => ({ ...map, [key]: Number.MAX_SAFE_INTEGER }));
      return;
    }

    const timeout = dur * 1000;
    const existing = activeTimers.get(key);
    if (existing) clearTimeout(existing);

    const expiry = Date.now() + timeout;
    revealedSecrets.update((map) => ({ ...map, [key]: expiry }));

    const timer = setTimeout(() => {
      privacyShield.hideSecret(key);
    }, timeout);

    activeTimers.set(key, timer);
  },

  hideSecret(key: string) {
    const existing = activeTimers.get(key);
    if (existing) {
      clearTimeout(existing);
      activeTimers.delete(key);
    }
    revealedSecrets.update((map) => {
      const next = { ...map };
      delete next[key];
      return next;
    });
  },

  toggleSecret(key: string, durationSeconds?: number) {
    const map = get(revealedSecrets);
    const now = Date.now();
    const currentValue = map[key];
    const isCurrentlyRevealed = typeof currentValue === 'number' && currentValue > now;
    
    if (isCurrentlyRevealed) {
      privacyShield.hideSecret(key);
    } else {
      privacyShield.revealSecret(key, durationSeconds);
    }
  },

  isSecretRevealed(key: string): boolean {
    const map = get(revealedSecrets);
    const currentValue = map[key];
    return typeof currentValue === 'number' && currentValue > Date.now();
  },

  formatSecret(value: string | null | undefined, key: string, forceMask = false): string {
    if (!value) return '';
    if (!forceMask && privacyShield.isSecretRevealed(key)) {
      return value;
    }

    const cfg = get(configStore);
    const shieldActive = get(isShieldActive);
    const streamer = get(streamerMode);

    if (!forceMask && !shieldActive) {
      return value;
    }

    const style = cfg.maskStyle;
    if (style === 'bullets') {
      return '•'.repeat(Math.min(Math.max(value.length, 8), 24));
    } else if (style === 'asterisks') {
      return '*'.repeat(Math.min(Math.max(value.length, 8), 24));
    } else if (style === 'hidden') {
      return '[GİZLENDİ]';
    }
    return '••••••••••••';
  },

  resetToDefaults() {
    configStore.set({ ...DEFAULT_CONFIG });
    revealedSecrets.set({});
    activeTimers.forEach((t) => clearTimeout(t));
    activeTimers.clear();
  },
};

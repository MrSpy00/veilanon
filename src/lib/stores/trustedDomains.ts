/**
 * veilanon — Trusted Domains & Safe Link Redirect Manager
 * Handles whitelist of safe domains, confirmation prompts, and preferences.
 */
import { writable, get } from 'svelte/store';

const DEFAULT_TRUSTED_DOMAINS = [
  'github.com',
  'google.com',
  'youtube.com',
  'youtu.be',
  'aegissoft.com.tr',
  'veilanon.com',
  'gitlab.com',
  'wikipedia.org',
  'twitter.com',
  'x.com',
  'discord.com',
  'tenor.com',
  'giphy.com',
];

export interface TrustedDomainsConfig {
  trustedDomains: string[];
  directRedirectForTrusted: boolean;
  alwaysOpenWithoutPrompt: boolean;
}

const STORAGE_KEY = 'veilanon_trusted_domains_config';

function loadConfig(): TrustedDomainsConfig {
  if (typeof window === 'undefined') {
    return {
      trustedDomains: DEFAULT_TRUSTED_DOMAINS,
      directRedirectForTrusted: true,
      alwaysOpenWithoutPrompt: false,
    };
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        trustedDomains: Array.isArray(parsed.trustedDomains) && parsed.trustedDomains.length > 0
          ? parsed.trustedDomains
          : DEFAULT_TRUSTED_DOMAINS,
        directRedirectForTrusted: parsed.directRedirectForTrusted ?? true,
        alwaysOpenWithoutPrompt: parsed.alwaysOpenWithoutPrompt ?? false,
      };
    }
  } catch {
    // fallback
  }
  return {
    trustedDomains: DEFAULT_TRUSTED_DOMAINS,
    directRedirectForTrusted: true,
    alwaysOpenWithoutPrompt: false,
  };
}

function saveConfig(cfg: TrustedDomainsConfig) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg));
  } catch {
    // ignored
  }
}

export function extractDomain(urlOrDomain: string): string {
  try {
    let clean = urlOrDomain.trim().toLowerCase();
    if (!clean.startsWith('http://') && !clean.startsWith('https://')) {
      clean = 'https://' + clean;
    }
    const parsed = new URL(clean);
    return parsed.hostname.replace(/^www\./, '');
  } catch {
    return urlOrDomain.trim().toLowerCase().replace(/^www\./, '');
  }
}

function createTrustedDomainsStore() {
  const { subscribe, set, update } = writable<TrustedDomainsConfig>(loadConfig());

  return {
    subscribe,

    isTrusted(urlOrDomain: string): boolean {
      const state = get({ subscribe });
      if (state.alwaysOpenWithoutPrompt) return true;
      const domain = extractDomain(urlOrDomain);
      return state.trustedDomains.some(td => {
        const cleanTd = td.toLowerCase().trim().replace(/^www\./, '');
        return domain === cleanTd || domain.endsWith('.' + cleanTd);
      });
    },

    shouldDirectRedirect(urlOrDomain: string): boolean {
      const state = get({ subscribe });
      if (state.alwaysOpenWithoutPrompt) return true;
      if (state.directRedirectForTrusted && this.isTrusted(urlOrDomain)) return true;
      return false;
    },

    addTrustedDomain(domain: string) {
      const clean = extractDomain(domain);
      if (!clean) return;
      update(s => {
        if (s.trustedDomains.includes(clean)) return s;
        const next = { ...s, trustedDomains: [...s.trustedDomains, clean] };
        saveConfig(next);
        return next;
      });
    },

    removeTrustedDomain(domain: string) {
      const clean = extractDomain(domain);
      update(s => {
        const next = { ...s, trustedDomains: s.trustedDomains.filter(d => d !== clean) };
        saveConfig(next);
        return next;
      });
    },

    setDirectRedirectForTrusted(enabled: boolean) {
      update(s => {
        const next = { ...s, directRedirectForTrusted: enabled };
        saveConfig(next);
        return next;
      });
    },

    setAlwaysOpenWithoutPrompt(enabled: boolean) {
      update(s => {
        const next = { ...s, alwaysOpenWithoutPrompt: enabled };
        saveConfig(next);
        return next;
      });
    },

    resetToDefaults() {
      const next: TrustedDomainsConfig = {
        trustedDomains: DEFAULT_TRUSTED_DOMAINS,
        directRedirectForTrusted: true,
        alwaysOpenWithoutPrompt: false,
      };
      saveConfig(next);
      set(next);
    },
  };
}

export const trustedDomainsStore = createTrustedDomainsStore();

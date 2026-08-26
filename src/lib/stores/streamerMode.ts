import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';

export type MaskStyle = 'asterisks' | 'blur' | 'bullets' | 'hidden';

export type StreamerPreset = 'max_privacy' | 'streamer_balanced' | 'minimal' | 'custom';

export interface StreamerModeConfig {
  enabled: boolean;
  autoEnableOnScreenShare: boolean;
  autoDisableOnScreenShareEnd: boolean;
  preset: StreamerPreset;
  maskStyle: MaskStyle;
  hideAccountDetails: boolean;
  hideUserIds: boolean;
  hideInviteLinks: boolean;
  hideDmContent: boolean;
  hideVoiceParticipants: boolean;
  suppressNotificationPopups: boolean;
  suppressAudioAlerts: boolean;
  blurMediaAttachments: boolean;
  hideSystemDiagnostics: boolean;
  hideServerDetails: boolean;
}

export const PRESET_CONFIGS: Record<Exclude<StreamerPreset, 'custom'>, Omit<StreamerModeConfig, 'enabled' | 'autoEnableOnScreenShare' | 'autoDisableOnScreenShareEnd' | 'preset'>> = {
  max_privacy: {
    maskStyle: 'blur',
    hideAccountDetails: true,
    hideUserIds: true,
    hideInviteLinks: true,
    hideDmContent: true,
    hideVoiceParticipants: true,
    suppressNotificationPopups: true,
    suppressAudioAlerts: true,
    blurMediaAttachments: true,
    hideSystemDiagnostics: true,
    hideServerDetails: true,
  },
  streamer_balanced: {
    maskStyle: 'asterisks',
    hideAccountDetails: true,
    hideUserIds: true,
    hideInviteLinks: true,
    hideDmContent: true,
    hideVoiceParticipants: false,
    suppressNotificationPopups: true,
    suppressAudioAlerts: true,
    blurMediaAttachments: true,
    hideSystemDiagnostics: true,
    hideServerDetails: true,
  },
  minimal: {
    maskStyle: 'asterisks',
    hideAccountDetails: true,
    hideUserIds: false,
    hideInviteLinks: true,
    hideDmContent: false,
    hideVoiceParticipants: false,
    suppressNotificationPopups: true,
    suppressAudioAlerts: false,
    blurMediaAttachments: false,
    hideSystemDiagnostics: false,
    hideServerDetails: false,
  },
};

const DEFAULT_CONFIG: StreamerModeConfig = {
  enabled: false,
  autoEnableOnScreenShare: true,
  autoDisableOnScreenShareEnd: true,
  preset: 'streamer_balanced',
  maskStyle: 'asterisks',
  hideAccountDetails: true,
  hideUserIds: true,
  hideInviteLinks: true,
  hideDmContent: true,
  hideVoiceParticipants: false,
  suppressNotificationPopups: true,
  suppressAudioAlerts: true,
  blurMediaAttachments: true,
  hideSystemDiagnostics: true,
  hideServerDetails: true,
};

const STORAGE_KEY = 'veilanon_streamer_mode';

function loadInitialConfig(): StreamerModeConfig {
  if (!browser) return DEFAULT_CONFIG;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return { ...DEFAULT_CONFIG, ...JSON.parse(raw) };
    }
  } catch (err) {
    console.warn('[StreamerMode] Failed to load cached config:', err);
  }
  return DEFAULT_CONFIG;
}

function syncDocumentAttributes(config: StreamerModeConfig) {
  if (!browser) return;
  const root = document.documentElement;
  if (config.enabled) {
    root.setAttribute('data-streamer-mode', 'true');
    root.setAttribute('data-mask-style', config.maskStyle);
    if (config.blurMediaAttachments) {
      root.setAttribute('data-streamer-blur-media', 'true');
    } else {
      root.removeAttribute('data-streamer-blur-media');
    }
  } else {
    root.removeAttribute('data-streamer-mode');
    root.removeAttribute('data-mask-style');
    root.removeAttribute('data-streamer-blur-media');
  }
}

function createStreamerModeStore() {
  const initial = loadInitialConfig();
  const { subscribe, set, update } = writable<StreamerModeConfig>(initial);

  if (browser) {
    syncDocumentAttributes(initial);
  }

  function saveAndSync(config: StreamerModeConfig) {
    if (browser) {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
      } catch (err) {
        console.warn('[StreamerMode] Failed to persist config:', err);
      }
      syncDocumentAttributes(config);
    }
  }

  return {
    subscribe,
    toggle(forcedState?: boolean) {
      update((curr) => {
        const next = { ...curr, enabled: forcedState !== undefined ? forcedState : !curr.enabled };
        saveAndSync(next);
        return next;
      });
    },
    setEnabled(enabled: boolean) {
      update((curr) => {
        const next = { ...curr, enabled };
        saveAndSync(next);
        return next;
      });
    },
    setPreset(preset: StreamerPreset) {
      update((curr) => {
        let next: StreamerModeConfig;
        if (preset === 'custom') {
          next = { ...curr, preset: 'custom' };
        } else {
          const presetVals = PRESET_CONFIGS[preset];
          next = {
            ...curr,
            preset,
            ...presetVals,
          };
        }
        saveAndSync(next);
        return next;
      });
    },
    setMaskStyle(maskStyle: MaskStyle) {
      update((curr) => {
        const next = { ...curr, maskStyle, preset: 'custom' as StreamerPreset };
        saveAndSync(next);
        return next;
      });
    },
    updateSetting<K extends keyof StreamerModeConfig>(key: K, value: StreamerModeConfig[K]) {
      update((curr) => {
        const next = { ...curr, [key]: value };
        if (key !== 'enabled' && key !== 'autoEnableOnScreenShare' && key !== 'preset') {
          next.preset = 'custom';
        }
        saveAndSync(next);
        return next;
      });
    },
    resetToDefaults() {
      const next = { ...DEFAULT_CONFIG };
      saveAndSync(next);
      set(next);
    },
  };
}

export const streamerMode = createStreamerModeStore();

// ── Text & Data Masking Helpers ──────────────────────────────────────────────

export function maskText(text: string | null | undefined, style?: MaskStyle, length?: number): string {
  if (!text) return '';
  const current = get(streamerMode);
  if (!current.enabled) return text;

  const resolvedStyle = style || current.maskStyle;
  const len = length ?? Math.min(Math.max(text.length, 6), 16);

  switch (resolvedStyle) {
    case 'asterisks':
      return '*'.repeat(len);
    case 'bullets':
      return '•'.repeat(len);
    case 'hidden':
      return '[GİZLENDİ]';
    case 'blur':
    default:
      return '*'.repeat(len);
  }
}

export function maskEmail(email: string | null | undefined): string {
  if (!email) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideAccountDetails) return email;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ E-POSTA]';
  if (currentStyle === 'bullets') return '••••••••••@•••••••.•••';
  return '**********@*******.***';
}

export function maskUserId(id: string | null | undefined): string {
  if (!id) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideUserIds) return id;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ KİMLİK]';
  if (currentStyle === 'bullets') return '••••-••••-••••';
  return '****-****-****';
}

export function maskInviteLink(linkOrCode: string | null | undefined): string {
  if (!linkOrCode) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideInviteLinks) return linkOrCode;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ DAVET BAĞLANTISI]';
  if (currentStyle === 'bullets') return 'veilanon://join/••••••••';
  return 'veilanon://join/********';
}

export function maskDmText(text: string | null | undefined): string {
  if (!text) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideDmContent) return text;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ MESAJ İÇERİĞİ]';
  if (currentStyle === 'bullets') return '••••••••••••••••••••';
  return '********************';
}

export function maskPath(filePath: string | null | undefined): string {
  if (!filePath) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideSystemDiagnostics) return filePath;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ DOSYA YOLU]';
  if (currentStyle === 'bullets') return '••••/••••/••••/logs';
  return '****/****/****/logs';
}

export function maskToken(token: string | null | undefined): string {
  if (!token) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideAccountDetails) return token;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ GÜVENLİK ANAHTARI]';
  if (currentStyle === 'bullets') return '••••••••••••••••••••••••••••••••';
  return '********************************';
}

export function maskWebhookUrl(url: string | null | undefined): string {
  if (!url) return '';
  const current = get(streamerMode);
  if (!current.enabled || !current.hideServerDetails) return url;

  const currentStyle = current.maskStyle;
  if (currentStyle === 'hidden') return '[GİZLİ WEBHOOK BAĞLANTISI]';
  if (currentStyle === 'bullets') return 'https://discord.com/api/webhooks/••••/••••';
  return 'https://discord.com/api/webhooks/****/****';
}

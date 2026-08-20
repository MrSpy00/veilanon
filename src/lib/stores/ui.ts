/**
 * UI state store — panels, modals, themes
 */
import { writable } from 'svelte/store';
import { spaceStore } from './spaces';
import { presenceApi } from '$lib/api/tauri';
import { applyThemeTokensToDom, applyCustomCssNode } from '$lib/utils/theme-apply';
import { sanitizeCss } from '$lib/utils/css-sanitizer';

export type Theme = 'dark' | 'light' | 'system';
export type Presence = 'online' | 'away' | 'dnd' | 'offline' | 'invisible';
export type Modal =
  | 'settings'
  | 'create-space'
  | 'create-channel'
  | 'create-group-dm'
  | 'invite'
  | 'user-profile'
  | 'channel-settings'
  | 'channel-edit'
  | 'space-settings'
  | 'role-editor'
  | null;

interface UiState {
  theme: Theme;
  presetThemeId: string;
  customThemeName: string;
  customCss: string;
  customCssEnabled: boolean;
  customBgImage: string;
  customBgVideo: string;
  customBgOpacity: number;
  activeSpaceId: string | null;
  activeChannelId: string | null;
  activeDmId: string | null;
  showMemberList: boolean;
  showChannelList: boolean;
  openModal: Modal;
  modalData: unknown;
  compactMode: boolean;
  sidebarCollapsed: boolean;
  settingsTab: string;
  /** Current presence — drives every avatar dot and the status menu. */
  presence: Presence;
  /** Custom confirmation dialog (replaces window.confirm / webview prompts). */
  confirmDialog: { title: string; message: string; confirmLabel: string; danger: boolean } | null;
  /** Custom single-input dialog (replaces window.prompt / webview prompts). */
  inputDialog: { title: string; message: string; placeholder: string; secret: boolean; confirmLabel: string; defaultValue: string } | null;
  /** Reply target shown above the message input (per-channel, cleared on send). */
  replyTo: { channelId: string; messageId: string; author: string; content: string } | null;
}

function createUiStore() {
  const { subscribe, set, update } = writable<UiState>({
    theme: 'dark',
    presetThemeId: 'veil-origin',
    customThemeName: 'Kişisel Tema',
    customCss: '',
    customCssEnabled: false,
    customBgImage: '',
    customBgVideo: '',
    customBgOpacity: 0.26,
    activeSpaceId: null,
    activeChannelId: null,
    activeDmId: null,
    showMemberList: true,
    showChannelList: true,
    openModal: null,
    modalData: null,
    compactMode: false,
    sidebarCollapsed: false,
    settingsTab: 'account',
    presence: 'online',
    confirmDialog: null,
    inputDialog: null,
    replyTo: null,
  });

  let confirmResolver: ((value: boolean) => void) | null = null;
  let inputResolver: ((value: string | null) => void) | null = null;

  function refreshDomTheme(state: UiState) {
    if (typeof document === 'undefined') return;

    const resolvedMode = state.theme === 'system'
      ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
      : state.theme;

    document.documentElement.setAttribute('data-theme', resolvedMode);

    const isDark = resolvedMode === 'dark';
    const accent = localStorage.getItem('veilanon-accent');
    const amoled = localStorage.getItem('veilanon-amoled') === 'true';

    applyThemeTokensToDom(state.presetThemeId, isDark, amoled, accent);
    applyCustomCssNode(state.customCss, state.customCssEnabled);
  }

  return {
    subscribe,

    setTheme(theme: Theme) {
      update(s => {
        const next = { ...s, theme };
        localStorage.setItem('veilanon-theme', theme);
        refreshDomTheme(next);
        return next;
      });
    },

    setPresetTheme(presetThemeId: string) {
      update(s => {
        const next = { ...s, presetThemeId };
        localStorage.setItem('veilanon-preset', presetThemeId);
        refreshDomTheme(next);
        return next;
      });
    },

    setAccentColor(color: string | null) {
      if (!color) {
        localStorage.removeItem('veilanon-accent');
      } else {
        localStorage.setItem('veilanon-accent', color);
      }
      update(s => {
        refreshDomTheme(s);
        return s;
      });
    },

    setAmoledMode(enabled: boolean) {
      if (enabled) {
        localStorage.setItem('veilanon-amoled', 'true');
      } else {
        localStorage.removeItem('veilanon-amoled');
      }
      update(s => {
        refreshDomTheme(s);
        return s;
      });
    },

    setCustomCss(css: string) {
      const sanitized = sanitizeCss(css);
      update(s => {
        const next = { ...s, customCss: sanitized.safe };
        localStorage.setItem('veilanon-custom-css', sanitized.safe);
        refreshDomTheme(next);
        return next;
      });
    },

    toggleCustomCss(enabled: boolean) {
      update(s => {
        const next = { ...s, customCssEnabled: enabled };
        localStorage.setItem('veilanon-custom-css-enabled', enabled ? 'true' : 'false');
        refreshDomTheme(next);
        return next;
      });
    },

    setCustomBackground(image: string, video: string, opacity: number) {
      const clampedOpacity = Math.max(0, Math.min(0.6, opacity));
      update(s => {
        const next = {
          ...s,
          customBgImage: image,
          customBgVideo: video,
          customBgOpacity: clampedOpacity,
        };
        localStorage.setItem('veilanon-bg-image', image);
        localStorage.setItem('veilanon-bg-video', video);
        localStorage.setItem('veilanon-bg-opacity', String(clampedOpacity));
        return next;
      });
    },

    setCustomThemeName(customThemeName: string) {
      update(s => {
        localStorage.setItem('veilanon-custom-theme-name', customThemeName);
        return { ...s, customThemeName };
      });
    },

    clearMediaOnError() {
      update(s => {
        localStorage.removeItem('veilanon-bg-image');
        localStorage.removeItem('veilanon-bg-video');
        return { ...s, customBgImage: '', customBgVideo: '' };
      });
    },

    resetCustomLayer() {
      update(s => {
        localStorage.removeItem('veilanon-custom-css');
        localStorage.removeItem('veilanon-custom-css-enabled');
        localStorage.removeItem('veilanon-bg-image');
        localStorage.removeItem('veilanon-bg-video');
        localStorage.removeItem('veilanon-bg-opacity');
        localStorage.removeItem('veilanon-custom-theme-name');
        const next = {
          ...s,
          customThemeName: 'Kişisel Tema',
          customCss: '',
          customCssEnabled: false,
          customBgImage: '',
          customBgVideo: '',
          customBgOpacity: 0.26,
        };
        refreshDomTheme(next);
        return next;
      });
    },

    initTheme() {
      const storedTheme = (localStorage.getItem('veilanon-theme') as Theme) || 'dark';
      const storedPreset = localStorage.getItem('veilanon-preset') || 'veil-origin';
      const customCss = localStorage.getItem('veilanon-custom-css') || '';
      const customCssEnabled = localStorage.getItem('veilanon-custom-css-enabled') === 'true';
      const customBgImage = localStorage.getItem('veilanon-bg-image') || '';
      const customBgVideo = localStorage.getItem('veilanon-bg-video') || '';
      const storedOpacity = parseFloat(localStorage.getItem('veilanon-bg-opacity') || '0.26');
      const customThemeName = localStorage.getItem('veilanon-custom-theme-name') || 'Kişisel Tema';

      update(s => {
        const next = {
          ...s,
          theme: storedTheme,
          presetThemeId: storedPreset,
          customThemeName,
          customCss,
          customCssEnabled,
          customBgImage,
          customBgVideo,
          customBgOpacity: isNaN(storedOpacity) ? 0.26 : storedOpacity,
        };
        refreshDomTheme(next);
        return next;
      });
    },

    navigate(spaceId: string | null, channelId: string | null) {
      if (spaceId && channelId) {
        try {
          localStorage.setItem(`veil_last_channel_${spaceId}`, channelId);
        } catch { /* storage full */ }
      }
      update(s => ({
        ...s,
        activeSpaceId: spaceId,
        activeChannelId: channelId,
        activeDmId: null,
      }));
    },

    async navigateSpace(spaceId: string) {
      let targetChannelId: string | null = null;
      try {
        targetChannelId = localStorage.getItem(`veil_last_channel_${spaceId}`);
      } catch { /* ignored */ }

      update(s => ({
        ...s,
        activeSpaceId: spaceId,
        activeChannelId: targetChannelId,
        activeDmId: null,
      }));

      const channels = await spaceStore.loadChannels(spaceId);
      if (channels && channels.length > 0) {
        if (!targetChannelId || !channels.some(c => c.id === targetChannelId)) {
          const defaultText = channels.find(c => c.channelType === 'text') ?? channels[0];
          targetChannelId = defaultText?.id ?? null;
          if (targetChannelId) {
            try {
              localStorage.setItem(`veil_last_channel_${spaceId}`, targetChannelId);
            } catch { /* ignored */ }
          }
          update(s => ({
            ...s,
            activeChannelId: targetChannelId,
          }));
        }
      }
    },

    navigateDm(dmId: string) {
      update(s => ({
        ...s,
        activeSpaceId: null,
        activeChannelId: null,
        activeDmId: dmId,
      }));
    },

    openModal(modal: Modal, data?: unknown) {
      update(s => {
        const tab = (data as { tab?: string } | null)?.tab;
        return {
          ...s,
          openModal: modal,
          modalData: data ?? null,
          settingsTab: tab && modal === 'settings' ? tab : s.settingsTab,
        };
      });
    },

    closeModal() {
      update(s => ({ ...s, openModal: null, modalData: null }));
    },

    toggleMemberList() {
      update(s => ({ ...s, showMemberList: !s.showMemberList }));
    },

    toggleChannelList() {
      update(s => ({ ...s, showChannelList: !s.showChannelList }));
    },

    setSettingsTab(tab: string) {
      update(s => ({ ...s, settingsTab: tab }));
    },

    setPresence(presence: Presence) {
      update(s => ({ ...s, presence }));
      presenceApi.update(presence).catch(() => {});
    },

    setCompactMode(compact: boolean) {
      update(s => ({ ...s, compactMode: compact }));
      document.documentElement.setAttribute('data-compact', compact ? 'true' : 'false');
    },

    /** Promise-based confirmation — renders ConfirmDialog instead of the
     *  browser/webview confirm(). */
    confirm(message: string, opts: { title?: string; confirmLabel?: string; danger?: boolean } = {}) {
      return new Promise<boolean>((resolve) => {
        confirmResolver = resolve;
        update(s => ({
          ...s,
          confirmDialog: {
            title: opts.title ?? 'Onayla',
            message,
            confirmLabel: opts.confirmLabel ?? 'Onayla',
            danger: opts.danger ?? false,
          },
        }));
      });
    },

    resolveConfirm(value: boolean) {
      confirmResolver?.(value);
      confirmResolver = null;
      update(s => ({ ...s, confirmDialog: null }));
    },

    /** Promise-based single-input prompt — replaces window.prompt. */
    promptInput(message: string, opts: { title?: string; placeholder?: string; secret?: boolean; confirmLabel?: string; defaultValue?: string } = {}) {
      return new Promise<string | null>((resolve) => {
        inputResolver = resolve;
        update(s => ({
          ...s,
          inputDialog: {
            title: opts.title ?? 'Giriş',
            message,
            placeholder: opts.placeholder ?? '',
            secret: opts.secret ?? false,
            confirmLabel: opts.confirmLabel ?? 'Tamam',
            defaultValue: opts.defaultValue ?? '',
          },
        }));
      });
    },

    resolveInput(value: string | null) {
      inputResolver?.(value);
      inputResolver = null;
      update(s => ({ ...s, inputDialog: null }));
    },

    setReplyTo(replyTo: { channelId: string; messageId: string; author: string; content: string } | null) {
      update(s => ({ ...s, replyTo }));
    },

    clearReplyIfChannel(channelId: string) {
      update(s => {
        if (s.replyTo && s.replyTo.channelId === channelId) {
          return { ...s, replyTo: null };
        }
        return s;
      });
    },
  };
}

export const uiStore = createUiStore();

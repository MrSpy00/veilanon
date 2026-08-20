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

export interface BgPlaylistItem {
  id: string;
  url: string;
  type: 'image' | 'video';
  title?: string;
}

interface UiState {
  theme: Theme;
  presetThemeId: string;
  customThemeName: string;
  customCss: string;
  customCssEnabled: boolean;
  customBgImage: string;
  customBgVideo: string;
  customBgOpacity: number;
  messageBackdropBlur: number;
  bgPlaylist: BgPlaylistItem[];
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

let activeUserId: string | null = null;

function userBgKey(key: string, uid?: string | null): string {
  const id = uid || activeUserId;
  return id ? `${key}_${id}` : key;
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
    messageBackdropBlur: 8,
    bgPlaylist: [],
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

    document.documentElement.style.setProperty('--veil-msg-blur', `${state.messageBackdropBlur ?? 8}px`);
    document.documentElement.setAttribute(
      'data-has-custom-bg',
      Boolean(state.customBgImage || state.customBgVideo) ? 'true' : 'false'
    );
  }

  return {
    subscribe,

    loadUserTheme(userId: string | null) {
      activeUserId = userId;
      if (!userId) {
        this.resetThemeToDefault();
        return;
      }
      const storedTheme = (localStorage.getItem(userBgKey('veilanon-theme', userId)) || localStorage.getItem('veilanon-theme') || 'dark') as Theme;
      const storedPreset = localStorage.getItem(userBgKey('veilanon-preset', userId)) || localStorage.getItem('veilanon-preset') || 'veil-origin';
      const storedAccent = localStorage.getItem(userBgKey('veilanon-accent', userId)) || localStorage.getItem('veilanon-accent') || null;
      const storedAmoled = (localStorage.getItem(userBgKey('veilanon-amoled', userId)) || localStorage.getItem('veilanon-amoled')) === 'true';
      const storedImage = localStorage.getItem(userBgKey('veilanon-bg-image', userId)) || '';
      const storedVideo = localStorage.getItem(userBgKey('veilanon-bg-video', userId)) || '';
      const storedOpacity = parseFloat(localStorage.getItem(userBgKey('veilanon-bg-opacity', userId)) || '0.26');
      const storedBlur = parseFloat(localStorage.getItem(userBgKey('veilanon-msg-blur', userId)) || '8');
      
      let storedPlaylist: BgPlaylistItem[] = [];
      try {
        const raw = localStorage.getItem(userBgKey('veilanon-bg-playlist', userId));
        if (raw) storedPlaylist = JSON.parse(raw);
      } catch { /* ignored */ }

      update(s => {
        const next = {
          ...s,
          theme: storedTheme,
          presetThemeId: storedPreset,
          customBgImage: storedImage,
          customBgVideo: storedVideo,
          customBgOpacity: isNaN(storedOpacity) ? 0.26 : storedOpacity,
          messageBackdropBlur: isNaN(storedBlur) ? 8 : storedBlur,
          bgPlaylist: storedPlaylist,
        };
        refreshDomTheme(next);
        return next;
      });
    },

    resetThemeToDefault() {
      activeUserId = null;
      update(s => {
        const next = {
          ...s,
          customBgImage: '',
          customBgVideo: '',
          customBgOpacity: 0.26,
          messageBackdropBlur: 8,
          bgPlaylist: [],
        };
        refreshDomTheme(next);
        return next;
      });
    },

    setTheme(theme: Theme) {
      update(s => {
        const next = { ...s, theme };
        localStorage.setItem('veilanon-theme', theme);
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-theme'), theme);
        refreshDomTheme(next);
        return next;
      });
    },

    setPresetTheme(presetThemeId: string) {
      update(s => {
        const next = { ...s, presetThemeId };
        localStorage.setItem('veilanon-preset', presetThemeId);
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-preset'), presetThemeId);
        refreshDomTheme(next);
        return next;
      });
    },

    setAccentColor(color: string | null) {
      if (!color) {
        localStorage.removeItem('veilanon-accent');
        if (activeUserId) localStorage.removeItem(userBgKey('veilanon-accent'));
      } else {
        localStorage.setItem('veilanon-accent', color);
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-accent'), color);
      }
      update(s => {
        refreshDomTheme(s);
        return s;
      });
    },

    setAmoledMode(enabled: boolean) {
      if (enabled) {
        localStorage.setItem('veilanon-amoled', 'true');
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-amoled'), 'true');
      } else {
        localStorage.removeItem('veilanon-amoled');
        if (activeUserId) localStorage.removeItem(userBgKey('veilanon-amoled'));
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
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-custom-css'), sanitized.safe);
        refreshDomTheme(next);
        return next;
      });
    },

    toggleCustomCss(enabled: boolean) {
      update(s => {
        const next = { ...s, customCssEnabled: enabled };
        localStorage.setItem('veilanon-custom-css-enabled', enabled ? 'true' : 'false');
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-custom-css-enabled'), enabled ? 'true' : 'false');
        refreshDomTheme(next);
        return next;
      });
    },

    setCustomBackground(image: string, video: string, opacity: number) {
      const clampedOpacity = Math.max(0, Math.min(1.0, opacity));
      update(s => {
        const next = {
          ...s,
          customBgImage: image,
          customBgVideo: video,
          customBgOpacity: clampedOpacity,
        };
        localStorage.setItem(userBgKey('veilanon-bg-image'), image);
        localStorage.setItem(userBgKey('veilanon-bg-video'), video);
        localStorage.setItem(userBgKey('veilanon-bg-opacity'), String(clampedOpacity));
        refreshDomTheme(next);
        return next;
      });
    },

    setMessageBlur(blur: number) {
      const clampedBlur = Math.max(0, Math.min(30, blur));
      update(s => {
        const next = { ...s, messageBackdropBlur: clampedBlur };
        localStorage.setItem(userBgKey('veilanon-msg-blur'), String(clampedBlur));
        refreshDomTheme(next);
        return next;
      });
    },

    setPlaylist(playlist: BgPlaylistItem[]) {
      update(s => {
        const next = { ...s, bgPlaylist: playlist };
        localStorage.setItem(userBgKey('veilanon-bg-playlist'), JSON.stringify(playlist));
        return next;
      });
    },

    addPlaylistItem(item: BgPlaylistItem) {
      update(s => {
        const list = [...s.bgPlaylist.filter(p => p.url !== item.url), item];
        const next = { ...s, bgPlaylist: list };
        localStorage.setItem(userBgKey('veilanon-bg-playlist'), JSON.stringify(list));
        return next;
      });
    },

    removePlaylistItem(id: string) {
      update(s => {
        const list = s.bgPlaylist.filter(p => p.id !== id);
        const next = { ...s, bgPlaylist: list };
        localStorage.setItem(userBgKey('veilanon-bg-playlist'), JSON.stringify(list));
        return next;
      });
    },

    cyclePlaylistNext() {
      update(s => {
        if (!s.bgPlaylist || s.bgPlaylist.length === 0) return s;
        const curUrl = s.customBgImage || s.customBgVideo;
        const curIdx = s.bgPlaylist.findIndex(p => p.url === curUrl);
        const nextIdx = (curIdx + 1) % s.bgPlaylist.length;
        const target = s.bgPlaylist[nextIdx];
        if (!target) return s;
        const img = target.type === 'image' ? target.url : '';
        const vid = target.type === 'video' ? target.url : '';
        const next = {
          ...s,
          customBgImage: img,
          customBgVideo: vid,
        };
        localStorage.setItem(userBgKey('veilanon-bg-image'), img);
        localStorage.setItem(userBgKey('veilanon-bg-video'), vid);
        refreshDomTheme(next);
        return next;
      });
    },

    setCustomThemeName(customThemeName: string) {
      update(s => {
        localStorage.setItem('veilanon-custom-theme-name', customThemeName);
        if (activeUserId) localStorage.setItem(userBgKey('veilanon-custom-theme-name'), customThemeName);
        return { ...s, customThemeName };
      });
    },

    clearMediaOnError() {
      update(s => {
        localStorage.removeItem(userBgKey('veilanon-bg-image'));
        localStorage.removeItem(userBgKey('veilanon-bg-video'));
        const next = { ...s, customBgImage: '', customBgVideo: '' };
        refreshDomTheme(next);
        return next;
      });
    },

    resetCustomLayer() {
      update(s => {
        localStorage.removeItem('veilanon-custom-css');
        localStorage.removeItem('veilanon-custom-css-enabled');
        localStorage.removeItem(userBgKey('veilanon-bg-image'));
        localStorage.removeItem(userBgKey('veilanon-bg-video'));
        localStorage.removeItem(userBgKey('veilanon-bg-opacity'));
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
      const customBgImage = activeUserId ? (localStorage.getItem(userBgKey('veilanon-bg-image')) || '') : '';
      const customBgVideo = activeUserId ? (localStorage.getItem(userBgKey('veilanon-bg-video')) || '') : '';
      const storedOpacity = parseFloat(localStorage.getItem(userBgKey('veilanon-bg-opacity')) || '0.26');
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

      // Synchronously pick existing channels if already loaded in memory to prevent <Home /> flash
      let inMemoryChannels: any[] = [];
      try {
        const { get } = await import('svelte/store');
        const spState = get(spaceStore);
        inMemoryChannels = spState.channelsBySpace[spaceId] ?? [];
      } catch { /* ignored */ }

      if (!targetChannelId && inMemoryChannels.length > 0) {
        const defaultText = inMemoryChannels.find(c => c.channelType === 'text' && (c.name.toLowerCase() === 'genel' || c.name.toLowerCase() === 'general'))
          ?? inMemoryChannels.find(c => c.channelType === 'text')
          ?? inMemoryChannels[0];
        targetChannelId = defaultText?.id ?? null;
      }

      update(s => ({
        ...s,
        activeSpaceId: spaceId,
        activeChannelId: targetChannelId,
        activeDmId: null,
      }));

      const channels = await spaceStore.loadChannels(spaceId);
      if (channels && channels.length > 0) {
        if (!targetChannelId || !channels.some(c => c.id === targetChannelId)) {
          const defaultText = channels.find(c => c.channelType === 'text' && (c.name.toLowerCase() === 'genel' || c.name.toLowerCase() === 'general' || c.name.toLowerCase() === 'welcome' || c.name.toLowerCase() === 'hosgeldiniz'))
            ?? channels.find(c => c.channelType === 'text')
            ?? channels[0];
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

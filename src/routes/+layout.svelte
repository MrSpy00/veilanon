<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { authStore } from '$lib/stores/auth';
  import { uiStore } from '$lib/stores/ui';
  import { streamerMode } from '$lib/stores/streamerMode';
  import { privacyShield, isShieldActive, isScreenShareActive } from '$lib/stores/privacyShield';
  import { installWebviewGuard } from '$lib/utils/webview-guard';
  import { handleDeepLink } from '$lib/utils/deeplink';
  import { toastStore } from '$lib/stores/notifications';
  import { loadAllPlugins } from '$lib/effects/plugin';
  import ToastContainer from '$lib/components/ui/ToastContainer.svelte';
  import AppLogo from '$lib/components/ui/AppLogo.svelte';

  let { children } = $props();
  let mounted = $state(false);
  let cleanupWebviewGuard: (() => void) | undefined;
  const unlistens: Array<() => void> = [];

  /** Resolves null if `p` doesn't settle within `ms` — keeps startup resilient
   *  against slow IPC / Argon2 / keychain stalls instead of hanging the splash. */
  function withTimeout<T>(p: Promise<T>, ms: number): Promise<T | null> {
    return Promise.race([
      p,
      new Promise<null>((resolve) => setTimeout(() => resolve(null), ms)),
    ]);
  }

  $effect(() => {
    if (typeof document === 'undefined') return;
    const sm = $streamerMode;
    const shield = $privacyShield;
    const screenSharing = $isScreenShareActive;
    const shieldActive = $isShieldActive;

    if (sm.enabled) {
      document.documentElement.setAttribute('data-streamer-mode', 'true');
      document.documentElement.setAttribute('data-mask-style', sm.maskStyle);
    } else {
      document.documentElement.removeAttribute('data-streamer-mode');
      document.documentElement.removeAttribute('data-mask-style');
    }

    if (screenSharing) {
      document.documentElement.setAttribute('data-screen-sharing', 'true');
    } else {
      document.documentElement.removeAttribute('data-screen-sharing');
    }

    if (shieldActive || sm.enabled) {
      document.documentElement.setAttribute('data-privacy-shield', 'true');
    } else {
      document.documentElement.removeAttribute('data-privacy-shield');
    }

    if (sm.blurMediaAttachments || (screenSharing && shield.blurMediaOnShare)) {
      document.documentElement.setAttribute('data-streamer-blur-media', 'true');
    } else {
      document.documentElement.removeAttribute('data-streamer-blur-media');
    }

    const ui = $uiStore;
    if (ui.customBgImage || ui.customBgVideo) {
      document.documentElement.setAttribute('data-has-custom-bg', 'true');
    } else {
      document.documentElement.removeAttribute('data-has-custom-bg');
    }
  });

  $effect(() => {
    if (mounted && $authStore.isAuthenticated && typeof window !== 'undefined') {
      const pending = localStorage.getItem('veilanon-pending-deeplink');
      if (pending) {
        localStorage.removeItem('veilanon-pending-deeplink');
        setTimeout(() => {
          void handleDeepLink(pending);
        }, 350);
      }
    }
  });

  onMount(async () => {
    splashTimeout = setTimeout(() => {
      if (!mounted) mounted = true;
    }, 15000);

    uiStore.initTheme();

    try {
      const { settingsApi } = await import('$lib/api/tauri');
      const s = await withTimeout(settingsApi.get(), 4000);
      if (s) {
        if (s.theme) {
          uiStore.setTheme(s.theme as any);
        }
        if (s.presetThemeId) {
          uiStore.setPresetTheme(s.presetThemeId);
        }
        if (s.customCss !== undefined) {
          uiStore.setCustomCss(s.customCss);
        }
        if (s.customCssEnabled !== undefined) {
          uiStore.toggleCustomCss(s.customCssEnabled);
        }
        if (s.customThemeName) {
          uiStore.setCustomThemeName(s.customThemeName);
        }
        if (s.fontSize && s.fontSize !== 14) {
          document.documentElement.style.fontSize = `${s.fontSize}px`;
        }
        if (s.reduceMotion) {
          document.documentElement.setAttribute('data-reduce-motion', 'true');
        }
        if (s.compactMode) {
          document.documentElement.setAttribute('data-compact', 'true');
        }
        if (s.accentColor) uiStore.setAccentColor(s.accentColor);
        if (s.amoledMode !== undefined) uiStore.setAmoledMode(s.amoledMode);
      }
    } catch { /* backend yok (tarayıcı önizleme) */ }

    void loadAllPlugins().catch(() => { /* Python plugin yüklenemedi */ });

    if ('__TAURI_INTERNALS__' in window) {
      let registered = false;
      try {
        const { onOpenUrl } = await import('@tauri-apps/plugin-deep-link');
        // Register without awaiting resolution — the listener attaches synchronously
        // and a stalled plugin promise must not block the splash.
        void onOpenUrl((urls) => {
          for (const u of urls) {
            if ($authStore.isAuthenticated) {
              void handleDeepLink(u);
            } else {
              localStorage.setItem('veilanon-pending-deeplink', u);
            }
          }
        }).catch(() => { /* ignore */ });
        registered = true;
      } catch { /* fallback to event listener if plugin hook fails */ }

      if (!registered) {
        listen<string[] | string>('deep-link://new-url', (e) => {
          const payload = e.payload;
          const urls = Array.isArray(payload) ? payload : [payload];
          for (const u of urls) {
            if (u && typeof u === 'string') {
              if ($authStore.isAuthenticated) {
                void handleDeepLink(u);
              } else {
                localStorage.setItem('veilanon-pending-deeplink', u);
              }
            }
          }
        }).then((u) => unlistens.push(u)).catch(() => {});
      }
    }

    if ('__TAURI_INTERNALS__' in window) {
      window.addEventListener('error', (e) => {
        invoke('log_client_error', { level: 'error', message: String(e.message ?? e.error).slice(0, 500) }).catch(() => {});
      });
      window.addEventListener('unhandledrejection', (e) => {
        invoke('log_client_error', { level: 'error', message: String(e.reason).slice(0, 500) }).catch(() => {});
      });

      const origDebug = console.debug.bind(console);
      const origInfo = console.info.bind(console);
      const origLog = console.log.bind(console);
      const origWarn = console.warn.bind(console);
      const origError = console.error.bind(console);

      const bridge = (orig: (...args: any[]) => void, level: string) => (...args: unknown[]) => {
        try { orig(...args); } catch { /* best effort */ }
        let text = args
          .map(a => (typeof a === 'string' ? a : (a instanceof Error ? `${a.name}: ${a.message}` : JSON.stringify(a))))
          .join(' ')
          .slice(0, 1000);
        const lower = text.toLowerCase();
        if (lower.includes('token') || lower.includes('passphrase') || lower.includes('ciphertext') || lower.includes('veilanon_supabase') || lower.includes('livekit_secret') || lower.includes('authorization')) {
          text = `[redacted ${level}]`;
        }
        invoke('log_client_error', { level, message: text }).catch(() => {});
      };
      console.debug = bridge(origDebug, 'debug');
      console.info = bridge(origInfo, 'info');
      console.log = bridge(origLog, 'info');
      console.warn = bridge(origWarn, 'warn');
      console.error = bridge(origError, 'error');

      listen<string>('tray:set-presence', async (e) => {
        try {
          const { presenceApi } = await import('$lib/api/tauri');
          const p = e.payload as any;
          await presenceApi.update(p);
          uiStore.setPresence(p);
        } catch { /* ignored */ }
      }).then((u) => unlistens.push(u)).catch(() => {});

      listen('tray:toggle-mute', async () => {
        try {
          const { mediaStore } = await import('$lib/stores/media');
          await mediaStore.toggleMute();
        } catch { /* ignored */ }
      }).then((u) => unlistens.push(u)).catch(() => {});

      listen('tray:toggle-deafen', async () => {
        try {
          const { mediaStore } = await import('$lib/stores/media');
          await mediaStore.toggleDeafen();
        } catch { /* ignored */ }
      }).then((u) => unlistens.push(u)).catch(() => {});

      listen('tray:leave-voice', async () => {
        try {
          const { mediaStore } = await import('$lib/stores/media');
          await mediaStore.leaveVoice();
        } catch { /* ignored */ }
      }).then((u) => unlistens.push(u)).catch(() => {});

      listen<string>('tray:open-settings', (e) => {
        const tab = typeof e.payload === 'string' ? e.payload : 'account';
        uiStore.openModal('settings', { tab });
      }).then((u) => unlistens.push(u)).catch(() => {});

      cleanupWebviewGuard = installWebviewGuard();
    }

    // Global Push-to-Talk window listener
    let pttKeyDown = false;
    const onWindowKeyDown = async (e: KeyboardEvent) => {
      if (pttKeyDown) return;
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
        return;
      }
      try {
        const { settingsApi } = await import('$lib/api/tauri');
        const s = await settingsApi.get();
        if (s.pushToTalk) {
          const key = (s.pushToTalkKey || 'V').toUpperCase();
          if (e.key.toUpperCase() === key) {
            pttKeyDown = true;
            const { mediaStore } = await import('$lib/stores/media');
            void mediaStore.pttPress();
          }
        }
      } catch { /* ignored */ }
    };

    const onWindowKeyUp = async (e: KeyboardEvent) => {
      if (!pttKeyDown) return;
      try {
        const { settingsApi } = await import('$lib/api/tauri');
        const s = await settingsApi.get();
        if (s.pushToTalk) {
          const key = (s.pushToTalkKey || 'V').toUpperCase();
          if (e.key.toUpperCase() === key) {
            pttKeyDown = false;
            const { mediaStore } = await import('$lib/stores/media');
            void mediaStore.pttRelease();
          }
        }
      } catch { /* ignored */ }
    };

    window.addEventListener('keydown', onWindowKeyDown);
    window.addEventListener('keyup', onWindowKeyUp);

    cleanupPtt = () => {
      window.removeEventListener('keydown', onWindowKeyDown);
      window.removeEventListener('keyup', onWindowKeyUp);
    };

    const handleOnline = () => {
      isOffline = false;
      toastStore.success('İnternet bağlantısı yeniden kuruldu.');
    };
    const handleOffline = () => {
      isOffline = true;
      toastStore.warn('İnternet bağlantısı kesildi. Çevrimdışı moddasınız.');
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Periodic presence heartbeat: ensures active session stays online and cleans up stale presence
    const presenceHeartbeat = setInterval(async () => {
      if ($authStore.isAuthenticated) {
        try {
          const { presenceApi } = await import('$lib/api/tauri');
          const curr = $uiStore.presence;
          if (curr !== 'invisible' && curr !== 'offline') {
            await presenceApi.update(curr);
          }
        } catch { /* best effort */ }
      }
    }, 20000);

    const onBeforeUnload = () => {
      if ($authStore.isAuthenticated) {
        try {
          invoke('presence_update', { status: 'offline' }).catch(() => {});
        } catch { /* best effort */ }
      }
    };
    window.addEventListener('beforeunload', onBeforeUnload);

    // Auth unlock (Argon2 / keychain) must never hold the splash hostage.
    // Race it: on timeout the shell mounts and the store flips reactively
    // once initialize() eventually settles in the background.
    try {
      await withTimeout(authStore.initialize(), 6000);
    } catch { /* initialize keeps running; auth state updates reactively */ }
    if (splashTimeout) clearTimeout(splashTimeout);
    mounted = true;

    cleanupNetwork = () => {
      clearInterval(presenceHeartbeat);
      window.removeEventListener('beforeunload', onBeforeUnload);
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  });

  let cleanupPtt: (() => void) | undefined;
  let cleanupNetwork: (() => void) | undefined;
  let isOffline = $state(typeof navigator !== 'undefined' ? !navigator.onLine : false);
  let splashTimeout: ReturnType<typeof setTimeout> | undefined;

  onDestroy(() => {
    if (splashTimeout) clearTimeout(splashTimeout);
    cleanupWebviewGuard?.();
    cleanupPtt?.();
    cleanupNetwork?.();
    for (const u of unlistens) {
      u();
    }
  });
</script>

{#if mounted}
  {#if isOffline}
    <div class="veil-offline-banner" role="status" aria-live="polite">
      <span class="veil-offline-pulse"></span>
      <span>Bağlantı kesildi. Çevrimdışı mod — internet sağlandığında otomatik senkronize edilecek.</span>
    </div>
  {/if}
  {@render children()}
{:else}
  <div class="veil-splash">
    <div class="veil-splash-content">
      <AppLogo size={96} radius={28} />
      <div class="veil-spinner"></div>
    </div>
  </div>
{/if}

<ToastContainer />

<style>
  .veil-splash {
    position: fixed;
    inset: 0;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    width: 100vw;
    height: 100vh;
    height: 100dvh;
    display: grid;
    place-items: center;
    align-content: center;
    justify-content: center;
    background: var(--veil-bg-void);
    z-index: 99999;
    margin: 0;
    padding: 0;
    text-align: center;
    box-sizing: border-box;
  }
  .veil-splash-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-6, 1.5rem);
    margin: auto;
    animation: veil-fade-in 0.3s ease-out;
  }
  .veil-splash-content :global(.veil-app-logo) {
    display: block;
    margin: 0 auto;
    box-shadow: var(--shadow-2xl, 0 25px 50px -12px rgba(0, 0, 0, 0.5));
  }
  .veil-splash-content :global(.veil-spinner) {
    display: block;
    margin: 0 auto;
  }
  .veil-offline-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 99990;
    background: color-mix(in srgb, var(--veil-warning) 90%, black);
    color: #fff;
    font-size: var(--text-xs);
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: 6px var(--space-3);
    text-align: center;
    box-shadow: var(--shadow-md);
    animation: veil-fade-in 0.25s ease-out;
  }
  .veil-offline-pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 8px #fff;
    animation: offline-blink 1.5s infinite ease-in-out;
  }
  @keyframes offline-blink {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.3; transform: scale(0.8); }
  }
</style>

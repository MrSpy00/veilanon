<script lang="ts">
  import { mediaStore } from '$lib/stores/media';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { settingsApi } from '$lib/api/tauri';
  import { effectsStore } from '$lib/effects/store';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '../ui/Icon.svelte';
  import ScreenShareModal from './ScreenShareModal.svelte';
  import EffectsPanel from './EffectsPanel.svelte';

  const media = $derived($mediaStore);
  const spaces = $derived($spaceStore);
  const ui = $derived($uiStore);

  let screenShareModalOpen = $state(false);
  let audioPopOpen = $state(false);
  let masterVolume = $state(100);
  const fx = $derived($effectsStore);

  const channelName = $derived(
    (function() {
      if (!media.channelId) return 'Ses Kanalı';
      const name = Object.values(spaces.channelsBySpace).flat().find(c => c.id === media.channelId)?.name
        ?? spaces.dmChannels.find(c => c.id === media.channelId)?.name;
      if (name && name.length === 36 && name.includes('-')) return 'Ses Kanalı';
      return name ?? 'Ses Kanalı';
    })()
  );

  let pttKey = $state<string | null>(null);
  let pttHeld = $state(false);

  function navigateToCallChannel() {
    if (!media.channelId) return;
    const dm = spaces.dmChannels.find(d => d.id === media.channelId);
    if (dm) {
      uiStore.navigateDm(dm.id);
    } else {
      for (const spaceId in spaces.channelsBySpace) {
        const ch = spaces.channelsBySpace[spaceId].find(c => c.id === media.channelId);
        if (ch) {
          uiStore.navigate(spaceId, ch.id);
          return;
        }
      }
    }
  }

  function applyMasterVolume(val: number) {
    masterVolume = val;
    mediaStore.setMasterVolume(val / 100);
  }

  // Bas-konuş: arama sırasında tuş basılı tutulduğunda mikrofon açık kalır.
  $effect(() => {
    if (!media.isInCall) {
      pttKey = null;
      pttHeld = false;
      return;
    }
    let cancelled = false;
    void settingsApi.get().then((s) => {
      if (!cancelled) pttKey = s.pushToTalk ? (s.pushToTalkKey || 'V') : null;
    }).catch(() => {});
    return () => { cancelled = true; };
  });

  $effect(() => {
    if (!media.isInCall || !pttKey) return;
    const key = pttKey;
    const isEditable = (el: EventTarget | null) => {
      const node = el as HTMLElement | null;
      return !!node && (node.tagName === 'INPUT' || node.tagName === 'TEXTAREA' || node.isContentEditable);
    };
    const onDown = (e: KeyboardEvent) => {
      if (isEditable(e.target)) return;
      if (e.key.toUpperCase() === key && !e.repeat) {
        pttHeld = true;
        void mediaStore.pttPress();
      }
    };
    const onUp = (e: KeyboardEvent) => {
      if (e.key.toUpperCase() === key) {
        pttHeld = false;
        void mediaStore.pttRelease();
      }
    };
    const onBlur = () => {
      if (pttHeld) {
        pttHeld = false;
        void mediaStore.pttRelease();
      }
    };
    window.addEventListener('keydown', onDown, true);
    window.addEventListener('keyup', onUp, true);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('keydown', onDown, true);
      window.removeEventListener('keyup', onUp, true);
      window.removeEventListener('blur', onBlur);
    };
  });
</script>

{#if media.isInCall}
  <div class="veil-voice-bar" role="status" aria-label="Sesli arama aktif">
    <!-- Channel / Peer info and click-to-navigate -->
    <button
      type="button"
      class="veil-vb-info"
      onclick={navigateToCallChannel}
      title="Görüşme Odasına Git"
    >
      <span
        class="veil-vb-signal"
        class:good={media.connectionState === 'connected' && (media.latencyMs === null || media.latencyMs < 150)}
        class:mid={media.connectionState === 'connected' && media.latencyMs !== null && media.latencyMs >= 150}
        aria-hidden="true"
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V8"/><path d="M22 4v16"/></svg>
      </span>
      <div class="veil-vb-text">
        <span class="veil-vb-channel">
          <Icon name="volume" size={13} />
          <span class="veil-vb-name-text">{channelName}</span>
        </span>
        <span class="veil-vb-status">
          {#if media.connectionState === 'reconnecting'}
            <span style="color:var(--veil-warning);">yeniden bağlanıyor…</span>
          {:else if media.connectionState === 'connecting'}
            <span style="color:var(--veil-warning);">bağlanıyor…</span>
          {:else if pttKey && media.isMuted && pttHeld}
            <span style="color:var(--veil-success);">Konuşuyorsun…</span>
          {:else if pttKey}
            <span style="color:var(--veil-warning);">Bas-konuş: {pttKey}</span>
          {:else}
            <span>{media.latencyMs ?? 20} ms · Bağlandı</span>
          {/if}
          {#if media.isE2ee}
            <span class="veil-vb-e2ee-tag">· E2EE</span>
          {/if}
        </span>
      </div>
    </button>

    <!-- Controls -->
    <div class="veil-vb-controls">
      <!-- Mic Toggle -->
      <button
        class="veil-voice-btn"
        class:active={!media.isMuted}
        class:danger={media.isMuted}
        class:speaking={media.isSpeaking && !media.isMuted}
        onclick={() => mediaStore.toggleMute()}
        title={media.isMuted ? 'Mikrofonu Aç' : 'Mikrofonu Kapat'}
        aria-label={media.isMuted ? 'Mikrofonu aç' : 'Mikrofonu kapat'}
        aria-pressed={!media.isMuted}
      >
        <Icon name={media.isMuted ? 'mic-off' : 'mic'} size={18} />
      </button>

      <!-- Deafen Toggle -->
      <button
        class="veil-voice-btn"
        class:active={!media.isDeafened}
        class:danger={media.isDeafened}
        onclick={() => mediaStore.toggleDeafen()}
        title={media.isDeafened ? 'Kulaklığı Aç' : 'Kulaklığı Kapat'}
        aria-label={media.isDeafened ? 'Kulaklığı aç' : 'Kulaklığı kapat'}
        aria-pressed={!media.isDeafened}
      >
        <Icon name={media.isDeafened ? 'volume-x' : 'volume'} size={18} />
      </button>

      <!-- Camera Toggle -->
      <button
        class="veil-voice-btn"
        class:active={media.isCameraOn}
        class:danger={!media.isCameraOn}
        onclick={() => mediaStore.toggleCamera()}
        title={media.isCameraOn ? 'Kamerayı Kapat' : 'Kamerayı Aç'}
        aria-label="Kamerayı aç/kapat"
        aria-pressed={media.isCameraOn}
      >
        <Icon name={media.isCameraOn ? 'camera' : 'camera-off'} size={18} />
      </button>

      <!-- Screen Share Toggle -->
      <button
        class="veil-voice-btn"
        class:active={media.isScreenSharing}
        class:danger={!media.isScreenSharing}
        onclick={() => {
          if (media.isScreenSharing) {
            mediaStore.stopScreenShare();
          } else {
            screenShareModalOpen = true;
          }
        }}
        title="Ekran Paylaşımı"
        aria-label="Ekran paylaşımını aç/kapat"
        aria-pressed={media.isScreenSharing}
      >
        <Icon name="monitor" size={18} />
      </button>

      <!-- Effects Toggle -->
      <button
        class="veil-voice-btn"
        class:active={fx.activeEffect !== null}
        onclick={() => {
          if (!media.isCameraOn) {
            toastStore.info('Efekt kullanmak için kamerayı açman gerekiyor');
            return;
          }
          effectsStore.togglePanel();
        }}
        title={media.isCameraOn ? (fx.activeEffect ? 'Efektler Aktif' : 'Efektler') : 'Efektler (Kamera Kapalı)'}
        aria-label="Efektleri aç/kapat"
        aria-pressed={fx.activeEffect !== null}
      >
        <Icon name="sparkle" size={18} />
      </button>

      <!-- Quick Volume Popover -->
      <div class="veil-vb-popover-wrap">
        <button
          class="veil-voice-btn"
          class:active={audioPopOpen}
          onclick={() => (audioPopOpen = !audioPopOpen)}
          title="Ses Düzeyi & Ayarları"
          aria-label="Ses düzeyi ve ayarları"
        >
          <Icon name="volume" size={18} />
        </button>

        {#if audioPopOpen}
          <div class="veil-audio-pop-backdrop" onclick={() => (audioPopOpen = false)} aria-hidden="true"></div>
          <div class="veil-audio-pop" role="dialog" aria-label="Genel ses düzeyi">
            <div class="veil-audio-pop-header">
              <span class="veil-audio-pop-title">Genel Ses Düzeyi</span>
              <span class="veil-audio-pop-val">%{masterVolume}</span>
            </div>
            <div class="veil-audio-pop-row">
              <Icon name={masterVolume === 0 ? 'volume-x' : 'volume'} size={15} />
              <input
                type="range"
                min="0"
                max="100"
                value={masterVolume}
                oninput={(e) => applyMasterVolume(Number((e.target as HTMLInputElement).value))}
                class="veil-slider"
                aria-label="Genel ses düzeyi"
              />
            </div>
            <button
              class="btn btn-secondary btn-sm"
              style="width:100%; font-size: var(--text-xs); margin-top:4px;"
              onclick={() => { audioPopOpen = false; uiStore.openModal('settings', { tab: 'audio-video' }); }}
            >
              <Icon name="settings" size={13} />
              Gelişmiş Ses Ayarları
            </button>
          </div>
        {/if}
      </div>

      <!-- Leave Call Button -->
      <button
        class="veil-voice-btn danger end-call"
        onclick={() => mediaStore.leaveVoice()}
        title="Aramayı Bitir"
        aria-label="Sesli aramadan ayrıl"
      >
        <Icon name="phone-off" size={18} />
      </button>
    </div>
  </div>

  <ScreenShareModal
    open={screenShareModalOpen}
    onClose={() => (screenShareModalOpen = false)}
  />

  <EffectsPanel />
{/if}

<style>
  .veil-voice-bar {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    padding: 8px 18px;
    background: color-mix(in srgb, var(--veil-bg-elevated, #111420) 92%, transparent);
    backdrop-filter: blur(28px) saturate(1.8);
    -webkit-backdrop-filter: blur(28px) saturate(1.8);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.18));
    border-radius: var(--radius-full, 9999px);
    gap: var(--space-3, 12px);
    z-index: 95;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.1);
    box-sizing: border-box;
    animation: veil-vb-slide-up 0.3s cubic-bezier(0.2, 0, 0, 1);
    max-width: calc(100vw - 40px);
  }
  @keyframes veil-vb-slide-up {
    from { transform: translateY(30px) scale(0.95); opacity: 0; }
    to { transform: translateY(0) scale(1); opacity: 1; }
  }

  .veil-vb-info {
    display: flex;
    align-items: center;
    gap: var(--space-3, 10px);
    min-width: 0;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    padding: 4px 8px;
    border-radius: var(--radius-md, 8px);
    transition: background 0.15s ease;
  }
  .veil-vb-info:hover {
    background: rgba(255, 255, 255, 0.07);
  }

  .veil-vb-signal {
    color: var(--veil-text-muted, #94a3b8);
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
  }
  .veil-vb-signal.good { color: var(--veil-success, #22c55e); }
  .veil-vb-signal.mid { color: var(--veil-warning, #f59e0b); }

  .veil-vb-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .veil-vb-channel {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm, 13px);
    font-weight: 600;
    color: var(--veil-text-primary, #f1f5f9);
    min-width: 0;
  }
  .veil-vb-name-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 220px;
  }

  .veil-vb-status {
    font-size: 11px;
    color: var(--veil-text-muted, #94a3b8);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .veil-vb-e2ee-tag {
    color: var(--veil-brand, #818cf8);
    font-weight: 600;
  }

  .veil-vb-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .veil-vb-popover-wrap {
    position: relative;
    display: inline-flex;
  }

  .veil-audio-pop-backdrop {
    position: fixed;
    inset: 0;
    z-index: 250;
  }
  .veil-audio-pop {
    position: absolute;
    bottom: 50px;
    right: 0;
    width: 240px;
    background: color-mix(in srgb, var(--veil-bg-elevated, #131724) 96%, transparent);
    backdrop-filter: blur(16px);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.15));
    border-radius: var(--radius-xl, 14px);
    padding: var(--space-3, 12px) var(--space-4, 16px);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    z-index: 260;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .veil-audio-pop-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .veil-audio-pop-title {
    font-size: var(--text-xs, 12px);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--veil-text-muted, #94a3b8);
  }
  .veil-audio-pop-val {
    font-size: var(--text-xs, 12px);
    font-weight: 600;
    color: var(--veil-text-primary, #f1f5f9);
    font-variant-numeric: tabular-nums;
  }
  .veil-audio-pop-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-xs, 12px);
    color: var(--veil-text-secondary, #cbd5e1);
  }
  .veil-audio-pop-row .veil-slider {
    flex: 1;
    height: 5px;
    accent-color: var(--veil-brand, #6366f1);
    border-radius: 9999px;
    cursor: pointer;
  }

  @media (max-width: 768px) {
    .veil-voice-bar {
      left: 50%;
      transform: translateX(-50%);
      padding: 6px 12px;
      gap: 8px;
      bottom: 16px;
    }
    .veil-vb-name-text {
      max-width: 120px;
    }
    .veil-vb-controls {
      gap: 4px;
    }
    .veil-voice-btn {
      width: 38px;
      height: 38px;
    }
  }
</style>

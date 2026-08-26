<script lang="ts">
  import { onMount } from 'svelte';
  import type { Room, RemoteParticipant } from 'livekit-client';
  import { Track } from 'livekit-client';
  import { mediaStore } from '$lib/stores/media';
  import { spaceStore } from '$lib/stores/spaces';
  import { uiStore } from '$lib/stores/ui';
  import { channelNameFor } from '$lib/utils/channel';
  import Icon from '../ui/Icon.svelte';
  import ParticipantGrid from './ParticipantGrid.svelte';
  import ScreenShare from './ScreenShare.svelte';
  import E2eeCallBadge from './E2eeCallBadge.svelte';
  import ScreenShareModal from './ScreenShareModal.svelte';
  import EffectsPanel from './EffectsPanel.svelte';
  import { effectsStore } from '$lib/effects/store';
  import { toastStore } from '$lib/stores/notifications';
  import type { EffectBroadcastPayload } from '$lib/effects/types';
  import { onEffectBroadcast, startBroadcastLoop, stopBroadcastLoop } from '$lib/effects/broadcast';

  const media = $derived($mediaStore);
  const spaces = $derived($spaceStore);
  const ui = $derived($uiStore);
  const fx = $derived($effectsStore);

  const resolvedChannelName = $derived(
    (function() {
      const name = channelNameFor(spaces.channelsBySpace, spaces.dmChannels, ui.activeSpaceId, media.channelId);
      if (name && name.length === 36 && name.includes('-')) return 'Ses Kanalı';
      return name || 'Ses Kanalı';
    })()
  );

  let containerEl = $state<HTMLDivElement | null>(null);
  let room = $state<Room | null>(null);
  let remotes = $state<RemoteParticipant[]>([]);
  let remoteScreenSharer = $state<RemoteParticipant | null>(null);
  let screenShareModalOpen = $state(false);
  let isFullscreen = $state(false);
  let viewMode = $state<'grid' | 'speaker' | 'stage' | 'focus'>('grid');

  function setViewMode(mode: 'grid' | 'speaker' | 'stage' | 'focus') {
    viewMode = mode;
  }
  let audioPopOpen = $state(false);
  let masterVolume = $state(100);

  // Katılım bildirimi state
  let joinedHint = $state(false);
  let joinedHintTimer: ReturnType<typeof setTimeout> | null = null;

  // Remote participant effect broadcasts (identity → payload)
  let remoteEffects = $state<Map<string, EffectBroadcastPayload>>(new Map());
  let broadcastCleanup = $state<(() => void) | null>(null);

  function syncParticipantsAndTracks() {
    const r = mediaStore.getRoom();
    room = r;
    if (!r) {
      remotes = [];
      remoteScreenSharer = null;
      return;
    }

    remotes = Array.from(r.remoteParticipants.values());
    const sharer = remotes.find(p => {
      const pub = p.getTrackPublication(Track.Source.ScreenShare);
      return pub && (pub.isSubscribed || !pub.isMuted) && pub.track;
    });
    remoteScreenSharer = sharer ?? null;
  }

  // Reactive effect to watch room and participant updates + DataChannel wiring
  $effect(() => {
    const currentRoom = mediaStore.getRoom();
    if (currentRoom !== room) {
      // Cleanup previous broadcast listeners
      broadcastCleanup?.();
      broadcastCleanup = null;
      remoteEffects = new Map();

      room = currentRoom;
      syncParticipantsAndTracks();

      // Wire DataChannel broadcast for new room
      if (currentRoom) {
        startBroadcastLoop(currentRoom);
        const unsub = onEffectBroadcast(currentRoom, (payload) => {
          remoteEffects = new Map(remoteEffects).set(payload.userId, payload);
        });
        broadcastCleanup = () => {
          unsub();
          stopBroadcastLoop();
        };
      }
    }
  });

  onMount(() => {
    effectsStore.resetSession();
    syncParticipantsAndTracks();

    // Katılım bildirimi: 3sn göster, sonra kaybol
    joinedHint = true;
    if (joinedHintTimer) clearTimeout(joinedHintTimer);
    joinedHintTimer = setTimeout(() => { joinedHint = false; }, 3000);

    const interval = setInterval(syncParticipantsAndTracks, 200);

    const onFullscreenChange = () => {
      isFullscreen = !!document.fullscreenElement;
    };
    document.addEventListener('fullscreenchange', onFullscreenChange);

    return () => {
      clearInterval(interval);
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      if (joinedHintTimer) clearTimeout(joinedHintTimer);
      broadcastCleanup?.();
      stopBroadcastLoop();
      effectsStore.resetSession();
    };
  });

  const localHasScreenTrack = $derived(
    media.isScreenSharing && (room?.localParticipant.isScreenShareEnabled ?? false)
  );

  const hasAnyScreenShare = $derived(
    localHasScreenTrack || !!remoteScreenSharer
  );

  let prevHasScreenShare = $state(false);
  // Auto-switch to stage when a new screen share starts, or back to grid when it ends
  $effect(() => {
    if (hasAnyScreenShare && !prevHasScreenShare) {
      if (viewMode === 'grid') {
        viewMode = 'stage';
      }
    } else if (!hasAnyScreenShare && prevHasScreenShare) {
      if (viewMode === 'stage' || viewMode === 'focus') {
        viewMode = 'grid';
      }
    }
    prevHasScreenShare = hasAnyScreenShare;
  });

  async function toggleFullscreen() {
    if (!containerEl) return;
    if (!document.fullscreenElement) {
      await containerEl.requestFullscreen().catch(() => {});
    } else {
      await document.exitFullscreen().catch(() => {});
    }
  }

  function applyMasterVolume(val: number) {
    masterVolume = val;
    mediaStore.setMasterVolume(val / 100);
  }
</script>

<div
  class="veil-video-call"
  bind:this={containerEl}
  role="region"
  aria-label="Görüntülü görüşme"
>
  <!-- Top Bar -->
  <header class="veil-vc-topbar">
    <div class="veil-vc-topbar-left">
      <E2eeCallBadge />
      <span class="veil-vc-room">
        <Icon name="volume" size={14} />
        <span class="veil-vc-room-title">{resolvedChannelName}</span>
        <span class="veil-vc-count">· {Math.max(1, remotes.length + 1)} kişi</span>
      </span>
    </div>

    <div class="veil-vc-topbar-right">
      <!-- Interactive View Switcher -->
      <div class="veil-view-switcher" role="group" aria-label="Görünüm modu">
        {#if hasAnyScreenShare}
          <button
            class="veil-view-btn"
            class:active={viewMode === 'stage'}
            onclick={() => setViewMode('stage')}
            title="Sahne Modu (Yayın + Katılımcılar)"
          >
            <Icon name="layout" size={13} />
            <span>Sahne</span>
          </button>
          <button
            class="veil-view-btn"
            class:active={viewMode === 'focus'}
            onclick={() => setViewMode('focus')}
            title="Yayın Odak Modu"
          >
            <Icon name="monitor" size={13} />
            <span>Yayın</span>
          </button>
        {/if}

        <button
          class="veil-view-btn"
          class:active={viewMode === 'grid'}
          onclick={() => setViewMode('grid')}
          title="Izgara Modu"
        >
          <Icon name="grid" size={13} />
          <span>Izgara</span>
        </button>

        <button
          class="veil-view-btn"
          class:active={viewMode === 'speaker'}
          onclick={() => setViewMode('speaker')}
          title="Konuşmacı / Odak Modu"
        >
          <Icon name="user" size={13} />
          <span>Konuşmacı</span>
        </button>
      </div>

      <button
        class="veil-topbar-action-btn"
        onclick={toggleFullscreen}
        title={isFullscreen ? 'Tam Ekrandan Çık' : 'Tam Ekran Yap'}
        aria-label="Tam ekran"
      >
        <Icon name={isFullscreen ? 'minimize-2' : 'maximize-2'} size={15} />
      </button>
    </div>
  </header>

  <!-- Main Stage Content -->
  <main class="veil-vc-content">
    {#if viewMode === 'stage' && hasAnyScreenShare}
      <div class="veil-vc-stage">
        <div class="veil-vc-stage-main">
          {#if localHasScreenTrack}
            <ScreenShare participant={room?.localParticipant ?? null} isLocal />
          {:else if remoteScreenSharer}
            <ScreenShare participant={remoteScreenSharer} />
          {/if}
        </div>
        <div class="veil-vc-stage-side">
          <ParticipantGrid
            participants={remotes}
            localParticipant={room?.localParticipant ?? null}
            {room}
            compact
            {remoteEffects}
          />
        </div>
      </div>
    {:else if viewMode === 'focus' && hasAnyScreenShare}
      <div class="veil-vc-focus">
        {#if localHasScreenTrack}
          <ScreenShare participant={room?.localParticipant ?? null} isLocal />
        {:else if remoteScreenSharer}
          <ScreenShare participant={remoteScreenSharer} />
        {/if}
      </div>
    {:else}
      <div class="veil-vc-grid-wrap">
        <ParticipantGrid
          participants={remotes}
          localParticipant={room?.localParticipant ?? null}
          {room}
          viewMode={viewMode === 'speaker' ? 'speaker' : 'grid'}
          {remoteEffects}
        />
        {#if joinedHint && remotes.length === 0 && media.connectionState === 'connected'}
          <div class="veil-vc-empty-hint joined-hint" role="status">
            Ses kanalına bağlandın — diğer üyeler katıldığında burada görünür.
          </div>
        {:else if !joinedHint && remotes.length === 0 && media.connectionState === 'connected'}
          <div class="veil-vc-empty-hint faded" role="status">
            Ses kanalına bağlandın — diğer üyeler katıldığında burada görünür.
          </div>
        {/if}
      </div>
    {/if}
  </main>

  <!-- Centered Floating Glass Controls Dock -->
  <div class="veil-voice-controls" role="toolbar" aria-label="Arama kontrolleri">
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
      <Icon name={media.isMuted ? 'mic-off' : 'mic'} size={19} />
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
      <Icon name={media.isDeafened ? 'volume-x' : 'volume'} size={19} />
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
      <Icon name={media.isCameraOn ? 'camera' : 'camera-off'} size={19} />
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
      <Icon name="monitor" size={19} />
    </button>

    <!-- Effects Toggle -->
    <button
      class="veil-voice-btn"
      class:active={fx.activeEffects.length > 0 || fx.activeEffect !== null}
      onclick={() => {
        if (!media.isCameraOn) {
          toastStore.info('Efekt kullanmak için kamerayı açman gerekiyor');
          return;
        }
        effectsStore.togglePanel();
      }}
      title={media.isCameraOn ? (fx.activeEffects.length > 0 ? `${fx.activeEffects.length} Efekt Aktif` : 'Efektler') : 'Efektler (Kamera Kapalı)'}
      aria-label="Efektleri aç/kapat"
      aria-pressed={fx.activeEffects.length > 0 || fx.activeEffect !== null}
    >
      <Icon name="sparkle" size={19} />
    </button>

    <!-- Master Volume Popover -->
    <div class="veil-vc-popover-wrap">
      <button
        class="veil-voice-btn"
        class:active={audioPopOpen}
        onclick={() => (audioPopOpen = !audioPopOpen)}
        title="Ses Düzeyi & Ayarları"
        aria-label="Ses düzeyi ve ayarları"
      >
        <Icon name="volume" size={19} />
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

    <!-- Disconnect / End Call Button -->
    <button
      class="veil-voice-btn danger end-call"
      onclick={() => mediaStore.leaveVoice()}
      title="Aramayı Bitir"
      aria-label="Sesli aramadan ayrıl"
    >
      <Icon name="phone-off" size={19} />
    </button>
  </div>

  <ScreenShareModal
    open={screenShareModalOpen}
    onClose={() => (screenShareModalOpen = false)}
  />

  <EffectsPanel />
</div>

<style>
  .veil-video-call {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    min-height: 0;
    min-width: 0;
    background: var(--veil-bg-void, #08090f);
    position: relative;
    overflow: hidden;
  }

  .veil-vc-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3, 12px);
    padding: 8px 16px;
    border-bottom: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
    background: color-mix(in srgb, var(--veil-bg-surface, #0f121d) 94%, transparent);
    backdrop-filter: blur(12px);
    flex-shrink: 0;
    z-index: 10;
  }

  .veil-vc-topbar-left {
    display: flex;
    align-items: center;
    gap: var(--space-3, 12px);
    min-width: 0;
  }

  .veil-vc-topbar-right {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
  }

  .veil-vc-room {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-sm, 13px);
    font-weight: 600;
    color: var(--veil-text-primary, #f8fafc);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-vc-count {
    color: var(--veil-text-muted, #94a3b8);
    font-weight: 500;
    font-size: var(--text-xs, 12px);
  }

  .veil-view-switcher {
    display: inline-flex;
    align-items: center;
    background: var(--veil-bg-elevated, #161b2a);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.1));
    border-radius: var(--radius-md, 8px);
    padding: 3px;
    gap: 3px;
  }

  .veil-view-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: var(--text-xs, 12px);
    font-weight: 600;
    color: var(--veil-text-secondary, #94a3b8);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.2, 0, 0, 1);
  }

  .veil-view-btn:hover {
    color: var(--veil-text-primary, #f8fafc);
    background: rgba(255, 255, 255, 0.08);
  }

  .veil-view-btn.active {
    color: #fff;
    background: var(--veil-brand, #7c3aed);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--veil-brand, #7c3aed) 40%, transparent);
  }

  .veil-topbar-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-md, 8px);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.1));
    background: var(--veil-bg-elevated, #161b2a);
    color: var(--veil-text-secondary, #94a3b8);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-topbar-action-btn:hover {
    color: var(--veil-text-primary, #f8fafc);
    border-color: var(--veil-border-prominent, rgba(255, 255, 255, 0.2));
    background: var(--veil-bg-surface, #1e2438);
  }

  .veil-vc-content {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    width: 100%;
    height: 100%;
    position: relative;
    overflow: hidden;
  }

  .veil-vc-grid-wrap,
  .veil-vc-focus {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
    padding: var(--space-2, 8px);
    box-sizing: border-box;
  }

  .veil-vc-empty-hint {
    position: absolute;
    bottom: 84px;
    top: auto;
    left: 50%;
    transform: translateX(-50%);
    max-width: 90%;
    padding: 8px 18px;
    border-radius: var(--radius-full, 9999px);
    font-size: var(--text-xs, 12px);
    font-weight: 500;
    color: var(--veil-text-secondary, #cbd5e1);
    background: color-mix(in srgb, var(--veil-bg-elevated, #111420) 90%, transparent);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.15));
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    text-align: center;
    pointer-events: none;
    z-index: 25;
  }

  .veil-vc-empty-hint.joined-hint {
    animation: veil-hint-enter 0.3s cubic-bezier(0.16, 1, 0.3, 1), veil-hint-fadeout 0.7s cubic-bezier(0.16, 1, 0.3, 1) 2.3s forwards;
    transform: translateX(-50%) translateY(0);
  }

  .veil-vc-empty-hint.faded {
    opacity: 0;
    visibility: hidden;
  }

  @keyframes veil-hint-enter {
    from { opacity: 0; transform: translateX(-50%) translateY(10px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  @keyframes veil-hint-fadeout {
    0% { opacity: 1; transform: translateX(-50%) translateY(0); }
    100% { opacity: 0; transform: translateX(-50%) translateY(-8px); visibility: hidden; pointer-events: none; }
  }

  @keyframes veil-ring-pulse {
    0%, 100% { transform: scale(1); box-shadow: 0 0 0 0 color-mix(in srgb, var(--veil-brand, #8b5cf6) 30%, transparent); }
    50% { transform: scale(1.06); box-shadow: 0 0 0 12px transparent; }
  }

  .veil-vc-stage {
    display: flex;
    flex: 1;
    min-height: 0;
    min-width: 0;
    gap: var(--space-3, 12px);
    padding: var(--space-3, 12px);
    box-sizing: border-box;
    overflow: hidden;
    width: 100%;
    height: 100%;
  }

  .veil-vc-stage-main {
    flex: 3;
    min-width: 0;
    min-height: 0;
    display: flex;
    border-radius: var(--radius-xl, 14px);
    overflow: hidden;
    background: #000;
  }

  .veil-vc-stage-side {
    flex: 1;
    min-width: 240px;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--veil-bg-elevated, #111420);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.1));
    border-radius: var(--radius-xl, 14px);
    overflow-y: auto;
    overflow-x: hidden;
    box-sizing: border-box;
  }

  /* Centered Floating Glass Controls Dock */
  .veil-voice-controls {
    position: absolute;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background: color-mix(in srgb, var(--veil-bg-elevated, #111420) 90%, transparent);
    backdrop-filter: blur(20px) saturate(1.8);
    -webkit-backdrop-filter: blur(20px) saturate(1.8);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.15));
    border-radius: var(--radius-full, 9999px);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(255, 255, 255, 0.08);
    z-index: 40;
    will-change: opacity;
  }

  .veil-voice-btn {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--veil-bg-surface, #1e2438);
    color: var(--veil-text-primary, #f1f5f9);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.1));
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.2, 0, 0, 1);
  }

  .veil-voice-btn:hover {
    transform: translateY(-2px) scale(1.05);
    background: color-mix(in srgb, var(--veil-bg-surface, #1e2438) 80%, white);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
  }

  .veil-voice-btn.active {
    background: var(--veil-brand, #7c3aed);
    color: #fff;
    border-color: var(--veil-brand, #7c3aed);
    box-shadow: 0 4px 14px color-mix(in srgb, var(--veil-brand, #7c3aed) 40%, transparent);
  }

  .veil-voice-btn.danger {
    background: var(--veil-danger, #ef4444);
    color: #fff;
    border-color: var(--veil-danger, #ef4444);
  }

  .veil-voice-btn.danger.end-call {
    background: #ef4444;
    color: #fff;
    box-shadow: 0 4px 14px rgba(239, 68, 68, 0.45);
  }

  .veil-voice-btn.danger.end-call:hover {
    background: #dc2626;
    box-shadow: 0 6px 20px rgba(239, 68, 68, 0.6);
  }

  .veil-vc-popover-wrap {
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
    bottom: 58px;
    left: 50%;
    transform: translateX(-50%);
    width: 240px;
    background: color-mix(in srgb, var(--veil-bg-elevated, #131724) 96%, transparent);
    backdrop-filter: blur(20px);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.15));
    border-radius: var(--radius-xl, 14px);
    padding: var(--space-3, 12px) var(--space-4, 16px);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.65);
    z-index: 260;
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 10px);
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
    gap: var(--space-2, 8px);
    font-size: var(--text-xs, 12px);
    color: var(--veil-text-secondary, #cbd5e1);
  }
  .veil-audio-pop-row .veil-slider {
    flex: 1;
    height: 5px;
    accent-color: var(--veil-brand, #7c3aed);
    border-radius: var(--radius-full, 9999px);
    cursor: pointer;
  }

  @media (max-width: 860px), (orientation: portrait) {
    .veil-vc-stage {
      flex-direction: column;
      gap: var(--space-2, 8px);
      padding: var(--space-2, 8px);
    }
    .veil-vc-stage-main {
      flex: 2;
      width: 100%;
    }
    .veil-vc-stage-side {
      flex: 1;
      width: 100%;
      min-width: 100%;
      max-width: 100%;
      max-height: 40%;
    }
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { RoomEvent, Track } from 'livekit-client';
  import type { Participant, Room, Track as TrackType } from 'livekit-client';
  import { mediaStore } from '$lib/stores/media';
  import { authStore } from '$lib/stores/auth';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore } from '$lib/stores/notifications';
  import { settingsApi } from '$lib/api/tauri';
  import Avatar from '../ui/Avatar.svelte';
  import Icon from '../ui/Icon.svelte';
  import ContextMenu, { type ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import EffectsCanvas from './EffectsCanvas.svelte';
  import { effectsStore } from '$lib/effects/store';
  import type { EffectBroadcastPayload, TrackingResult } from '$lib/effects/types';
  import { getEffect } from '$lib/effects/effects';

  let {
    participant,
    isLocal = false,
    room = null,
    compact = false,
    remoteEffect = null,
  }: {
    participant: Participant;
    isLocal?: boolean;
    room?: Room | null;
    compact?: boolean;
    remoteEffect?: EffectBroadcastPayload | null;
  } = $props();

  const auth = $derived($authStore);
  const media = $derived($mediaStore);
  const fx = $derived($effectsStore);

  let videoEl = $state<HTMLVideoElement | null>(null);
  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);
  let volumePopOpen = $state(false);
  let userVolume = $state(100);
  let isMirrored = $state(true);

  let isTrackAttached = $state(false);
  let remoteVideoActive = $state(false);
  let remoteMicActive = $state(true);
  let remoteSpeaking = $state(false);

  onMount(() => {
    settingsApi.get().then((s) => {
      if (s.mirrorCamera !== undefined) {
        isMirrored = s.mirrorCamera;
      }
    }).catch(() => {});
  });

  const isVideoOn = $derived(isLocal ? media.isCameraOn : remoteVideoActive);
  const isMicOn = $derived(isLocal ? !media.isMuted : remoteMicActive);
  const isSpeaking = $derived(
    isLocal
      ? (!media.isMuted && media.isSpeaking)
      : (remoteSpeaking && remoteMicActive)
  );

  // Remote effect overlay state
  let remoteCanvasEl = $state<HTMLCanvasElement | null>(null);
  const hasRemoteEffect = $derived(!isLocal && !!remoteEffect?.effectId);
  let remoteAnimFrame = $state<number | null>(null);

  /** Decompress flattened number[] back to Landmark[]. */
  function decompressLandmarks(arr: number[]): { x: number; y: number; z: number }[] {
    const out: { x: number; y: number; z: number }[] = [];
    for (let i = 0; i + 2 < arr.length; i += 3) {
      out.push({ x: arr[i], y: arr[i + 1], z: arr[i + 2] });
    }
    return out;
  }

  /** Build a TrackingResult from broadcast payload landmarks. */
  function buildTrackingFromBroadcast(payload: EffectBroadcastPayload): TrackingResult {
    return {
      face: payload.landmarks?.face ? [decompressLandmarks(payload.landmarks.face)] : undefined,
      hands: payload.landmarks?.hands ? [decompressLandmarks(payload.landmarks.hands)] : undefined,
      pose: payload.landmarks?.pose ? [decompressLandmarks(payload.landmarks.pose)] : undefined,
      timestamp: performance.now(),
    };
  }

  function renderRemoteEffect() {
    if (!remoteCanvasEl || !remoteEffect?.effectId) return;
    const effect = getEffect(remoteEffect.effectId);
    if (!effect) return;
    const ctx = remoteCanvasEl.getContext('2d');
    if (!ctx) return;
    const w = remoteCanvasEl.width;
    const h = remoteCanvasEl.height;
    if (w <= 0 || h <= 0) return;
    const tracking = buildTrackingFromBroadcast(remoteEffect);
    ctx.clearRect(0, 0, w, h);
    try {
      effect.process(ctx, w, h, tracking, remoteEffect.params, performance.now());
    } catch { /* effect render error — ignore */ }
    remoteAnimFrame = requestAnimationFrame(renderRemoteEffect);
  }

  $effect(() => {
    if (hasRemoteEffect && remoteCanvasEl) {
      remoteAnimFrame = requestAnimationFrame(renderRemoteEffect);
    }
    return () => {
      if (remoteAnimFrame) {
        cancelAnimationFrame(remoteAnimFrame);
        remoteAnimFrame = null;
      }
    };
  });

  const participantName = $derived(
    isLocal
      ? (auth.identity?.displayName || auth.identity?.username || 'Sen')
      : (participant.name ?? participant.identity)
  );

  const avatarHash = $derived.by(() => {
    if (isLocal) return auth.identity?.avatarHash ?? null;
    try {
      if (participant.metadata) {
        const meta = JSON.parse(participant.metadata);
        return meta.avatarHash || meta.avatar_hash || null;
      }
    } catch { /* ignored */ }
    return null;
  });

  const speakingColor = $derived.by(() => {
    // Her kullanıcı kendi tema rengini kullanır
    if (isLocal) {
      return 'var(--veil-accent, var(--veil-brand, #7c3aed))';
    }
    if (participant.metadata) {
      try {
        const meta = JSON.parse(participant.metadata);
        if (meta.accentColor || meta.themeColor || meta.roleColor) {
          return meta.accentColor || meta.themeColor || meta.roleColor;
        }
      } catch { /* ignored */ }
    }
    return 'var(--veil-accent, var(--veil-brand, #7c3aed))';
  });

  const avatarSize = $derived(compact ? 'md' : 'xl');

  let attachedTrack: TrackType | null = null;
  let retryTimer: ReturnType<typeof setInterval> | null = null;

  function stopRetryTimer() {
    if (retryTimer) {
      clearInterval(retryTimer);
      retryTimer = null;
    }
  }

  function syncVideoAttachment() {
    const node = videoEl;
    if (!node) return;

    if (isLocal) {
      if (!media.isCameraOn) {
        stopRetryTimer();
        if (attachedTrack) {
          try { attachedTrack.detach(node); } catch { /* best effort */ }
          attachedTrack = null;
        }
        node.srcObject = null;
        isTrackAttached = false;
        return;
      }

      // Camera is ON locally — find local camera track
      const lp = room?.localParticipant ?? (typeof (participant as any)?.getTrackPublication === 'function' ? participant : null);
      const pub = lp?.getTrackPublication?.(Track.Source.Camera);
      const track = pub?.track ?? (pub as any)?.videoTrack;

      if (track && track.kind === 'video') {
        stopRetryTimer();
        if (attachedTrack !== track || !isTrackAttached) {
          if (attachedTrack && attachedTrack !== track) {
            try { attachedTrack.detach(node); } catch { /* best effort */ }
          }
          attachedTrack = track as TrackType;
          try {
            (track as TrackType).attach(node);
          } catch {
            if ((track as any).mediaStreamTrack) {
              node.srcObject = new MediaStream([(track as any).mediaStreamTrack]);
            }
          }
        }
        node.muted = true;
        node.autoplay = true;
        node.playsInline = true;
        if (node.paused) {
          void node.play().catch(() => {});
        }
        isTrackAttached = true;
      } else {
        // Track not yet published in WebRTC room, start quick retry loop
        if (!retryTimer) {
          let attempts = 0;
          retryTimer = setInterval(() => {
            attempts++;
            const currLp = room?.localParticipant;
            const currPub = currLp?.getTrackPublication(Track.Source.Camera);
            const currTrack = currPub?.track ?? (currPub as any)?.videoTrack;
            if (currTrack && currTrack.kind === 'video' && videoEl) {
              stopRetryTimer();
              syncVideoAttachment();
            } else if (attempts > 60 || !media.isCameraOn) {
              stopRetryTimer();
            }
          }, 80);
        }
      }
    } else {
      // Remote participant
      const p = participant;
      if (!p) return;
      const camPub = p.getTrackPublication(Track.Source.Camera);
      const isSubbed = Boolean(camPub?.isEnabled && camPub?.isSubscribed && camPub?.track && camPub.track.kind === 'video');
      remoteVideoActive = isSubbed;
      remoteMicActive = p.isMicrophoneEnabled;
      remoteSpeaking = p.isSpeaking;

      if (isSubbed && camPub?.track) {
        if (attachedTrack !== camPub.track) {
          if (attachedTrack) {
            try { attachedTrack.detach(node); } catch { /* best effort */ }
          }
          attachedTrack = camPub.track;
          try { camPub.track.attach(node); } catch { /* best effort */ }
        }
        node.muted = false;
        node.autoplay = true;
        node.playsInline = true;
        if (node.paused) {
          void node.play().catch(() => {});
        }
        isTrackAttached = true;
      } else if (attachedTrack) {
        try { attachedTrack.detach(node); } catch { /* best effort */ }
        attachedTrack = null;
        isTrackAttached = false;
      }
    }
  }

  // Reactive effect to keep video element in sync whenever camera, participant or room updates
  $effect(() => {
    const _cam = media.isCameraOn;
    const _p = participant;
    const _r = room;
    const _v = videoEl;
    if (_v) {
      syncVideoAttachment();
    }
  });

  function setupVideoElement(node: HTMLVideoElement) {
    videoEl = node;
    syncVideoAttachment();

    const p = participant;
    const onTrackEvent = () => {
      syncVideoAttachment();
    };

    if (p && typeof (p as any).on === 'function') {
      (p as any).on(RoomEvent.TrackSubscribed, onTrackEvent);
      (p as any).on(RoomEvent.TrackUnsubscribed, onTrackEvent);
      (p as any).on(RoomEvent.TrackMuted, onTrackEvent);
      (p as any).on(RoomEvent.TrackUnmuted, onTrackEvent);
      (p as any).on('isSpeakingChanged' as any, onTrackEvent);
    }
    if (room && typeof room.on === 'function') {
      room.on(RoomEvent.LocalTrackPublished, onTrackEvent);
      room.on(RoomEvent.LocalTrackUnpublished, onTrackEvent);
      room.on(RoomEvent.TrackSubscribed, onTrackEvent);
      room.on(RoomEvent.TrackUnsubscribed, onTrackEvent);
    }

    return {
      update() {
        syncVideoAttachment();
      },
      destroy() {
        stopRetryTimer();
        if (p && typeof (p as any).off === 'function') {
          (p as any).off(RoomEvent.TrackSubscribed, onTrackEvent);
          (p as any).off(RoomEvent.TrackUnsubscribed, onTrackEvent);
          (p as any).off(RoomEvent.TrackMuted, onTrackEvent);
          (p as any).off(RoomEvent.TrackUnmuted, onTrackEvent);
          (p as any).off('isSpeakingChanged' as any, onTrackEvent);
        }
        if (room && typeof room.off === 'function') {
          room.off(RoomEvent.LocalTrackPublished, onTrackEvent);
          room.off(RoomEvent.LocalTrackUnpublished, onTrackEvent);
          room.off(RoomEvent.TrackSubscribed, onTrackEvent);
          room.off(RoomEvent.TrackUnsubscribed, onTrackEvent);
        }
        if (attachedTrack) {
          try { attachedTrack.detach(node); } catch { /* best effort */ }
          attachedTrack = null;
        }
        node.srcObject = null;
        isTrackAttached = false;
      }
    };
  }

  async function toggleMirror() {
    const next = !isMirrored;
    isMirrored = next;
    try {
      await settingsApi.update({ mirrorCamera: next });
    } catch { /* best effort */ }
  }

  function applyVolume(vol: number) {
    userVolume = vol;
    if (!isLocal) {
      mediaStore.setParticipantVolume(participant.sid, vol / 100);
    }
  }

  function openContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();

    if (isLocal) {
      menuItems = [
        {
          label: media.isMuted ? 'Mikrofonu Aç' : 'Mikrofonu Kapat',
          icon: media.isMuted ? 'mic-off' : 'mic',
          onClick: () => void mediaStore.toggleMute(),
        },
        {
          label: media.isCameraOn ? 'Kamerayı Kapat' : 'Kamerayı Aç',
          icon: media.isCameraOn ? 'camera' : 'video-off',
          onClick: () => void mediaStore.toggleCamera(),
        },
        {
          label: isMirrored ? 'Kamera Aynalamayı Kapat' : 'Kamerayı Aynala',
          icon: 'camera',
          onClick: () => void toggleMirror(),
        },
        {
          label: media.isDeafened ? 'Kulaklığı Aç' : 'Kulaklığı Kapat',
          icon: 'volume',
          onClick: () => void mediaStore.toggleDeafen(),
        },
        { label: '', separator: true },
        {
          label: 'Kullanıcı Adını Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(`@${auth.identity?.username || participantName}`);
            toastStore.success('Kullanıcı adı kopyalandı.');
          },
        },
        {
          label: 'Ses & Görüntü Ayarları',
          icon: 'settings',
          onClick: () => uiStore.openModal('settings'),
        },
      ];
    } else {
      menuItems = [
        {
          label: 'Kullanıcı Sesi',
          icon: 'volume',
          isSlider: true,
          sliderValue: userVolume,
          sliderMin: 0,
          sliderMax: 200,
          onSliderChange: (val: number) => {
            applyVolume(val);
          },
        },
        {
          label: userVolume === 0 ? 'Sesi Aç' : 'Kullanıcıyı Sustur',
          icon: userVolume === 0 ? 'volume' : 'volume-x',
          onClick: () => applyVolume(userVolume === 0 ? 100 : 0),
        },
        { label: '', separator: true },
        {
          label: 'Profili Gör',
          icon: 'user',
          onClick: () => {
            uiStore.openModal('user-profile', {
              userId: participant.identity,
              username: participant.name ?? participant.identity,
              displayName: participant.name ?? participant.identity,
              avatarHash,
            });
          },
        },
        {
          label: 'Kullanıcı Adını Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(`@${participant.name ?? participant.identity}`);
            toastStore.success('Kullanıcı adı kopyalandı.');
          },
        },
        {
          label: 'Kullanıcı ID\'sini Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(participant.identity);
            toastStore.success('Kullanıcı ID\'si kopyalandı.');
          },
        },
      ];
    }
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }
</script>

<div
  class="veil-participant-tile"
  class:speaking={isSpeaking}
  class:video-on={isVideoOn && isTrackAttached}
  class:compact
  style="--speaking-ring-color: {speakingColor};"
  oncontextmenu={openContextMenu}
  role="region"
  aria-label={participantName}
>
  <video
    class="veil-tile-video"
    class:mirrored={isLocal && isMirrored}
    use:setupVideoElement
    aria-hidden="true"
  ></video>

  {#if isLocal && (fx.activeEffects.length > 0 || fx.activeEffect)}
    <EffectsCanvas videoElement={videoEl} mirrored={isLocal && isMirrored} />
  {/if}

  {#if hasRemoteEffect}
    <canvas
      bind:this={remoteCanvasEl}
      class="veil-remote-fx-canvas"
      width={640}
      height={480}
      aria-hidden="true"
    ></canvas>
    <div class="veil-fx-badge" aria-label="Uzaktan efekt aktif">
      <span class="veil-fx-badge-text">FX</span>
    </div>
  {/if}

  <div class="veil-tile-fallback" aria-hidden="true">
    <div class="veil-tile-avatar-wrap" class:speaking={isSpeaking}>
      <Avatar
        name={participantName}
        size={avatarSize}
        hash={avatarHash}
        speaking={isSpeaking}
        themeColor={speakingColor}
      />
    </div>
  </div>

  <div class="veil-tile-meta">
    <span class="veil-tile-name">
      <span class="veil-tile-label">{isLocal ? 'Sen' : participantName}</span>
      {#if isSpeaking}
        <span class="veil-tile-speaking" title="Konuşuyor" aria-label="Konuşuyor">
          <span class="veil-pulse-wave"></span>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6.5 8.5H3.5v7h3L11 19Z"/><path d="M15.5 8.5a5 5 0 0 1 0 7"/><path d="M18 6a8.5 8.5 0 0 1 0 12"/></svg>
        </span>
      {/if}
      {#if !isMicOn}
        <span class="veil-tile-muted" title="Sesi kapalı" aria-label="Sesi kapalı">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 2 2 22"/><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11.5a6.5 6.5 0 0 0 13 0"/><path d="M12 18v3"/></svg>
        </span>
      {/if}
      {#if !isLocal && userVolume !== 100}
        <span class="veil-tile-vol-badge">%{userVolume}</span>
      {/if}
    </span>
  </div>

  {#if volumePopOpen}
    <div class="veil-vol-pop-backdrop" onclick={() => (volumePopOpen = false)} aria-hidden="true"></div>
    <div class="veil-vol-pop" role="dialog" aria-label="Ses seviyesi ayarı">
      <div class="veil-vol-pop-title">{participantName} Ses Seviyesi</div>
      <div class="veil-vol-pop-row">
        <input
          type="range"
          min="0"
          max="200"
          value={userVolume}
          oninput={(e) => applyVolume(Number((e.target as HTMLInputElement).value))}
          class="veil-slider"
        />
        <span class="veil-vol-pop-val">%{userVolume}</span>
      </div>
      <button class="btn btn-primary btn-sm" onclick={() => (volumePopOpen = false)}>Tamam</button>
    </div>
  {/if}
</div>

<ContextMenu open={menuOpen} x={menuX} y={menuY} items={menuItems} onClose={() => (menuOpen = false)} />

<style>
  .veil-participant-tile {
    position: relative;
    border-radius: var(--radius-xl, 14px);
    background: var(--veil-bg-elevated, #111420);
    overflow: hidden;
    border: 2px solid transparent;
    transition: border-color 0.15s ease, box-shadow 0.15s ease, transform 0.15s ease;
    width: 100%;
    height: 100%;
    max-width: 100%;
    max-height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    box-sizing: border-box;
  }
  .veil-participant-tile.speaking {
    border-color: var(--speaking-ring-color, var(--veil-brand, #7c3aed));
    box-shadow:
      0 0 0 3px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 35%, transparent),
      0 0 20px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 40%, transparent),
      0 8px 24px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 25%, transparent);
    animation: veil-speaking-glow 1.5s ease-in-out infinite;
  }
  @keyframes veil-speaking-glow {
    0%, 100% {
      box-shadow:
        0 0 0 2px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 30%, transparent),
        0 0 16px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 35%, transparent),
        0 6px 20px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 20%, transparent);
    }
    50% {
      box-shadow:
        0 0 0 3px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 45%, transparent),
        0 0 28px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 55%, transparent),
        0 10px 30px color-mix(in srgb, var(--speaking-ring-color, var(--veil-brand, #7c3aed)) 30%, transparent);
    }
  }
  .veil-tile-video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: none;
    background: #000;
  }
  .veil-tile-video.mirrored {
    transform: scaleX(-1);
  }
  .veil-participant-tile.video-on .veil-tile-video { display: block; }
  .veil-participant-tile.video-on .veil-tile-fallback { display: none; }
  .veil-tile-fallback {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    background:
      radial-gradient(100% 100% at 50% 40%, rgba(99, 102, 241, 0.15), transparent 75%),
      linear-gradient(160deg, #131724, #080a10);
  }
  .veil-tile-avatar-wrap {
    transition: transform 0.18s cubic-bezier(0.34, 1.56, 0.64, 1);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .veil-tile-avatar-wrap.speaking {
    transform: scale(1.08);
  }
  .veil-tile-meta {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 6px 10px;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.85) 0%, rgba(0, 0, 0, 0.4) 60%, transparent 100%);
    font-size: var(--text-xs, 12px);
    color: #fff;
    z-index: 10;
    pointer-events: none;
  }
  .veil-tile-name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
    min-width: 0;
  }
  .veil-tile-label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .veil-tile-speaking {
    color: var(--veil-success, #22c55e);
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .veil-pulse-wave {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--veil-success, #22c55e);
    animation: veil-speak-pulse 1.2s infinite ease-in-out;
  }
  @keyframes veil-speak-pulse {
    0%, 100% { transform: scale(0.9); opacity: 0.8; }
    50% { transform: scale(1.4); opacity: 1; }
  }
  .veil-tile-muted {
    color: var(--veil-danger, #ef4444);
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
  }
  .veil-tile-vol-badge {
    font-size: 10px;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    padding: 1px 5px;
    border-radius: 4px;
    color: var(--veil-brand, #818cf8);
    font-weight: 600;
    flex-shrink: 0;
  }
  .veil-vol-pop {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 100;
    background: var(--veil-bg-surface, #1e2230);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.12));
    border-radius: var(--radius-lg, 10px);
    padding: var(--space-3, 12px);
    box-shadow: var(--shadow-2xl, 0 20px 40px rgba(0, 0, 0, 0.6));
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
    width: 200px;
  }
  .veil-vol-pop-title {
    font-size: var(--text-xs, 12px);
    font-weight: 700;
    color: var(--veil-text-primary, #f1f5f9);
  }
  .veil-vol-pop-row {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    font-size: var(--text-xs, 12px);
    color: var(--veil-text-secondary, #94a3b8);
  }
  .veil-vol-pop-backdrop {
    position: fixed;
    inset: 0;
    z-index: 99;
  }
  .veil-vol-pop-val {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  .veil-participant-tile.compact {
    border-radius: var(--radius-lg, 10px);
  }
  .veil-participant-tile.compact .veil-tile-meta {
    padding: 3px 6px;
    font-size: 11px;
  }

  .veil-remote-fx-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    z-index: 5;
    pointer-events: none;
    border-radius: inherit;
  }
  .veil-fx-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 6;
    background: rgba(124, 58, 237, 0.85);
    backdrop-filter: blur(8px);
    border-radius: 6px;
    padding: 2px 6px;
    pointer-events: none;
    animation: veil-fx-badge-pulse 2s ease-in-out infinite;
  }
  .veil-fx-badge-text {
    font-size: 10px;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  @keyframes veil-fx-badge-pulse {
    0%, 100% { opacity: 0.85; }
    50% { opacity: 1; }
  }
</style>

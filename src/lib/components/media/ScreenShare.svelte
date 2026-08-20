<script lang="ts">
  import { RoomEvent, Track } from 'livekit-client';
  import type { Participant, Track as TrackType } from 'livekit-client';
  import Icon from '../ui/Icon.svelte';

  let {
    participant = null as Participant | null,
    isLocal = false,
  }: {
    participant?: Participant | null;
    isLocal?: boolean;
  } = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let videoEl = $state<HTMLVideoElement | null>(null);
  let isSharingActive = $state(false);
  let isFullscreen = $state(false);
  let objectFit = $state<'contain' | 'cover'>('contain');

  $effect(() => {
    const el = videoEl;
    const p = participant;
    if (!el || !p) {
      isSharingActive = false;
      return;
    }
    let attached: TrackType | null = null;

    const tryAttach = () => {
      const pub = p.getTrackPublication(Track.Source.ScreenShare);
      const canAttach = pub?.track && (isLocal ? true : pub.isSubscribed) && pub.track.kind === 'video';
      isSharingActive = !!canAttach;
      if (canAttach && pub?.track) {
        if (attached !== pub.track) {
          attached?.detach(el);
          attached = pub.track;
          pub.track.attach(el);
        }
        el.muted = isLocal;
        void el.play().catch(() => {});
      } else if (attached) {
        attached.detach(el);
        attached = null;
      }
    };

    tryAttach();
    if (typeof p?.on === 'function') {
      p.on(RoomEvent.TrackSubscribed, tryAttach);
      p.on(RoomEvent.TrackUnsubscribed, tryAttach);
      p.on(RoomEvent.TrackMuted, tryAttach);
      p.on(RoomEvent.TrackUnmuted, tryAttach);
      p.on(RoomEvent.LocalTrackPublished, tryAttach);
      p.on(RoomEvent.LocalTrackUnpublished, tryAttach);
    }

    const onFullscreenChange = () => {
      isFullscreen = !!document.fullscreenElement;
    };
    document.addEventListener('fullscreenchange', onFullscreenChange);

    return () => {
      if (typeof p?.off === 'function') {
        p.off(RoomEvent.TrackSubscribed, tryAttach);
        p.off(RoomEvent.TrackUnsubscribed, tryAttach);
        p.off(RoomEvent.TrackMuted, tryAttach);
        p.off(RoomEvent.TrackUnmuted, tryAttach);
        p.off(RoomEvent.LocalTrackPublished, tryAttach);
        p.off(RoomEvent.LocalTrackUnpublished, tryAttach);
      }
      document.removeEventListener('fullscreenchange', onFullscreenChange);
      if (attached && el) {
        attached.detach(el);
      }
    };
  });

  async function toggleFullscreen() {
    if (!containerEl) return;
    if (!document.fullscreenElement) {
      await containerEl.requestFullscreen().catch(() => {});
    } else {
      await document.exitFullscreen().catch(() => {});
    }
  }

  function toggleFit() {
    objectFit = objectFit === 'contain' ? 'cover' : 'contain';
  }
</script>

<div
  class="veil-screen-share"
  bind:this={containerEl}
  aria-label="Ekran paylaşımı"
>
  <video
    class="veil-screen-video"
    class:hidden={!isSharingActive}
    style:object-fit={objectFit}
    bind:this={videoEl}
    aria-label="Paylaşılan ekran"
    playsinline
  ></video>

  {#if isSharingActive}
    <div class="veil-screen-overlay">
      <div class="veil-screen-label">
        <Icon name="monitor" size={13} />
        <span>{isLocal ? 'Sen ekranını paylaşıyorsun' : `${participant?.name ?? participant?.identity ?? 'Kullanıcı'} ekranını paylaşıyor`}</span>
      </div>

      <div class="veil-screen-actions">
        <button
          class="veil-screen-btn"
          onclick={toggleFit}
          title={objectFit === 'contain' ? 'Ekranı Doldur' : 'Orijinal Boyuta Sığdır'}
          aria-label="Boyutlandırma modunu değiştir"
        >
          <Icon name={objectFit === 'contain' ? 'maximize' : 'minimize'} size={14} />
        </button>

        <button
          class="veil-screen-btn"
          onclick={toggleFullscreen}
          title={isFullscreen ? 'Tam Ekrandan Çık' : 'Tam Ekran Yap'}
          aria-label={isFullscreen ? 'Tam ekrandan çık' : 'Tam ekran yap'}
        >
          <Icon name={isFullscreen ? 'minimize-2' : 'maximize-2'} size={14} />
        </button>
      </div>
    </div>
  {:else}
    <div class="veil-screen-empty">
      <div class="veil-screen-empty-icon" aria-hidden="true">
        <Icon name="monitor" size={36} />
      </div>
      <p>Ekran paylaşımı başlatılıyor…</p>
    </div>
  {/if}
</div>

<style>
  .veil-screen-share {
    position: relative;
    flex: 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    min-width: 0;
    background: #000;
    border-radius: var(--radius-xl);
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .veil-screen-video {
    width: 100%;
    height: 100%;
    background: #000;
    display: block;
  }
  .veil-screen-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3);
    background: linear-gradient(to bottom, rgba(0,0,0,0.7) 0%, transparent 100%);
    pointer-events: none;
    z-index: 5;
    opacity: 0;
    transition: opacity 0.2s ease;
  }
  .veil-screen-share:hover .veil-screen-overlay,
  .veil-screen-share:focus-within .veil-screen-overlay {
    opacity: 1;
  }
  .veil-screen-label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    background: hsl(220 20% 4% / 0.85);
    color: #fff;
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    font-weight: 500;
    backdrop-filter: blur(8px);
    pointer-events: auto;
  }
  .veil-screen-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    pointer-events: auto;
  }
  .veil-screen-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: hsl(220 20% 4% / 0.85);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
    color: var(--veil-text-primary);
    cursor: pointer;
    backdrop-filter: blur(8px);
    transition: all 0.15s ease;
  }
  .veil-screen-btn:hover {
    background: var(--veil-brand);
    color: #fff;
    border-color: var(--veil-brand);
  }
  .veil-screen-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    color: var(--veil-text-muted);
    width: 100%;
  }
  .veil-screen-empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--veil-brand);
  }
</style>

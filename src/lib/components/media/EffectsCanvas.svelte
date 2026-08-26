<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { effectsStore } from '$lib/effects/store';
  import { effectEngine } from '$lib/effects/engine';
  import { mediaStore } from '$lib/stores/media';

  let {
    videoElement = null,
    width = 640,
    height = 480,
    mirrored = false,
  }: {
    videoElement?: HTMLVideoElement | null;
    width?: number;
    height?: number;
    mirrored?: boolean;
  } = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let retryCount = 0;
  const MAX_RETRIES = 3;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;
  const effects = $derived($effectsStore);
  const media = $derived($mediaStore);

  const hasActiveEffect = $derived(effects.activeEffects.length > 0 || !!effects.activeEffect);
  const isActive = $derived(media.isCameraOn && hasActiveEffect);

  let resizeObserver: ResizeObserver | null = null;

  function syncCanvasSize() {
    if (!canvasEl || !videoElement) return;
    const vw = videoElement.videoWidth || width;
    const vh = videoElement.videoHeight || height;
    if (vw <= 0 || vh <= 0) return;
    if (canvasEl.width !== vw || canvasEl.height !== vh) {
      canvasEl.width = vw;
      canvasEl.height = vh;
    }
  }

  async function startEngineWithRetry() {
    if (!canvasEl || !videoElement) return;
    try {
      syncCanvasSize();
      await effectEngine.start();
      retryCount = 0;
    } catch (err) {
      console.warn('Effect engine start failed:', err);
      if (retryCount < MAX_RETRIES) {
        retryCount++;
        retryTimer = setTimeout(() => startEngineWithRetry(), 1000 * retryCount);
      }
    }
  }

  onMount(() => {
    if (canvasEl) {
      effectsStore.setCanvas(canvasEl);
    }
    if (videoElement) {
      effectsStore.setVideoElement(videoElement);
      resizeObserver = new ResizeObserver(() => syncCanvasSize());
      resizeObserver.observe(videoElement);
      syncCanvasSize();
    }
  });

  onDestroy(() => {
    if (retryTimer) {
      clearTimeout(retryTimer);
      retryTimer = null;
    }
    resizeObserver?.disconnect();
    effectEngine.stop();
    effectsStore.setCanvas(null);
    effectsStore.setVideoElement(null);
  });

  $effect(() => {
    effectsStore.setVideoElement(videoElement);
    syncCanvasSize();
  });

  $effect(() => {
    if (isActive && canvasEl && videoElement) {
      startEngineWithRetry();
    } else if (!isActive && effectEngine.getState().isTracking) {
      effectEngine.stop();
    }
  });
</script>

{#if isActive}
  <canvas
    bind:this={canvasEl}
    class="veil-effects-canvas"
    class:mirrored
    width={width}
    height={height}
    aria-hidden="true"
  ></canvas>
  <div class="veil-fx-badge" aria-label="Efekt aktif">
    <span class="veil-fx-badge-text">FX</span>
  </div>
{/if}

<style>
  .veil-effects-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    z-index: 5;
    pointer-events: none;
    border-radius: inherit;
    transition: transform 0.2s cubic-bezier(0.2, 0, 0, 1);
  }
  .veil-effects-canvas.mirrored {
    transform: scaleX(-1);
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

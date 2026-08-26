<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toastStore } from '$lib/stores/notifications';

  interface Props {
    title?: string;
    aspectRatio?: number; // e.g. 1 for avatar, 3 for banner
    onCapture: (dataUrl: string) => void;
    onClose: () => void;
  }

  let {
    title = 'Fotoğraf Çek',
    aspectRatio = 1,
    onCapture,
    onClose,
  }: Props = $props();

  let videoEl = $state<HTMLVideoElement | null>(null);
  let stream = $state<MediaStream | null>(null);
  let devices = $state<MediaDeviceInfo[]>([]);
  let selectedDeviceId = $state<string>('');
  let isMirrored = $state(true);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);

  // Countdown & shutter state
  let countdownTimer = $state<number>(0);
  let countdownSeconds = $state<number>(0);
  let isFlashing = $state(false);

  // Review state
  let capturedDataUrl = $state<string | null>(null);

  async function loadDevices() {
    try {
      const allDevices = await navigator.mediaDevices.enumerateDevices();
      devices = allDevices.filter((d) => d.kind === 'videoinput');
      if (devices.length > 0 && !selectedDeviceId) {
        selectedDeviceId = devices[0].deviceId;
      }
    } catch {
      // Ignored
    }
  }

  async function startCamera() {
    stopCamera();
    loading = true;
    errorMsg = null;
    try {
      const constraints: MediaStreamConstraints = {
        video: selectedDeviceId
          ? { deviceId: { exact: selectedDeviceId }, width: { ideal: 1920 }, height: { ideal: 1080 } }
          : { width: { ideal: 1920 }, height: { ideal: 1080 } },
        audio: false,
      };
      const s = await navigator.mediaDevices.getUserMedia(constraints);
      stream = s;
      // videoEl DOM mount and actual frame decoding check
      await new Promise<void>((resolve) => {
        let attempts = 0;
        const attachStream = () => {
          attempts++;
          if (videoEl) {
            videoEl.srcObject = s;
            const checkReady = () => {
              if (videoEl && videoEl.readyState >= 2 && videoEl.videoWidth > 0) {
                resolve();
              } else if (attempts < 80) {
                setTimeout(checkReady, 50);
              } else {
                resolve();
              }
            };
            videoEl.onloadeddata = checkReady;
            videoEl.oncanplay = checkReady;
            void videoEl.play().then(checkReady).catch(() => {});
            checkReady();
          } else if (attempts < 50) {
            setTimeout(attachStream, 20);
          } else {
            resolve();
          }
        };
        attachStream();
      });
      await loadDevices();
      loading = false;
    } catch (err: unknown) {
      loading = false;
      const msg = err instanceof Error ? err.message : 'Kamera erişimi sağlanamadı.';
      errorMsg = msg;
      toastStore.error('Kameraya erişilemedi. Lütfen izinleri kontrol edin.');
    }
  }

  // videoEl DOM bind watcher
  $effect(() => {
    const vid = videoEl;
    const s = stream;
    if (vid && s && vid.srcObject !== s) {
      vid.srcObject = s;
      vid.onloadeddata = () => {
        if (vid.videoWidth > 0) loading = false;
      };
      void vid.play().catch(() => {});
    }
  });


  function stopCamera() {
    if (stream) {
      stream.getTracks().forEach((t) => t.stop());
      stream = null;
    }
    if (videoEl) {
      videoEl.srcObject = null;
    }
  }

  function snapPhoto() {
    if (!videoEl || videoEl.videoWidth === 0 || loading || errorMsg) return;
    isFlashing = true;
    setTimeout(() => {
      isFlashing = false;
    }, 150);

    const canvas = document.createElement('canvas');
    canvas.width = videoEl.videoWidth || 1280;
    canvas.height = videoEl.videoHeight || 720;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    if (isMirrored) {
      ctx.translate(canvas.width, 0);
      ctx.scale(-1, 1);
    }
    ctx.drawImage(videoEl, 0, 0, canvas.width, canvas.height);
    capturedDataUrl = canvas.toDataURL('image/jpeg', 0.95);
    stopCamera();
  }

  function triggerShutter() {
    if (loading || errorMsg || countdownTimer > 0) return;
    if (countdownSeconds > 0) {
      countdownTimer = countdownSeconds;
      const interval = setInterval(() => {
        countdownTimer -= 1;
        if (countdownTimer <= 0) {
          clearInterval(interval);
          snapPhoto();
        }
      }, 1000);
    } else {
      snapPhoto();
    }
  }

  function retake() {
    capturedDataUrl = null;
    countdownTimer = 0;
    void startCamera();
  }

  function confirmPhoto() {
    if (capturedDataUrl) {
      onCapture(capturedDataUrl);
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'Enter' && capturedDataUrl) {
      e.preventDefault();
      confirmPhoto();
    }
  }

  onMount(() => {
    void startCamera();
  });

  onDestroy(() => {
    stopCamera();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="veil-cam-backdrop" role="dialog" aria-modal="true" aria-label={title}>
  <div class="veil-cam-modal veil-pop-in">
    <!-- Header -->
    <div class="veil-cam-header">
      <div class="veil-cam-title">
        <Icon name="camera" size={18} />
        <span>{title}</span>
      </div>
      <button type="button" class="btn-icon" onclick={onClose} aria-label="Kapat">
        <Icon name="x" size={16} />
      </button>
    </div>

    <!-- Viewfinder Area -->
    <div class="veil-cam-viewport" class:is-flashing={isFlashing}>
      {#if capturedDataUrl}
        <img src={capturedDataUrl} alt="Çekilen Fotoğraf" class="veil-cam-preview-img" />
      {:else}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          bind:this={videoEl}
          autoplay
          playsinline
          muted
          class="veil-cam-video"
          class:mirrored={isMirrored}
        ></video>

        {#if loading}
          <div class="veil-cam-overlay-state">
            <div class="veil-spinner" style="width:32px;height:32px;border-width:3px;"></div>
            <p>Kamera başlatılıyor…</p>
          </div>
        {:else if errorMsg}
          <div class="veil-cam-overlay-state error">
            <Icon name="camera" size={36} />
            <p>Kameraya ulaşılamadı</p>
            <span class="veil-cam-err-sub">{errorMsg}</span>
            <button type="button" class="btn btn-secondary btn-sm" onclick={startCamera}>Yeniden Dene</button>
          </div>
        {/if}

        {#if countdownTimer > 0}
          <div class="veil-cam-countdown-overlay">
            <span class="veil-countdown-number">{countdownTimer}</span>
          </div>
        {/if}

        <!-- Aspect ratio guide frame -->
        <div
          class="veil-cam-guide-box"
          class:circle={aspectRatio === 1}
          style={aspectRatio > 1 ? `aspect-ratio: ${aspectRatio};` : 'aspect-ratio: 1;'}
        ></div>
      {/if}
    </div>

    <!-- Footer Controls -->
    <div class="veil-cam-footer">
      {#if capturedDataUrl}
        <div class="veil-cam-review-actions">
          <button type="button" class="btn btn-secondary" onclick={retake}>
            <Icon name="refresh-cw" size={15} />
            <span>Tekrar Çek</span>
          </button>
          <button type="button" class="btn btn-primary" onclick={confirmPhoto}>
            <Icon name="check" size={15} />
            <span>Fotoğrafı Kullan</span>
          </button>
        </div>
      {:else}
        <div class="veil-cam-settings-bar">
          {#if devices.length > 1}
            <select
              bind:value={selectedDeviceId}
              onchange={startCamera}
              class="veil-cam-select"
              aria-label="Kamera seç"
            >
              {#each devices as dev, i}
                <option value={dev.deviceId}>{dev.label || `Kamera ${i + 1}`}</option>
              {/each}
            </select>
          {/if}

          <button
            type="button"
            class="btn-icon veil-cam-tool-btn"
            class:active={isMirrored}
            title="Ayna Görünümü"
            onclick={() => (isMirrored = !isMirrored)}
          >
            <Icon name="sparkle" size={16} />
          </button>

          <div class="veil-cam-timer-selector">
            <button
              type="button"
              class="veil-cam-timer-btn"
              class:active={countdownSeconds === 0}
              onclick={() => (countdownSeconds = 0)}
            >0s</button>
            <button
              type="button"
              class="veil-cam-timer-btn"
              class:active={countdownSeconds === 3}
              onclick={() => (countdownSeconds = 3)}
            >3s</button>
            <button
              type="button"
              class="veil-cam-timer-btn"
              class:active={countdownSeconds === 5}
              onclick={() => (countdownSeconds = 5)}
            >5s</button>
          </div>
        </div>

        <div class="veil-cam-shutter-bar">
          <button
            type="button"
            class="veil-shutter-btn"
            onclick={triggerShutter}
            disabled={loading || !!errorMsg || countdownTimer > 0}
            aria-label="Fotoğraf Çek"
          >
            <div class="veil-shutter-inner">
              <Icon name="camera" size={24} />
            </div>
          </button>
          <span class="veil-shutter-label">
            {#if countdownTimer > 0}
              {countdownTimer} saniye sonra çekiliyor…
            {:else if countdownSeconds > 0}
              {countdownSeconds}s Zamanlayıcı ile Çek
            {:else}
              Fotoğrafı Çek
            {/if}
          </span>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .veil-cam-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.85);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    padding: var(--space-4);
  }

  .veil-cam-modal {
    width: 100%;
    max-width: 580px;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-2xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .veil-cam-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--veil-border);
  }

  .veil-cam-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-base);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .veil-cam-viewport {
    position: relative;
    width: 100%;
    aspect-ratio: 4 / 3;
    background: #000;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .veil-cam-viewport.is-flashing::after {
    content: '';
    position: absolute;
    inset: 0;
    background: #fff;
    opacity: 0.9;
    z-index: 100;
    pointer-events: none;
    animation: flash-anim 0.15s ease-out;
  }

  @keyframes flash-anim {
    from { opacity: 0.9; }
    to { opacity: 0; }
  }

  .veil-cam-video,
  .veil-cam-preview-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .veil-cam-video.mirrored {
    transform: scaleX(-1);
  }

  .veil-cam-overlay-state {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    background: rgba(0, 0, 0, 0.7);
    color: var(--veil-text-secondary);
    z-index: 10;
  }

  .veil-cam-overlay-state.error {
    color: var(--veil-danger);
  }

  .veil-cam-err-sub {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    max-width: 80%;
    text-align: center;
  }

  .veil-cam-countdown-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.4);
    z-index: 20;
    pointer-events: none;
  }

  .veil-countdown-number {
    font-size: 72px;
    font-weight: 800;
    color: #fff;
    text-shadow: 0 4px 16px rgba(0, 0, 0, 0.8);
    animation: countdown-pop 1s infinite cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes countdown-pop {
    0% { transform: scale(1.4); opacity: 0; }
    30% { transform: scale(1); opacity: 1; }
    100% { transform: scale(0.9); opacity: 0.8; }
  }

  .veil-cam-guide-box {
    position: absolute;
    border: 2px dashed rgba(255, 255, 255, 0.6);
    box-shadow: 0 0 0 9999px rgba(0, 0, 0, 0.35);
    pointer-events: none;
    max-width: 80%;
    max-height: 80%;
  }

  .veil-cam-guide-box.circle {
    border-radius: 50%;
  }

  .veil-cam-footer {
    padding: var(--space-4) var(--space-5);
    background: var(--veil-bg-surface);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-cam-settings-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .veil-cam-select {
    flex: 1;
    max-width: 220px;
    padding: 6px 10px;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    color: var(--veil-text-primary);
    font-size: var(--text-xs);
    outline: none;
  }

  .veil-cam-tool-btn.active {
    color: var(--veil-brand);
    background: color-mix(in srgb, var(--veil-brand) 15%, transparent);
  }

  .veil-cam-timer-selector {
    display: flex;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    padding: 2px;
    gap: 2px;
  }

  .veil-cam-timer-btn {
    border: none;
    background: transparent;
    color: var(--veil-text-muted);
    font-size: 11px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--t-fast);
  }

  .veil-cam-timer-btn.active {
    background: var(--veil-brand);
    color: #fff;
  }

  .veil-cam-shutter-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    padding-top: var(--space-2);
  }

  .veil-shutter-btn {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.4);
    background: transparent;
    padding: 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform var(--t-fast), border-color var(--t-fast);
  }

  .veil-shutter-btn:hover:not(:disabled) {
    transform: scale(1.06);
    border-color: #fff;
  }

  .veil-shutter-btn:active:not(:disabled) {
    transform: scale(0.95);
  }

  .veil-shutter-inner {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: var(--veil-brand);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 12px var(--veil-brand-subtle);
  }

  .veil-cam-review-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
  }
</style>

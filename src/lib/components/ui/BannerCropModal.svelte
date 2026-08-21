<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import Avatar from '$lib/components/ui/Avatar.svelte';
  import { readLocalImageAsDataUrl } from '$lib/utils/image-loader';

  let {
    src,
    aspectRatio = 3, // 3:1 standard Discord-style banner ratio
    title = 'Bannerı Ayarla',
    hasAvatarPreview = true,
    avatarName = 'Kullanıcı',
    avatarHash = null,
    onSave,
    onClose,
  }: {
    src: string;
    aspectRatio?: number;
    title?: string;
    hasAvatarPreview?: boolean;
    avatarName?: string;
    avatarHash?: string | null;
    onSave: (croppedDataUrl: string) => void;
    onClose: () => void;
  } = $props();

  let resolvedSrc = $state<string>('');
  let imgElement = $state<HTMLImageElement | null>(null);
  let containerElement = $state<HTMLDivElement | null>(null);

  let scale = $state(1);
  let posX = $state(0);
  let posY = $state(0);
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let initialPosX = $state(0);
  let initialPosY = $state(0);

  let imgNaturalWidth = $state(0);
  let imgNaturalHeight = $state(0);
  let isProcessing = $state(false);
  let isImageLoaded = $state(false);
  let loadError = $state(false);
  let cropError = $state(false);

  $effect(() => {
    let active = true;
    if (src) {
      void readLocalImageAsDataUrl(src)
        .then((dataUrl) => {
          if (active) {
            resolvedSrc = dataUrl;
            loadError = false;
            resetPosition();
          }
        })
        .catch((err) => {
          console.warn('Görsel kaynağı çözümlenemedi:', err);
          if (active) {
            resolvedSrc = '';
            loadError = true;
          }
        });
    } else {
      resolvedSrc = '';
      loadError = false;
    }
    return () => {
      active = false;
    };
  });

  function handleImageLoad(e: Event) {
    const img = e.target as HTMLImageElement;
    imgNaturalWidth = img.naturalWidth || 800;
    imgNaturalHeight = img.naturalHeight || 600;
    isImageLoaded = true;
    resetPosition();
  }

  function resetPosition() {
    scale = 1;
    posX = 0;
    posY = 0;
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    isDragging = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    initialPosX = posX;
    initialPosY = posY;
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  function onMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    const dx = e.clientX - dragStartX;
    const dy = e.clientY - dragStartY;
    posX = initialPosX + dx;
    posY = initialPosY + dy;
  }

  function onMouseUp() {
    isDragging = false;
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
  }

  function onTouchStart(e: TouchEvent) {
    if (e.touches.length !== 1) return;
    isDragging = true;
    dragStartX = e.touches[0].clientX;
    dragStartY = e.touches[0].clientY;
    initialPosX = posX;
    initialPosY = posY;
  }

  function onTouchMove(e: TouchEvent) {
    if (!isDragging || e.touches.length !== 1) return;
    const dx = e.touches[0].clientX - dragStartX;
    const dy = e.touches[0].clientY - dragStartY;
    posX = initialPosX + dx;
    posY = initialPosY + dy;
  }

  function onTouchEnd() {
    isDragging = false;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -0.08 : 0.08;
    scale = Math.min(3, Math.max(1, +(scale + delta).toFixed(2)));
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') { posX -= 10; e.preventDefault(); }
    else if (e.key === 'ArrowRight') { posX += 10; e.preventDefault(); }
    else if (e.key === 'ArrowUp') { posY -= 10; e.preventDefault(); }
    else if (e.key === 'ArrowDown') { posY += 10; e.preventDefault(); }
    else if (e.key === '+' || e.key === '=') { scale = Math.min(3, scale + 0.1); e.preventDefault(); }
    else if (e.key === '-') { scale = Math.max(1, scale - 0.1); e.preventDefault(); }
    else if (e.key === 'Escape') { onClose(); }
  }

  async function applyCrop() {
    if (!resolvedSrc || loadError) return;
    isProcessing = true;
    cropError = false;
    try {
      const cropWidth = 1200;
      const cropHeight = Math.round(1200 / aspectRatio);

      const canvas = document.createElement('canvas');
      canvas.width = cropWidth;
      canvas.height = cropHeight;
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas context not available');

      let sourceImage: HTMLImageElement;
      if (imgElement && imgElement.complete && imgElement.naturalWidth > 0) {
        sourceImage = imgElement;
      } else {
        const img = new Image();
        if (!resolvedSrc.startsWith('data:')) {
          img.crossOrigin = 'anonymous';
        }
        await new Promise<void>((resolve, reject) => {
          img.onload = () => resolve();
          img.onerror = () => reject(new Error('Görsel yüklenemedi'));
          img.src = resolvedSrc;
        });
        sourceImage = img;
      }

      const nw = sourceImage.naturalWidth || 1200;
      const nh = sourceImage.naturalHeight || 400;

      const containerRect = containerElement?.getBoundingClientRect() || { width: 480, height: 480 / aspectRatio };
      const scaleFactor = cropWidth / containerRect.width;

      // Base fill
      ctx.fillStyle = '#0f1117';
      ctx.fillRect(0, 0, cropWidth, cropHeight);

      // Compute cover dimensions matching preview
      const containerRatio = containerRect.width / containerRect.height;
      const imageRatio = nw / nh;
      let baseW = containerRect.width;
      let baseH = containerRect.height;

      if (imageRatio > containerRatio) {
        baseH = containerRect.height;
        baseW = containerRect.height * imageRatio;
      } else {
        baseW = containerRect.width;
        baseH = containerRect.width / imageRatio;
      }

      const renderW = baseW * scale * scaleFactor;
      const renderH = baseH * scale * scaleFactor;

      const drawX = posX * scaleFactor + (cropWidth - renderW) / 2;
      const drawY = posY * scaleFactor + (cropHeight - renderH) / 2;

      ctx.drawImage(sourceImage, drawX, drawY, renderW, renderH);

      const dataUrl = canvas.toDataURL('image/png', 0.95);
      onSave(dataUrl);
    } catch (err) {
      console.error('Crop error:', err);
      cropError = true;
    } finally {
      isProcessing = false;
    }
  }
</script>

<div class="veil-crop-backdrop" role="dialog" aria-modal="true" aria-labelledby="crop-modal-title">
  <div class="veil-crop-modal">
    <div class="veil-crop-header">
      <h3 id="crop-modal-title" class="veil-crop-title">{title}</h3>
      <button class="btn-icon" onclick={onClose} aria-label="Kapat">
        <Icon name="x" size={16} />
      </button>
    </div>

    <div class="veil-crop-body">
      <p class="veil-crop-hint">Görseli istediğin gibi konumlandırmak için sürükle veya yakınlaştır.</p>

      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="veil-crop-viewport"
        bind:this={containerElement}
        onmousedown={onMouseDown}
        ontouchstart={onTouchStart}
        ontouchmove={onTouchMove}
        ontouchend={onTouchEnd}
        onwheel={onWheel}
        onkeydown={onKeyDown}
        tabindex="0"
        role="region"
        aria-label="Banner kırpma önizleme alanı"
        style="aspect-ratio: {aspectRatio};"
      >
        {#if resolvedSrc}
          <img
            bind:this={imgElement}
            src={resolvedSrc}
            alt="Banner Önizleme"
            onload={handleImageLoad}
            onerror={() => (loadError = true)}
            draggable="false"
            style="transform: translate({posX}px, {posY}px) scale({scale});"
          />
        {/if}

        {#if hasAvatarPreview}
          <div class="veil-crop-avatar-preview">
            <div class="veil-crop-avatar-circle">
              <Avatar hash={avatarHash} name={avatarName} size="md" />
            </div>
          </div>
        {/if}

        <div class="veil-crop-grid-overlay" aria-hidden="true"></div>
      </div>

      {#if loadError}
        <p class="veil-crop-error" role="alert">Görsel yüklenemedi.</p>
      {:else if cropError}
        <p class="veil-crop-error" role="alert">Kırpma işlemi başarısız oldu, lütfen tekrar deneyin.</p>
      {/if}

      <div class="veil-crop-controls">
        <div class="veil-crop-slider-row">
          <Icon name="zoom-out" size={16} />
          <input
            type="range"
            min="1"
            max="3"
            step="0.05"
            bind:value={scale}
            aria-label="Yakınlaştırma ölçeği"
          />
          <Icon name="zoom-in" size={16} />
        </div>
        <button class="btn btn-ghost btn-sm" onclick={resetPosition} type="button">
          <Icon name="refresh-cw" size={13} />
          Ortala
        </button>
      </div>
    </div>

    <div class="veil-crop-footer">
      <button class="btn btn-ghost" onclick={onClose} disabled={isProcessing} type="button">İptal</button>
      <button class="btn btn-primary" onclick={applyCrop} disabled={isProcessing || !resolvedSrc || loadError} type="button">
        {#if isProcessing}
          <div class="veil-spinner veil-spinner-sm"></div>
          Kaydediliyor…
        {:else}
          Uygula ve Kaydet
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .veil-crop-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10000;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
  }
  .veil-crop-modal {
    width: 100%;
    max-width: 560px;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-2xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: veilModalPop 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes veilModalPop {
    from { opacity: 0; transform: scale(0.96); }
    to { opacity: 1; transform: scale(1); }
  }
  .veil-crop-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    border-bottom: 1px solid var(--veil-border-subtle);
  }
  .veil-crop-title {
    font-size: var(--text-base);
    font-weight: 700;
    margin: 0;
  }
  .veil-crop-body {
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .veil-crop-hint {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin: 0;
  }
  .veil-crop-error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--veil-danger, #ef4444);
  }
  .veil-crop-viewport {
    width: 100%;
    position: relative;
    overflow: hidden;
    border-radius: var(--radius-xl);
    background: #090a0f;
    border: 2px solid var(--veil-brand);
    cursor: grab;
    user-select: none;
    display: flex;
    align-items: center;
    justify-content: center;
    outline: none;
  }
  .veil-crop-viewport:focus-visible {
    box-shadow: var(--shadow-glow-brand);
  }
  .veil-crop-viewport:active {
    cursor: grabbing;
  }
  .veil-crop-viewport img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    pointer-events: none;
    transition: transform 0.05s ease-out;
  }
  .veil-crop-avatar-preview {
    position: absolute;
    bottom: var(--space-2);
    left: var(--space-4);
    pointer-events: none;
    z-index: 10;
  }
  .veil-crop-avatar-circle {
    width: 54px;
    height: 54px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-surface);
    border: 3px solid var(--veil-bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--veil-text-muted);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  .veil-crop-grid-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    border: 1px dashed rgba(255, 255, 255, 0.15);
    background:
      linear-gradient(to right, rgba(255,255,255,0.05) 1px, transparent 1px) 0 0 / 33.33% 100%,
      linear-gradient(to bottom, rgba(255,255,255,0.05) 1px, transparent 1px) 0 0 / 100% 33.33%;
  }
  .veil-crop-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }
  .veil-crop-slider-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    color: var(--veil-text-muted);
  }
  .veil-crop-slider-row input[type="range"] {
    flex: 1;
    accent-color: var(--veil-brand);
    cursor: pointer;
  }
  .veil-crop-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-raised);
    border-top: 1px solid var(--veil-border-subtle);
  }
</style>

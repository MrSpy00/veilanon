<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { readLocalImageAsDataUrl } from '$lib/utils/image-loader';

  let {
    src,
    shape = 'rect',
    aspectRatio = shape === 'circle' ? 1 : 3,
    title = shape === 'circle' ? 'Profil Fotoğrafını Ayarla' : 'Bannerı Ayarla',
    onSave,
    onClose,
  }: {
    src: string;
    shape?: 'circle' | 'rect';
    aspectRatio?: number;
    title?: string;
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
    if (!resolvedSrc) return;
    isProcessing = true;
    try {
      const targetDim = shape === 'circle' ? 512 : 1200;
      const cropWidth = targetDim;
      const cropHeight = Math.round(targetDim / aspectRatio);

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

      const nw = sourceImage.naturalWidth || cropWidth;
      const nh = sourceImage.naturalHeight || cropHeight;

      const containerRect = containerElement?.getBoundingClientRect() || { width: 360, height: 360 / aspectRatio };
      const scaleFactor = cropWidth / containerRect.width;

      if (shape === 'circle') {
        ctx.save();
        ctx.beginPath();
        ctx.arc(cropWidth / 2, cropHeight / 2, Math.min(cropWidth, cropHeight) / 2, 0, Math.PI * 2);
        ctx.closePath();
        ctx.clip();
      } else {
        ctx.fillStyle = '#0f1117';
        ctx.fillRect(0, 0, cropWidth, cropHeight);
      }

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

      if (shape === 'circle') {
        ctx.restore();
      }

      const dataUrl = canvas.toDataURL('image/png', 0.95);
      onSave(dataUrl);
    } catch (err) {
      console.error('Kırpma hatası:', err);
    } finally {
      isProcessing = false;
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="veil-crop-backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="veil-crop-modal"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="0"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="veil-crop-header">
      <div class="veil-crop-title-wrap">
        <Icon name={shape === 'circle' ? 'camera' : 'film'} size={18} />
        <h3>{title}</h3>
      </div>
      <button class="veil-crop-close-btn" onclick={onClose} aria-label="Kapat" title="Kapat">
        <Icon name="x" size={16} />
      </button>
    </div>

    <div class="veil-crop-body">
      <p class="veil-crop-hint">
        Görseli sürükleyerek ortalayabilir, kaydırıcı veya fare tekerleğiyle yaklaştırabilirsiniz.
      </p>

      <!-- Interactive Viewport -->
      <div
        class="veil-crop-viewport-container"
        class:is-circle={shape === 'circle'}
        style="aspect-ratio: {aspectRatio};"
        bind:this={containerElement}
        onmousedown={onMouseDown}
        ontouchstart={onTouchStart}
        ontouchmove={onTouchMove}
        ontouchend={onTouchEnd}
        onwheel={onWheel}
      >
        {#if resolvedSrc}
          <img
            bind:this={imgElement}
            src={resolvedSrc}
            alt="Önizleme"
            class="veil-crop-source-img"
            style="transform: translate({posX}px, {posY}px) scale({scale});"
            onload={handleImageLoad}
            onerror={() => (loadError = true)}
            draggable="false"
          />
        {:else}
          <div class="veil-crop-loading">
            <div class="veil-spinner"></div>
            <span>Görsel yükleniyor…</span>
          </div>
        {/if}

        <div class="veil-crop-grid-overlay" class:circle-mask={shape === 'circle'}></div>
      </div>

      {#if loadError}
        <p class="veil-crop-error" role="alert">Görsel yüklenemedi.</p>
      {/if}

      <!-- Controls: Zoom slider & buttons -->
      <div class="veil-crop-controls">
        <div class="veil-crop-zoom-group">
          <button
            class="veil-crop-btn-sm"
            onclick={() => (scale = Math.max(1, +(scale - 0.1).toFixed(2)))}
            title="Uzaklaştır"
            aria-label="Uzaklaştır"
          >
            <Icon name="zoom-out" size={14} />
          </button>

          <input
            type="range"
            min="1"
            max="3"
            step="0.01"
            bind:value={scale}
            class="veil-range-slider"
            aria-label="Yakınlaştırma seviyesi"
          />

          <button
            class="veil-crop-btn-sm"
            onclick={() => (scale = Math.min(3, +(scale + 0.1).toFixed(2)))}
            title="Yakınlaştır"
            aria-label="Yakınlaştır"
          >
            <Icon name="zoom-in" size={14} />
          </button>
        </div>

        <button
          type="button"
          class="btn btn-secondary btn-xs veil-crop-reset-btn"
          onclick={resetPosition}
          title="Pozisyonu Sıfırla"
        >
          <Icon name="refresh-cw" size={12} />
          <span>Ortala & Sıfırla</span>
        </button>
      </div>
    </div>

    <div class="veil-crop-footer">
      <button class="btn btn-ghost btn-sm" onclick={onClose} disabled={isProcessing}>
        İptal
      </button>
      <button class="btn btn-primary btn-sm" onclick={applyCrop} disabled={isProcessing || !resolvedSrc || loadError}>
        {#if isProcessing}
          <div class="veil-spinner veil-spinner-sm"></div>
          İşleniyor…
        {:else}
          <Icon name="check" size={14} />
          Uygula & Kaydet
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .veil-crop-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.78);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
    animation: veil-fade-in 0.18s ease-out forwards;
  }

  .veil-crop-modal {
    width: 100%;
    max-width: 520px;
    background: var(--veil-bg-elevated, #16181d);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.12));
    border-radius: var(--radius-xl, 16px);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: veil-scale-up 0.2s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  .veil-crop-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
  }

  .veil-crop-title-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-md);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .veil-crop-close-btn {
    background: transparent;
    border: none;
    color: var(--veil-text-muted);
    cursor: pointer;
    padding: 6px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .veil-crop-close-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--veil-text-primary);
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
    line-height: var(--leading-normal);
  }

  .veil-crop-error {
    margin: 0;
    font-size: var(--text-xs);
    color: var(--veil-danger, #ef4444);
  }

  .veil-crop-viewport-container {
    width: 100%;
    max-height: 280px;
    position: relative;
    overflow: hidden;
    border-radius: var(--radius-lg, 12px);
    background: #08090c;
    border: 2px solid var(--veil-brand, #7c3aed);
    cursor: grab;
    user-select: none;
    touch-action: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .veil-crop-viewport-container.is-circle {
    max-width: 260px;
    margin: 0 auto;
    border-radius: 50%;
  }

  .veil-crop-viewport-container:active {
    cursor: grabbing;
  }

  .veil-crop-source-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
    transition: transform 0.03s linear;
  }

  .veil-crop-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    color: var(--veil-text-muted);
    font-size: var(--text-xs);
  }

  .veil-crop-grid-overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
    border: 1px dashed rgba(255, 255, 255, 0.25);
  }

  .veil-crop-grid-overlay.circle-mask {
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.35);
  }

  .veil-crop-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    margin-top: var(--space-1);
  }

  .veil-crop-zoom-group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
  }

  .veil-crop-btn-sm {
    background: var(--veil-bg-surface, rgba(255, 255, 255, 0.06));
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.1));
    color: var(--veil-text-secondary);
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .veil-crop-btn-sm:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--veil-text-primary);
  }

  .veil-crop-reset-btn {
    white-space: nowrap;
  }

  .veil-crop-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: color-mix(in srgb, var(--veil-bg-void) 40%, transparent);
    border-top: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
  }
</style>

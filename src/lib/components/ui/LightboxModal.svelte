<script lang="ts">
  import Icon from './Icon.svelte';

  let {
    open = false,
    src = '',
    alt = '',
    onClose,
  }: {
    open?: boolean;
    src?: string;
    alt?: string;
    onClose?: () => void;
  } = $props();

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose?.();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose?.();
  }
</script>

<svelte:window onkeydown={open ? onKeyDown : undefined} />

{#if open}
  <div
    class="veil-lightbox-overlay"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={onOverlayClick}
    onkeydown={onKeyDown}
  >
    <button class="veil-lightbox-close btn-icon" onclick={onClose} aria-label="Kapat">
      <Icon name="x" size={24} />
    </button>
    <div class="veil-lightbox-content">
      <img {src} {alt} class="veil-lightbox-img" />
    </div>
  </div>
{/if}

<style>
  .veil-lightbox-overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: hsl(220 25% 4% / 0.92);
    backdrop-filter: blur(12px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    animation: veil-fade-in 180ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  .veil-lightbox-close {
    position: absolute;
    top: var(--space-4);
    right: var(--space-4);
    width: 44px;
    height: 44px;
    border-radius: var(--radius-full);
    background: hsl(220 20% 12% / 0.7);
    color: #fff;
    cursor: pointer;
    border: 1px solid var(--veil-border);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--t-fast);
  }
  .veil-lightbox-close:hover {
    background: hsl(220 20% 18% / 0.9);
    transform: scale(1.05);
  }
  .veil-lightbox-content {
    max-width: 90vw;
    max-height: 90vh;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .veil-lightbox-img {
    max-width: 100%;
    max-height: 85vh;
    object-fit: contain;
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-2xl);
    user-select: none;
  }
</style>

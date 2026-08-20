<script lang="ts">
  import type { Snippet } from 'svelte';
  import { tick } from 'svelte';
  import { createFocusTrap, handleEscape } from '$lib/utils/accessibility';

  let {
    open = false,
    title = '',
    size = 'md',
    onClose,
    labelledby = '',
    children,
    footer,
  }: {
    open?: boolean;
    title?: string;
    size?: 'md' | 'lg' | 'xl';
    onClose: () => void;
    labelledby?: string;
    children?: Snippet;
    footer?: Snippet;
  } = $props();

  let overlayEl = $state<HTMLDivElement | null>(null);
  let cleanupTrap: (() => void) | null = null;
  let cleanupEsc: (() => void) | null = null;

  $effect(() => {
    if (!open) return;
    const overlay = overlayEl;
    tick().then(() => {
      if (!overlayEl) return;
      cleanupTrap?.();
      cleanupTrap = createFocusTrap(overlayEl);
    });
    cleanupEsc?.();
    cleanupEsc = handleEscape(onClose);
    return () => {
      cleanupTrap?.();
      cleanupTrap = null;
      cleanupEsc?.();
      cleanupEsc = null;
    };
  });

  function onOverlayClick(e: MouseEvent) {
    if (e.target === overlayEl) onClose();
  }
</script>

{#if open}
  <div
    class="veil-overlay"
    bind:this={overlayEl}
    role="presentation"
    onclick={onOverlayClick}
  >
    <div
      class="veil-modal {size === 'lg' ? 'veil-modal-lg' : ''} {size === 'xl' ? 'veil-modal-xl' : ''}"
      role="dialog"
      aria-modal="true"
      aria-label={labelledby || title || undefined}
    >
      {#if title}
        <div class="veil-modal-header">
          <h2 class="veil-modal-title">{title}</h2>
          <button class="btn-icon" onclick={onClose} title="Kapat" aria-label="Kapat">
            <svg class="veil-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18"/></svg>
          </button>
        </div>
      {/if}
      <div class="veil-modal-body">
        {@render children?.()}
      </div>
      {#if footer}
        <div class="veil-modal-footer">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}

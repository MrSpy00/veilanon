<script lang="ts">
  import { tick } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { createFocusTrap, handleEscape } from '$lib/utils/accessibility';
  import Icon from './Icon.svelte';

  const dialog = $derived($uiStore.confirmDialog);
  let overlayEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!dialog) return;
    const overlay = overlayEl;
    if (!overlay) return;
    let cleanupTrap: (() => void) | null = null;
    tick().then(() => {
      cleanupTrap = createFocusTrap(overlay);
    });
    const cleanupEsc = handleEscape(() => uiStore.resolveConfirm(false));
    return () => {
      cleanupTrap?.();
      cleanupEsc();
    };
  });
</script>

{#if dialog}
  <div
    class="veil-overlay veil-confirm-overlay"
    bind:this={overlayEl}
    role="presentation"
    onclick={(e) => { if (e.target === overlayEl) uiStore.resolveConfirm(false); }}
  >
    <div
      class="veil-modal veil-confirm"
      role="alertdialog"
      aria-modal="true"
      aria-label={dialog.title}
    >
      <div class="veil-confirm-icon" class:danger={dialog.danger} aria-hidden="true">
        <Icon name={dialog.danger ? 'warning' : 'info'} size={22} />
      </div>
      <h2 class="veil-confirm-title">{dialog.title}</h2>
      <p class="veil-confirm-message">{dialog.message}</p>
      <div class="veil-confirm-actions">
        <button
          type="button"
          class="btn btn-secondary"
          onclick={() => uiStore.resolveConfirm(false)}
        >
          Vazgeç
        </button>
        <button
          type="button"
          class={dialog.danger ? 'btn btn-danger' : 'btn btn-primary'}
          onclick={() => uiStore.resolveConfirm(true)}
        >
          {dialog.confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .veil-confirm-overlay { z-index: 2000; }
  .veil-confirm {
    max-width: 400px;
    padding: var(--space-6);
    gap: var(--space-3);
    text-align: center;
    animation: modal-enter var(--transition-slow) var(--ease-spring);
  }
  .veil-confirm-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
  }
  .veil-confirm-icon.danger {
    background: hsl(0 72% 62% / 0.12);
    color: var(--veil-danger);
  }
  .veil-confirm-title {
    font-size: var(--text-xl);
    font-weight: 700;
    letter-spacing: var(--tracking-tight);
  }
  .veil-confirm-message {
    font-size: var(--text-base);
    color: var(--veil-text-secondary);
    line-height: var(--leading-relaxed);
  }
  .veil-confirm-actions {
    display: flex;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }
  .veil-confirm-actions .btn { flex: 1; }
</style>

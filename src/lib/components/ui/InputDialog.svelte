<script lang="ts">
  import { tick } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { createFocusTrap, handleEscape } from '$lib/utils/accessibility';
  import Icon from './Icon.svelte';

  const dialog = $derived($uiStore.inputDialog);
  let overlayEl = $state<HTMLDivElement | null>(null);
  let value = $state('');
  let showSecret = $state(false);

  $effect(() => {
    if (!dialog) return;
    value = dialog.defaultValue ?? '';
    showSecret = false;
    const overlay = overlayEl;
    if (!overlay) return;
    let cleanupTrap: (() => void) | null = null;
    tick().then(() => {
      cleanupTrap = createFocusTrap(overlay);
    });
    const cleanupEsc = handleEscape(() => uiStore.resolveInput(null));
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
    onclick={(e) => { if (e.target === overlayEl) uiStore.resolveInput(null); }}
  >
    <form
      class="veil-modal veil-confirm"
      aria-label={dialog.title}
      onsubmit={(e) => { e.preventDefault(); uiStore.resolveInput(value); }}
    >
      <div class="veil-confirm-icon" aria-hidden="true">
        <Icon name="lock" size={22} />
      </div>
      <h2 class="veil-confirm-title">{dialog.title}</h2>
      <p class="veil-confirm-message">{dialog.message}</p>
      <div class="veil-input-wrap">
        <div class="veil-pass-field">
          <input
            class="veil-input"
            type={dialog.secret && !showSecret ? 'password' : 'text'}
            bind:value={value}
            placeholder={dialog.placeholder}
            autocomplete="off"
            spellcheck={false}
            required={dialog.secret}
          />
          {#if dialog.secret}
            <button
              type="button"
              class="btn-icon veil-pass-toggle"
              title={showSecret ? 'Gizle' : 'Göster'}
              aria-label={showSecret ? 'Gizle' : 'Göster'}
              onclick={() => (showSecret = !showSecret)}
            >
              <Icon name={showSecret ? 'eye-off' : 'eye'} size={16} />
            </button>
          {/if}
        </div>
      </div>
      <div class="veil-confirm-actions">
        <button
          type="button"
          class="btn btn-secondary"
          onclick={() => uiStore.resolveInput(null)}
        >
          Vazgeç
        </button>
        <button type="submit" class="btn btn-primary" disabled={dialog.secret && value.length === 0}>
          {dialog.confirmLabel}
        </button>
      </div>
    </form>
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
  .veil-input-wrap { text-align: left; }
  .veil-confirm-actions {
    display: flex;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }
  .veil-confirm-actions .btn { flex: 1; }
</style>

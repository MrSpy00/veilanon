<script lang="ts">
  /**
   * VeilSelect — tema ile bütünleşik modern açılır menü.
   * Native <select> yerine geçer: klavye desteği, dış-tık kapatma, animasyon.
   */
  import { tick } from 'svelte';
  import { handleEscape } from '$lib/utils/accessibility';
  import Icon from './Icon.svelte';

  let {
    options = [],
    value = '',
    label = '',
    disabled = false,
    placeholder = 'Seç…',
    onChange,
    class: className = '',
  }: {
    options: Array<{ value: string; label: string }>;
    value?: string;
    label?: string;
    disabled?: boolean;
    placeholder?: string;
    onChange?: (value: string) => void;
    class?: string;
  } = $props();

  const uid = `veil-select-${Math.random().toString(36).slice(2, 9)}`;
  let open = $state(false);
  let openUpward = $state(false);
  let rootEl = $state<HTMLDivElement | null>(null);

  const selected = $derived(options.find(o => o.value === value) ?? null);

  function toggleOpen() {
    if (disabled) return;
    if (!open && rootEl) {
      const rect = rootEl.getBoundingClientRect();
      openUpward = rect.bottom + 280 > window.innerHeight && rect.top > 280;
    }
    open = !open;
  }

  $effect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootEl && !rootEl.contains(e.target as Node)) open = false;
    };
    document.addEventListener('mousedown', onDown);
    const escCleanup = handleEscape(() => (open = false));
    return () => {
      document.removeEventListener('mousedown', onDown);
      escCleanup();
    };
  });

  function pick(option: { value: string; label: string }) {
    open = false;
    if (option.value !== value) onChange?.(option.value);
  }

  function onTriggerKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      toggleOpen();
    }
  }

  function onMenuKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const buttons = Array.from(rootEl?.querySelectorAll<HTMLButtonElement>('[role="option"]') ?? []);
      if (!buttons.length) return;
      const idx = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = e.key === 'ArrowDown' ? (idx + 1) % buttons.length : (idx - 1 + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }
  }

  $effect(() => {
    if (!open) return;
    tick().then(() => {
      rootEl?.querySelector<HTMLButtonElement>('[role="option"][aria-selected="true"]')?.focus();
    });
  });
</script>

<div class="veil-select-wrap {className}" bind:this={rootEl}>
  {#if label}
    <span class="veil-select-label" id={`${uid}-label`}>{label}</span>
  {/if}
  <button
    type="button"
    class="veil-select-trigger"
    class:open
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-labelledby={label ? `${uid}-label` : undefined}
    disabled={disabled}
    onclick={toggleOpen}
    onkeydown={onTriggerKeydown}
  >
    <span class="veil-select-value">{selected?.label ?? placeholder}</span>
    <span class="veil-select-caret" class:open aria-hidden="true">
      <Icon name="arrow-right" size={14} />
    </span>
  </button>

  {#if open}
    <div
      class="veil-select-menu veil-pop-in"
      class:upward={openUpward}
      role="listbox"
      tabindex="-1"
      aria-label={label || undefined}
      onkeydown={onMenuKeydown}
    >
      {#each options as option (option.value)}
        <button
          type="button"
          role="option"
          aria-selected={option.value === value}
          class:active={option.value === value}
          onclick={() => pick(option)}
        >
          <span class="veil-select-option-label">{option.label}</span>
          {#if option.value === value}
            <Icon name="check" size={14} class="veil-select-check" />
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .veil-select-wrap { position: relative; display: inline-flex; flex-direction: column; gap: var(--space-1); }
  .veil-select-label { font-size: var(--text-xs); font-weight: 600; color: var(--veil-text-muted); }
  .veil-select-trigger {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    min-width: 200px;
    width: 100%;
    max-width: 360px;
    padding: 9px 12px;
    background: color-mix(in srgb, var(--veil-bg-elevated) 88%, transparent);
    backdrop-filter: blur(10px);
    color: var(--veil-text-primary);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color var(--t-fast), box-shadow var(--t-fast), background var(--t-fast), transform var(--t-fast);
    box-shadow: 0 2px 10px rgba(0,0,0,0.12);
  }
  .veil-select-trigger:hover:not(:disabled) { border-color: var(--veil-border); background: color-mix(in srgb, var(--veil-bg-elevated) 94%, transparent); }
  .veil-select-trigger:focus-visible { outline: none; border-color: var(--veil-brand); box-shadow: 0 0 0 3px hsl(262 72% 60% / 0.18); }
  .veil-select-trigger.open { border-color: var(--veil-brand); box-shadow: 0 4px 16px hsl(262 72% 60% / 0.18), 0 0 0 3px hsl(262 72% 60% / 0.15); transform: translateY(-1px); }
  .veil-select-trigger:disabled { opacity: 0.45; cursor: not-allowed; }
  .veil-select-value {
    flex: 1;
    min-width: 0;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-select-caret {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transform: rotate(90deg);
    opacity: 0.6;
    transition: transform var(--t-fast);
  }
  .veil-select-caret.open {
    transform: rotate(270deg);
  }
  .veil-select-menu {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    min-width: 100%;
    width: 100%;
    z-index: 1200;
    background: color-mix(in srgb, var(--veil-bg-raised) 92%, transparent);
    backdrop-filter: blur(18px) saturate(1.4);
    -webkit-backdrop-filter: blur(18px) saturate(1.4);
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 14px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(255, 255, 255, 0.06);
    padding: 6px;
    max-height: 280px;
    overflow-y: auto;
    animation: veilDropdownIn 0.16s cubic-bezier(0.16,1,0.3,1);
  }
  @keyframes veilDropdownIn { from { opacity:0; transform: translateY(-6px) scale(0.98); } to { opacity:1; transform: translateY(0) scale(1); } }
  .veil-select-menu.upward {
    top: auto;
    bottom: calc(100% + 6px);
  }
  .veil-select-menu button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: 9px 10px;
    border: none;
    background: transparent;
    border-radius: 10px;
    color: var(--veil-text-secondary);
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background var(--t-fast), color var(--t-fast), transform var(--t-fast);
  }
  .veil-select-menu button:hover { background: rgba(255,255,255,0.07); color: var(--veil-text-primary); }
  .veil-select-menu button:active { transform: scale(0.98); }
  .veil-select-menu button.active { background: var(--veil-brand); color: #fff; font-weight: 600; box-shadow: 0 2px 10px hsl(262 72% 60% / 0.35); }
  .veil-select-menu button:focus-visible { outline: none; background: rgba(255,255,255,0.07); }
  .veil-select-option-label { flex: 1; min-width: 0; white-space: nowrap; }
  .veil-select-check { flex-shrink: 0; }
</style>

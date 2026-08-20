<script lang="ts">
  import { tick } from 'svelte';
  import Icon, { type IconName } from './Icon.svelte';
  import { handleEscape } from '$lib/utils/accessibility';

  export interface ContextMenuItem {
    label: string;
    icon?: IconName;
    shortcut?: string;
    chevron?: boolean;
    danger?: boolean;
    disabled?: boolean;
    separator?: boolean;
    isSlider?: boolean;
    sliderValue?: number;
    sliderMin?: number;
    sliderMax?: number;
    sliderStep?: number;
    onSliderChange?: (val: number) => void;
    onClick?: () => void;
  }

  let {
    open = false,
    x = 0,
    y = 0,
    items = [] as ContextMenuItem[],
    onClose,
  }: {
    open?: boolean;
    x?: number;
    y?: number;
    items?: ContextMenuItem[];
    onClose: () => void;
  } = $props();

  // eslint-disable-next-line svelte/no-ignored-unsubscribe
  let menuEl = $state<HTMLDivElement | null>(null);
  let pos = $state({ x: 0, y: 0 });
  let activeIndex = $state(-1);
  let sliderValues = $state<Record<number, number>>({});

  function itemButtons(): HTMLButtonElement[] {
    return Array.from(menuEl?.querySelectorAll<HTMLButtonElement>('button[role="menuitem"]:not(:disabled)') ?? []);
  }

  function enabledIndices(): number[] {
    return items
      .map((it, i) => (it.separator || it.disabled ? -1 : i))
      .filter((i) => i >= 0);
  }

  $effect(() => {
    if (!open) return;
    activeIndex = -1;
    // Initial guess clamped
    pos = {
      x: Math.max(12, Math.min(x, (typeof window !== 'undefined' ? window.innerWidth : 800) - 240)),
      y: Math.max(12, Math.min(y, (typeof window !== 'undefined' ? window.innerHeight : 600) - 200)),
    };
    const initVals: Record<number, number> = {};
    items.forEach((it, idx) => {
      if (it.isSlider) {
        initVals[idx] = it.sliderValue ?? 100;
      }
    });
    sliderValues = initVals;
    const cleanup = handleEscape(onClose);
    tick().then(() => {
      if (!menuEl) return;
      const rect = menuEl.getBoundingClientRect();
      const targetX = Math.max(12, Math.min(x, window.innerWidth - rect.width - 12));
      const targetY = Math.max(12, Math.min(y, window.innerHeight - rect.height - 12));
      pos = { x: targetX, y: targetY };
      const first = enabledIndices()[0];
      if (first !== undefined) {
        activeIndex = first;
        const btns = itemButtons();
        btns[0]?.focus();
      }
    });
    const onDown = (e: MouseEvent) => {
      if (menuEl && !menuEl.contains(e.target as Node)) onClose();
    };
    document.addEventListener('mousedown', onDown);
    return () => {
      cleanup();
      document.removeEventListener('mousedown', onDown);
    };
  });

  function move(delta: number) {
    const list = enabledIndices();
    if (!list.length) return;
    const cur = list.indexOf(activeIndex);
    const nextIdx = (cur + delta + list.length) % list.length;
    const next = list[nextIdx];
    activeIndex = next;
    const btns = itemButtons();
    btns[nextIdx]?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      move(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      move(-1);
    }
    // Enter/Space: natively activated on the focused item button
  }

  function handleItem(item: ContextMenuItem) {
    if (item.disabled) return;
    onClose();
    item.onClick?.();
  }
</script>

{#if open}
  <div
    class="veil-context-menu veil-pop-in"
    bind:this={menuEl}
    role="menu"
    aria-label="menu"
    tabindex="-1"
    style="left:{pos.x}px;top:{pos.y}px;"
    onkeydown={onKeydown}
  >
    {#each items as item, i (i)}
      {#if item.separator}
        <div class="veil-context-sep" role="separator"></div>
      {:else if item.isSlider}
        <div class="veil-context-slider-wrap" role="none" onclick={(e) => e.stopPropagation()}>
          <div class="veil-context-slider-header">
            <span class="veil-context-slider-label">
              {#if item.icon}<Icon name={item.icon} size={14} />{/if}
              {item.label}
            </span>
            <span class="veil-context-slider-val">%{sliderValues[i] ?? item.sliderValue ?? 100}</span>
          </div>
          <input
            type="range"
            min={item.sliderMin ?? 0}
            max={item.sliderMax ?? 200}
            step={item.sliderStep ?? 1}
            value={sliderValues[i] ?? item.sliderValue ?? 100}
            oninput={(e) => {
              const v = Number((e.target as HTMLInputElement).value);
              sliderValues = { ...sliderValues, [i]: v };
              item.onSliderChange?.(v);
            }}
            class="veil-slider"
            aria-label={item.label}
          />
        </div>
      {:else}
        <button
          class="veil-context-item"
          class:danger={item.danger}
          class:active={i === activeIndex}
          role="menuitem"
          disabled={item.disabled}
          tabindex="-1"
          onmouseenter={() => {
            if (!item.disabled) activeIndex = i;
          }}
          onclick={() => handleItem(item)}
        >
          <span class="veil-context-icon" aria-hidden="true">
            {#if item.icon}<Icon name={item.icon} size={16} />{/if}
          </span>
          <span class="veil-context-label">{item.label}</span>
          {#if item.shortcut}<span class="veil-context-shortcut">{item.shortcut}</span>{/if}
          {#if item.chevron}
            <span class="veil-context-chevron" aria-hidden="true">
              <Icon name="arrow-right" size={14} />
            </span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .veil-context-menu {
    position: fixed;
    z-index: 300;
    min-width: 208px;
    max-width: 280px;
    padding: var(--space-1);
    background: color-mix(in srgb, var(--veil-bg-raised) 92%, transparent);
    backdrop-filter: blur(14px);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-xl);
    display: flex;
    flex-direction: column;
  }
  .veil-context-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: 32px;
    padding: var(--space-1) var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--veil-text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-base);
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-context-item:hover:not(:disabled),
  .veil-context-item.active:not(:disabled) {
    background: var(--veil-bg-overlay);
  }
  .veil-context-item:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .veil-context-item.danger {
    color: var(--veil-danger);
  }
  .veil-context-item.danger:hover:not(:disabled),
  .veil-context-item.danger.active:not(:disabled) {
    background: hsl(0 72% 62% / 0.12);
  }
  .veil-context-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    flex-shrink: 0;
    color: var(--veil-text-muted);
  }
  .veil-context-item:hover:not(:disabled) .veil-context-icon,
  .veil-context-item.active:not(:disabled) .veil-context-icon,
  .veil-context-item.danger .veil-context-icon {
    color: inherit;
  }
  .veil-context-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .veil-context-shortcut {
    margin-left: var(--space-4);
    font-size: var(--text-xs);
    font-weight: 500;
    color: var(--veil-text-muted);
    font-variant-numeric: tabular-nums;
  }
  .veil-context-chevron {
    display: inline-flex;
    flex-shrink: 0;
    color: var(--veil-text-muted);
  }
  .veil-context-sep {
    height: 1px;
    margin: var(--space-1) var(--space-2);
    background: var(--veil-border-subtle);
  }
  .veil-context-slider-wrap {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
  }
  .veil-context-slider-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    font-weight: 500;
  }
  .veil-context-slider-label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .veil-context-slider-val {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--veil-text-primary);
  }
  .veil-context-slider-wrap .veil-slider {
    width: 100%;
    height: 4px;
    accent-color: var(--veil-brand);
    border-radius: var(--radius-full);
    cursor: pointer;
  }
</style>

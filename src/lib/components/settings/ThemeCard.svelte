<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import type { ThemePreset } from '$lib/themes/presets';

  interface Props {
    preset: ThemePreset;
    active: boolean;
    isDark: boolean;
    onSelect: (id: string) => void;
  }

  let { preset, active, isDark, onSelect }: Props = $props();

  const CATEGORY_LABELS: Record<string, string> = {
    signature: 'İmza',
    aurora: 'Aurora',
    editorial: 'Editoryal',
    nature: 'Doğa',
    mono: 'Mono',
  };

  const tokens = $derived(isDark ? preset.dark : preset.light);
</script>

<button
  type="button"
  class="veil-preset-card"
  class:active
  onclick={() => onSelect(preset.id)}
  aria-pressed={active}
  aria-label={`Tema: ${preset.name}`}
>
  <!-- Miniature preview built strictly with CSS variables -->
  <div
    class="veil-preset-preview"
    style="
      --p-void: {tokens['--veil-bg-void']};
      --p-base: {tokens['--veil-bg-base']};
      --p-surface: {tokens['--veil-bg-surface']};
      --p-sidebar: {tokens['--veil-sidebar-bg']};
      --p-brand: {tokens['--veil-brand']};
      --p-text: {tokens['--veil-text-primary']};
      --p-muted: {tokens['--veil-text-muted']};
      --p-border: {tokens['--veil-border-subtle']};
    "
  >
    <!-- Left mini sidebar -->
    <div class="mini-sidebar">
      <span class="mini-dot brand-dot"></span>
      <span class="mini-dot"></span>
      <span class="mini-dot"></span>
    </div>

    <!-- Mini channel list -->
    <div class="mini-channels">
      <div class="mini-line mini-line-header"></div>
      <div class="mini-line mini-line-active"></div>
      <div class="mini-line"></div>
      <div class="mini-line"></div>
    </div>

    <!-- Mini chat area -->
    <div class="mini-chat">
      <div class="mini-bubble mini-bubble-in">
        <span class="mini-avatar"></span>
        <span class="mini-text"></span>
      </div>
      <div class="mini-bubble mini-bubble-own">
        <span class="mini-text short"></span>
      </div>
      <div class="mini-input">
        <span class="mini-btn"></span>
      </div>
    </div>

    {#if active}
      <div class="active-badge" aria-hidden="true">
        <Icon name="check" size={12} />
      </div>
    {/if}
  </div>

  <!-- Card Info -->
  <div class="veil-preset-info">
    <div class="veil-preset-header">
      <span class="color-indicator" style="background: {tokens['--veil-brand']};"></span>
      <span class="veil-preset-name">{preset.name}</span>
      <span class="category-badge category-{preset.category}">{CATEGORY_LABELS[preset.category]}</span>
    </div>
    <p class="veil-preset-desc">{preset.description}</p>
  </div>
</button>

<style>
  .veil-preset-card {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-2, 0.5rem);
    padding: var(--space-2, 0.5rem);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl, 1rem);
    cursor: pointer;
    text-align: left;
    transition: all var(--t-fast, 150ms ease);
    position: relative;
    outline: none;
    user-select: none;
  }

  .veil-preset-card:hover {
    border-color: var(--veil-border);
    transform: translateY(-2px);
    box-shadow: var(--shadow-md, 0 4px 12px rgba(0, 0, 0, 0.25));
  }

  .veil-preset-card.active {
    border-color: var(--veil-brand);
    background: var(--veil-brand-subtle);
    box-shadow: 0 0 0 2px var(--veil-brand), 0 8px 20px var(--veil-theme-glow, rgba(124, 58, 237, 0.2));
  }

  .veil-preset-preview {
    position: relative;
    height: 78px;
    border-radius: var(--radius-lg, 0.75rem);
    background: var(--p-void);
    border: 1px solid var(--p-border);
    display: flex;
    overflow: hidden;
    gap: 1px;
    padding: 0;
  }

  .mini-sidebar {
    width: 22px;
    background: var(--p-sidebar);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 6px 0;
    gap: 4px;
    border-right: 1px solid var(--p-border);
  }

  .mini-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--p-surface);
  }

  .mini-dot.brand-dot {
    background: var(--p-brand);
    box-shadow: 0 0 4px var(--p-brand);
  }

  .mini-channels {
    width: 44px;
    background: var(--p-base);
    padding: 6px 4px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-right: 1px solid var(--p-border);
  }

  .mini-line {
    height: 5px;
    border-radius: 3px;
    background: var(--p-surface);
    width: 80%;
  }

  .mini-line-header {
    width: 50%;
    background: var(--p-muted);
    opacity: 0.6;
    margin-bottom: 2px;
  }

  .mini-line-active {
    width: 95%;
    background: var(--p-brand);
    opacity: 0.8;
  }

  .mini-chat {
    flex: 1;
    background: var(--p-void);
    padding: 6px 6px 4px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .mini-bubble {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .mini-avatar {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--p-muted);
    opacity: 0.8;
    flex-shrink: 0;
  }

  .mini-text {
    height: 6px;
    border-radius: 3px;
    background: var(--p-surface);
    flex: 1;
  }

  .mini-bubble-own {
    justify-content: flex-end;
  }

  .mini-bubble-own .mini-text {
    flex: none;
    width: 55%;
    background: var(--p-brand);
    opacity: 0.35;
  }

  .mini-input {
    height: 12px;
    background: var(--p-surface);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 0 3px;
  }

  .mini-btn {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--p-brand);
  }

  .active-badge {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
    animation: scale-up 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes scale-up {
    from { transform: scale(0.5); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }

  .veil-preset-info {
    padding: 2px 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .veil-preset-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .color-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .veil-preset-name {
    font-size: var(--text-sm, 12px);
    font-weight: 700;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-preset-desc {
    font-size: 11px;
    color: var(--veil-text-secondary);
    line-height: 1.35;
    margin: 0;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .category-badge {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 2px 5px;
    border-radius: 4px;
    opacity: 0.85;
    margin-left: auto;
    flex-shrink: 0;
  }
  .category-signature { background: hsl(262 72% 60% / 0.2); color: hsl(262 72% 75%); }
  .category-aurora { background: hsl(280 80% 60% / 0.2); color: hsl(280 80% 75%); }
  .category-editorial { background: hsl(340 70% 60% / 0.2); color: hsl(340 70% 75%); }
  .category-nature { background: hsl(142 70% 40% / 0.2); color: hsl(142 70% 55%); }
  .category-mono { background: hsl(220 14% 50% / 0.2); color: hsl(220 14% 65%); }
</style>

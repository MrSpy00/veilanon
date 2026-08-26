<script lang="ts">
  import { uiStore } from '$lib/stores/ui';
  import { settingsApi } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import { THEME_CATALOG, THEME_CATEGORIES, type ThemeCategory } from '$lib/themes/presets';
  import ThemeCard from './ThemeCard.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';

  const ui = $derived($uiStore);

  let selectedCategory = $state<ThemeCategory>('all');
  let searchQuery = $state('');

  const isDark = $derived(
    ui.theme === 'system'
      ? (typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)').matches : true)
      : ui.theme === 'dark'
  );

  const filteredThemes = $derived(
    THEME_CATALOG.filter((preset) => {
      // Category filter
      if (selectedCategory !== 'all' && preset.category !== selectedCategory) {
        return false;
      }
      // Search query filter
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase().trim();
        const matchName = preset.name.toLowerCase().includes(q);
        const matchNameEn = preset.nameEn.toLowerCase().includes(q);
        const matchDesc = preset.description.toLowerCase().includes(q);
        return matchName || matchNameEn || matchDesc;
      }
      return true;
    })
  );

  // Count display for results
  const resultCount = $derived(filteredThemes.length);
  const totalCount = THEME_CATALOG.length;

  async function handleSelectTheme(id: string) {
    uiStore.setPresetTheme(id);
    const preset = THEME_CATALOG.find((t) => t.id === id);
    try {
      await settingsApi.update({ presetThemeId: id });
      if (preset) {
        toastStore.success(`"${preset.name}" teması uygulandı.`);
      }
    } catch {
      toastStore.error('Tema tercihi kaydedilemedi.');
    }
  }
</script>

<div class="veil-theme-gallery-container">
  <!-- Controls Bar: Category Tabs & Search -->
  <div class="veil-gallery-header">
    <div class="veil-category-tabs" role="tablist" aria-label="Tema kategorileri">
      {#each THEME_CATEGORIES as cat (cat.id)}
        {@const count = cat.id === 'all' ? THEME_CATALOG.length : THEME_CATALOG.filter(t => t.category === cat.id).length}
        <button
          type="button"
          role="tab"
          class="veil-cat-tab"
          class:active={selectedCategory === cat.id}
          aria-selected={selectedCategory === cat.id}
          onclick={() => (selectedCategory = cat.id)}
        >
          <span>{cat.label}</span>
          <span class="cat-count">{count}</span>
        </button>
      {/each}
    </div>

    <div class="veil-gallery-search">
      <Icon name="search" size={14} />
      <input
        type="text"
        class="veil-search-input"
        placeholder="25 tema içinde ara…"
        bind:value={searchQuery}
        aria-label="Temalarda ara"
      />
      {#if searchQuery}
        <button
          type="button"
          class="btn-icon"
          style="width: 20px; height: 20px;"
          title="Temizle"
          onclick={() => (searchQuery = '')}
        >
          <Icon name="x" size={12} />
        </button>
      {/if}
    </div>
  </div>

  <!-- Results count + Grid -->
  {#if searchQuery.trim() || selectedCategory !== 'all'}
    <div class="veil-results-info">
      {#if resultCount === 0}
        <span>Sonuç bulunamadı</span>
      {:else}
        <span>{resultCount} tema bulundu</span>
      {/if}
    </div>
  {/if}

  {#if filteredThemes.length === 0}
    <div class="veil-gallery-empty">
      <Icon name="search" size={32} />
      <p>Arama kriterlerinize uygun tema bulunamadı.</p>
      <button type="button" class="btn btn-ghost btn-sm" onclick={() => { searchQuery = ''; selectedCategory = 'all'; }}>Filtreleri Temizle</button>
    </div>
  {:else}
    <div class="veil-preset-grid" role="radiogroup" aria-label="Hazır Tema Kataloğu">
      {#each filteredThemes as preset (preset.id)}
        <ThemeCard
          {preset}
          active={ui.presetThemeId === preset.id}
          {isDark}
          onSelect={handleSelectTheme}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .veil-theme-gallery-container {
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 0.75rem);
  }

  .veil-gallery-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3, 0.75rem);
    flex-wrap: wrap;
    padding-bottom: var(--space-2, 0.5rem);
  }

  .veil-category-tabs {
    display: flex;
    align-items: center;
    gap: var(--space-1, 0.25rem);
    background: var(--veil-bg-surface);
    padding: 3px;
    border-radius: var(--radius-lg, 0.75rem);
    border: 1px solid var(--veil-border-subtle);
  }

  .veil-cat-tab {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font-size: var(--text-xs, 11px);
    font-weight: 600;
    color: var(--veil-text-secondary);
    background: transparent;
    border: none;
    border-radius: var(--radius-md, 0.5rem);
    cursor: pointer;
    transition: all var(--t-fast, 150ms ease);
  }

  .veil-cat-tab:hover {
    color: var(--veil-text-primary);
    background: var(--veil-bg-elevated);
  }

  .veil-cat-tab.active {
    background: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
  }

  .cat-count {
    font-size: 10px;
    opacity: 0.8;
    background: rgba(0, 0, 0, 0.2);
    padding: 1px 5px;
    border-radius: 999px;
  }

  .veil-cat-tab.active .cat-count {
    background: rgba(255, 255, 255, 0.25);
  }

  .veil-gallery-search {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.5rem);
    padding: 0 var(--space-3, 0.75rem);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg, 0.75rem);
    color: var(--veil-text-muted);
    min-width: 180px;
    flex: 1;
    max-width: 260px;
    transition: border-color var(--t-fast, 150ms ease);
  }

  .veil-gallery-search:focus-within {
    border-color: var(--veil-brand);
  }

  .veil-search-input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    color: var(--veil-text-primary);
    font-size: var(--text-xs, 12px);
    font-family: var(--font-sans);
    padding: var(--space-2, 0.5rem) 0;
    outline: none;
  }

  .veil-preset-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-3, 0.75rem);
  }

  .veil-gallery-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-6, 1.5rem);
    gap: var(--space-2, 0.5rem);
    color: var(--veil-text-muted);
    text-align: center;
    background: var(--veil-bg-surface);
    border-radius: var(--radius-xl, 1rem);
    border: 1px dashed var(--veil-border-subtle);
  }

  .veil-results-info {
    font-size: 11px;
    color: var(--veil-text-muted);
    padding: 0 2px;
  }
</style>

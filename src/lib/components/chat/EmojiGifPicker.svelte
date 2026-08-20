<script lang="ts">
  /**
   * EmojiGifPicker — gelişmiş emoji + GIF seçici.
   * Emoji: arama, kategoriler, favoriler. GIF: Tenor/Giphy arama, trend, favoriler.
   */
  import { onMount } from 'svelte';
  import { gifApi, type GifResult } from '$lib/api/tauri';
  import Icon from '../ui/Icon.svelte';
  import {
    EMOJI_CATEGORIES,
    loadFavoriteEmojis,
    saveFavoriteEmoji,
    removeFavoriteEmoji,
    searchEmojis,
    type EmojiEntry,
  } from '$lib/utils/emoji-data';

  let { onPickEmoji, onPickGif, onClose }: {
    onPickEmoji: (emoji: string) => void;
    onPickGif: (gif: GifResult) => void;
    onClose: () => void;
  } = $props();

  type Tab = 'emoji' | 'gif';
  let tab = $state<Tab>('emoji');
  let emojiQuery = $state('');
  let activeCategory = $state(EMOJI_CATEGORIES[0].id);
  let favorites = $state<string[]>([]);

  let gifQuery = $state('');
  let gifs = $state<GifResult[]>([]);
  let gifLoading = $state(false);
  let gifError = $state<string | null>(null);
  let gifFavorites = $state<GifResult[]>([]);
  let gifSearched = $state(false);

  const GIF_FAV_KEY = 'veilanon-fav-gifs';

  const currentEmojis = $derived(
    emojiQuery.trim()
      ? searchEmojis(emojiQuery)
      : (EMOJI_CATEGORIES.find(c => c.id === activeCategory)?.emojis ?? [])
  );

  const favEmojis = $derived(favorites.filter(f => EMOJI_CATEGORIES.some(c => c.emojis.some(e => e.e === f))));

  onMount(() => {
    favorites = loadFavoriteEmojis();
    try {
      const raw = localStorage.getItem(GIF_FAV_KEY);
      gifFavorites = raw ? JSON.parse(raw) : [];
    } catch {
      gifFavorites = [];
    }
    // Eagerly prefetch trending GIFs on picker mount
    void loadTrending();
  });

  function pickEmoji(entry: EmojiEntry) {
    onPickEmoji(entry.e);
  }

  function toggleFav(entry: EmojiEntry, e: MouseEvent | KeyboardEvent) {
    e.stopPropagation();
    if (favorites.includes(entry.e)) {
      removeFavoriteEmoji(entry.e);
      favorites = favorites.filter(f => f !== entry.e);
    } else {
      saveFavoriteEmoji(entry.e);
      favorites = loadFavoriteEmojis();
    }
  }

  async function loadTrending() {
    gifLoading = true;
    gifError = null;
    gifSearched = true;
    try {
      gifs = await gifApi.trending(24);
    } catch (err) {
      gifError = String(err).replace(/^Error:\s*/, '');
      gifs = [];
    } finally {
      gifLoading = false;
    }
  }

  let searchDebounce: ReturnType<typeof setTimeout> | null = null;

  function onGifInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    gifQuery = val;
    if (searchDebounce) clearTimeout(searchDebounce);
    if (!val.trim()) {
      void loadTrending();
      return;
    }
    searchDebounce = setTimeout(() => {
      void searchGifs();
    }, 450);
  }

  async function searchGifs() {
    const q = gifQuery.trim();
    if (!q) {
      void loadTrending();
      return;
    }
    gifLoading = true;
    gifError = null;
    gifSearched = true;
    try {
      gifs = await gifApi.search(q, 24);
    } catch (err) {
      gifError = 'GIF arama şu anda kullanılamıyor.';
      gifs = [];
    } finally {
      gifLoading = false;
    }
  }

  function toggleGifFav(gif: GifResult, e: MouseEvent | KeyboardEvent) {
    e.stopPropagation();
    if (gifFavorites.some(g => g.id === gif.id)) {
      gifFavorites = gifFavorites.filter(g => g.id !== gif.id);
    } else {
      gifFavorites = [gif, ...gifFavorites].slice(0, 24);
    }
    localStorage.setItem(GIF_FAV_KEY, JSON.stringify(gifFavorites));
  }

  function pickGif(gif: GifResult) {
    onPickGif(gif);
  }

  function switchTab(t: Tab) {
    tab = t;
    if (t === 'gif' && gifs.length === 0 && !gifLoading) {
      void loadTrending();
    }
  }
</script>

<div class="veil-eg-picker veil-pop-in" role="dialog" aria-label="Emoji ve GIF seçici">
  <div class="veil-eg-tabs" role="tablist">
    <button
      type="button"
      role="tab"
      aria-selected={tab === 'emoji'}
      class:active={tab === 'emoji'}
      onclick={() => switchTab('emoji')}
    >
      <Icon name="sparkle" size={14} />
      İfadeler
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={tab === 'gif'}
      class:active={tab === 'gif'}
      onclick={() => switchTab('gif')}
    >
      <Icon name="search" size={14} />
      GIF
    </button>
    <button type="button" class="veil-eg-close" aria-label="Kapat" title="Kapat" onclick={onClose}>
      <Icon name="x" size={14} />
    </button>
  </div>

  {#if tab === 'emoji'}
    <div class="veil-eg-search">
      <span class="veil-eg-search-icon" aria-hidden="true"><Icon name="search" size={14} /></span>
      <input
        type="text"
        placeholder="Emoji ara…"
        aria-label="Emoji ara"
        autocomplete="off"
        bind:value={emojiQuery}
      />
    </div>

    {#if !emojiQuery.trim()}
      <div class="veil-eg-cats" role="tablist" aria-label="Kategoriler">
        {#each EMOJI_CATEGORIES as cat (cat.id)}
          <button
            type="button"
            role="tab"
            aria-selected={activeCategory === cat.id}
            class:active={activeCategory === cat.id}
            title={cat.label}
            onclick={() => (activeCategory = cat.id)}
          >
            {cat.emojis[0]?.e ?? '❓'}
          </button>
        {/each}
      </div>
    {/if}

    <div class="veil-eg-scroll">
      {#if favEmojis.length > 0 && !emojiQuery.trim()}
        <div class="veil-eg-section-label">
          <Icon name="star" size={11} />
          Favoriler
        </div>
        <div class="veil-eg-grid">
          {#each favEmojis as e (e)}
            <button
              type="button"
              class="veil-eg-emoji"
              aria-label={e}
              onclick={() => onPickEmoji(e)}
            >
              {e}
            </button>
          {/each}
        </div>
      {/if}

      {#if currentEmojis.length === 0}
        <p class="veil-eg-empty">Eşleşen emoji yok.</p>
      {:else}
        <div class="veil-eg-grid">
          {#each currentEmojis as entry (entry.e + entry.k)}
            <button
              type="button"
              class="veil-eg-emoji"
              aria-label={entry.e}
              title={entry.e}
              onclick={() => pickEmoji(entry)}
              oncontextmenu={(e) => { e.preventDefault(); toggleFav(entry, e); }}
            >
              {entry.e}
              <span
                class="veil-eg-fav"
                class:on={favorites.includes(entry.e)}
                role="button"
                tabindex="0"
                aria-label="Favorilere ekle/kaldır"
                onclick={(e) => toggleFav(entry, e)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleFav(entry, e); } }}
              >
                <Icon name="star" size={9} />
              </span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <div class="veil-eg-search veil-eg-gif-search">
      <input
        type="text"
        placeholder="GIF ara…"
        aria-label="GIF ara"
        autocomplete="off"
        value={gifQuery}
        oninput={onGifInput}
        onkeydown={(e) => { if (e.key === 'Enter') searchGifs(); }}
      />
      <button type="button" class="btn btn-primary btn-sm" onclick={searchGifs} disabled={!gifQuery.trim() || gifLoading}>
        Ara
      </button>
      <button
        type="button"
        class="btn-icon"
        title="Popüler GIF'ler"
        aria-label="Popüler GIF'ler"
        onclick={loadTrending}
      >
        <Icon name="sparkle" size={15} />
      </button>
    </div>

    <div class="veil-eg-scroll">
      {#if gifLoading}
        <div class="veil-eg-loading"><div class="veil-spinner veil-spinner-sm"></div></div>
      {:else if gifError}
        <div class="veil-eg-error">
          <Icon name="warning" size={16} />
          <p>{gifError}</p>
          <button type="button" class="btn btn-secondary btn-sm" style="margin-top: var(--space-2);" onclick={() => (gifQuery.trim() ? searchGifs() : loadTrending())}>
            <Icon name="refresh-cw" size={13} />
            Tekrar Dene
          </button>
        </div>
      {:else if gifFavorites.length > 0 && !gifQuery.trim() && gifs.length === 0}
        <div class="veil-eg-section-label">
          <Icon name="star" size={11} />
          Favori GIF'ler
        </div>
        <div class="veil-eg-gif-grid">
          {#each gifFavorites as gif (gif.id)}
            <button
              type="button"
              class="veil-eg-gif"
              title={gif.title || gif.provider}
              onclick={() => pickGif(gif)}
            >
              <img src={gif.preview} alt={gif.title} loading="lazy" referrerpolicy="no-referrer" />
              <span
                class="veil-eg-fav veil-eg-fav-gif"
                role="button"
                tabindex="0"
                aria-label="Favorilere ekle/kaldır"
                onclick={(e) => toggleGifFav(gif, e)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleGifFav(gif, e); } }}
              >
                <Icon name="star" size={9} />
              </span>
            </button>
          {/each}
        </div>
      {/if}

      {#if gifs.length > 0}
        <div class="veil-eg-section-label">{gifQuery.trim() ? `Sonuçlar — ${gifs.length}` : 'Popüler GIF\'ler'}</div>
        <div class="veil-eg-gif-grid">
          {#each gifs as gif (gif.id)}
            <button
              type="button"
              class="veil-eg-gif"
              title={gif.title || gif.provider}
              onclick={() => pickGif(gif)}
            >
              <img src={gif.preview} alt={gif.title} loading="lazy" referrerpolicy="no-referrer" />
              <span
                class="veil-eg-fav veil-eg-fav-gif"
                class:on={gifFavorites.some(g => g.id === gif.id)}
                role="button"
                tabindex="0"
                aria-label="Favorilere ekle/kaldır"
                onclick={(e) => toggleGifFav(gif, e)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleGifFav(gif, e); } }}
              >
                <Icon name="star" size={9} />
              </span>
            </button>
          {/each}
        </div>
      {:else if !gifLoading && !gifError && gifFavorites.length === 0}
        <div class="veil-eg-empty-state">
          <Icon name="film" size={28} />
          <p class="veil-eg-empty">Popüler GIF'leri keşfet veya yukarıdan ara</p>
          <div class="veil-eg-quick-chips">
            {#each ['Trend', 'Tepkiler', 'Gaming', 'Anime', 'Memeler', 'Komik'] as chip}
              <button
                type="button"
                class="btn btn-secondary btn-xs"
                onclick={() => {
                  gifQuery = chip;
                  void searchGifs();
                }}
              >
                {chip}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .veil-eg-picker {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    z-index: 60;
    width: 372px;
    max-width: calc(100vw - var(--space-4));
    background: color-mix(in srgb, var(--veil-bg-raised) 94%, transparent);
    backdrop-filter: blur(14px);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .veil-eg-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
  }
  .veil-eg-tabs button[role="tab"] {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-eg-tabs button[role="tab"]:hover { color: var(--veil-text-primary); }
  .veil-eg-tabs button[role="tab"].active { background: var(--veil-brand-subtle); color: var(--veil-brand); }
  .veil-eg-close {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--veil-text-muted);
    cursor: pointer;
  }
  .veil-eg-close:hover { background: var(--veil-bg-overlay); color: var(--veil-text-primary); }

  .veil-eg-search {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
    position: relative;
  }
  .veil-eg-search input {
    flex: 1;
    min-width: 0;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
    color: var(--veil-text-primary);
    font-size: var(--text-sm);
    font-family: var(--font-sans);
    padding: var(--space-1) var(--space-2) var(--space-1) 26px;
    outline: none;
  }
  .veil-eg-search input:focus { border-color: var(--veil-brand); }
  .veil-eg-search-icon {
    position: absolute;
    left: 12px;
    color: var(--veil-text-muted);
    pointer-events: none;
  }
  .veil-eg-gif-search { gap: var(--space-1); }
  .veil-eg-gif-search input { padding-left: var(--space-2); }

  .veil-eg-cats {
    display: flex;
    gap: 2px;
    padding: var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
    overflow-x: auto;
  }
  .veil-eg-cats button {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    font-size: 17px;
    cursor: pointer;
    transition: background var(--t-fast), transform var(--t-fast);
  }
  .veil-eg-cats button:hover { background: var(--veil-bg-overlay); transform: scale(1.08); }
  .veil-eg-cats button.active { background: var(--veil-brand-subtle); }

  .veil-eg-scroll {
    max-height: 300px;
    overflow-y: auto;
    padding: var(--space-2);
  }
  .veil-eg-section-label {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-1);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-eg-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 2px;
  }
  .veil-eg-emoji {
    position: relative;
    width: 40px;
    height: 40px;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    font-size: 23px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background var(--t-fast), transform var(--t-fast);
  }
  .veil-eg-emoji:hover { background: var(--veil-bg-overlay); transform: scale(1.12); }
  .veil-eg-fav {
    position: absolute;
    top: 2px;
    right: 2px;
    display: none;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    color: var(--veil-text-muted);
    cursor: pointer;
    z-index: 2;
  }
  .veil-eg-fav.on { display: inline-flex; color: var(--veil-warning); border-color: hsl(38 92% 50% / 0.4); }
  .veil-eg-emoji:hover .veil-eg-fav { display: inline-flex; }
  .veil-eg-fav:hover { color: var(--veil-warning); background: var(--veil-bg-elevated); }

  .veil-eg-gif-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-1);
  }
  .veil-eg-gif {
    position: relative;
    aspect-ratio: 4/3;
    border: none;
    padding: 0;
    background: var(--veil-bg-surface);
    border-radius: var(--radius-md);
    overflow: hidden;
    cursor: pointer;
    transition: transform var(--t-fast), box-shadow var(--t-fast);
  }
  .veil-eg-gif:hover { transform: scale(1.03); box-shadow: 0 0 0 2px var(--veil-brand); }
  .veil-eg-gif img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .veil-eg-fav-gif { display: none; }
  .veil-eg-gif:hover .veil-eg-fav-gif { display: inline-flex; }
  .veil-eg-fav-gif.on { display: inline-flex; }

  .veil-eg-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-6) var(--space-3);
    text-align: center;
    color: var(--veil-text-muted);
  }
  .veil-eg-empty {
    padding: var(--space-2) var(--space-3);
    text-align: center;
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
    margin: 0;
  }
  .veil-eg-quick-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    justify-content: center;
    margin-top: var(--space-3);
  }
  .veil-eg-loading {
    display: flex;
    justify-content: center;
    padding: var(--space-6) 0;
  }
  .veil-eg-error {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-3);
    color: var(--veil-warning);
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
  }
  .veil-eg-error p { word-break: break-word; }
</style>

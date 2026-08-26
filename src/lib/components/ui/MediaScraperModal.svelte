<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toastStore } from '$lib/stores/notifications';

  interface MediaCandidate {
    url: string;
    media_type: string;
    source: string;
    poster?: string | null;
  }

  interface ScrapeResult {
    success: boolean;
    media_urls: MediaCandidate[];
    title?: string;
    error?: string;
  }

  interface Props {
    title?: string;
    aspectRatio?: number; // 1 for avatar, 3 for banner
    onSelect: (urlOrDataUrl: string) => void;
    onClose: () => void;
  }

  let {
    title = 'Webden / Medyadan İçe Aktar',
    aspectRatio = 1,
    onSelect,
    onClose,
  }: Props = $props();

  let targetUrl = $state('');
  let isScraping = $state(false);
  let pageTitle = $state<string | null>(null);
  let candidates = $state<MediaCandidate[]>([]);
  let selectedCandidate = $state<MediaCandidate | null>(null);
  let previewLoading = $state(false);

  async function handleScrape() {
    const url = targetUrl.trim();
    if (!url) {
      toastStore.warning('Lütfen geçerli bir web veya medya URL\'si girin.');
      return;
    }
    if (!url.startsWith('http://') && !url.startsWith('https://')) {
      targetUrl = 'https://' + url;
    }

    isScraping = true;
    pageTitle = null;
    candidates = [];
    selectedCandidate = null;

    try {
      const res = await invoke<ScrapeResult>('scrape_url', { url: targetUrl.trim() });
      if (res && res.success && res.media_urls && res.media_urls.length > 0) {
        candidates = res.media_urls;
        pageTitle = res.title || null;
        selectedCandidate = candidates[0];
        toastStore.success(`${candidates.length} adet medya bulundu!`);
      } else {
        toastStore.error(res?.error || 'Bu sayfada uygun medya bulunamadı.');
      }
    } catch (err) {
      toastStore.error(`Tarama hatası: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      isScraping = false;
    }
  }

  async function handleConfirm() {
    if (!selectedCandidate) return;
    previewLoading = true;
    try {
      onSelect(selectedCandidate.url);
    } catch {
      toastStore.error('Seçilen medya işlenemedi.');
    } finally {
      previewLoading = false;
    }
  }
</script>

<div class="veil-modal-backdrop" onclick={onClose} role="presentation">
  <div
    class="veil-modal-content veil-scraper-modal"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-labelledby="scraper-modal-title"
    aria-modal="true"
  >
    <div class="veil-modal-header">
      <div class="veil-modal-header-title-wrap">
        <div class="veil-scraper-icon-badge">
          <Icon name="globe" size={18} />
        </div>
        <div>
          <h3 id="scraper-modal-title" class="veil-modal-title">{title}</h3>
          <p class="veil-modal-subtitle">Web sayfası URL'si veya doğrudan görsel/video linki girin.</p>
        </div>
      </div>
      <button class="btn-icon veil-modal-close" onclick={onClose} aria-label="Kapat" type="button">
        <Icon name="x" size={16} />
      </button>
    </div>

    <div class="veil-scraper-body">
      <!-- URL Search Form -->
      <form class="veil-scraper-input-row" onsubmit={(e) => { e.preventDefault(); handleScrape(); }}>
        <div class="veil-scraper-input-box">
          <Icon name="link" size={16} />
          <input
            type="url"
            class="veil-scraper-input"
            placeholder="https://example.com/gallery veya https://.../image.png"
            bind:value={targetUrl}
            disabled={isScraping}
            required
          />
        </div>
        <button class="btn btn-primary veil-scraper-btn" type="submit" disabled={isScraping || !targetUrl.trim()}>
          {#if isScraping}
            <span class="veil-spinner veil-spinner-xs"></span>
            <span>Taranıyor…</span>
          {:else}
            <Icon name="search" size={14} />
            <span>Medyayı Çek</span>
          {/if}
        </button>
      </form>

      {#if pageTitle}
        <div class="veil-scraper-page-title">
          <Icon name="info" size={13} />
          <span>Sayfa Başlığı: <strong>{pageTitle}</strong></span>
        </div>
      {/if}

      {#if candidates.length > 0}
        <div class="veil-scraper-grid-container">
          <div class="veil-scraper-section-header">
            <span>Bulunan Medyalar ({candidates.length})</span>
            <span class="veil-scraper-hint">Birini seçin ve uygulayın</span>
          </div>

          <div class="veil-scraper-grid">
            {#each candidates as cand, idx}
              {@const isSelected = selectedCandidate === cand}
              <button
                type="button"
                class="veil-scraper-card"
                class:selected={isSelected}
                onclick={() => { selectedCandidate = cand; }}
              >
                <div class="veil-scraper-thumb-wrap">
                  {#if cand.media_type === 'video'}
                    {#if cand.poster}
                      <img src={cand.poster} alt="Video Önizleme" class="veil-scraper-thumb" />
                    {:else}
                      <video src={cand.url} class="veil-scraper-thumb" muted preload="metadata"></video>
                    {/if}
                    <div class="veil-scraper-type-pill video">
                      <Icon name="play" size={10} />
                      <span>Video</span>
                    </div>
                  {:else}
                    <img src={cand.url} alt="Görsel Önizleme" class="veil-scraper-thumb" loading="lazy" />
                    <div class="veil-scraper-type-pill image">
                      <Icon name="image" size={10} />
                      <span>Görsel</span>
                    </div>
                  {/if}

                  {#if isSelected}
                    <div class="veil-scraper-selected-check">
                      <Icon name="check" size={14} />
                    </div>
                  {/if}
                </div>
                <div class="veil-scraper-source-tag">{cand.source}</div>
              </button>
            {/each}
          </div>
        </div>
      {:else if !isScraping}
        <div class="veil-scraper-empty-state">
          <Icon name="globe" size={32} />
          <p>Herhangi bir web sayfası linki (ör. MotionBGs, Unsplash, Pexels, vb.) veya doğrudan görsel linki yapıştırarak tarayın.</p>
        </div>
      {/if}
    </div>

    <div class="veil-modal-footer">
      <button class="btn btn-ghost" type="button" onclick={onClose}>İptal</button>
      <button
        class="btn btn-primary"
        type="button"
        onclick={handleConfirm}
        disabled={!selectedCandidate || previewLoading}
      >
        {#if previewLoading}
          <span class="veil-spinner veil-spinner-xs"></span>
          <span>İşleniyor…</span>
        {:else}
          <Icon name="check" size={14} />
          <span>Seçileni Kullan</span>
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .veil-scraper-modal {
    max-width: 680px;
    width: 95vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .veil-modal-header-title-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .veil-scraper-icon-badge {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-lg);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .veil-scraper-body {
    padding: var(--space-4) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    overflow-y: auto;
    flex: 1;
    min-height: 200px;
  }
  .veil-scraper-input-row {
    display: flex;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  .veil-scraper-input-box {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
    padding: 0 var(--space-3);
    color: var(--veil-text-muted);
  }
  .veil-scraper-input-box:focus-within {
    border-color: var(--veil-brand);
    box-shadow: 0 0 0 2px var(--veil-brand-subtle);
  }
  .veil-scraper-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    padding: var(--space-2) var(--space-2);
    color: var(--veil-text);
    font-size: var(--text-sm);
  }
  .veil-scraper-btn {
    flex-shrink: 0;
  }
  .veil-scraper-page-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-surface);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    border-left: 3px solid var(--veil-brand);
  }
  .veil-scraper-grid-container {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-scraper-section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-muted);
  }
  .veil-scraper-hint {
    font-size: 11px;
    color: var(--veil-brand);
  }
  .veil-scraper-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: var(--space-3);
    max-height: 340px;
    overflow-y: auto;
    padding: 2px;
  }
  .veil-scraper-card {
    display: flex;
    flex-direction: column;
    border: 2px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    background: var(--veil-bg-surface);
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    transition: all var(--t-fast);
    padding: 0;
  }
  .veil-scraper-card:hover {
    border-color: var(--veil-border-hover);
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
  }
  .veil-scraper-card.selected {
    border-color: var(--veil-brand);
    box-shadow: 0 0 0 2px var(--veil-brand-subtle);
  }
  .veil-scraper-thumb-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: var(--veil-bg-void);
    overflow: hidden;
  }
  .veil-scraper-thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .veil-scraper-type-pill {
    position: absolute;
    top: 4px;
    left: 4px;
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 2px 6px;
    border-radius: var(--radius-full);
    font-size: 10px;
    font-weight: 700;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    color: #fff;
  }
  .veil-scraper-type-pill.video {
    color: #38bdf8;
  }
  .veil-scraper-type-pill.image {
    color: #a78bfa;
  }
  .veil-scraper-selected-check {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 20px;
    height: 20px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
  }
  .veil-scraper-source-tag {
    padding: var(--space-1) var(--space-2);
    font-size: 10px;
    color: var(--veil-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    background: var(--veil-bg-elevated);
    font-family: var(--font-mono);
  }
  .veil-scraper-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-4);
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
  }

  .veil-modal-footer {
    flex-shrink: 0;
  }
</style>

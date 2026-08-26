<script lang="ts">
  import { profilePlaylistStore, type PlaylistItem } from '$lib/stores/profilePlaylist';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import { toastStore } from '$lib/stores/notifications';
  import { copyText } from '$lib/utils/clipboard';

  interface Props {
    initialTab?: 'avatar' | 'banner';
    onClose: () => void;
    onAddViaScraper?: (type: 'avatar' | 'banner') => void;
    onAddViaCamera?: (type: 'avatar' | 'banner') => void;
  }

  let {
    initialTab = 'avatar',
    onClose,
    onAddViaScraper,
    onAddViaCamera,
  }: Props = $props();

  let activeTab = $state<'avatar' | 'banner'>(initialTab);
  let newName = $state('');
  let newUrl = $state('');
  let fileInputEl = $state<HTMLInputElement | null>(null);
  let mediaFileInputEl = $state<HTMLInputElement | null>(null);

  const playlist = $derived($profilePlaylistStore);
  const currentItems = $derived(activeTab === 'avatar' ? playlist.avatarItems : playlist.bannerItems);
  const currentConfig = $derived(activeTab === 'avatar' ? playlist.avatarConfig : playlist.bannerConfig);

  const INTERVAL_OPTIONS = [
    { label: '5 saniye', value: 5 },
    { label: '10 saniye', value: 10 },
    { label: '30 saniye', value: 30 },
    { label: '1 dakika', value: 60 },
    { label: '5 dakika', value: 300 },
    { label: '15 dakika', value: 900 },
    { label: '1 saat', value: 3600 },
  ];

  function handleAddItem() {
    const raw = newUrl.trim();
    if (!raw) {
      toastStore.warning('Lütfen bir medya bağlantısı girin.');
      return;
    }
    const lines = raw.split(/[\n,]+/).map(u => u.trim()).filter(u => u.length > 0);
    if (lines.length > 1) {
      let count = 0;
      for (const u of lines) {
        const itemTitle = `${activeTab === 'avatar' ? 'Avatar' : 'Banner'} #${currentItems.length + count + 1}`;
        if (activeTab === 'avatar') {
          profilePlaylistStore.addAvatarItem({ name: itemTitle, url: u });
        } else {
          profilePlaylistStore.addBannerItem({ name: itemTitle, url: u });
        }
        count++;
      }
      toastStore.success(`${count} adet medya listeye eklendi.`);
      newName = '';
      newUrl = '';
      return;
    }

    const name = newName.trim() || `${activeTab === 'avatar' ? 'Avatar' : 'Banner'} #${currentItems.length + 1}`;

    if (activeTab === 'avatar') {
      profilePlaylistStore.addAvatarItem({ name, url: raw });
    } else {
      profilePlaylistStore.addBannerItem({ name, url: raw });
    }

    newName = '';
    newUrl = '';
  }

  function handlePickLocalMedia() {
    mediaFileInputEl?.click();
  }

  function handleLocalMediaSelected(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        const name = file.name.replace(/\.[^/.]+$/, '') || `${activeTab === 'avatar' ? 'Avatar' : 'Banner'} #${currentItems.length + 1}`;
        if (activeTab === 'avatar') {
          profilePlaylistStore.addAvatarItem({ name, url: reader.result, dataUrl: reader.result });
        } else {
          profilePlaylistStore.addBannerItem({ name, url: reader.result, dataUrl: reader.result });
        }
      }
    };
    reader.readAsDataURL(file);
    if (mediaFileInputEl) mediaFileInputEl.value = '';
  }

  function handleRemoveItem(id: string) {
    if (activeTab === 'avatar') {
      profilePlaylistStore.removeAvatarItem(id);
    } else {
      profilePlaylistStore.removeBannerItem(id);
    }
  }

  function handleApplyNow(item: PlaylistItem) {
    if (activeTab === 'avatar') {
      void profilePlaylistStore.applyAvatarItemNow(item);
    } else {
      void profilePlaylistStore.applyBannerItemNow(item);
    }
  }

  async function handleCopyItemUrl(item: PlaylistItem) {
    await copyText(item.url || item.dataUrl || '');
    toastStore.success('Medya bağlantısı kopyalandı.');
  }

  function handleToggleEnabled(enabled: boolean) {
    if (activeTab === 'avatar') {
      profilePlaylistStore.updateAvatarConfig({ enabled });
    } else {
      profilePlaylistStore.updateBannerConfig({ enabled });
    }
  }

  function handleIntervalChange(e: Event) {
    const val = Number((e.target as HTMLSelectElement).value);
    if (activeTab === 'avatar') {
      profilePlaylistStore.updateAvatarConfig({ intervalSeconds: val });
    } else {
      profilePlaylistStore.updateBannerConfig({ intervalSeconds: val });
    }
  }

  function handleModeChange(mode: 'sequential' | 'shuffle') {
    if (activeTab === 'avatar') {
      profilePlaylistStore.updateAvatarConfig({ mode });
    } else {
      profilePlaylistStore.updateBannerConfig({ mode });
    }
  }

  function handleExportJson() {
    const json = profilePlaylistStore.exportJson();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `veilanon_profile_playlist_${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toastStore.success('Oynatma listesi JSON olarak indirildi.');
  }

  function handleImportClick() {
    fileInputEl?.click();
  }

  async function handleFileSelected(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      profilePlaylistStore.importJson(text);
    } catch {
      toastStore.error('Dosya okunamadı.');
    } finally {
      if (fileInputEl) fileInputEl.value = '';
    }
  }
</script>

<div class="veil-modal-backdrop" onclick={onClose} role="presentation">
  <div
    class="veil-modal-content veil-playlist-modal"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-labelledby="playlist-modal-title"
    aria-modal="true"
  >
    <div class="veil-modal-header">
      <div class="veil-modal-header-title-wrap">
        <div class="veil-playlist-icon-badge">
          <Icon name="refresh-cw" size={18} />
        </div>
        <div>
          <h3 id="playlist-modal-title" class="veil-modal-title">Profil & Banner Oynatma Listesi</h3>
          <p class="veil-modal-subtitle">Otomatik dönen slayt gösterisi ve medya kütüphanesi.</p>
        </div>
      </div>
      <button class="btn-icon veil-modal-close" onclick={onClose} aria-label="Kapat" type="button">
        <Icon name="x" size={16} />
      </button>
    </div>

    <!-- Hidden file input for JSON import -->
    <input
      type="file"
      accept="application/json"
      bind:this={fileInputEl}
      style="display: none;"
      onchange={handleFileSelected}
    />

    <!-- Navigation Tabs -->
    <div class="veil-playlist-tabs" role="tablist">
      <button
        type="button"
        class="veil-playlist-tab"
        class:active={activeTab === 'avatar'}
        onclick={() => (activeTab = 'avatar')}
      >
        <Icon name="user" size={14} />
        <span>Avatar Oynatma Listesi ({playlist.avatarItems.length})</span>
      </button>
      <button
        type="button"
        class="veil-playlist-tab"
        class:active={activeTab === 'banner'}
        onclick={() => (activeTab = 'banner')}
      >
        <Icon name="image" size={14} />
        <span>Banner Oynatma Listesi ({playlist.bannerItems.length})</span>
      </button>
    </div>

    <div class="veil-playlist-body">
      <!-- Rotation Config Card -->
      <div class="veil-playlist-config-card">
        <div class="veil-playlist-config-row">
          <div class="veil-playlist-config-info">
            <span class="veil-playlist-config-title">Otomatik Döndürme (Slayt Gösterisi)</span>
            <span class="veil-playlist-config-desc">
              Listede 2 veya daha fazla görsel olduğunda belirlenen sürede bir profilini günceller.
            </span>
          </div>
          <Toggle
            id="playlist-active-toggle"
            checked={currentConfig.enabled}
            onChange={handleToggleEnabled}
          />
        </div>

        {#if currentConfig.enabled}
          <div class="veil-playlist-config-details">
            <div class="veil-playlist-form-group">
              <label for="interval-select">Geçiş Aralığı</label>
              <select
                id="interval-select"
                class="veil-select"
                value={currentConfig.intervalSeconds}
                onchange={handleIntervalChange}
              >
                {#each INTERVAL_OPTIONS as opt}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </div>

            <div class="veil-playlist-form-group">
              <span class="veil-playlist-label-title">Sıralama Modu</span>
              <div class="veil-playlist-mode-pills">
                <button
                  type="button"
                  class="veil-playlist-mode-btn"
                  class:active={currentConfig.mode === 'sequential'}
                  onclick={() => handleModeChange('sequential')}
                >
                  <Icon name="arrow-right" size={12} />
                  <span>Sırayla</span>
                </button>
                <button
                  type="button"
                  class="veil-playlist-mode-btn"
                  class:active={currentConfig.mode === 'shuffle'}
                  onclick={() => handleModeChange('shuffle')}
                >
                  <Icon name="refresh-cw" size={12} />
                  <span>Karışık</span>
                </button>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Hidden media file input for local upload -->
      <input
        type="file"
        accept="image/*,video/*,.gif,.webp,.mp4,.webm"
        bind:this={mediaFileInputEl}
        style="display: none;"
        onchange={handleLocalMediaSelected}
      />

      <!-- Add New Media Item Form -->
      <form class="veil-playlist-add-card" onsubmit={(e) => { e.preventDefault(); handleAddItem(); }}>
        <div class="veil-playlist-add-inputs">
          <input
            type="text"
            class="veil-input veil-input-sm"
            placeholder="Öğe Başlığı (opsiyonel)"
            bind:value={newName}
          />
          <input
            type="url"
            class="veil-input veil-input-sm"
            placeholder="Görsel / Video URL'si (https://...)"
            bind:value={newUrl}
          />
        </div>
        <div class="veil-playlist-add-actions">
          <button
            class="btn btn-secondary btn-sm"
            type="button"
            onclick={handlePickLocalMedia}
            title="Bilgisayardan Dosya Yükle"
          >
            <Icon name="upload" size={13} />
            <span>Dosya Seç</span>
          </button>
          {#if onAddViaScraper}
            <button
              class="btn btn-secondary btn-sm"
              type="button"
              onclick={() => onAddViaScraper(activeTab)}
              title="Web Sayfasından Görsel/Video Tara"
            >
              <Icon name="globe" size={13} />
              <span>Webden Tara</span>
            </button>
          {/if}
          {#if onAddViaCamera}
            <button
              class="btn btn-secondary btn-sm"
              type="button"
              onclick={() => onAddViaCamera(activeTab)}
              title="Kameradan Çek & Ekle"
            >
              <Icon name="camera" size={13} />
              <span>Kamera</span>
            </button>
          {/if}
          <button class="btn btn-primary btn-sm" type="submit" disabled={!newUrl.trim()}>
            <Icon name="plus" size={13} />
            <span>URL Ekle</span>
          </button>
        </div>
      </form>

      <!-- Items Grid -->
      <div class="veil-playlist-items-section">
        <div class="veil-playlist-items-header">
          <span>Kayıtlı Öğeler ({currentItems.length})</span>
          <div class="veil-playlist-json-actions">
            <button class="btn btn-ghost btn-xs" type="button" onclick={handleImportClick}>
              <Icon name="upload" size={12} />
              <span>JSON İçe Aktar</span>
            </button>
            <button class="btn btn-ghost btn-xs" type="button" onclick={handleExportJson}>
              <Icon name="download" size={12} />
              <span>JSON Dışa Aktar</span>
            </button>
          </div>
        </div>

        {#if currentItems.length > 0}
          <div class="veil-playlist-grid">
            {#each currentItems as item, idx (item.id)}
              {@const isCurrent = currentConfig.currentIndex === idx && currentConfig.enabled}
              {@const isVid = (item.url || item.dataUrl || '').includes('.mp4') || (item.url || item.dataUrl || '').includes('.webm') || (item.dataUrl || '').startsWith('data:video/')}
              <div class="veil-playlist-item-card" class:active-item={isCurrent}>
                <div class="veil-playlist-thumb-box" class:banner-thumb={activeTab === 'banner'}>
                  {#if isVid}
                    <video src={item.dataUrl || item.url} class="veil-playlist-img" muted loop playsinline></video>
                  {:else}
                    <img src={item.dataUrl || item.url} alt={item.name} class="veil-playlist-img" />
                  {/if}
                  {#if isCurrent}
                    <div class="veil-playlist-live-badge">Şu Anda Aktif</div>
                  {/if}
                </div>
                <div class="veil-playlist-item-info">
                  <span class="veil-playlist-item-name" title={item.name}>{item.name}</span>
                  <div class="veil-playlist-item-actions">
                    <button
                      class="btn btn-secondary btn-xs"
                      type="button"
                      onclick={() => handleApplyNow(item)}
                      title="Şimdi Uygula"
                    >
                      <span>Uygula</span>
                    </button>
                    <button
                      class="btn-icon veil-playlist-copy-btn"
                      type="button"
                      onclick={() => handleCopyItemUrl(item)}
                      title="Bağlantıyı Kopyala"
                    >
                      <Icon name="copy" size={12} />
                    </button>
                    <button
                      class="btn-icon veil-playlist-del-btn"
                      type="button"
                      onclick={() => handleRemoveItem(item.id)}
                      title="Listeden Kaldır"
                    >
                      <Icon name="trash" size={13} />
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="veil-playlist-empty">
            <Icon name="image" size={28} />
            <p>Bu listede henüz kayıtlı medya bulunmuyor. Yukarıdan URL ekleyebilir veya webden tarayabilirsiniz.</p>
          </div>
        {/if}
      </div>
    </div>

    <div class="veil-modal-footer">
      <button class="btn btn-secondary" type="button" onclick={onClose}>Tamam</button>
    </div>
  </div>
</div>

<style>
  .veil-playlist-modal {
    max-width: 680px;
    width: 95vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
  }
  .veil-modal-header-title-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .veil-playlist-icon-badge {
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
  .veil-playlist-tabs {
    display: flex;
    background: var(--veil-bg-void);
    border-bottom: 1px solid var(--veil-border);
    padding: 0 var(--space-6);
    gap: var(--space-4);
  }
  .veil-playlist-tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) 0;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--t-fast);
  }
  .veil-playlist-tab:hover {
    color: var(--veil-text);
  }
  .veil-playlist-tab.active {
    color: var(--veil-brand);
    border-bottom-color: var(--veil-brand);
    font-weight: 600;
  }
  .veil-playlist-body {
    padding: var(--space-4) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    overflow-y: auto;
    flex: 1;
  }
  .veil-playlist-config-card {
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .veil-playlist-config-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .veil-playlist-config-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-playlist-config-title {
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .veil-playlist-config-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-playlist-config-details {
    display: flex;
    gap: var(--space-4);
    padding-top: var(--space-2);
    border-top: 1px solid var(--veil-border-subtle);
  }
  .veil-playlist-form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  .veil-playlist-form-group label,
  .veil-playlist-label-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-muted);
  }
  .veil-playlist-mode-pills {
    display: flex;
    gap: 4px;
  }
  .veil-playlist-mode-btn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 6px 10px;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    color: var(--veil-text-muted);
    font-size: var(--text-xs);
    font-weight: 500;
    cursor: pointer;
  }
  .veil-playlist-mode-btn.active {
    background: var(--veil-brand);
    color: #fff;
    border-color: var(--veil-brand);
  }
  .veil-playlist-add-card {
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-playlist-add-inputs {
    display: flex;
    gap: var(--space-2);
  }
  .veil-playlist-add-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  .veil-playlist-items-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-playlist-items-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-muted);
  }
  .veil-playlist-json-actions {
    display: flex;
    gap: var(--space-2);
  }
  .veil-playlist-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--space-3);
    max-height: 280px;
    overflow-y: auto;
    padding: 2px;
  }
  .veil-playlist-item-card {
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: all var(--t-fast);
  }
  .veil-playlist-item-card.active-item {
    border-color: var(--veil-brand);
    box-shadow: 0 0 0 2px var(--veil-brand-subtle);
  }
  .veil-playlist-thumb-box {
    position: relative;
    width: 100%;
    aspect-ratio: 1 / 1;
    background: var(--veil-bg-void);
    overflow: hidden;
  }
  .veil-playlist-thumb-box.banner-thumb {
    aspect-ratio: 3 / 1;
  }
  .veil-playlist-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .veil-playlist-live-badge {
    position: absolute;
    bottom: 4px;
    left: 4px;
    background: var(--veil-success);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: var(--radius-full);
  }
  .veil-playlist-item-info {
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .veil-playlist-item-name {
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-playlist-item-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .veil-playlist-del-btn {
    color: var(--veil-text-muted);
  }
  .veil-playlist-del-btn:hover {
    color: var(--veil-danger);
  }
  .veil-playlist-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-2);
    padding: var(--space-6) var(--space-4);
    color: var(--veil-text-muted);
    font-size: var(--text-xs);
  }
</style>

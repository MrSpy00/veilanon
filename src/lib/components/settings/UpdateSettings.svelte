<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { updaterApi, settingsApi, type UpdateCheckResult, type AppSettings } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';

  let checking = $state(false);
  let installing = $state(false);
  let result = $state<UpdateCheckResult | null>(null);
  let error = $state<string | null>(null);
  let settings = $state<AppSettings | null>(null);

  onMount(async () => {
    try {
      settings = await settingsApi.get();
      // Auto check on load
      await check();
    } catch { /* ignored */ }
  });

  async function check() {
    checking = true;
    error = null;
    try {
      result = await updaterApi.check();
      if (result.updateAvailable && result.currentVersion !== result.latestVersion) {
        toastStore.info(`Yeni sürüm mevcut: v${result.latestVersion}`);
      }
    } catch (err) {
      error = String(err).replace(/^Error:\s*/, '');
    } finally {
      checking = false;
    }
  }

  async function installUpdate() {
    if (!result || !result.downloadUrl || !result.assetName || installing) return;
    installing = true;
    try {
      toastStore.info('Güncelleme indiriliyor ve kuruluyor… Uygulama yeniden başlayacak.');
      await updaterApi.downloadAndInstall(result.downloadUrl, result.assetName);
    } catch (err) {
      installing = false;
      toastStore.error(`Güncelleme başarısız: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function downloadManually() {
    if (!result?.downloadUrl) return;
    try {
      await openUrl(result.downloadUrl);
    } catch {
      window.open(result.downloadUrl, '_blank');
    }
  }

  async function toggleAutoCheck(enabled: boolean) {
    if (!settings) return;
    try {
      settings = await settingsApi.update({ ...settings, autoUpdateCheck: enabled });
      toastStore.success('Güncelleme tercihi kaydedildi.');
    } catch {
      toastStore.error('Tercih kaydedilemedi.');
    }
  }

  function formatBytes(bytes?: number | null): string {
    if (!bytes) return '';
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="veil-updater-card">
  <div class="veil-updater-header">
    <div class="veil-updater-info">
      <div class="veil-updater-version-tag">
        Mevcut Sürüm: <strong>v{result?.currentVersion || '0.0.1'}</strong>
      </div>
      <div class="veil-updater-plat-tag">
        Platform: {result?.platform || 'windows-x86_64'}
      </div>
    </div>

    <button class="btn btn-secondary btn-sm" onclick={check} disabled={checking || installing}>
      <Icon name="sparkle" size={14} />
      <span>{checking ? 'Denetleniyor…' : 'Güncellemeleri Denetle'}</span>
    </button>
  </div>

  {#if checking}
    <div class="veil-updater-loading">
      <div class="veil-spinner veil-spinner-sm"></div>
      <span>GitHub Releases üzerinden en son sürüm denetleniyor…</span>
    </div>
  {:else if error}
    <div class="veil-updater-error">
      <Icon name="warning" size={16} />
      <span>{error}</span>
    </div>
  {:else if result}
    {#if result.updateAvailable}
      <div class="veil-updater-banner available">
        <div class="veil-updater-badge">
          {result.isSameVersionNewerBuild ? 'YENİ DERLEME MEVCUT' : 'YENİ SÜRÜM MEVCUT'}
        </div>
        <h4 class="veil-updater-rel-title">
          {result.releaseName || `veilanon v${result.latestVersion}`}
        </h4>
        <div class="veil-updater-meta-row">
          <span>Yayınlanma: {result.publishedAt ? new Date(result.publishedAt).toLocaleDateString('tr-TR', { day: 'numeric', month: 'long', year: 'numeric' }) : 'Son Sürüm'}</span>
          <span>·</span>
          <span>Hedef: {result.platform}</span>
          {#if result.detectionMethod && result.detectionMethod !== 'none'}
            <span>·</span>
            <span class="veil-method-tag">{result.detectionMethod} ile doğrulandı</span>
          {/if}
        </div>

        {#if result.releaseNotes}
          <div class="veil-updater-notes">
            <div class="veil-notes-label">Yenilikler & Değişiklikler:</div>
            <pre class="veil-notes-content">{result.releaseNotes}</pre>
          </div>
        {/if}

        <div class="veil-updater-actions">
          <button
            class="btn btn-primary"
            onclick={installUpdate}
            disabled={installing || !result.downloadUrl}
          >
            <Icon name="sparkle" size={16} />
            <span>{installing ? 'İndiriliyor ve Kuruluyor…' : 'Otomatik Güncelle & Yeniden Başlat'}</span>
          </button>

          {#if result.downloadUrl}
            <button class="btn btn-secondary" onclick={downloadManually} disabled={installing}>
              <Icon name="download" size={16} />
              <span>Manuel İndir ({formatBytes(result.assetSize)})</span>
            </button>
          {/if}
        </div>
      </div>
    {:else}
      <div class="veil-updater-banner uptodate">
        <Icon name="check" size={20} />
        <div class="veil-updater-uptodate-body">
          <strong>veilanon güncel!</strong>
          <p>{result.statusMessage}</p>
          {#if result.publishedAt}
            <span class="veil-updater-rel-date">
              Son sürüm tarihi: {new Date(result.publishedAt).toLocaleDateString('tr-TR', { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' })}
            </span>
          {/if}

          {#if result.releaseNotes}
            <div class="veil-updater-notes" style="margin-top: var(--space-2);">
              <div class="veil-notes-label">Mevcut Sürüm Yenilikleri:</div>
              <pre class="veil-notes-content">{result.releaseNotes}</pre>
            </div>
          {/if}

          {#if result.downloadUrl}
            <div class="veil-updater-reinstall-row">
              <button
                class="btn btn-secondary btn-xs"
                onclick={installUpdate}
                disabled={installing}
                title="Aynı sürümün en son derlemesini indirip mevcut uygulamanın üzerine yazar ve yeniden başlatır"
              >
                <Icon name="sparkle" size={12} />
                <span>{installing ? 'Yükleniyor…' : 'Son Derlemeyi Yeniden Kur / Onar'}</span>
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    {#if result.allAssets && result.allAssets.length > 0}
      <div class="veil-all-assets-section">
        <div class="veil-notes-label">Tüm Platform Kurulum Paketleri ({result.allAssets.length}):</div>
        <div class="veil-assets-grid">
          {#each result.allAssets as asset (asset.name)}
            <a
              class="veil-asset-pill"
              href={asset.downloadUrl}
              target="_blank"
              rel="noopener noreferrer"
              title="Doğrudan İndir: {asset.name}"
            >
              <Icon name="download" size={12} />
              <span class="veil-asset-name">{asset.name}</span>
              <span class="veil-asset-size">{formatBytes(asset.size)}</span>
            </a>
          {/each}
        </div>
      </div>
    {/if}
  {/if}

  {#if settings}
    <div class="veil-updater-auto-check">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Açılışta otomatik güncelleme denetimi</div>
        <div class="veil-settings-row-desc">
          Uygulama her açıldığında yeni sürüm olup olmadığını sessizce kontrol et.
        </div>
      </div>
      <Toggle
        checked={settings.autoUpdateCheck ?? true}
        onChange={toggleAutoCheck}
        label="Otomatik güncelleme denetimi"
      />
    </div>
  {/if}
</div>

<style>
  .veil-updater-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: 100%;
    max-width: 520px;
    margin: var(--space-2) 0;
    text-align: left;
  }
  .veil-updater-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
  }
  .veil-updater-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-updater-version-tag {
    font-size: var(--text-sm);
    color: var(--veil-text-primary);
  }
  .veil-updater-plat-tag {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-family: var(--font-mono);
  }
  .veil-updater-loading {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--veil-bg-surface);
    border-radius: var(--radius-lg);
    font-size: var(--text-sm);
    color: var(--veil-text-secondary);
  }
  .veil-updater-error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: color-mix(in srgb, var(--veil-danger) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--veil-danger) 30%, transparent);
    border-radius: var(--radius-lg);
    color: var(--veil-danger);
    font-size: var(--text-sm);
  }
  .veil-updater-banner {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    border-radius: var(--radius-xl);
  }
  .veil-updater-banner.uptodate {
    flex-direction: row;
    align-items: center;
    background: color-mix(in srgb, var(--veil-success) 10%, var(--veil-bg-elevated));
    border: 1px solid color-mix(in srgb, var(--veil-success) 30%, transparent);
    color: var(--veil-success);
    font-size: var(--text-sm);
  }
  .veil-updater-banner.uptodate p {
    color: var(--veil-text-secondary);
    font-size: var(--text-xs);
    margin-top: 2px;
  }
  .veil-updater-banner.available {
    background:
      radial-gradient(120% 160% at 0% 0%, var(--veil-brand-subtle), transparent 70%),
      var(--veil-bg-elevated);
    border: 1px solid var(--veil-brand-border);
  }
  .veil-updater-badge {
    display: inline-block;
    align-self: flex-start;
    padding: 2px 8px;
    background: var(--veil-brand);
    color: #fff;
    border-radius: var(--radius-full);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.08em;
  }
  .veil-updater-rel-title {
    font-size: var(--text-lg);
    font-weight: 700;
    color: var(--veil-text-primary);
    margin: 0;
  }
  .veil-updater-notes {
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    max-height: 180px;
    overflow-y: auto;
  }
  .veil-notes-label {
    font-size: var(--text-xs);
    font-weight: 700;
    color: var(--veil-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: var(--space-1);
  }
  .veil-notes-content {
    font-size: var(--text-xs);
    font-family: var(--font-sans);
    line-height: var(--leading-relaxed);
    color: var(--veil-text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }
  .veil-updater-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
    margin-top: var(--space-1);
  }
  .veil-updater-auto-check {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    gap: var(--space-3);
  }
  .veil-updater-uptodate-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  .veil-updater-rel-date {
    font-size: 11px;
    color: var(--veil-text-muted);
  }
  .veil-updater-reinstall-row {
    margin-top: var(--space-2);
  }
  .veil-all-assets-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
  }
  .veil-assets-grid {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 180px;
    overflow-y: auto;
  }
  .veil-asset-pill {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-md);
    background: var(--veil-bg-base);
    color: var(--veil-text-primary);
    text-decoration: none;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    border: 1px solid var(--veil-border-subtle);
    transition: all var(--t-fast);
  }
  .veil-asset-pill:hover {
    background: var(--veil-bg-overlay);
    border-color: var(--veil-brand);
    color: var(--veil-brand);
  }
  .veil-asset-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .veil-asset-size {
    color: var(--veil-text-muted);
    font-size: 10px;
    flex-shrink: 0;
  }
  .veil-updater-meta-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
</style>

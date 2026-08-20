<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { trustedDomainsStore, extractDomain } from '$lib/stores/trustedDomains';
  import { toastStore } from '$lib/stores/notifications';
  import { privacyToolsApi, type UrlScanResult } from '$lib/api/tauri';
  import Icon from './Icon.svelte';

  let {
    open = false,
    url = '',
    onClose,
  }: {
    open?: boolean;
    url?: string;
    onClose?: () => void;
  } = $props();

  let rememberDomain = $state(true);
  const domain = $derived(url ? extractDomain(url) : '');

  // ── URLhaus Threat Scan ──────────────────────────────────────────
  let threatResult = $state<UrlScanResult | null>(null);
  let threatLoading = $state(false);
  let threatChecked = $state(false);

  // Auto-scan when modal opens with a new URL
  $effect(() => {
    if (open && url) {
      threatResult = null;
      threatChecked = false;
      threatLoading = true;
      privacyToolsApi.scanUrl(url)
        .then((result) => {
          threatResult = result;
        })
        .catch(() => {
          // Scan failed — don't block user, just show no badge
          threatResult = null;
        })
        .finally(() => {
          threatLoading = false;
          threatChecked = true;
        });
    }
  });

  const isMalicious = $derived(threatResult?.isMalicious === true);
  const isSafe = $derived(threatChecked && !isMalicious);

  async function handleConfirm() {
    if (!url) return;
    if (isMalicious) {
      // Extra confirmation for dangerous links
      const proceed = await Promise.resolve(
        window.confirm(`⚠️ Bu bağlantı zararlı olarak işaretlenmiş!\n\nTehdit: ${threatResult?.threat ?? 'Bilinmiyor'}\n\nYine de açmak istiyor musunuz?`)
      );
      if (!proceed) return;
    }
    if (rememberDomain && domain) {
      trustedDomainsStore.addTrustedDomain(domain);
    }
    try {
      await openUrl(url);
    } catch {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
    onClose?.();
  }

  function handleCancel() {
    onClose?.();
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) handleCancel();
  }
</script>

{#if open}
  <div class="veil-overlay" role="presentation" onclick={onOverlayClick}>
    <div class="veil-modal veil-link-modal" role="dialog" aria-modal="true" aria-labelledby="link-modal-title">
      <div class="veil-link-header">
        <div class="veil-link-icon-wrap" class:danger-icon={isMalicious} class:safe-icon={isSafe}>
          {#if isMalicious}
            <Icon name="shield" size={24} />
          {:else}
            <Icon name="shield" size={24} />
          {/if}
        </div>
        <div>
          <h3 id="link-modal-title" class="veil-link-title">Harici Bağlantı Uyarısı</h3>
          <p class="veil-link-subtitle">veilanon dışındaki bir web sitesine yönlendiriliyorsunuz.</p>
        </div>
      </div>

      <!-- Threat Scan Status Banner -->
      <div class="threat-banner-wrap">
        {#if threatLoading}
          <div class="threat-banner threat-checking">
            <span class="veil-spinner-xs"></span>
            <span>URLhaus tehdit veritabanı sorgulanıyor…</span>
          </div>
        {:else if isMalicious}
          <div class="threat-banner threat-danger" role="alert">
            <Icon name="shield" size={14} />
            <div class="threat-text">
              <strong>⚠️ TEHLİKELİ BAĞLANTI TESPİT EDİLDİ</strong>
              <span>
                {#if threatResult?.threat}
                  Tehdit türü: <span class="threat-type">{threatResult.threat}</span>
                {/if}
                {#if threatResult?.urlStatus}
                  · Durum: <span class="threat-type">{threatResult.urlStatus}</span>
                {/if}
                {#if (threatResult?.tags ?? []).length > 0}
                  · Etiketler: {(threatResult?.tags ?? []).join(', ')}
                {/if}
              </span>
              {#if threatResult?.urlhausReference}
                <span class="threat-ref">Kaynak: URLhaus Abuse.ch · {threatResult.urlhausReference}</span>
              {/if}
            </div>
          </div>
        {:else if isSafe}
          <div class="threat-banner threat-ok">
            <Icon name="shield" size={13} />
            <span>URLhaus'ta bilinen tehdit bulunamadı.</span>
          </div>
        {/if}
      </div>

      <div class="veil-link-body">
        <div class="veil-link-url-card">
          <div class="url-card-top">
            <span class="veil-link-domain-badge">{domain || 'Harici Site'}</span>
            {#if isMalicious}
              <span class="url-threat-pill">
                <Icon name="x" size={10} />
                Kötü Amaçlı
              </span>
            {:else if isSafe}
              <span class="url-safe-pill">
                <Icon name="shield" size={10} />
                Temiz
              </span>
            {/if}
          </div>
          <p class="veil-link-url-text" title={url}>{url}</p>
        </div>

        <label class="veil-link-remember">
          <input type="checkbox" bind:checked={rememberDomain} />
          <span><strong>{domain}</strong> domainini güvenilir listeme ekle (bir daha sorma)</span>
        </label>
      </div>

      <div class="veil-link-actions">
        <button class="btn btn-secondary" onclick={handleCancel}>İptal</button>
        <button
          class="btn"
          class:btn-primary={!isMalicious}
          class:btn-danger={isMalicious}
          onclick={handleConfirm}
        >
          <Icon name="external-link" size={14} />
          <span>{isMalicious ? '⚠️ Yine de Aç' : 'Bağlantıyı Aç'}</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .veil-link-modal {
    max-width: 480px;
    padding: var(--space-5);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-2xl);
  }
  .veil-link-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-3);
  }
  .veil-link-icon-wrap {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-xl);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-link-icon-wrap.danger-icon {
    background: color-mix(in srgb, var(--veil-danger) 15%, transparent);
    color: var(--veil-danger);
    border: 1px solid color-mix(in srgb, var(--veil-danger) 30%, transparent);
  }
  .veil-link-icon-wrap.safe-icon {
    background: color-mix(in srgb, var(--veil-success) 12%, transparent);
    color: var(--veil-success);
    border: 1px solid color-mix(in srgb, var(--veil-success) 25%, transparent);
  }
  .veil-link-title {
    font-size: var(--text-lg);
    font-weight: 700;
    margin: 0;
    color: var(--veil-text-primary);
  }
  .veil-link-subtitle {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin: 2px 0 0;
  }

  /* ── Threat Banner ───────────────────────────────────────────── */
  .threat-banner-wrap {
    margin-bottom: var(--space-3);
    min-height: 0;
  }

  .threat-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-lg);
    font-size: var(--text-xs);
    line-height: var(--leading-relaxed);
    border: 1px solid transparent;
  }

  .threat-checking {
    color: var(--veil-text-muted);
    background: var(--veil-bg-void);
    border-color: var(--veil-border-subtle);
    align-items: center;
  }

  .threat-danger {
    background: color-mix(in srgb, var(--veil-danger) 10%, transparent);
    color: var(--veil-danger);
    border-color: color-mix(in srgb, var(--veil-danger) 30%, transparent);
    animation: danger-pulse 2s ease-in-out infinite;
  }

  @keyframes danger-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--veil-danger) 20%, transparent); }
    50% { box-shadow: 0 0 0 4px color-mix(in srgb, var(--veil-danger) 0%, transparent); }
  }

  .threat-ok {
    background: color-mix(in srgb, var(--veil-success) 8%, transparent);
    color: var(--veil-success);
    border-color: color-mix(in srgb, var(--veil-success) 25%, transparent);
    align-items: center;
  }

  .threat-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .threat-type {
    font-family: var(--font-mono);
    font-weight: 700;
    background: color-mix(in srgb, var(--veil-danger) 15%, transparent);
    padding: 1px 4px;
    border-radius: var(--radius-sm);
  }

  .threat-ref {
    color: color-mix(in srgb, var(--veil-danger) 60%, var(--veil-text-muted));
    font-size: 10px;
    margin-top: 2px;
  }

  .veil-link-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-bottom: var(--space-5);
  }
  .veil-link-url-card {
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    word-break: break-all;
  }

  .url-card-top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
    flex-wrap: wrap;
  }

  .veil-link-domain-badge {
    display: inline-flex;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
  }

  .url-threat-pill {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: color-mix(in srgb, var(--veil-danger) 15%, transparent);
    color: var(--veil-danger);
    border: 1px solid color-mix(in srgb, var(--veil-danger) 30%, transparent);
  }

  .url-safe-pill {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: color-mix(in srgb, var(--veil-success) 12%, transparent);
    color: var(--veil-success);
    border: 1px solid color-mix(in srgb, var(--veil-success) 25%, transparent);
  }

  .veil-link-url-text {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    line-height: var(--leading-normal);
    max-height: 80px;
    overflow-y: auto;
    margin: 0;
  }
  .veil-link-remember {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    cursor: pointer;
    user-select: none;
  }
  .veil-link-remember input {
    accent-color: var(--veil-brand);
    cursor: pointer;
  }
  .veil-link-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .btn-danger {
    background: var(--veil-danger, #ef4444);
    color: #fff;
    border-color: var(--veil-danger, #ef4444);
    animation: btn-danger-pulse 2s ease-in-out infinite;
  }

  .btn-danger:hover {
    background: color-mix(in srgb, var(--veil-danger) 85%, black);
    border-color: color-mix(in srgb, var(--veil-danger) 85%, black);
  }

  @keyframes btn-danger-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--veil-danger) 25%, transparent); }
    50% { box-shadow: 0 0 0 5px color-mix(in srgb, var(--veil-danger) 0%, transparent); }
  }

  /* Spinner */
  .veil-spinner-xs {
    width: 12px;
    height: 12px;
    border: 2px solid var(--veil-border);
    border-top-color: var(--veil-brand);
    border-radius: var(--radius-full);
    animation: spin 0.7s linear infinite;
    display: inline-block;
    flex-shrink: 0;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>

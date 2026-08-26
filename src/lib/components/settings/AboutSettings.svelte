<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl, revealItemInDir, openPath } from '@tauri-apps/plugin-opener';
  import Icon, { type IconName } from '$lib/components/ui/Icon.svelte';
  import AppLogo from '$lib/components/ui/AppLogo.svelte';
  import { settingsApi, diagnosticsApi, type AboutInfo, type Diagnostics } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import UpdateSettings from './UpdateSettings.svelte';

  const fallback: AboutInfo = {
    appName: 'veilanon',
    version: '0.0.1',
    description: 'Gizlilik, hız ve özgürlük odaklı olarak geliştirilen açık kaynaklı uçtan uca şifreli masaüstü iletişim platformu.',
    developer: 'aegisSoft',
    developerUrl: 'https://www.aegissoft.com.tr/',
    developerGithub: 'https://github.com/MrSpy00',
    projectGithub: 'https://github.com/MrSpy00/veilanon',
    supportUrl: 'https://buymeacoffee.com/aegissoft',
    license: 'AGPL-3.0',
    buildDate: '',
    rustVersion: '',
    platform: '',
  };

  let info = $state<AboutInfo>(fallback);
  let diagnostics = $state<Diagnostics | null>(null);

  let diagTimer: ReturnType<typeof setInterval> | null = null;
  async function refreshDiagnostics() {
    try {
      diagnostics = await diagnosticsApi.get();
    } catch {
      diagnostics = null;
    }
  }
  onMount(() => {
    void (async () => {
      try {
        info = { ...fallback, ...(await settingsApi.getAboutInfo()) };
      } catch {
      }
      await refreshDiagnostics();
    })();
    diagTimer = setInterval(() => { void refreshDiagnostics(); }, 15000);
    return () => { if (diagTimer) clearInterval(diagTimer); };
  });

  async function open(url: string) {
    try {
      await openUrl(url);
    } catch {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }

  async function openLogFolder() {
    try {
      await diagnosticsApi.openLogFolder();
      toastStore.success('Log klasörü açıldı.');
    } catch {
      try {
        const dir = await diagnosticsApi.getLogDirectory();
        await openPath(dir);
        toastStore.success('Log klasörü açıldı.');
      } catch {
        try {
          const dir = await diagnosticsApi.getLogDirectory();
          await revealItemInDir(dir);
          toastStore.success('Log klasörü açıldı.');
        } catch {
          toastStore.error('Log klasörü açılamadı.');
        }
      }
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function githubName(url: string | null | undefined, fallbackName: string): string {
    if (!url) return fallbackName;
    const seg = url.replace(/\/+$/, '').split('/').filter(Boolean).pop();
    return seg || fallbackName;
  }

  const links = $derived(
    [
      {
        icon: 'monitor' as IconName,
        label: 'veilanon.com',
        tag: 'Site',
        url: 'https://veilanon.com',
      },
      {
        icon: 'monitor' as IconName,
        label: info.developer || 'aegisSoft',
        tag: 'Geliştirici',
        url: info.developerUrl,
      },
      {
        icon: 'user' as IconName,
        label: githubName(info.developerGithub, 'MrSpy00'),
        tag: 'GitHub',
        url: info.developerGithub,
      },
      {
        icon: 'users' as IconName,
        label: githubName(info.projectGithub, 'veilanon'),
        tag: 'GitHub',
        url: info.projectGithub,
      },
      {
        icon: 'chat' as IconName,
        label: 'Buy Me a Coffee',
        tag: 'Destek',
        url: info.supportUrl,
      },
    ].filter((l) => l.url),
  );

  const facts = $derived(
    [
      { label: 'Geliştirici', value: info.developer },
      { label: 'Lisans', value: info.license },
      { label: 'Platform', value: info.platform },
      { label: 'Rust', value: info.rustVersion },
      { label: 'Derleme', value: info.buildDate },
    ].filter((f) => f.value),
  );

  const securityFeatures = [
    {
      icon: 'lock' as IconName,
      title: 'Double Ratchet & RFC 9420 MLS',
      desc: 'Birebir ve grup mesajlaşmalarında mükemmel iletme gizliliği (PFS) ve kırılma sonrası kurtarma (PCS).',
    },
    {
      icon: 'shield' as IconName,
      title: 'Uygulama Katmanı AES-256-GCM',
      desc: 'Yerel SQLite veritabanı, oturum anahtarları ve dosya önbelleği cihaz düzeyinde şifrelenir.',
    },
    {
      icon: 'video' as IconName,
      title: 'LiveKit E2EE WebRTC Kanalları',
      desc: 'Ses ve görüntü odalarında uçtan uca anahtar değişimiyle sunucuda şifresiz veri barınmaz.',
    },
    {
      icon: 'eye-off' as IconName,
      title: 'Sıfır Telemetri & Sıfır İzleme',
      desc: 'Kullanıcı verileri, IP günlükleri veya analitik toplanmaz; dış API sorguları k-anonymity ile korunur.',
    },
  ];

  const statusRows = $derived(
    diagnostics
      ? [
          { label: 'Kontrol düzlemi (Supabase)', ok: diagnostics.supabaseConfigured, hint: !diagnostics.supabaseConfigured ? 'Yapılandırılmadı' : !diagnostics.supabaseReachable ? 'Bağlanıyor…' : 'Bağlı' },
          { label: 'Ses/Görüntü (LiveKit)', ok: diagnostics.livekitConfigured, hint: diagnostics.livekitConfigured ? 'Bağlı' : 'Yapılandırılmadı' },
          { label: 'Blob depolama (R2)', ok: diagnostics.r2Configured, hint: diagnostics.r2Configured ? 'Bağlı' : 'Yapılandırılmadı' },
          { label: 'Gerçek zamanlı bağlantı', ok: diagnostics.realtimeConnected, hint: diagnostics.realtimeConnected ? 'Bağlı' : diagnostics.supabaseConfigured ? 'Bağlanıyor…' : 'Beklemede' },
        ]
      : [],
  );
</script>

<section aria-labelledby="hakkinda-title">
  <h2 class="veil-settings-title" id="hakkinda-title">Hakkında</h2>

  <div class="veil-about">
    <div class="veil-about-logo" aria-hidden="true">
      <AppLogo size={84} radius={21} />
    </div>
    <h3 class="veil-about-name">{info.appName || 'veilanon'}</h3>
    <p class="veil-about-version">v{info.version} — açık kaynak</p>

    <p class="veil-about-text">{info.description}</p>

    <!-- ── Güvenlik & Gizlilik Mimarisi ──────────────────────────── -->
    <div class="veil-specs-section">
      <div class="veil-specs-header">
        <Icon name="shield" size={14} />
        <span class="veil-specs-title">Güvenlik & Kriptografi Mimarisi</span>
      </div>
      <div class="veil-specs-grid">
        {#each securityFeatures as feat (feat.title)}
          <div class="veil-spec-card">
            <div class="veil-spec-icon-wrap" aria-hidden="true">
              <Icon name={feat.icon} size={15} />
            </div>
            <div class="veil-spec-content">
              <span class="veil-spec-title">{feat.title}</span>
              <p class="veil-spec-desc">{feat.desc}</p>
            </div>
          </div>
        {/each}
      </div>
    </div>

    {#if facts.length > 0}
      <dl class="veil-about-facts">
        {#each facts as fact (fact.label)}
          <div class="veil-about-fact">
            <dt>{fact.label}</dt>
            <dd title={fact.value}>{fact.value}</dd>
          </div>
        {/each}
      </dl>
    {/if}

    <UpdateSettings />

    {#if links.length > 0}
      <div class="veil-about-links">
        {#each links as link (link.url)}
          <button
            class="veil-about-link"
            type="button"
            title={link.url}
            onclick={() => open(link.url)}
          >
            <span class="veil-about-link-icon" aria-hidden="true"><Icon name={link.icon} size={16} /></span>
            <span class="veil-about-link-label">{link.label}</span>
            <span class="veil-about-link-tag">{link.tag}</span>
            <span class="veil-about-link-arrow" aria-hidden="true"><Icon name="arrow-right" size={14} /></span>
          </button>
        {/each}
      </div>
    {/if}

    {#if diagnostics}
      <div class="veil-diagnostics">
        <div class="veil-diagnostics-header">
          <Icon name="activity" size={14} />
          <span class="veil-diagnostics-title">Tanılama & Sistem Durumu</span>
        </div>

        <div class="veil-diagnostics-status-grid">
          {#each statusRows as row (row.label)}
            <div class="veil-diag-card" class:ok={row.ok}>
              <div class="veil-diag-card-left">
                <span class="veil-diag-dot" class:ok={row.ok} aria-hidden="true"></span>
                <span class="veil-diag-label">{row.label}</span>
              </div>
              <span class="veil-diag-badge" class:ok={row.ok}>{row.ok ? 'Bağlı / Aktif' : (row.hint ?? 'Beklemede')}</span>
            </div>
          {/each}
        </div>

        <dl class="veil-about-facts">
          <div class="veil-about-fact"><dt>Mesaj</dt><dd>{diagnostics.messageCount}</dd></div>
          <div class="veil-about-fact"><dt>Arkadaş</dt><dd>{diagnostics.friendCount}</dd></div>
          <div class="veil-about-fact"><dt>Topluluk</dt><dd>{diagnostics.spaceCount}</dd></div>
          <div class="veil-about-fact"><dt>Kuyruk</dt><dd>{diagnostics.queuedCount}</dd></div>
          <div class="veil-about-fact"><dt>Dosya</dt><dd>{diagnostics.fileCount}</dd></div>
          <div class="veil-about-fact"><dt>Veritabanı</dt><dd>{formatBytes(diagnostics.databaseSizeBytes)}</dd></div>
        </dl>

        <button class="btn btn-secondary btn-sm veil-log-btn" type="button" onclick={openLogFolder}>
          <Icon name="info" size={14} />
          <span>Log Klasörünü Aç</span>
        </button>
      </div>
    {/if}

    <p class="veil-about-text veil-about-footer">
      veilanon açık kaynak katkılarına, güvenlik denetimlerine ve topluluk desteğine açıktır.
    </p>
  </div>
</section>

<style>
  .veil-about {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
  }
  .veil-about-logo {
    width: 84px;
    height: 84px;
    border-radius: var(--radius-xl);
    background: transparent;
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 32px;
    font-weight: 800;
  }
  .veil-about-name { font-size: var(--text-2xl); font-weight: 700; letter-spacing: var(--tracking-tight); }
  .veil-about-version { font-size: var(--text-sm); color: var(--veil-text-muted); font-family: var(--font-mono); }
  .veil-about-text {
    font-size: var(--text-base);
    line-height: var(--leading-relaxed);
    color: var(--veil-text-secondary);
    max-width: 48ch;
  }
  .veil-about-footer { color: var(--veil-text-muted); font-size: var(--text-sm); margin-top: var(--space-2); }

  /* ── Security Architecture Specs ─────────────────────────────── */
  .veil-specs-section {
    width: 100%;
    max-width: 540px;
    margin: var(--space-2) 0;
    padding-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    text-align: left;
  }

  .veil-specs-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--veil-brand);
  }

  .veil-specs-title {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }

  .veil-specs-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }

  @media (max-width: 520px) {
    .veil-specs-grid { grid-template-columns: 1fr; }
  }

  .veil-spec-card {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    transition: border-color var(--t-fast);
  }

  .veil-spec-card:hover {
    border-color: var(--veil-border);
  }

  .veil-spec-icon-wrap {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .veil-spec-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .veil-spec-title {
    font-size: var(--text-xs);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .veil-spec-desc {
    font-size: 11px;
    color: var(--veil-text-muted);
    line-height: var(--leading-relaxed);
    margin: 0;
  }

  .veil-about-facts {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: var(--space-2);
    width: 100%;
    max-width: 540px;
    margin: var(--space-2) 0;
    text-align: left;
  }
  .veil-about-fact {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-3);
    min-width: 0;
  }
  .veil-about-fact dt {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
    margin-bottom: 2px;
  }
  .veil-about-fact dd {
    font-size: var(--text-sm);
    color: var(--veil-text-primary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-about-links {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    max-width: 440px;
    margin: var(--space-2) 0;
  }
  .veil-about-link {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    color: var(--veil-text-secondary);
    font-size: var(--text-sm);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--t-fast), color var(--t-fast), background var(--t-fast);
  }
  .veil-about-link:hover {
    border-color: var(--veil-border);
    background: var(--veil-bg-surface);
    color: var(--veil-text-primary);
  }
  .veil-about-link-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    color: var(--veil-brand);
  }
  .veil-about-link-label {
    flex: 1;
    min-width: 0;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-about-link-tag {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    flex-shrink: 0;
  }
  .veil-about-link-arrow {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    color: var(--veil-text-muted);
    transition: color var(--t-fast);
  }
  .veil-about-link:hover .veil-about-link-arrow { color: var(--veil-brand); }

  /* ── Diagnostics ─────────────────────────────────────────────── */
  .veil-diagnostics {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-3);
    width: 100%;
    max-width: 540px;
    margin: var(--space-2) 0;
    padding-top: var(--space-4);
    border-top: 1px solid var(--veil-border-subtle);
  }
  .veil-diagnostics-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--veil-brand, #818cf8);
  }
  .veil-diagnostics-title {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-diagnostics-status-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    width: 100%;
  }
  .veil-diag-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 11px;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    transition: all 0.15s ease;
  }
  .veil-diag-card.ok {
    border-color: rgba(34, 197, 94, 0.25);
    background: rgba(34, 197, 94, 0.04);
  }
  .veil-diag-card-left {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--veil-text, #f1f5f9);
    min-width: 0;
  }
  .veil-diag-label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-diag-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--veil-danger, #ef4444);
    box-shadow: 0 0 6px rgba(239, 68, 68, 0.5);
    flex-shrink: 0;
  }
  .veil-diag-dot.ok {
    background: var(--veil-success, #22c55e);
    box-shadow: 0 0 6px rgba(34, 197, 94, 0.6);
  }
  .veil-diag-badge {
    font-size: 9.5px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(239, 68, 68, 0.12);
    color: var(--veil-danger, #ef4444);
    flex-shrink: 0;
  }
  .veil-diag-badge.ok {
    background: rgba(34, 197, 94, 0.12);
    color: var(--veil-success, #22c55e);
  }
  .veil-log-btn {
    align-self: center;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
</style>

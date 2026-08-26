<script lang="ts">
  import { onMount } from 'svelte';
  import { save, open } from '@tauri-apps/plugin-dialog';
  import { identityApi, dataApi, privacyToolsApi, type SessionInfo } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { formatRelativeTime } from '$lib/utils/format';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import { trustedDomainsStore } from '$lib/stores/trustedDomains';

  let sessions = $state<SessionInfo[]>([]);
  let loadingSessions = $state(true);
  let recoveryCode = $state<string | null>(null);
  let recoveryError = $state<string | null>(null);
  let autoUnlock = $state(false);

  // Trusted domains management
  const trustedConfig = $derived($trustedDomainsStore);
  let newDomainInput = $state('');
  let newDomainError = $state<string | null>(null);

  function handleAddDomain() {
    newDomainError = null;
    const clean = newDomainInput.trim().toLowerCase();
    if (!clean) return;
    if (!clean.includes('.') || clean.length < 3) {
      newDomainError = 'Lütfen geçerli bir alan adı girin (örn. ornek.com).';
      return;
    }
    trustedDomainsStore.addTrustedDomain(clean);
    newDomainInput = '';
    toastStore.success(`${clean} güvenilir sitelere eklendi.`);
  }

  // Data management
  let busy = $state(false);

  onMount(async () => {
    await loadSessions();
    try {
      autoUnlock = await identityApi.hasAutoUnlock();
    } catch { /* ignored */ }
  });

  async function toggleAutoUnlock(enabled: boolean) {
    if (enabled) {
      const pass = await uiStore.promptInput('Açılışta oturumu hatırlamak için parolanı doğrula:', {
        title: 'Beni Hatırla',
        secret: true,
        confirmLabel: 'Kaydet',
      });
      if (!pass) return;
      try {
        await identityApi.setAutoUnlock(true, pass);
        autoUnlock = true;
        toastStore.success('Açılışta otomatik kilit açma etkinleştirildi.');
      } catch {
        autoUnlock = false;
        toastStore.error('Parola doğrulanamadı.');
      }
    } else {
      try {
        await identityApi.setAutoUnlock(false);
        autoUnlock = false;
        toastStore.success('Açılışta otomatik kilit açma kapatıldı.');
      } catch {
        toastStore.error('İşlem başarısız.');
      }
    }
  }

  async function loadSessions() {
    loadingSessions = true;
    // Ağ yavaşsa/sunucu yanıt vermiyorsa arayüz sonsuza dek "yükleniyor"da
    // kalmasın — 8 sn sonra yerel fallback ile devam edilir.
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('timeout')), 8000)
    );
    try {
      sessions = await Promise.race([identityApi.listSessions(), timeout]);
    } catch {
      try {
        sessions = await Promise.race([identityApi.listSessions(), timeout]);
      } catch {
        toastStore.error('Oturum listesi alınamadı.');
        sessions = [];
      }
    } finally {
      loadingSessions = false;
    }
  }

  async function revoke(deviceId: string) {
    const ok = await uiStore.confirm(
      'Bu oturumu sonlandırmak istediğine emin misin?',
      { title: 'Oturumu Sonlandır', confirmLabel: 'Sonlandır', danger: true }
    );
    if (!ok) return;
    try {
      await identityApi.revokeSession(deviceId);
      toastStore.success('Oturum sonlandırıldı.');
      await loadSessions();
    } catch {
      toastStore.error('Oturum sonlandırılamadı.');
    }
  }

  async function verifyPassphrase() {
    const pass = await uiStore.promptInput('Mevcut parolanı gir:', {
      title: 'Parolayı Doğrula',
      secret: true,
      confirmLabel: 'Doğrula',
    });
    if (!pass) return;
    try {
      const ok = await identityApi.verifyPassphrase(pass);
      toastStore[ok ? 'success' : 'error'](ok ? 'Parola doğru.' : 'Parola yanlış.');
    } catch {
      toastStore.error('Doğrulama yapılamadı.');
    }
  }

  async function showRecoveryCode() {
    if (recoveryCode) { recoveryCode = null; return; }
    recoveryError = null;
    const pass = await uiStore.promptInput('Kurtarma kodunu göstermek için parolanı gir:', {
      title: 'Kurtarma Kodu',
      secret: true,
      confirmLabel: 'Göster',
    });
    if (!pass) return;
    try {
      recoveryCode = await identityApi.getRecoveryCode(pass);
      toastStore.warning('Kurtarma kodunu güvenli bir yerde sakla.');
    } catch {
      recoveryError = 'Parola yanlış veya kod alınamadı.';
    }
  }

  async function checkPasswordLeak() {
    const pass = await uiStore.promptInput('Sızıntı veri tabanında denetlemek istediğin parolayı gir (k-anonymity ile korunur, parola hiçbir yere gönderilmez):', {
      title: 'Sıfır-Bilgi Parola Sızıntısı Denetimi',
      secret: true,
      confirmLabel: 'Denetle',
    });
    if (!pass) return;
    try {
      const res = await privacyToolsApi.checkPasswordPwned(pass);
      if (res.isPwned) {
        toastStore.error(`⚠️ Bu parola ${res.breachCount.toLocaleString('tr-TR')} veri sızıntısında bulundu! Değiştirmeniz önerilir.`);
      } else {
        toastStore.success('✔ Bu parola bilinen hiçbir veri sızıntısında bulunamadı (Temiz).');
      }
    } catch {
      toastStore.error('Sızıntı veritabanına ulaşılamadı.');
    }
  }

  async function exportData() {
    const path = await save({ title: 'veilanon yedek dosyası', defaultPath: 'veilanon-backup.veil', filters: [{ name: 'veilanon arşivi', extensions: ['veil', 'zip'] }] });
    if (!path) return;
    busy = true;
    try {
      await dataApi.exportData(path);
      toastStore.success('Veriler dışa aktarıldı.');
    } catch {
      toastStore.error('Dışa aktarma başarısız.');
    } finally {
      busy = false;
    }
  }

  async function importData() {
    const path = await open({ title: 'veilanon arşivi seç', multiple: false, filters: [{ name: 'veilanon arşivi', extensions: ['veil', 'zip'] }] });
    if (!path || typeof path !== 'string') return;
    const pass = await uiStore.promptInput('Arşivi açmak için parolanı gir:', {
      title: 'Verileri İçe Aktar',
      secret: true,
      confirmLabel: 'Aç',
    });
    if (!pass) return;
    busy = true;
    try {
      await dataApi.importData({ archivePath: path, passphrase: pass });
      toastStore.success('Veriler içe aktarıldı.');
    } catch {
      toastStore.error('İçe aktarma başarısız. Parola yanlış olabilir.');
    } finally {
      busy = false;
    }
  }

  async function clearLocalData() {
    const confirmed = await uiStore.confirm(
      'Tüm yerel veriler kalıcı olarak silinecek. Bu işlem geri alınamaz. Devam et?',
      { title: 'Yerel verileri temizle', confirmLabel: 'Temizle', danger: true }
    );
    if (!confirmed) return;
    const pass = await uiStore.promptInput('Onaylamak için parolanı gir:', {
      title: 'Yerel verileri temizle',
      secret: true,
      confirmLabel: 'Sil',
    });
    if (!pass) return;
    busy = true;
    try {
      await dataApi.clearLocalData(pass);
      toastStore.success('Yerel veriler temizlendi.');
    } catch {
      toastStore.error('Temizleme başarısız.');
    } finally {
      busy = false;
    }
  }
</script>

<section aria-labelledby="guvenlik-title">
  <h2 class="veil-settings-title" id="guvenlik-title">Güvenlik</h2>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Parola & Kimlik Güvenliği</div>
    <p class="veil-settings-row-desc" style="margin-bottom: var(--space-3);">
      Hesap parolanızı doğrulayın, acil durum kurtarma kodunuzu görüntüleyin veya sıfır-bilgi (k-anonymity) sızıntı denetimi yapın.
    </p>
    <div class="veil-security-actions">
      <button class="veil-security-btn" onclick={verifyPassphrase} disabled={busy}>
        <span class="veil-security-btn-icon"><Icon name="key" size={16} /></span>
        <span class="veil-security-btn-info">
          <span class="veil-security-btn-label">Parolayı Doğrula</span>
          <span class="veil-security-btn-desc">Mevcut parolanın doğruluğunu kontrol et</span>
        </span>
        <Icon name="arrow-right" size={14} class="veil-security-btn-arrow" />
      </button>
      <button class="veil-security-btn" onclick={showRecoveryCode} disabled={busy}>
        <span class="veil-security-btn-icon"><Icon name={recoveryCode ? 'eye-off' : 'eye'} size={16} /></span>
        <span class="veil-security-btn-info">
          <span class="veil-security-btn-label">{recoveryCode ? 'Kurtarma Kodunu Gizle' : 'Kurtarma Kodunu Göster'}</span>
          <span class="veil-security-btn-desc">Acil kurtarma kodunu görüntüle veya gizle</span>
        </span>
        <Icon name="arrow-right" size={14} class="veil-security-btn-arrow" />
      </button>
      <button class="veil-security-btn" onclick={checkPasswordLeak} disabled={busy}>
        <span class="veil-security-btn-icon"><Icon name="shield" size={16} /></span>
        <span class="veil-security-btn-info">
          <span class="veil-security-btn-label">Sızıntı Denetimi (k-Anonymity)</span>
          <span class="veil-security-btn-desc">Parolanı veri ihlali veritabanlarında denetle</span>
        </span>
        <Icon name="arrow-right" size={14} class="veil-security-btn-arrow" />
      </button>
    </div>
    {#if recoveryCode}
      <div class="veil-recovery-box veil-pop-in">
        <div class="veil-recovery-head">
          <span class="veil-recovery-label">
            <Icon name="key" size={14} />
            <span>Acil Durum Kurtarma Kodunuz</span>
          </span>
          <span class="veil-recovery-warning">Bu kodu güvenli bir yerde saklayın</span>
        </div>
        <pre class="veil-recovery" aria-label="Kurtarma kodu">{recoveryCode}</pre>
        <div class="veil-recovery-actions">
          <button
            class="btn btn-secondary btn-sm"
            onclick={async () => { await copyText(recoveryCode!); toastStore.success('Kurtarma kodu kopyalandı.'); }}
          >
            <Icon name="copy" size={14} />
            <span>Kodu Kopyala</span>
          </button>
          <button
            class="btn btn-ghost btn-sm"
            onclick={() => (recoveryCode = null)}
          >
            <Icon name="eye-off" size={14} />
            <span>Gizle</span>
          </button>
        </div>
      </div>
    {/if}
    {#if recoveryError}
      <p class="veil-form-error">{recoveryError}</p>
    {/if}
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Başlangıç Kilidi</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Açılışta oturum parolasını hatırla</div>
        <div class="veil-settings-row-desc">
          Uygulama her açıldığında parola sormadan oturumu doğrudan aç. Anahtarlar OS anahtar kasasında güvenle korunur.
        </div>
      </div>
      <Toggle checked={autoUnlock} onChange={toggleAutoUnlock} label="Açılışta oturum parolasını hatırla" />
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Oturumlar / Cihazlar</div>
    {#if loadingSessions}
      <div class="veil-spinner" style="margin:1rem 0;"></div>
    {:else if sessions.length === 0}
      <p class="veil-settings-row-desc">Başka cihaz yok.</p>
    {:else}
      {#each sessions as s (s.deviceId)}
        <div class="veil-settings-row">
          <div class="veil-settings-row-info">
            <div class="veil-settings-row-label">
              {s.name}{s.isCurrent ? ' (bu cihaz)' : ''}
            </div>
            <div class="veil-settings-row-desc">
              {#if s.platform}<span class="veil-session-platform">{s.platform}</span>{/if}
              Son etkinlik: {formatRelativeTime(s.lastActiveAt)}
            </div>
          </div>
          {#if !s.isCurrent}
            <button class="btn btn-danger btn-sm" onclick={() => revoke(s.deviceId)}>Sonlandır</button>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Güvenilir Alan Adları ve Bağlantı Yönlendirme</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Güvenilir sitelere direkt yönlendir</div>
        <div class="veil-settings-row-desc">
          Listede yer alan güvenilir alan adlarına tıklandığında uyarı ekranı göstermeden doğrudan aç.
        </div>
      </div>
      <Toggle
        checked={trustedConfig.directRedirectForTrusted}
        onChange={(v) => trustedDomainsStore.setDirectRedirectForTrusted(v)}
        label="Güvenilir sitelere direkt yönlendir"
      />
    </div>

    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Tüm bağlantıları sormadan aç</div>
        <div class="veil-settings-row-desc">
          Harici linklere tıklanırken hiçbir uyarı modalı açılmaz (dikkat: kimlik avı koruması devre dışı kalır).
        </div>
      </div>
      <Toggle
        checked={trustedConfig.alwaysOpenWithoutPrompt}
        onChange={(v) => trustedDomainsStore.setAlwaysOpenWithoutPrompt(v)}
        label="Tüm bağlantıları sormadan aç"
      />
    </div>

    <div class="veil-domains-manager">
      <div class="veil-domains-form">
        <input
          type="text"
          class="veil-input"
          placeholder="yeni-domain.com"
          bind:value={newDomainInput}
          onkeydown={(e) => { if (e.key === 'Enter') handleAddDomain(); }}
        />
        <button class="btn btn-secondary" onclick={handleAddDomain} disabled={!newDomainInput.trim()}>
          <Icon name="plus" size={14} />
          Ekle
        </button>
        <button class="btn btn-ghost btn-sm" onclick={() => trustedDomainsStore.resetToDefaults()}>
          Varsayılana Sıfırla
        </button>
      </div>
      {#if newDomainError}
        <p class="veil-form-error">{newDomainError}</p>
      {/if}

      <div class="veil-domains-list" aria-label="Güvenilir alan adları listesi">
        {#each trustedConfig.trustedDomains as domain (domain)}
          <span class="veil-domain-chip">
            <Icon name="shield" size={12} />
            <span class="veil-domain-text">{domain}</span>
            <button
              class="veil-domain-remove btn-icon"
              onclick={() => trustedDomainsStore.removeTrustedDomain(domain)}
              title="{domain} listesinden kaldır"
              aria-label="{domain} kaldır"
            >
              <Icon name="x" size={12} />
            </button>
          </span>
        {/each}
      </div>
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Veri Yönetimi & Yedekleme</div>
    <div class="veil-data-actions-grid">
      <button class="veil-data-action-card" onclick={exportData} disabled={busy}>
        <span class="veil-data-action-icon"><Icon name="download" size={18} /></span>
        <span class="veil-data-action-info">
          <span class="veil-data-action-title">Dışa Aktar</span>
          <span class="veil-data-action-desc">Şifreli hesap yedeği oluştur</span>
        </span>
      </button>
      <button class="veil-data-action-card" onclick={importData} disabled={busy}>
        <span class="veil-data-action-icon"><Icon name="upload" size={18} /></span>
        <span class="veil-data-action-info">
          <span class="veil-data-action-title">İçe Aktar</span>
          <span class="veil-data-action-desc">Yedek dosyasını geri yükle</span>
        </span>
      </button>
      <button class="veil-data-action-card danger" onclick={clearLocalData} disabled={busy}>
        <span class="veil-data-action-icon danger"><Icon name="trash" size={18} /></span>
        <span class="veil-data-action-info">
          <span class="veil-data-action-title">Verileri Temizle</span>
          <span class="veil-data-action-desc">Bu cihazdaki yerel belleği sıfırla</span>
        </span>
      </button>
    </div>
    <p class="veil-settings-row-desc veil-note">
      Yedekler uçtan uca şifrelidir; arşiv dosyası yalnızca ana hesap parolanla açılabilir.
    </p>
  </div>
</section>

<style>
  /* ── Güvenlik Premium Butonları ─────────────────────────────── */
  .veil-security-actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
  }

  .veil-security-btn {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--t-fast), background var(--t-fast), box-shadow var(--t-fast);
    color: var(--veil-text-primary);
  }

  .veil-security-btn:hover:not(:disabled) {
    border-color: var(--veil-border-focus);
    background: var(--veil-bg-surface);
    box-shadow: var(--shadow-sm);
  }

  .veil-security-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .veil-security-btn-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    flex-shrink: 0;
  }

  .veil-security-btn-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .veil-security-btn-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .veil-security-btn-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  :global(.veil-security-btn-arrow) {
    color: var(--veil-text-muted);
    flex-shrink: 0;
    transition: transform var(--t-fast);
  }

  .veil-security-btn:hover:not(:disabled) :global(.veil-security-btn-arrow) {
    transform: translateX(2px);
    color: var(--veil-text-secondary);
  }

  /* ── Oturum Platformu Etiketi ───────────────────────────────── */
  .veil-session-platform {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-full);
    padding: 1px var(--space-2);
    margin-right: var(--space-2);
  }

  /* ── Kurtarma Kodu ──────────────────────────────────────────── */
  .veil-recovery-box {
    margin-top: var(--space-3);
    padding: var(--space-4);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-focus);
    border-radius: var(--radius-xl);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-recovery-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .veil-recovery-label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--veil-brand);
  }
  .veil-recovery-warning {
    font-size: 11px;
    color: var(--veil-warning);
    font-weight: 500;
  }
  .veil-recovery {
    margin: 0;
    padding: var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    letter-spacing: 0.05em;
    user-select: text;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--veil-text-primary);
  }
  .veil-recovery-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .veil-note { margin-top: var(--space-2); }

  /* ── Güvenilir Domainler ────────────────────────────────────── */
  .veil-domains-manager {
    margin-top: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-domains-form {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .veil-domains-form .veil-input {
    flex: 1;
    max-width: 320px;
  }

  .veil-domains-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .veil-domain-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px var(--space-3);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    color: var(--veil-text-primary);
    transition: border-color var(--t-fast);
  }

  .veil-domain-chip:hover {
    border-color: var(--veil-brand);
  }

  .veil-domain-text {
    font-family: var(--font-mono);
  }

  .veil-domain-remove {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--veil-text-muted);
    padding: 2px;
    display: flex;
    align-items: center;
    border-radius: var(--radius-full);
  }

  .veil-domain-remove:hover {
    color: var(--veil-danger);
    background: hsl(var(--veil-danger-hsl, 0 84% 60%) / 0.15);
  }

  /* ── Veri Kartları Grid ─────────────────────────────────────── */
  .veil-data-actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  .veil-data-action-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: all var(--t-fast);
    color: var(--veil-text-primary);
  }

  .veil-data-action-card:hover:not(:disabled) {
    background: var(--veil-bg-surface);
    border-color: var(--veil-border-focus);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .veil-data-action-card.danger:hover:not(:disabled) {
    border-color: var(--veil-danger);
    background: rgba(243, 139, 168, 0.08);
  }

  .veil-data-action-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .veil-data-action-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    flex-shrink: 0;
  }

  .veil-data-action-icon.danger {
    background: rgba(243, 139, 168, 0.15);
    color: var(--veil-danger);
  }

  .veil-data-action-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .veil-data-action-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .veil-data-action-card.danger .veil-data-action-title {
    color: var(--veil-danger);
  }

  .veil-data-action-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    line-height: var(--leading-normal);
  }
</style>


<script lang="ts">
  import { onMount } from 'svelte';
  import Toggle from '../ui/Toggle.svelte';
  import VeilSelect from '../ui/VeilSelect.svelte';
  import Icon from '../ui/Icon.svelte';
  import {
    settingsApi,
    localAiApi,
    privacyToolsApi,
    type AppSettings,
    type PresenceVisibility,
    type NotificationPreview,
    type MultiDohResult,
    type NetworkAsnResult,
    type TorStatusResult,
    type ClockSkewResult,
    type NetworkProxyMode,
    type NetworkPrivacySettings,
    type ProxyTestResult,
    type SystemVpnDetectionResult,
    type WireguardValidationResult,
    type PrivacyEndpointInfo,
  } from '$lib/api/tauri';
  import { privacyShield, isScreenShareActive, revealedMap } from '$lib/stores/privacyShield';
  import { toastStore } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { authStore } from '$lib/stores/auth';

  let settings = $state<AppSettings | null>(null);
  let loading = $state(true);

  // Yerel AI durumu (gerçek Ollama bağlantısı)
  let aiAvailable = $state(false);
  let aiModel = $state<string | null>(null);
  let aiChecking = $state(false);
  let aiTestResult = $state<string | null>(null);
  let aiTesting = $state(false);

  // ── Network Privacy, Tor & VPN ─────────────────────────────────
  let proxyTestRunning = $state(false);
  let proxyTestResult = $state<ProxyTestResult | null>(null);
  let autoScanRunning = $state(false);
  let systemVpnResult = $state<SystemVpnDetectionResult | null>(null);
  let privacyEndpoints = $state<PrivacyEndpointInfo[]>([]);
  let wireguardInput = $state('');
  let wireguardValidation = $state<WireguardValidationResult | null>(null);
  let wireguardValidating = $state(false);

  // Demo simulator secret for user testing
  const DEMO_SECRET = 'veilanon-demo-preview-sample-secret';

  // ── Multi-DoH Benchmark ────────────────────────────────────────
  let dohResult = $state<MultiDohResult | null>(null);
  let dohLoading = $state(false);
  let dohError = $state(false);

  // ── Network ASN ────────────────────────────────────────────────
  let asnResult = $state<NetworkAsnResult | null>(null);
  let asnLoading = $state(false);
  let asnError = $state(false);

  // ── Tor Anonymity ──────────────────────────────────────
  let torResult = $state<TorStatusResult | null>(null);
  let torLoading = $state(false);

  // ── Clock Skew ─────────────────────────────────────────────────
  let skewResult = $state<ClockSkewResult | null>(null);
  let skewLoading = $state(false);

  onMount(() => {
    void (async () => {
      try {
        settings = await settingsApi.get();
        if (settings?.localAiEnabled) void checkAi();
        if (settings?.networkPrivacy?.wireguardProfile) {
          wireguardInput = settings.networkPrivacy.wireguardProfile;
          validateWireguard(wireguardInput);
        }
      } catch {
        toastStore.error('Gizlilik ayarları yüklenemedi.');
      } finally {
        loading = false;
      }
      try {
        privacyEndpoints = await privacyToolsApi.getPrivacyEndpointsAndRelays();
      } catch {
        privacyEndpoints = [];
      }
      void runAllDiagnostics();
      void privacyToolsApi.detectSystemVpnServices().then(r => { systemVpnResult = r; }).catch(() => {});
    })();
    const diagInterval = setInterval(() => { void runAllDiagnostics(); }, 45000);
    const aiInterval = setInterval(() => { if (settings?.localAiEnabled) void checkAi(); }, 30000);
    return () => { clearInterval(diagInterval); clearInterval(aiInterval); };
  });

  async function runAllDiagnostics() {
    void runDohBenchmark();
    void runAsnLookup();
    void runTorCheck();
    void runSkewCheck();
  }

  async function runFullAutoScan() {
    autoScanRunning = true;
    try {
      const [sysVpn, torStat, asn, doh] = await Promise.all([
        privacyToolsApi.detectSystemVpnServices(),
        privacyToolsApi.checkTorStatus().catch(() => null),
        privacyToolsApi.getNetworkAsnInfo().catch(() => null),
        privacyToolsApi.checkMultiDohStatus().catch(() => null),
      ]);

      systemVpnResult = sysVpn;
      torResult = torStat;
      if (asn) asnResult = asn;
      if (doh) dohResult = doh;
      const currentMode = getNetworkPrivacy().mode;
      if (currentMode === 'direct' && sysVpn.recommendedMode && sysVpn.recommendedMode !== 'direct') {
        const rec = sysVpn.recommendedMode as NetworkProxyMode;
        const recEndpoint = sysVpn.recommendedEndpoint || '';
        let host = '127.0.0.1';
        let port = 9050;
        if (recEndpoint.includes(':')) {
          const parts = recEndpoint.split(':');
          host = parts[0] || host;
          const p = parseInt(parts[1] || '', 10);
          if (!isNaN(p)) port = p;
        }
        if (rec === 'tor' && sysVpn.torStandalone) port = 9050;
        else if (rec === 'tor' && sysVpn.torBrowser) port = 9150;
        else if (rec === 'cloudflare_warp') { host = '127.0.0.1'; port = 40000; }
        updateNetworkPrivacy({ mode: rec, proxyHost: host, proxyPort: port, activePreset: rec });
        toastStore.success(`Otomatik gizlilik: ${rec} tüneli tespit edildi ve uygulandı.`);
      } else if (sysVpn.torStandalone) {
        toastStore.success('Yerel Tor Daemon (9050) tespit edildi!');
      } else if (sysVpn.torBrowser) {
        toastStore.success('Tor Browser SOCKS tüneli (9150) tespit edildi!');
      } else if (sysVpn.cloudflareWarpRunning) {
        toastStore.success('Cloudflare WARP bağlantısı tespit edildi!');
      } else {
        toastStore.info('Ağ taraması tamamlandı: Doğrudan bağlantı aktif.');
      }
    } catch (err) {
      toastStore.error('Ağ taraması sırasında hata oluştu.');
    } finally {
      autoScanRunning = false;
    }
  }

  async function runTorCheck() {
    torLoading = true;
    try {
      torResult = await privacyToolsApi.checkTorStatus();
    } catch {
      torResult = null;
    } finally {
      torLoading = false;
    }
  }

  async function runSkewCheck() {
    skewLoading = true;
    try {
      skewResult = await privacyToolsApi.detectClockSkew();
    } catch {
      skewResult = null;
    } finally {
      skewLoading = false;
    }
  }

  async function runDohBenchmark() {
    dohLoading = true;
    dohError = false;
    try {
      dohResult = await privacyToolsApi.checkMultiDohStatus();
    } catch {
      dohError = true;
    } finally {
      dohLoading = false;
    }
  }

  async function runAsnLookup() {
    asnLoading = true;
    asnError = false;
    try {
      asnResult = await privacyToolsApi.getNetworkAsnInfo();
    } catch {
      asnError = true;
    } finally {
      asnLoading = false;
    }
  }

  function latencyColor(ms: number): string {
    if (ms === 0) return 'var(--veil-text-disabled)';
    if (ms < 80) return 'var(--veil-success)';
    if (ms < 200) return 'hsl(36 100% 55%)';
    return 'var(--veil-warning)';
  }

  function latencyBar(ms: number, max = 400): number {
    if (ms === 0) return 0;
    return Math.min(100, Math.round((ms / max) * 100));
  }

  let aiModels: string[] = $state([]);
  async function checkAi() {
    aiChecking = true;
    try {
      const status: any = await localAiApi.status();
      aiAvailable = status.available;
      aiModel = status.model;
      aiModels = status.models ?? (status.model ? [status.model] : []);
      if (aiAvailable && !aiModels.length) {
        aiModels = aiModel ? [aiModel] : [];
      }
    } catch {
      aiAvailable = false;
      aiModel = null;
      aiModels = [];
    } finally {
      aiChecking = false;
    }
  }

  async function testAi() {
    aiTesting = true;
    aiTestResult = null;
    try {
      const reply = await localAiApi.chat({
        message: 'Tek cümleyle kendini tanıt.',
        model: aiModel,
      });
      aiTestResult = reply;
      toastStore.success('Yerel yapay zekâ çalışıyor.');
    } catch (err) {
      aiTestResult = null;
      toastStore.error(String(err).replace(/^Error:\s*/, ''));
    } finally {
      aiTesting = false;
    }
  }

  async function testProxy() {
    proxyTestRunning = true;
    proxyTestResult = null;
    try {
      const res = await privacyToolsApi.testProxyConnection();
      proxyTestResult = res;
      if (res.connected) {
        if (res.isTor) {
          toastStore.success(`Tor bağlantısı doğrulandı! Çıkış IP: ${res.exitIp ?? '—'}`);
        } else {
          toastStore.success(`Tünel bağlantısı başarılı! Çıkış IP: ${res.exitIp ?? '—'}`);
        }
      } else {
        toastStore.error(res.errorMessage || 'Proxy sunucusuna bağlanılamadı.');
      }
    } catch {
      toastStore.error('Bağlantı test çağrısı başarısız oldu.');
    } finally {
      proxyTestRunning = false;
    }
  }

  function validateWireguard(text: string) {
    if (!text.trim()) {
      wireguardValidation = null;
      return;
    }
    wireguardValidating = true;
    try {
      privacyToolsApi.validateWireguardProfile(text).then((res) => {
        wireguardValidation = res;
        wireguardValidating = false;
      });
    } catch {
      wireguardValidation = null;
      wireguardValidating = false;
    }
  }

  function getNetworkPrivacy(): NetworkPrivacySettings {
    return (
      settings?.networkPrivacy || {
        mode: 'direct',
        proxyHost: '127.0.0.1',
        proxyPort: 9050,
        strictMode: false,
        routeAppOnly: true,
        customProxyUrl: null,
        wireguardProfile: null,
        autoStartTor: false,
        verifyExitNode: true,
        torBridgeType: null,
        activePreset: null,
        wireguardEndpoint: null,
        wireguardPublicKey: null,
        wireguardAllowedIps: null,
      }
    );
  }

  function updateNetworkPrivacy(patch: Partial<NetworkPrivacySettings>) {
    const current = getNetworkPrivacy();
    const next = { ...current, ...patch };
    void save({ networkPrivacy: next });
  }

  function setProxyPreset(mode: NetworkProxyMode, port: number, host = '127.0.0.1', presetName?: string) {
    updateNetworkPrivacy({
      mode,
      proxyHost: host,
      proxyPort: port,
      activePreset: presetName ?? mode,
      customProxyUrl:
        mode === 'custom_socks'
          ? `socks5h://${host}:${port}`
          : mode === 'custom_http'
            ? `http://${host}:${port}`
            : mode === 'cloudflare_warp'
              ? `socks5h://${host}:${port}`
              : null,
    });
    toastStore.info(`Bağlantı modu değiştirildi: ${presetName || mode}`);
  }

  async function save(patch: Partial<AppSettings>) {
    if (!settings) return;
    const previous = settings;
    const next = { ...settings, ...patch };
    settings = next;
    try {
      settings = await settingsApi.update(next);
      if (patch.localAiEnabled !== undefined) void checkAi();
      toastStore.success('Ayarlar kaydedildi.');
    } catch {
      settings = previous;
      toastStore.error('Ayarlar kaydedilemedi.');
    }
  }
</script>

<section aria-labelledby="gizlilik-title">
  <h2 class="veil-settings-title" id="gizlilik-title">Gizlilik, Ağ & Güvenlik</h2>

  {#if loading}
    <div class="veil-spinner" style="margin:3rem auto;"></div>
  {:else if settings}
    <!-- ── ZERO-LEAK OTOMATİK EKRAN PAYLAŞIMI KALKANI ──────────────────────── -->
    <div class="veil-settings-group privacy-shield-box">
      <div class="veil-settings-group-label shield-header">
        <div class="shield-header-left">
          <Icon name="shield" size={16} />
          <span>Otomatik Ekran Paylaşımı & Canlı Yayın Kalkanı</span>
        </div>
        {#if $isScreenShareActive}
          <span class="screen-share-shield-badge">
            <span class="pulse-dot"></span>
            Canlı Yayın Koruması Devrede
          </span>
        {/if}
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">
            Hassas Bilgileri Otomatik Sansürle (Varsayılan Açık)
          </div>
          <div class="veil-settings-row-desc">
            Ekran paylaşımı veya yayın sırasında e-posta, parola, kurtarma anahtarları, cihaz kimlikleri ve köprü tokenlarını otomatik maskeler.
          </div>
        </div>
        <Toggle
          checked={$privacyShield.autoShieldEnabled}
          onChange={(v) => {
            privacyShield.setMasterToggle(v);
            if (v) {
              toastStore.success('Otomatik ekran paylaşımı koruması etkinleştirildi.');
            } else {
              toastStore.info('Otomatik koruma kapatıldı. Bilgiler standart düz metin olarak gösterilecektir.');
            }
          }}
          label="Otomatik Ekran Paylaşımı Koruması"
        />
      </div>

      {#if $privacyShield.autoShieldEnabled}
        <!-- Alt İnce Ayarlar -->
        <div class="shield-sub-settings">
          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">E-posta ve Cihaz Kimliklerini Koru</div>
              <div class="veil-settings-row-desc">Hesap e-postası ve donanım kimliklerini sansürler.</div>
            </div>
            <Toggle
              checked={$privacyShield.protectEmailsAndIds}
              onChange={(v) => privacyShield.updateConfig('protectEmailsAndIds', v)}
              label="E-postaları koru"
            />
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Parola ve Kurtarma Anahtarlarını Koru</div>
              <div class="veil-settings-row-desc">Kurtarma kelimelerini ve şifreleme anahtarlarını sansürler.</div>
            </div>
            <Toggle
              checked={$privacyShield.protectPasswordsAndKeys}
              onChange={(v) => privacyShield.updateConfig('protectPasswordsAndKeys', v)}
              label="Parolaları koru"
            />
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Davet ve Özel Topluluk Bağlantılarını Koru</div>
              <div class="veil-settings-row-desc">Sunucu davet kodları ve özel bağlantıları yayınlarda gizler.</div>
            </div>
            <Toggle
              checked={$privacyShield.protectInvites}
              onChange={(v) => privacyShield.updateConfig('protectInvites', v)}
              label="Davetleri koru"
            />
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Discord Webhook ve Entegrasyon Tokenlarını Koru</div>
              <div class="veil-settings-row-desc">Harici bot köprü adreslerini ve API anahtarlarını maskeler.</div>
            </div>
            <Toggle
              checked={$privacyShield.protectWebhooks}
              onChange={(v) => privacyShield.updateConfig('protectWebhooks', v)}
              label="Webhookları koru"
            />
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Ekran Paylaşımında Medyaları Bulanıklaştır</div>
              <div class="veil-settings-row-desc">Kanallardaki resim ve video eklerini yayın esnasında fareyle üzerine gelinene kadar bulanık tutar.</div>
            </div>
            <Toggle
              checked={$privacyShield.blurMediaOnShare}
              onChange={(v) => privacyShield.updateConfig('blurMediaOnShare', v)}
              label="Medyaları Bulanıklaştır"
            />
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Açılan Parolaları Otomatik Yeniden Kilitleme Süresi</div>
              <div class="veil-settings-row-desc">
                Görünür yapılan parolaları unutulmaya karşı otomatik olarak yeniden kilitler.
              </div>
            </div>
            <VeilSelect
              options={[
                { value: '0', label: 'Zamanlayıcı Yok (Manuel)' },
                { value: '3', label: '3 saniye' },
                { value: '5', label: '5 saniye (Önerilen)' },
                { value: '10', label: '10 saniye' },
                { value: '15', label: '15 saniye' },
                { value: '30', label: '30 saniye' },
              ]}
              value={String($privacyShield.autoHideTimeoutSeconds)}
              onChange={(v) => privacyShield.updateConfig('autoHideTimeoutSeconds', Number(v))}
            />
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Sansür Maskeleme Stili</div>
              <div class="veil-settings-row-desc">Hassas metinlerin sansürlenme biçimini seçin.</div>
            </div>
            <VeilSelect
              options={[
                { value: 'bullets', label: '•••••••• (Noktalar - Önerilen)' },
                { value: 'asterisks', label: '******** (Yıldızlar)' },
                { value: 'hidden', label: '[GİZLENDİ] (Etiket)' },
                { value: 'blur', label: 'Bulanıklaştırma (Blur)' },
              ]}
              value={$privacyShield.maskStyle}
              onChange={(v) => privacyShield.updateConfig('maskStyle', v as any)}
            />
          </div>

          <!-- Canlı Önizleme & Test Kutusu -->
          <div class="shield-preview-card">
            <div class="preview-head">
              <span class="preview-title">Canlı Kalkan Önizlemesi</span>
              <span class="preview-hint">Göz simgesine tıklayarak otomatik kilidi test edin:</span>
            </div>
            <div class="preview-secret-row">
              <div
                class="preview-secret-value veil-mono"
                data-auto-protect="secret"
                data-revealed={$revealedMap['demo-test-secret'] > Date.now()}
              >
                {privacyShield.formatSecretManual(DEMO_SECRET, 'demo-test-secret')}
              </div>
              <button
                type="button"
                class="btn btn-secondary btn-sm"
                onclick={() => privacyShield.toggleSecret('demo-test-secret')}
              >
                <Icon name={$revealedMap['demo-test-secret'] > Date.now() ? 'eye-off' : 'eye'} size={14} />
                <span>{$revealedMap['demo-test-secret'] > Date.now() ? 'Gizle' : `${$privacyShield.autoHideTimeoutSeconds > 0 ? `${$privacyShield.autoHideTimeoutSeconds}sn ` : ''}Göster`}</span>
              </button>
            </div>
          </div>
        </div>
      {:else}
        <div class="shield-disabled-notice">
          <Icon name="info" size={15} />
          <span>Otomatik kalkan kapalı. Tüm kimlik, parola ve bağlantılar standart açık metin olarak görüntülenir.</span>
        </div>
      {/if}
    </div>

    <!-- ── AĞ & BAĞLANTI GİZLİLİĞİ (TOR / VPN / SOCKS5h / WARP) ───────────── -->
    {@const netPriv = getNetworkPrivacy()}
    <div class="veil-settings-group net-privacy-box">
      <div class="veil-settings-group-label net-privacy-header">
        <div class="net-privacy-header-left">
          <Icon name="globe-lock" size={16} />
          <span>Ağ & Bağlantı Gizliliği</span>
        </div>
        <div class="status-pill-wrap">
          {#if netPriv.mode === 'tor'}
            <span class="net-badge net-badge-ok">
              <span class="pulse-dot"></span>
              <Icon name="tor" size={13} />
              <span>Tor SOCKS5h Aktif</span>
            </span>
          {:else if netPriv.mode === 'cloudflare_warp'}
            <span class="net-badge net-badge-ok">
              <span class="pulse-dot"></span>
              <Icon name="warp" size={13} />
              <span>Cloudflare WARP Aktif</span>
            </span>
          {:else if netPriv.mode === 'wireguard'}
            <span class="net-badge net-badge-ok">
              <span class="pulse-dot"></span>
              <Icon name="wireguard" size={13} />
              <span>WireGuard Tüneli</span>
            </span>
          {:else if netPriv.mode === 'custom_socks' || netPriv.mode === 'custom_http'}
            <span class="net-badge net-badge-ok">
              <span class="pulse-dot"></span>
              <Icon name="proxy" size={13} />
              <span>Özel Proxy Aktif</span>
            </span>
          {:else}
            <span class="net-badge net-badge-neutral">
              <Icon name="wifi" size={12} />
              <span>Doğrudan Bağlantı</span>
            </span>
          {/if}
        </div>
      </div>

      <!-- Bağlantı Modu Seçimi -->
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Trafik Yönlendirme Modu</div>
          <div class="veil-settings-row-desc">
            Tüm REST API çağrıları, dosya transferleri ve senkronizasyon isteklerinin geçeceği gizlilik tüneli.
          </div>
        </div>
        <VeilSelect
          options={[
            { value: 'direct', label: 'Doğrudan Bağlantı (Açık Ağ)' },
            { value: 'tor', label: 'Tor Ağı (Yerel Tor Daemon / SOCKS5h)' },
            { value: 'cloudflare_warp', label: 'Cloudflare WARP (1.1.1.1 WireGuard)' },
            { value: 'wireguard', label: 'WireGuard VPN Profili' },
            { value: 'custom_socks', label: 'Özel SOCKS5 / SOCKS5h Proxy' },
            { value: 'custom_http', label: 'Özel HTTP / HTTPS Proxy' },
          ]}
          value={netPriv.mode}
          onChange={(v) => updateNetworkPrivacy({ mode: v as NetworkProxyMode })}
        />
      </div>

      <!-- ── TEK TIKLA HAZIR GİZLİLİK ÖNAYARLARI ── -->
      <div class="preset-section">
        <div class="preset-section-header">
          <div class="preset-title">
            <Icon name="zap" size={14} />
            <span>Tek Tıkla Hazır Gizlilik Katmanları</span>
          </div>
          <button
            type="button"
            class="btn btn-secondary btn-xs"
            onclick={() => { void runFullAutoScan(); }}
            disabled={autoScanRunning}
          >
            {#if autoScanRunning}
              <span class="veil-spinner-xs"></span>
              Ağ Taranıyor…
            {:else}
              <Icon name="refresh-cw" size={12} />
              Ağı Otomatik Tara & Tespit Et
            {/if}
          </button>
        </div>

        <div class="preset-cards-grid">
          <!-- Tor Daemon 9050 -->
          <button
            type="button"
            class="preset-card"
            class:preset-card-active={netPriv.mode === 'tor' && netPriv.proxyPort === 9050}
            onclick={() => setProxyPreset('tor', 9050, '127.0.0.1', 'Tor SOCKS5h')}
          >
            <div class="preset-card-icon"><Icon name="tor" size={20} /></div>
            <div class="preset-card-body">
              <span class="preset-card-title">Tor Daemon</span>
              <span class="preset-card-desc">Port 9050 • Yerel 3-Hop Soğan Ağı</span>
            </div>
            {#if netPriv.mode === 'tor' && netPriv.proxyPort === 9050}
              <span class="preset-active-check"><Icon name="check" size={13} /></span>
            {/if}
          </button>

          <!-- Tor Browser 9150 -->
          <button
            type="button"
            class="preset-card"
            class:preset-card-active={netPriv.mode === 'tor' && netPriv.proxyPort === 9150}
            onclick={() => setProxyPreset('tor', 9150, '127.0.0.1', 'Tor Browser')}
          >
            <div class="preset-card-icon"><Icon name="globe" size={20} /></div>
            <div class="preset-card-body">
              <span class="preset-card-title">Tor Browser</span>
              <span class="preset-card-desc">Port 9150 • Tarayıcı Tüneli</span>
            </div>
            {#if netPriv.mode === 'tor' && netPriv.proxyPort === 9150}
              <span class="preset-active-check"><Icon name="check" size={13} /></span>
            {/if}
          </button>

          <!-- Cloudflare WARP -->
          <button
            type="button"
            class="preset-card"
            class:preset-card-active={netPriv.mode === 'cloudflare_warp'}
            onclick={() => setProxyPreset('cloudflare_warp', 40000, '127.0.0.1', 'Cloudflare WARP')}
          >
            <div class="preset-card-icon"><Icon name="warp" size={20} /></div>
            <div class="preset-card-body">
              <span class="preset-card-title">Cloudflare WARP</span>
              <span class="preset-card-desc">1.1.1.1 WireGuard Gizlilik Katmanı</span>
            </div>
            {#if netPriv.mode === 'cloudflare_warp'}
              <span class="preset-active-check"><Icon name="check" size={13} /></span>
            {/if}
          </button>

          <!-- WireGuard VPN -->
          <button
            type="button"
            class="preset-card"
            class:preset-card-active={netPriv.mode === 'wireguard'}
            onclick={() => updateNetworkPrivacy({ mode: 'wireguard' })}
          >
            <div class="preset-card-icon"><Icon name="wireguard" size={20} /></div>
            <div class="preset-card-body">
              <span class="preset-card-title">WireGuard VPN</span>
              <span class="preset-card-desc">Özel .conf Profili & Şifreleme</span>
            </div>
            {#if netPriv.mode === 'wireguard'}
              <span class="preset-active-check"><Icon name="check" size={13} /></span>
            {/if}
          </button>
        </div>

        {#if systemVpnResult}
          <div class="sys-detect-bar">
            <Icon name="info" size={14} />
            <span>{systemVpnResult.details}</span>
          </div>
        {/if}
      </div>

      {#if netPriv.mode === 'tor' || netPriv.mode === 'custom_socks' || netPriv.mode === 'custom_http' || netPriv.mode === 'cloudflare_warp'}
        <!-- Proxy Host / Port Konfigürasyonu -->
        <div class="proxy-config-grid">
          <div class="proxy-input-group">
            <label class="proxy-input-label" for="proxy-host-input">Proxy Sunucu Adresi / IP</label>
            <input
              id="proxy-host-input"
              type="text"
              class="veil-input veil-mono"
              placeholder="127.0.0.1"
              value={netPriv.proxyHost}
              onchange={(e) => updateNetworkPrivacy({ proxyHost: (e.target as HTMLInputElement).value })}
            />
          </div>
          <div class="proxy-input-group" style="max-width: 140px;">
            <label class="proxy-input-label" for="proxy-port-input">Port</label>
            <input
              id="proxy-port-input"
              type="number"
              class="veil-input veil-mono"
              placeholder={netPriv.mode === 'tor' ? '9050' : netPriv.mode === 'cloudflare_warp' ? '40000' : '1080'}
              value={netPriv.proxyPort}
              onchange={(e) => updateNetworkPrivacy({ proxyPort: Number((e.target as HTMLInputElement).value) || 9050 })}
            />
          </div>
          {#if netPriv.mode === 'custom_socks' || netPriv.mode === 'custom_http'}
            <div class="proxy-input-group" style="grid-column: 1 / -1;">
              <label class="proxy-input-label" for="proxy-custom-url">Özel Proxy URL (Opsiyonel Tam Adres)</label>
              <input
                id="proxy-custom-url"
                type="text"
                class="veil-input veil-mono"
                placeholder={netPriv.mode === 'custom_socks' ? 'socks5h://127.0.0.1:1080' : 'http://127.0.0.1:8080'}
                value={netPriv.customProxyUrl ?? ''}
                onchange={(e) => updateNetworkPrivacy({ customProxyUrl: (e.target as HTMLInputElement).value || null })}
              />
            </div>
          {/if}
        </div>

        <!-- Kesin Gizlilik Modu (Fail-Closed) -->
        <div class="veil-settings-row">
          <div class="veil-settings-row-info">
            <div class="veil-settings-row-label">
              Kesin Gizlilik Modu (Fail-Closed Sızıntı Kalkanı)
            </div>
            <div class="veil-settings-row-desc">
              Tor veya Proxy tüneli çökerse ya da bağlantı kesilirse, gerçek IP adresinizin açığa çıkmaması için doğrudan açık internete (clear-net) çıkışı engeller.
            </div>
          </div>
          <Toggle
            checked={netPriv.strictMode}
            onChange={(v) => updateNetworkPrivacy({ strictMode: v })}
            label="Kesin Gizlilik Modu"
          />
        </div>

        <!-- Yalnızca Veilanon Trafiği -->
        <div class="veil-settings-row">
          <div class="veil-settings-row-info">
            <div class="veil-settings-row-label">
              Yalnızca Veilanon Uygulama Trafiği (SOCKS5h)
            </div>
            <div class="veil-settings-row-desc">
              Proxy tüneli yalnızca Veilanon istemcisinin giden REST ve dosya transfer isteklerini yönlendirir; işletim sistemindeki diğer yazılımları etkilemez.
            </div>
          </div>
          <Toggle
            checked={netPriv.routeAppOnly}
            onChange={(v) => updateNetworkPrivacy({ routeAppOnly: v })}
            label="Yalnızca Veilanon Trafiği"
          />
        </div>
      {/if}

      <!-- ── WIREGUARD VPN PROFİLİ YÖNETİCİSİ ── -->
      {#if netPriv.mode === 'wireguard'}
        <div class="wireguard-card">
          <div class="wireguard-header">
            <div style="display:flex;align-items:center;gap:8px;">
              <Icon name="shield-check" size={16} />
              <span class="net-label">WireGuard .conf Profil Yapılandırması</span>
            </div>
          </div>
          <div class="wireguard-desc">
            Kendi WireGuard sunucunuzun veya ücretsiz sağlayıcıların (Proton, Mullvad vb.) oluşturduğu standart `[Interface]` / `[Peer]` profil metnini yapıştırın. Özel anahtarlarınız asla loglanmaz.
          </div>
          <div class="wireguard-input-wrap">
            <textarea
              class="veil-input veil-mono wireguard-textarea"
              rows={6}
              placeholder="[Interface]&#10;PrivateKey = ...&#10;Address = 10.2.0.2/32&#10;DNS = 1.1.1.1&#10;&#10;[Peer]&#10;PublicKey = ...&#10;Endpoint = 198.51.100.1:51820&#10;AllowedIPs = 0.0.0.0/0"
              value={wireguardInput}
              oninput={(e) => {
                const val = (e.target as HTMLTextAreaElement).value;
                wireguardInput = val;
                validateWireguard(val);
              }}
            ></textarea>
          </div>

          {#if wireguardValidation}
            <div class="wg-valid-box" class:wg-ok={wireguardValidation.isValid} class:wg-err={!wireguardValidation.isValid}>
              {#if wireguardValidation.isValid}
                <div class="wg-valid-grid">
                  <div class="wg-item">
                    <span class="wg-key">Arayüz IP</span>
                    <span class="wg-val veil-mono">{wireguardValidation.interfaceAddress || '—'}</span>
                  </div>
                  <div class="wg-item">
                    <span class="wg-key">Uç Nokta</span>
                    <span class="wg-val veil-mono">{wireguardValidation.peerEndpoint || '—'}</span>
                  </div>
                  <div class="wg-item">
                    <span class="wg-key">Eş Açık Anahtar</span>
                    <span class="wg-val veil-mono">{wireguardValidation.peerPublicKey ? `${wireguardValidation.peerPublicKey.slice(0, 12)}…` : '—'}</span>
                  </div>
                  <div class="wg-item">
                    <span class="wg-key">Yönlendirilen IP'ler</span>
                    <span class="wg-val veil-mono">{wireguardValidation.allowedIps || '0.0.0.0/0'}</span>
                  </div>
                </div>
                <div style="display:flex;justify-content:flex-end;margin-top:var(--space-2);">
                  <button
                    type="button"
                    class="btn btn-primary btn-sm"
                    onclick={() => {
                      updateNetworkPrivacy({
                        wireguardProfile: wireguardInput,
                        wireguardEndpoint: wireguardValidation?.peerEndpoint,
                        wireguardPublicKey: wireguardValidation?.peerPublicKey,
                        wireguardAllowedIps: wireguardValidation?.allowedIps,
                      });
                      toastStore.success('WireGuard profili kaydedildi ve etkinleştirildi.');
                    }}
                  >
                    <Icon name="check" size={13} />
                    Profili Kaydet & Uygula
                  </button>
                </div>
              {:else}
                <div class="wg-err-msg">
                  <Icon name="warning" size={14} />
                  <span>{wireguardValidation.errorMessage || 'Geçersiz WireGuard yapılandırma dosyası.'}</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- ── CANLI PROXY & TOR BAĞLANTI TEST KARTI ── -->
      <div class="proxy-test-card">
        <div class="proxy-test-header">
          <div class="proxy-test-title-wrap">
            <span class="net-label">Canlı Bağlantı & Sızıntı Denetimi</span>
            <span class="proxy-test-hint">Tünelin çalıştığını ve DNS sızıntı korumasının devrede olduğunu test edin.</span>
          </div>
          <button
            type="button"
            class="btn btn-secondary btn-sm"
            onclick={() => { void testProxy(); }}
            disabled={proxyTestRunning}
          >
            {#if proxyTestRunning}
              <span class="veil-spinner-xs"></span>
              Tünel Test Ediliyor…
            {:else}
              <Icon name="activity" size={14} />
              Canlı Bağlantıyı Test Et
            {/if}
          </button>
        </div>

        {#if proxyTestResult}
          <div class="proxy-test-result-box" class:proxy-success={proxyTestResult.connected} class:proxy-failed={!proxyTestResult.connected}>
            <div class="proxy-test-grid">
              <div class="proxy-test-item">
                <span class="proxy-item-key">Bağlantı Durumu</span>
                <span class="proxy-item-val" style="color:{proxyTestResult.connected ? 'var(--veil-success)' : 'var(--veil-danger)'};">
                  {proxyTestResult.connected ? (proxyTestResult.isTor ? '✔ Tor Tüneli Doğrulandı' : '✔ Tünel Bağlı') : '✖ Bağlantı Başarısız'}
                </span>
              </div>
              <div class="proxy-test-item">
                <span class="proxy-item-key">Görünen Çıkış IP</span>
                <span class="proxy-item-val veil-mono">{proxyTestResult.exitIp || '—'}</span>
              </div>
              <div class="proxy-test-item">
                <span class="proxy-item-key">Protokol & Şifreleme</span>
                <span class="proxy-item-val veil-mono">{proxyTestResult.protocol}</span>
              </div>
              <div class="proxy-test-item">
                <span class="proxy-item-key">DNS Sızıntı Koruması</span>
                <span class="proxy-item-val" style="color:{proxyTestResult.dnsLeakProtected ? 'var(--veil-success)' : 'var(--veil-warning)'};">
                  {proxyTestResult.dnsLeakProtected ? '✔ SOCKS5h (DNS Güvende)' : 'Standart Çözümleme'}
                </span>
              </div>
              <div class="proxy-test-item">
                <span class="proxy-item-key">Gecikme Süresi (Ping)</span>
                <span class="proxy-item-val veil-mono" style="color:{latencyColor(proxyTestResult.latencyMs)};">{proxyTestResult.latencyMs} ms</span>
              </div>
            </div>
            {#if proxyTestResult.errorMessage}
              <div class="proxy-err-msg">
                <Icon name="warning" size={14} />
                <span>{proxyTestResult.errorMessage}</span>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- ── AĞ TANILAMASI & DOH BENCHMARK ─────────────────────────────────── -->
    <div class="veil-settings-group net-diag-group">
      <div class="veil-settings-group-label net-diag-header">
        <div class="net-diag-header-left">
          <Icon name="server" size={16} />
          <span>Ağ Tanılama & DNS Güvenlik Denetimi</span>
        </div>
        <button
          class="btn btn-secondary btn-xs"
          onclick={() => { void runDohBenchmark(); void runAsnLookup(); void runTorCheck(); }}
          disabled={dohLoading || asnLoading || torLoading}
        >
          {#if dohLoading || asnLoading || torLoading}
            <span class="veil-spinner-xs"></span>
            Test Ediliyor…
          {:else}
            <Icon name="refresh-cw" size={12} />
            Yenile
          {/if}
        </button>
      </div>

      <!-- Network ASN Card -->
      <div class="net-asn-card">
        <div class="net-asn-header">
          <span class="net-label">IP, Servis Sağlayıcı & Ağ Kimliği</span>
          {#if asnLoading}
            <span class="veil-spinner-xs"></span>
          {:else if asnResult}
            <span class="net-badge net-badge-ok">
              <span class="pulse-dot"></span>
              Bağlı
            </span>
          {:else if asnError}
            <span class="net-badge net-badge-err">Çevrimdışı</span>
          {/if}
        </div>
        {#if asnResult}
          <div class="net-asn-grid">
            <div class="net-asn-item">
              <span class="net-asn-key">Görünen IP Adresi</span>
              <span class="net-asn-val veil-mono">{asnResult.ip || '—'}</span>
            </div>
            {#if asnResult.isp}
              <div class="net-asn-item">
                <span class="net-asn-key">İnternet Sağlayıcı</span>
                <span class="net-asn-val">{asnResult.isp}</span>
              </div>
            {/if}
            {#if asnResult.country}
              <div class="net-asn-item">
                <span class="net-asn-key">Konum / Ülke</span>
                <span class="net-asn-val">{asnResult.country}</span>
              </div>
            {/if}
            {#if asnResult.tlsVersion}
              <div class="net-asn-item">
                <span class="net-asn-key">TLS Sürümü</span>
                <span class="net-asn-val veil-mono" style="color:var(--veil-success);">{asnResult.tlsVersion}</span>
              </div>
            {/if}
            {#if asnResult.httpVersion}
              <div class="net-asn-item">
                <span class="net-asn-key">HTTP Protokolü</span>
                <span class="net-asn-val veil-mono">{asnResult.httpVersion}</span>
              </div>
            {/if}
            <div class="net-asn-item">
              <span class="net-asn-key">Tor Çıkış Düğümü</span>
              {#if torLoading}
                <span class="net-asn-val veil-mono">Kontrol ediliyor…</span>
              {:else if torResult}
                <span class="net-asn-val" style="color:{torResult.isTor ? 'var(--veil-success)' : 'var(--veil-text-muted)'};">
                  {torResult.isTor ? '✔ Tor Çıkış Düğümü' : 'Doğrudan Bağlantı'}
                </span>
              {:else}
                <span class="net-asn-val veil-mono">—</span>
              {/if}
            </div>
            <div class="net-asn-item">
              <span class="net-asn-key">Saat Senkronizasyonu</span>
              {#if skewLoading}
                <span class="net-asn-val veil-mono">Kontrol ediliyor…</span>
              {:else if skewResult}
                <span class="net-asn-val" style="color:{skewResult.isSkewed ? 'var(--veil-danger)' : 'var(--veil-success)'};">
                  {skewResult.isSkewed ? `⚠️ ${skewResult.skewSeconds}s Sapma` : '✔ Senkron'}
                </span>
              {:else}
                <span class="net-asn-val veil-mono">—</span>
              {/if}
            </div>
          </div>
        {:else if asnLoading}
          <div class="net-asn-skeleton"></div>
        {:else if asnError}
          <p class="net-err-text">Ağ bilgisi alınamadı. İnternet bağlantınızı kontrol edin.</p>
        {/if}
      </div>

      <!-- Multi-DoH Benchmark Card -->
      <div class="doh-bench-card">
        <div class="doh-bench-header">
          <span class="net-label">Şifreli DNS (DoH) Sağlayıcı Benchmark</span>
          {#if dohResult}
            {#if dohResult.censorshipTamperDetected}
              <span class="net-badge net-badge-warn">
                <Icon name="warning" size={11} />
                Sansür Şüphesi
              </span>
            {:else}
              <span class="net-badge net-badge-ok">
                <span class="pulse-dot"></span>
                Güvenli & Normal
              </span>
            {/if}
          {/if}
        </div>

        {#if dohLoading}
          <div class="doh-loading">
            <span class="veil-spinner-xs"></span>
            <span>5 DoH sağlayıcısı eşzamanlı test ediliyor…</span>
          </div>
        {:else if dohError}
          <p class="net-err-text">DoH testi başarısız. İnternet bağlantınızı kontrol edin.</p>
        {:else if dohResult}
          {#if dohResult.fastestProvider}
            <p class="doh-summary">
              En hızlı sağlayıcı: <strong>{dohResult.fastestProvider}</strong>
              — Ortalama yanıt: <strong class="veil-mono">{dohResult.averageLatencyMs}ms</strong>
            </p>
          {/if}

          <div class="doh-providers">
            {#each dohResult.providers as p (p.name)}
              <div class="doh-provider-row">
                <div class="doh-provider-info">
                  <span
                    class="doh-dot"
                    style="background:{p.isReachable ? 'var(--veil-success)' : 'var(--veil-danger)'};"
                  ></span>
                  <span class="doh-name">{p.name}</span>
                </div>
                {#if p.isReachable}
                  <div class="doh-bar-wrap">
                    <div
                      class="doh-bar-fill"
                      style="width:{latencyBar(p.latencyMs)}%;background:{latencyColor(p.latencyMs)};"
                    ></div>
                  </div>
                  <span class="doh-lat" style="color:{latencyColor(p.latencyMs)};">{p.latencyMs}ms</span>
                {:else}
                  <span class="doh-unreachable">Ulaşılamıyor</span>
                {/if}
              </div>
            {/each}
          </div>

          {#if dohResult.censorshipTamperDetected}
            <div class="doh-censor-alert">
              <Icon name="warning" size={14} />
              <span>
                <strong>Uyarı:</strong> 3'ten az DoH sağlayıcısına ulaşılabildi. Ağınızda DNS sansürü veya manipülasyon tespit edildi. Tor veya WireGuard tüneli kullanmanız önerilir.
              </span>
            </div>
          {/if}
        {/if}
      </div>
    </div>

    <!-- ── VARLIK (PRESENCE) ─────────────────────────────────────────────── -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Varlık (Presence)</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Çevrimiçi durumunu kim görebilir?</div>
          <div class="veil-settings-row-desc">Varlık bilgisi meta veridir — mesaj şifrelemesini etkilemez.</div>
        </div>
        <VeilSelect
          options={[
            { value: 'everyone', label: 'Herkes' },
            { value: 'contacts_only', label: 'Yalnızca arkadaşlar' },
            { value: 'nobody', label: 'Kimse' },
          ]}
          value={settings.presenceVisibility}
          onChange={(v) => save({ presenceVisibility: v as PresenceVisibility })}
        />
      </div>
    </div>

    <!-- ── MESAJLAR & ETKİLEŞİM ───────────────────────────────────────────── -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Mesajlar & Etkileşim</div>
      
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Direkt Mesaj Gizliliği</div>
          <div class="veil-settings-row-desc">Kimler size DM atabilir?</div>
        </div>
        <VeilSelect
          options={[
            { value: 'everyone', label: 'Herkes' },
            { value: 'friends', label: 'Yalnızca Arkadaşlar' },
            { value: 'same_server', label: 'Aynı Sunucudakiler' },
            { value: 'nobody', label: 'Kimse' },
          ]}
          value={$authStore.dmPrivacy || 'everyone'}
          onChange={(v) => authStore.setDmPrivacy(v as any)}
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Okundu bildirimleri</div>
          <div class="veil-settings-row-desc">Karşı taraf mesajını okuduğunda bilgi gönder.</div>
        </div>
        <Toggle checked={settings.showReadReceipts} onChange={(v) => save({ showReadReceipts: v })} label="Okundu bildirimleri" />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Yazıyor göstergesi</div>
          <div class="veil-settings-row-desc">Yazarken karşı tarafa durumunu göster.</div>
        </div>
        <Toggle checked={settings.showTypingIndicator} onChange={(v) => save({ showTypingIndicator: v })} label="Yazıyor göstergesi" />
      </div>
    </div>

    <!-- ── MEDYA & BAĞLANTILAR ────────────────────────────────────────────── -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Medya & Bağlantılar</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Medyayı otomatik indir</div>
          <div class="veil-settings-row-desc">Kanal açıldığında ekleri otomatik indir.</div>
        </div>
        <Toggle checked={settings.autoDownloadMedia} onChange={(v) => save({ autoDownloadMedia: v })} label="Medyayı otomatik indir" />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Bağlantı önizlemeleri</div>
          <div class="veil-settings-row-desc">Mesajlardaki bağlantılar için güvenli SSRF korumalı önizleme getir.</div>
        </div>
        <Toggle checked={settings.linkPreviews} onChange={(v) => save({ linkPreviews: v })} label="Bağlantı önizlemeleri" />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Bildirim önizlemesi</div>
          <div class="veil-settings-row-desc">Bildirimlerde mesaj içeriği nasıl gösterilsin?</div>
        </div>
        <VeilSelect
          options={[
            { value: 'full', label: 'Tam içerik' },
            { value: 'sender', label: 'Yalnızca gönderen' },
            { value: 'none', label: 'Gizli' },
          ]}
          value={settings.notificationPreview}
          onChange={(v) => save({ notificationPreview: v as NotificationPreview })}
        />
      </div>
    </div>

    <!-- ── YEREL YAPAY ZEKÂ (OLLAMA) ────────────────────────────────────── -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Yerel Yapay Zekâ (Ollama)</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Yerel AI etkin</div>
          <div class="veil-settings-row-desc">
            Mesajları özetleme, otomatik yanıt önerisi vb. için cihazındaki Ollama'yı kullan.
            Verilerin hiçbir harici sunucuya iletilmez, tamamen yerel makinede çalışır.
          </div>
        </div>
        <Toggle
          checked={settings.localAiEnabled}
          onChange={(v) => save({ localAiEnabled: v })}
          label="Yerel AI etkin"
        />
      </div>
      {#if settings.localAiEnabled}
        <div class="veil-settings-row">
          <div class="veil-settings-row-info">
            <div class="veil-settings-row-label">Ollama durumu</div>
            <div class="veil-settings-row-desc">
              {#if aiChecking}
                Kontrol ediliyor…
              {:else if aiAvailable}
                <span class="veil-ai-ok">
                  <span class="veil-ai-dot" aria-hidden="true"></span>
                  Bağlandı ({aiModel ?? 'varsayılan model'})
                </span>
              {:else}
                <span class="veil-ai-off">Ollama çalışmıyor (localhost:11434)</span>
              {/if}
            </div>
            <div class="veil-settings-row-desc">
              ollama.com'dan kur, sonra `ollama pull llama3.2` ile bir model indir.
            </div>
          </div>
          <div style="display:flex;gap:8px;align-items:center;">
          {#if aiAvailable && aiModels.length > 1}
            <VeilSelect options={aiModels.map(m => ({ value: m, label: m }))} value={aiModel ?? aiModels[0]} onChange={(v) => { aiModel = v; }} />
          {/if}
          {#if aiAvailable}
            <button class="btn btn-secondary btn-sm" onclick={testAi} disabled={aiTesting}>
              {aiTesting ? 'Test ediliyor…' : 'Test et'}
            </button>
          {:else}
            <button class="btn btn-secondary btn-sm" onclick={checkAi} disabled={aiChecking}>
              Yeniden kontrol et
            </button>
          {/if}
          </div>
        </div>
        {#if aiTestResult}
          <p class="veil-ai-reply">{aiTestResult}</p>
        {/if}
        {#if aiAvailable && aiModels.length > 0}
          <div class="veil-settings-row-desc" style="margin-top:4px;">Algılanan modeller: {aiModels.join(', ')}</div>
        {/if}
      {/if}
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Discord köprüsü</div>
          <div class="veil-settings-row-desc">
            Köprüden gelen mesajlar "bridged" olarak etiketlenir ve veilanon içinde uçtan uca şifreli DEĞİLDİR.
          </div>
        </div>
        <Toggle
          checked={settings.discordBridgeEnabled}
          label="Discord köprüsü"
          onChange={async (v) => {
            if (v) {
              const ok = await uiStore.confirm(
                'Discord köprüsü, köprü üzerinden gelen mesajların veilanon E2EE korumasına SAHİP OLMAYACAĞI anlamına gelir. Devam etmek istiyor musun?',
                { title: 'Discord Köprüsü', confirmLabel: 'Etkinleştir', danger: true }
              );
              if (!ok) return;
            }
            save({ discordBridgeEnabled: v });
          }}
        />
      </div>
    </div>
  {/if}
</section>

<style>
  .shield-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .shield-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .privacy-shield-box {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    padding: var(--space-4);
  }

  .shield-sub-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-3);
    padding-left: var(--space-3);
    border-left: 2px solid var(--veil-border-subtle);
  }

  .shield-preview-card {
    margin-top: var(--space-3);
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .preview-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preview-title {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-secondary);
  }

  .preview-hint {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .preview-secret-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    background: var(--veil-bg-surface);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--veil-border-subtle);
  }

  .preview-secret-value {
    font-size: var(--text-sm);
    color: var(--veil-text-primary);
    user-select: none;
  }

  .shield-disabled-notice {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: rgba(255, 255, 255, 0.03);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .screen-share-shield-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    background: color-mix(in srgb, var(--veil-brand) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--veil-brand) 30%, transparent);
    border-radius: var(--radius-full);
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-brand);
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    background: currentColor;
    box-shadow: 0 0 8px currentColor;
    display: inline-block;
  }

  /* ── Network Privacy & Tor ───────────────────────────────────── */
  .net-privacy-box {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
  }

  .net-privacy-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
  }

  .net-privacy-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-pill-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .preset-section {
    margin: var(--space-3) 0;
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border-radius: var(--radius-lg);
    border: 1px solid var(--veil-border-subtle);
  }

  .preset-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }

  .preset-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-secondary);
  }

  .preset-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: var(--space-2);
  }

  .preset-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: all 0.2s ease;
    position: relative;
    user-select: none;
  }

  .preset-card:hover {
    border-color: var(--veil-border);
    background: color-mix(in srgb, var(--veil-bg-surface) 80%, white 5%);
    transform: translateY(-1px);
  }

  .preset-card-active {
    border-color: var(--veil-brand) !important;
    background: color-mix(in srgb, var(--veil-brand) 12%, var(--veil-bg-surface)) !important;
  }

  .preset-card-icon {
    font-size: 20px;
    flex-shrink: 0;
  }

  .preset-card-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .preset-card-title {
    font-size: var(--text-xs);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .preset-card-desc {
    font-size: 11px;
    color: var(--veil-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preset-active-check {
    position: absolute;
    top: 8px;
    right: 8px;
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .sys-detect-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, var(--veil-brand) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--veil-brand) 20%, transparent);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
  }

  .proxy-config-grid {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-3);
    margin: var(--space-2) 0;
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border-radius: var(--radius-lg);
    border: 1px solid var(--veil-border-subtle);
  }

  .proxy-input-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .proxy-input-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-muted);
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
  }

  .proxy-test-card {
    margin-top: var(--space-3);
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .proxy-test-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .proxy-test-title-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .proxy-test-hint {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .proxy-test-result-box {
    padding: var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--veil-border-subtle);
    background: var(--veil-bg-surface);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    animation: fadeIn 0.25s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .proxy-test-result-box.proxy-success {
    border-color: color-mix(in srgb, var(--veil-success) 40%, transparent);
    background: color-mix(in srgb, var(--veil-success) 5%, var(--veil-bg-surface));
  }

  .proxy-test-result-box.proxy-failed {
    border-color: color-mix(in srgb, var(--veil-danger) 40%, transparent);
    background: color-mix(in srgb, var(--veil-danger) 5%, var(--veil-bg-surface));
  }

  .proxy-test-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--space-2);
  }

  .proxy-test-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
    background: var(--veil-bg-void);
    border-radius: var(--radius-sm);
    border: 1px solid var(--veil-border-subtle);
  }

  .proxy-item-key {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }

  .proxy-item-val {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .proxy-err-msg {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, var(--veil-danger) 12%, transparent);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
    color: var(--veil-danger);
  }

  .wireguard-card {
    margin-top: var(--space-3);
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .wireguard-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .wireguard-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    line-height: var(--leading-relaxed);
  }

  .wireguard-input-wrap {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .wireguard-textarea {
    width: 100%;
    resize: vertical;
    min-height: 100px;
    font-size: var(--text-xs);
    line-height: 1.5;
  }

  .wg-valid-box {
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
  }

  .wg-valid-box.wg-ok {
    border-color: color-mix(in srgb, var(--veil-success) 35%, transparent);
  }

  .wg-valid-box.wg-err {
    border-color: color-mix(in srgb, var(--veil-danger) 35%, transparent);
  }

  .wg-valid-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: var(--space-2);
  }

  .wg-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
    background: var(--veil-bg-void);
    border-radius: var(--radius-sm);
  }

  .wg-key {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--veil-text-muted);
  }

  .wg-val {
    font-size: var(--text-xs);
    color: var(--veil-text-primary);
  }

  .wg-err-msg {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--veil-danger);
    font-size: var(--text-xs);
  }

  /* ── Network Diagnostics ─────────────────────────────────────── */
  .net-diag-group {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
  }

  .net-diag-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .net-diag-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .net-label {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }

  .net-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 9px;
    border-radius: var(--radius-full);
    font-size: 11px;
    font-weight: 600;
  }

  .net-badge-ok {
    background: color-mix(in srgb, var(--veil-success) 15%, transparent);
    color: var(--veil-success);
    border: 1px solid color-mix(in srgb, var(--veil-success) 30%, transparent);
  }

  .net-badge-warn {
    background: color-mix(in srgb, var(--veil-warning) 15%, transparent);
    color: var(--veil-warning);
    border: 1px solid color-mix(in srgb, var(--veil-warning) 30%, transparent);
  }

  .net-badge-err {
    background: color-mix(in srgb, var(--veil-danger) 15%, transparent);
    color: var(--veil-danger);
    border: 1px solid color-mix(in srgb, var(--veil-danger) 30%, transparent);
  }

  .net-badge-neutral {
    background: rgba(255, 255, 255, 0.05);
    color: var(--veil-text-muted);
    border: 1px solid var(--veil-border-subtle);
  }

  .net-asn-card {
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .net-asn-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .net-asn-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: var(--space-2);
  }

  .net-asn-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
    background: var(--veil-bg-surface);
    border-radius: var(--radius-sm);
    border: 1px solid var(--veil-border-subtle);
  }

  .net-asn-key {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }

  .net-asn-val {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .net-asn-skeleton {
    height: 52px;
    background: linear-gradient(90deg, var(--veil-bg-surface) 25%, var(--veil-bg-elevated) 50%, var(--veil-bg-surface) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
    border-radius: var(--radius-md);
  }

  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .doh-bench-card {
    margin-top: var(--space-3);
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .doh-bench-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .doh-loading {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    padding: var(--space-2) 0;
  }

  .doh-summary {
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    margin: 0;
  }

  .doh-providers {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .doh-provider-row {
    display: grid;
    grid-template-columns: 120px 1fr 54px;
    align-items: center;
    gap: var(--space-2);
    padding: 3px 0;
  }

  .doh-provider-info {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .doh-dot {
    width: 7px;
    height: 7px;
    border-radius: var(--radius-full);
    flex-shrink: 0;
  }

  .doh-name {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .doh-bar-wrap {
    height: 6px;
    background: var(--veil-bg-surface);
    border-radius: var(--radius-full);
    overflow: hidden;
    border: 1px solid var(--veil-border-subtle);
  }

  .doh-bar-fill {
    height: 100%;
    border-radius: var(--radius-full);
    transition: width 0.6s ease;
  }

  .doh-lat {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    font-weight: 600;
    text-align: right;
  }

  .doh-unreachable {
    font-size: var(--text-xs);
    color: var(--veil-danger);
    grid-column: 2 / 4;
    font-style: italic;
  }

  .doh-censor-alert {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, var(--veil-warning) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--veil-warning) 30%, transparent);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    color: var(--veil-warning);
    line-height: var(--leading-relaxed);
  }

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

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .btn-xs {
    padding: 3px 9px;
    font-size: 11px;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-radius: var(--radius-md);
  }

  .veil-ai-ok { display: inline-flex; align-items: center; gap: var(--space-1); color: var(--veil-success); }
  .veil-ai-off { color: var(--veil-warning); }
  .veil-ai-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-success);
    display: inline-block;
  }
  .veil-ai-reply {
    margin-top: var(--space-2);
    padding: var(--space-3);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    font-size: var(--text-sm);
    color: var(--veil-text-secondary);
    line-height: var(--leading-relaxed);
    user-select: text;
  }
</style>

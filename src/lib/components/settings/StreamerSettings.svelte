<script lang="ts">
  import {
    streamerMode,
    type MaskStyle,
    type StreamerPreset,
    maskEmail,
    maskUserId,
    maskInviteLink,
    maskText,
  } from '$lib/stores/streamerMode';
  import Icon, { type IconName } from '$lib/components/ui/Icon.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';

  const presets: { id: StreamerPreset; title: string; desc: string; icon: IconName }[] = [
    {
      id: 'max_privacy',
      title: 'Maksimum Gizlilik',
      desc: 'Tüm kişisel veriler, DM içerikleri, ses katılımcıları ve medya ekleri tam bulanıklaştırılır.',
      icon: 'shield',
    },
    {
      id: 'streamer_balanced',
      title: 'Dengeli Yayıncı',
      desc: 'Yayın için ideal ayarlar: E-posta, davetler, tokenlar ve DM sansürlenir, ses avatarları görünür kalır.',
      icon: 'sparkle',
    },
    {
      id: 'minimal',
      title: 'Minimal Koruma',
      desc: 'Yalnızca kritik e-posta ve sunucu davet linklerini gizler, diğer arayüz öğeleri normal kalır.',
      icon: 'lock',
    },
    {
      id: 'custom',
      title: 'Özel Yapılandırma',
      desc: 'Sansür stillerini ve gizlenecek öğeleri tamamen kendi isteğinize göre özelleştirin.',
      icon: 'settings',
    },
  ];

  const maskStyles: { id: MaskStyle; label: string; preview: string }[] = [
    { id: 'asterisks', label: 'Yıldızlı', preview: '**********' },
    { id: 'bullets', label: 'Noktalı', preview: '••••••••••' },
    { id: 'blur', label: 'Bulanıklaştırma', preview: 'Bulanık Alan' },
    { id: 'hidden', label: 'Gizli Etiket', preview: '[GİZLENDİ]' },
  ];
</script>

<div class="veil-streamer-pane">
  <!-- ── Master Toggle Card ───────────────────────────────────── -->
  <div class="veil-streamer-master" class:enabled={$streamerMode.enabled}>
    <div class="veil-streamer-master-left">
      <div class="veil-streamer-master-icon" class:enabled={$streamerMode.enabled}>
        <Icon name="shield" size={22} />
      </div>
      <div class="veil-streamer-master-info">
        <div class="veil-streamer-master-title">Yayıncı Modu</div>
        <div class="veil-streamer-master-sub">
          Canlı yayın ve ekran paylaşımı sırasında kişisel bilgileri otomatik sansürler.
        </div>
      </div>
    </div>
    <Toggle
      checked={$streamerMode.enabled}
      onChange={(v) => streamerMode.setEnabled(v)}
      label="Yayıncı Modu"
    />
  </div>

  <!-- ── Auto-Enable Row ───────────────────────────────────────── -->
  <div class="veil-settings-row">
    <div class="veil-settings-row-info">
      <div class="veil-settings-row-label">
        <Icon name="monitor" size={15} />
        <span>Ekran Paylaşımında Otomatik Başlat</span>
      </div>
      <div class="veil-settings-row-desc">
        Ekran paylaşımına başladığında Yayıncı Modu kendiliğinden aktif olur.
      </div>
    </div>
    <Toggle
      checked={$streamerMode.autoEnableOnScreenShare}
      onChange={(v) => streamerMode.updateSetting('autoEnableOnScreenShare', v)}
      label="Ekran Paylaşımında Otomatik Başlat"
    />
  </div>

  <!-- ── Auto-Disable Row ──────────────────────────────────────── -->
  <div class="veil-settings-row">
    <div class="veil-settings-row-info">
      <div class="veil-settings-row-label">
        <Icon name="screen" size={15} />
        <span>Ekran Paylaşımı Bitince Otomatik Kapat</span>
      </div>
      <div class="veil-settings-row-desc">
        Ekran paylaşımı sona erdiğinde Yayıncı Modu kendiliğinden kapanır.
      </div>
    </div>
    <Toggle
      checked={$streamerMode.autoDisableOnScreenShareEnd}
      onChange={(v) => streamerMode.updateSetting('autoDisableOnScreenShareEnd', v)}
      label="Ekran Paylaşımı Bitince Otomatik Kapat"
    />
  </div>

  <!-- ── Privacy Profile Presets ──────────────────────────────── -->
  <div class="veil-streamer-section">
    <div class="veil-section-header">
      <span class="veil-section-title">Gizlilik Profili</span>
      <span class="veil-section-hint">Hızlı koruma şablonu seçin</span>
    </div>
    <div class="veil-presets-grid">
      {#each presets as p (p.id)}
        {@const isSelected = $streamerMode.preset === p.id}
        <button
          type="button"
          class="veil-preset-card"
          class:selected={isSelected}
          onclick={() => streamerMode.setPreset(p.id)}
        >
          <div class="veil-preset-head">
            <span class="veil-preset-icon" class:selected={isSelected}>
              <Icon name={p.icon} size={15} />
            </span>
            <span class="veil-preset-title">{p.title}</span>
            {#if isSelected}
              <span class="veil-preset-check"><Icon name="check" size={13} /></span>
            {/if}
          </div>
          <p class="veil-preset-desc">{p.desc}</p>
        </button>
      {/each}
    </div>
  </div>

  <!-- ── Masking Style ─────────────────────────────────────────── -->
  <div class="veil-streamer-section">
    <div class="veil-section-header">
      <span class="veil-section-title">Sansür Stili</span>
      <span class="veil-section-hint">Gizlenen verinin görünüm biçimi</span>
    </div>
    <div class="veil-mask-pills">
      {#each maskStyles as style (style.id)}
        {@const isActive = $streamerMode.maskStyle === style.id}
        <button
          type="button"
          class="veil-mask-pill"
          class:active={isActive}
          onclick={() => streamerMode.setMaskStyle(style.id)}
        >
          <span class="veil-mask-pill-label">{style.label}</span>
          <span class="veil-mask-pill-preview style-{style.id}">{style.preview}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- ── Live Simulation ───────────────────────────────────────── -->
  <div class="veil-sim-card">
    <div class="veil-sim-header">
      <div class="veil-sim-title">
        <Icon name="eye" size={14} />
        <span>Canlı Önizleme</span>
      </div>
      <span class="veil-sim-badge" class:live={$streamerMode.enabled}>
        <span class="veil-sim-dot"></span>
        {$streamerMode.enabled ? 'Sansür Aktif' : 'Normal Görünüm'}
      </span>
    </div>

    <div class="veil-sim-grid">
      <div class="veil-sim-row">
        <span class="veil-sim-key">E-Posta</span>
        <span class="veil-sim-val" class:blurred={$streamerMode.enabled && $streamerMode.hideAccountDetails && $streamerMode.maskStyle === 'blur'}>
          {maskEmail('yayinci@veilanon.com')}
        </span>
      </div>
      <div class="veil-sim-row">
        <span class="veil-sim-key">Cihaz ID</span>
        <span class="veil-sim-val veil-mono" class:blurred={$streamerMode.enabled && $streamerMode.hideUserIds && $streamerMode.maskStyle === 'blur'}>
          {maskUserId('4b92-91fa-8a12')}
        </span>
      </div>
      <div class="veil-sim-row">
        <span class="veil-sim-key">Davet Linki</span>
        <span class="veil-sim-val veil-brand-text" class:blurred={$streamerMode.enabled && $streamerMode.hideInviteLinks && $streamerMode.maskStyle === 'blur'}>
          {maskInviteLink('veilanon://join/vip-lounge')}
        </span>
      </div>
      <div class="veil-sim-row">
        <span class="veil-sim-key">DM Mesajı</span>
        <span class="veil-sim-val" class:blurred={$streamerMode.enabled && $streamerMode.hideDmContent && $streamerMode.maskStyle === 'blur'}>
          {$streamerMode.enabled && $streamerMode.hideDmContent ? maskText('Gizli: 8941-secret') : 'Gizli: 8941-secret'}
        </span>
      </div>
    </div>
  </div>

  <!-- ── Granular Toggles ──────────────────────────────────────── -->
  <div class="veil-streamer-section">
    <div class="veil-section-header">
      <span class="veil-section-title">Ayrıntılı Sansür Seçenekleri</span>
    </div>
    <div class="veil-toggles-list">
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Hesap & Güvenlik Bilgilerini Gizle</div>
          <div class="veil-settings-row-desc">E-posta, kurtarma kelimeleri ve hesap tokenları.</div>
        </div>
        <Toggle
          checked={$streamerMode.hideAccountDetails}
          onChange={(v) => streamerMode.updateSetting('hideAccountDetails', v)}
          label="Hesap bilgilerini gizle"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Kullanıcı & Cihaz Kimliklerini Gizle</div>
          <div class="veil-settings-row-desc">Benzersiz kullanıcı ID'leri, parmak izleri ve UUID'ler.</div>
        </div>
        <Toggle
          checked={$streamerMode.hideUserIds}
          onChange={(v) => streamerMode.updateSetting('hideUserIds', v)}
          label="Cihaz kimliklerini gizle"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Davet Bağlantılarını Gizle</div>
          <div class="veil-settings-row-desc">Topluluk davet linkleri ve özel vanity URL'ler.</div>
        </div>
        <Toggle
          checked={$streamerMode.hideInviteLinks}
          onChange={(v) => streamerMode.updateSetting('hideInviteLinks', v)}
          label="Davet bağlantılarını gizle"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">DM İçeriklerini Sansürle</div>
          <div class="veil-settings-row-desc">Direkt mesaj metinleri ve kişi listesi önizlemeleri.</div>
        </div>
        <Toggle
          checked={$streamerMode.hideDmContent}
          onChange={(v) => streamerMode.updateSetting('hideDmContent', v)}
          label="DM içeriklerini sansürle"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Ses Kanalı Katılımcılarını Gizle</div>
          <div class="veil-settings-row-desc">Aktif ses odasındaki kullanıcı isimleri ve avatarları.</div>
        </div>
        <Toggle
          checked={$streamerMode.hideVoiceParticipants}
          onChange={(v) => streamerMode.updateSetting('hideVoiceParticipants', v)}
          label="Ses katılımcılarını gizle"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Açılır Bildirimleri Sustur</div>
          <div class="veil-settings-row-desc">Yeni mesaj ve arkadaşlık bildirimleri.</div>
        </div>
        <Toggle
          checked={$streamerMode.suppressNotificationPopups}
          onChange={(v) => streamerMode.updateSetting('suppressNotificationPopups', v)}
          label="Açılır bildirimleri sustur"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Bildirim Ses Efektlerini Sustur</div>
          <div class="veil-settings-row-desc">Yayın akışını bozabilecek mesaj çan ve uyarı sesleri.</div>
        </div>
        <Toggle
          checked={$streamerMode.suppressAudioAlerts}
          onChange={(v) => streamerMode.updateSetting('suppressAudioAlerts', v)}
          label="Bildirim seslerini sustur"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Medya & Ekleri Bulanıklaştır</div>
          <div class="veil-settings-row-desc">Resim ve video ekleri hover'a kadar sansürlü kalır.</div>
        </div>
        <Toggle
          checked={$streamerMode.blurMediaAttachments}
          onChange={(v) => streamerMode.updateSetting('blurMediaAttachments', v)}
          label="Medyaları bulanıklaştır"
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Sistem & Tanılama Log Yollarını Gizle</div>
          <div class="veil-settings-row-desc">Hata günlükleri, yerel dosya yolları ve IP adresleri.</div>
        </div>
        <Toggle
          checked={$streamerMode.hideSystemDiagnostics}
          onChange={(v) => streamerMode.updateSetting('hideSystemDiagnostics', v)}
          label="Tanılama loglarını gizle"
        />
      </div>
    </div>
  </div>

  <div class="veil-reset-footer">
    <button type="button" class="btn btn-ghost btn-sm" onclick={() => streamerMode.resetToDefaults()}>
      <Icon name="arrow-left" size={13} />
      <span>Varsayılana Sıfırla</span>
    </button>
  </div>
</div>

<style>
  .veil-streamer-pane {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding-bottom: var(--space-4);
  }

  /* ── Master Toggle Card ─────────────────────────────────────── */
  .veil-streamer-master {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    transition: border-color var(--t-fast), box-shadow var(--t-fast);
  }

  .veil-streamer-master.enabled {
    border-color: var(--veil-brand);
    background: color-mix(in srgb, var(--veil-brand) 5%, var(--veil-bg-elevated));
    box-shadow: 0 0 24px color-mix(in srgb, var(--veil-brand) 10%, transparent);
  }

  .veil-streamer-master-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
  }

  .veil-streamer-master-icon {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-lg);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    color: var(--veil-text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: all var(--t-fast);
  }

  .veil-streamer-master-icon.enabled {
    background: var(--veil-brand-subtle);
    border-color: color-mix(in srgb, var(--veil-brand) 30%, transparent);
    color: var(--veil-brand);
  }

  .veil-streamer-master-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .veil-streamer-master-title {
    font-size: var(--text-base);
    font-weight: 700;
    color: var(--veil-text-primary);
    letter-spacing: var(--tracking-tight);
  }

  .veil-streamer-master-sub {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    line-height: var(--leading-relaxed);
  }

  /* ── Section Block ──────────────────────────────────────────── */
  .veil-streamer-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-section-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .veil-section-title {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--veil-text-muted);
  }

  .veil-section-hint {
    font-size: 11px;
    color: var(--veil-text-disabled);
  }

  /* ── Preset Cards ───────────────────────────────────────────── */
  .veil-presets-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-2);
  }

  @media (max-width: 500px) {
    .veil-presets-grid { grid-template-columns: 1fr; }
  }

  .veil-preset-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-3);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    text-align: left;
    cursor: pointer;
    transition: border-color var(--t-fast), background var(--t-fast);
  }

  .veil-preset-card:hover {
    border-color: var(--veil-border-focus);
    background: var(--veil-bg-surface);
  }

  .veil-preset-card.selected {
    border-color: var(--veil-brand);
    background: var(--veil-brand-subtle);
  }

  .veil-preset-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .veil-preset-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    background: var(--veil-bg-surface);
    color: var(--veil-text-muted);
    flex-shrink: 0;
    transition: all var(--t-fast);
  }

  .veil-preset-icon.selected {
    background: color-mix(in srgb, var(--veil-brand) 15%, transparent);
    color: var(--veil-brand);
  }

  .veil-preset-title {
    font-size: var(--text-sm);
    font-weight: 700;
    color: var(--veil-text-primary);
    flex: 1;
    min-width: 0;
  }

  .veil-preset-check {
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .veil-preset-desc {
    font-size: 11px;
    color: var(--veil-text-muted);
    margin: 0;
    line-height: var(--leading-relaxed);
  }

  /* ── Mask Style Pills ───────────────────────────────────────── */
  .veil-mask-pills {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .veil-mask-pill {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: var(--space-2) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-full);
    cursor: pointer;
    transition: all var(--t-fast);
    flex: 1;
    min-width: 80px;
  }

  .veil-mask-pill:hover {
    border-color: var(--veil-border-focus);
    background: var(--veil-bg-surface);
  }

  .veil-mask-pill.active {
    border-color: var(--veil-brand);
    background: var(--veil-brand-subtle);
  }

  .veil-mask-pill-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .veil-mask-pill-preview {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--veil-text-muted);
    letter-spacing: 0.02em;
  }

  .style-blur {
    filter: blur(3px);
    user-select: none;
  }

  /* ── Live Simulation Box ────────────────────────────────────── */
  .veil-sim-card {
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-sim-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--veil-border-subtle);
  }

  .veil-sim-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--veil-text-muted);
  }

  .veil-sim-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 10px;
    border-radius: var(--radius-full);
    font-size: 11px;
    font-weight: 700;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    color: var(--veil-text-muted);
    letter-spacing: 0.02em;
  }

  .veil-sim-badge.live {
    background: color-mix(in srgb, var(--veil-success) 10%, transparent);
    color: var(--veil-success);
    border-color: color-mix(in srgb, var(--veil-success) 22%, transparent);
  }

  .veil-sim-dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    background: currentColor;
  }

  .veil-sim-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--space-2);
  }

  .veil-sim-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
  }

  .veil-sim-key {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--veil-text-muted);
  }

  .veil-sim-val {
    font-size: var(--text-xs);
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: filter var(--t-fast);
  }

  .veil-sim-val.blurred {
    filter: blur(4px);
    user-select: none;
  }

  .veil-brand-text { color: var(--veil-brand); }
  .veil-mono { font-family: var(--font-mono); font-size: 11px; }

  /* ── Toggles List ───────────────────────────────────────────── */
  .veil-toggles-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* ── Reset Footer ───────────────────────────────────────────── */
  .veil-reset-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--veil-border-subtle);
  }
</style>

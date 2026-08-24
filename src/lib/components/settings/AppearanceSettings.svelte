<script lang="ts">
  import { onMount } from 'svelte';
  import Toggle from '../ui/Toggle.svelte';
  import VeilSelect from '../ui/VeilSelect.svelte';
  import Icon, { type IconName } from '$lib/components/ui/Icon.svelte';
  import { uiStore, type Theme } from '$lib/stores/ui';
  import { settingsApi, type AppSettings } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';

  const ThemeGalleryPromise = import('./ThemeGallery.svelte');
  const ThemeStudioPromise = import('./ThemeStudio.svelte');

  import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart';

  const ui = $derived($uiStore);

  const ACCENT_COLORS = [
    '#7c3aed', '#8b5cf6', '#a855f7', '#6366f1', '#3b82f6',
    '#06b6d4', '#10b981', '#22c55e', '#f59e0b', '#f97316',
    '#ef4444', '#ec4899',
  ] as const;
  const DEFAULT_ACCENT = '#7c3aed';

  let settings = $state<AppSettings | null>(null);
  let fontSize = $state(14);
  let reduceMotion = $state(false);
  let compactMode = $state(false);
  let accentColor = $state<string | null>(null);
  const activeAccent = $derived(accentColor ?? DEFAULT_ACCENT);
  let customHex = $state('');
  let hexError = $state<string | null>(null);
  let amoledMode = $state(false);
  let autostart = $state(false);

  onMount(async () => {
    try {
      settings = await settingsApi.get();
      fontSize = settings.fontSize ?? 14;
      reduceMotion = settings.reduceMotion ?? false;
      compactMode = settings.compactMode ?? ui.compactMode;
      accentColor = settings.accentColor ?? null;
      if (accentColor) uiStore.setAccentColor(accentColor);
      amoledMode = settings.amoledMode ?? (localStorage.getItem('veilanon-amoled') === 'true');
      uiStore.setAmoledMode(amoledMode);
      if ('__TAURI_INTERNALS__' in window) {
        try {
          autostart = await isAutostartEnabled();
        } catch {
          autostart = false;
        }
      }
    } catch {
      toastStore.error('Görünüm ayarları yüklenemedi.');
    }
  });

  $effect(() => {
    const preset = ui.presetThemeId;
    const currentTheme = ui.theme;
    const accentInStorage = typeof window !== 'undefined' ? localStorage.getItem('veilanon-accent') : null;
    if (!accentInStorage && accentColor !== null) {
      accentColor = null;
      customHex = '';
      hexError = null;
    }
    void preset;
    void currentTheme;
  });

  function setAmoled(v: boolean) {
    amoledMode = v;
    uiStore.setAmoledMode(v);
    save({ amoledMode: v });
    toastStore.success(v ? 'AMOLED Saf Siyah Modu açıldı.' : 'Standart Koyu Mod açıldı.');
  }

  async function setAutostart(v: boolean) {
    autostart = v;
    try {
      if (v) {
        await enableAutostart();
        toastStore.success('Bilgisayar başladığında otomatik başlatma açıldı.');
      } else {
        await disableAutostart();
        toastStore.info('Otomatik başlatma kapatıldı.');
      }
    } catch (err) {
      toastStore.error('Başlangıç ayarı uygulanamadı.');
    }
  }

  function setTheme(theme: Theme) {
    uiStore.setTheme(theme);
    save({ theme });
  }

  async function save(patch: Partial<AppSettings>) {
    const previous = settings;
    const next = { ...(settings ?? {}), ...patch } as AppSettings;
    settings = next;
    try {
      settings = await settingsApi.update(next);
    } catch {
      settings = previous;
      toastStore.error('Ayarlar kaydedilemedi.');
    }
  }

  function setFontSize(size: number) {
    fontSize = size;
    document.documentElement.style.fontSize = `${size}px`;
    save({ fontSize: size });
  }

  function setReduceMotion(v: boolean) {
    reduceMotion = v;
    document.documentElement.setAttribute('data-reduce-motion', v ? 'true' : 'false');
    save({ reduceMotion: v });
  }

  function setCompact(v: boolean) {
    compactMode = v;
    uiStore.setCompactMode(v);
    save({ compactMode: v });
  }

  function setAccent(color: string) {
    accentColor = color;
    customHex = color.startsWith('#') ? color : `#${color}`;
    hexError = null;
    uiStore.setAccentColor(color);
    save({ accentColor: color });
  }

  function applyCustomHex() {
    let hex = customHex.trim();
    if (!hex) return;
    if (!hex.startsWith('#')) hex = `#${hex}`;
    if (!/^#[0-9a-fA-F]{6}$/.test(hex) && !/^#[0-9a-fA-F]{3}$/.test(hex)) {
      hexError = 'Geçerli bir renk kodu gir (örn. #7c3aed).';
      return;
    }
    if (/^#[0-9a-fA-F]{3}$/.test(hex)) {
      const [r, g, b] = hex.slice(1).split('').map(c => c + c);
      hex = `#${r}${g}${b}`;
    }
    setAccent(hex.toLowerCase());
    toastStore.success('Vurgu rengi güncellendi.');
  }

  const themes: Array<{ id: Theme; label: string; icon: IconName }> = [
    { id: 'dark', label: 'Koyu', icon: 'moon' },
    { id: 'light', label: 'Açık', icon: 'sun' },
    { id: 'system', label: 'Sistem', icon: 'monitor' },
  ];
</script>

<section aria-labelledby="gorunum-title">
  <h2 class="veil-settings-title" id="gorunum-title">Görünüm</h2>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Tema</div>
    <div class="veil-theme-grid" role="radiogroup" aria-label="Tema">
      {#each themes as t (t.id)}
        <button
          class="veil-theme-card"
          class:active={ui.theme === t.id}
          role="radio"
          aria-checked={ui.theme === t.id}
          onclick={() => setTheme(t.id)}
        >
          <span class="veil-theme-preview veil-theme-preview-{t.id}" aria-hidden="true">
            <span class="veil-theme-preview-bar"></span>
            <span class="veil-theme-preview-dot"></span>
            <span class="veil-theme-preview-line"></span>
            <span class="veil-theme-preview-line short"></span>
            <span class="veil-theme-preview-line tiny"></span>
          </span>
          <span class="veil-theme-card-label">
            <Icon name={t.icon} size={14} />
            {t.label}
          </span>
          {#if ui.theme === t.id}
            <span class="veil-theme-check" aria-hidden="true"><Icon name="check" size={10} /></span>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Hazır Premium Temalar (25 Farklı Koleksiyon)</div>
    <p class="veil-settings-row-desc" style="margin-bottom: var(--space-3);">
      Mükemmel renk dengesi, yüksek kontrast ve WCAG standartlarına tam uyumlu 25 özel hazır tema.
    </p>
    {#await ThemeGalleryPromise then { default: ThemeGallery }}
      <ThemeGallery />
    {:catch}
      <div class="veil-home-loading"><div class="veil-spinner"></div></div>
    {/await}
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Vurgu Rengi (Accent)</div>
    <div class="veil-swatch-grid" role="radiogroup" aria-label="Vurgu rengi">
      {#each ACCENT_COLORS as color (color)}
        <button
          class="veil-swatch"
          class:active={activeAccent === color}
          style={`--swatch: ${color}`}
          role="radio"
          aria-checked={activeAccent === color}
          aria-label={`Vurgu rengi ${color}`}
          title={color}
          onclick={() => setAccent(color)}
        >
          <span class="veil-swatch-inner" aria-hidden="true"></span>
          {#if activeAccent === color}
            <Icon name="check" size={14} />
          {/if}
        </button>
      {/each}
    </div>

    <div class="veil-custom-color">
      <label class="veil-form-label" for="custom-accent">Özel renk</label>
      <div class="veil-custom-color-row">
        <input
          id="custom-accent-native"
          class="veil-color-native"
          type="color"
          value={activeAccent}
          aria-label="Renk seçici"
          title="Paletten seç"
          oninput={(e) => setAccent((e.currentTarget as HTMLInputElement).value)}
        />
        <input
          id="custom-accent"
          class="veil-input veil-custom-hex"
          bind:value={customHex}
          placeholder="#7c3aed"
          maxlength={7}
          autocomplete="off"
          aria-label="Renk kodu"
          onkeydown={(e) => { if (e.key === 'Enter') applyCustomHex(); }}
        />
        <button class="btn btn-secondary btn-sm" onclick={applyCustomHex} disabled={!customHex.trim()}>
          Uygula
        </button>
      </div>
      {#if hexError}<p class="veil-form-error" role="alert">{hexError}</p>{/if}
      <p class="veil-settings-row-desc veil-custom-hint">
        Hazır renklerden seç ya da #RRGGBB koduyla kendi rengini gir.
      </p>
    </div>

    <div class="veil-settings-row" style="margin-top: var(--space-4); border-top: 1px solid var(--veil-border-subtle); padding-top: var(--space-3);">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">AMOLED Saf Siyah Modu</div>
        <div class="veil-settings-row-desc">OLED ve AMOLED ekranlar için arka planları derin saf siyah (#000000) yapar, pil ve piksel tasarrufu sağlar.</div>
      </div>
      <Toggle checked={amoledMode} onChange={setAmoled} label="AMOLED Saf Siyah Modu" />
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Kişisel Tema Stüdyosu & CSS Editörü</div>
    <p class="veil-settings-row-desc" style="margin-bottom: var(--space-3);">
      Kendi CSS kodunuzu yazın, güvenli token şablonu yükleyin, AI ile tema üretin veya arka plan medyası ekleyin.
    </p>
    {#await ThemeStudioPromise then { default: ThemeStudio }}
      <ThemeStudio />
    {:catch}
      <div class="veil-home-loading"><div class="veil-spinner"></div></div>
    {/await}
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Metin & Görünüm</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Yazı boyutu</div>
        <div class="veil-settings-row-desc">Arayüz metinlerinin boyutu.</div>
      </div>
      <VeilSelect
        options={[12, 13, 14, 15, 16, 17, 18, 20].map(s => ({ value: String(s), label: `${s}px` }))}
        value={String(fontSize)}
        label="Yazı boyutu"
        onChange={(v) => setFontSize(Number(v))}
      />
    </div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Animasyonları azalt</div>
        <div class="veil-settings-row-desc">Hareket hassasiyeti için geçişleri ve animasyonları kapat.</div>
      </div>
      <Toggle checked={reduceMotion} onChange={setReduceMotion} label="Animasyonları azalt" />
    </div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Kompakt mod</div>
        <div class="veil-settings-row-desc">Daha küçük boşluklarla yoğun liste görünümü.</div>
      </div>
      <Toggle checked={compactMode} onChange={setCompact} label="Kompakt mod" />
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Sistem & Başlatma</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Bilgisayar Başlayınca Başlat</div>
        <div class="veil-settings-row-desc">Windows açıldığında veilanon'ı arka planda otomatik olarak başlatır.</div>
      </div>
      <Toggle checked={autostart} onChange={setAutostart} label="Bilgisayar Başlayınca Başlat" />
    </div>
  </div>
</section>

<style>
  .veil-theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
    gap: var(--space-3);
  }
  .veil-theme-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-2);
    padding: var(--space-2);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    cursor: pointer;
    color: var(--veil-text-secondary);
    font-size: var(--text-sm);
    transition: border-color var(--t-fast), background var(--t-fast), box-shadow var(--t-fast), transform var(--t-fast);
  }
  .veil-theme-card:hover { border-color: var(--veil-border); transform: translateY(-2px); box-shadow: var(--shadow-md); }
  .veil-theme-card.active {
    border-color: var(--veil-brand);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    box-shadow: 0 0 0 2px var(--veil-brand), 0 6px 16px hsl(262 72% 60% / 0.2);
  }
  .veil-theme-preview {
    display: block;
    height: 60px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--veil-border-subtle);
    padding: var(--space-2);
    position: relative;
    overflow: hidden;
  }
  .veil-theme-preview-dark { background: hsl(220, 20%, 9%); }
  .veil-theme-preview-light { background: hsl(220, 15%, 96%); }
  .veil-theme-preview-system {
    background: linear-gradient(135deg, hsl(220, 20%, 9%) 0 50%, hsl(220, 15%, 96%) 50% 100%);
  }
  .veil-theme-preview-bar {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 16px;
    background: hsl(220, 18%, 14%);
    border-right: 1px solid hsl(220, 13%, 22%);
  }
  .veil-theme-preview-light .veil-theme-preview-bar { background: hsl(220, 14%, 88%); border-right-color: hsl(220, 13%, 78%); }
  .veil-theme-preview-dot {
    position: absolute;
    left: 3px;
    top: 9px;
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    box-shadow: 0 0 0 2px hsl(220 20% 4% / 0.2);
  }
  .veil-theme-preview-line {
    position: absolute;
    left: 26px;
    right: 8px;
    top: 9px;
    height: 7px;
    border-radius: 4px;
    background: hsl(220, 13%, 24%);
  }
  .veil-theme-preview-line.short { top: 24px; width: 55%; right: auto; }
  .veil-theme-preview-line.tiny { top: 39px; width: 80%; right: auto; background: var(--veil-brand); opacity: 0.5; }
  .veil-theme-preview-light .veil-theme-preview-line { background: hsl(220, 13%, 84%); }
  .veil-theme-card-label {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    font-weight: 600;
    padding: 2px 0 2px;
  }
  .veil-theme-check {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    color: #fff;
    box-shadow: 0 2px 6px hsl(220 20% 4% / 0.35);
  }
  .veil-swatch-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }
  .veil-swatch {
    position: relative;
    width: 38px;
    height: 38px;
    border-radius: var(--radius-full);
    border: 2px solid transparent;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    background: var(--swatch);
    box-shadow: 0 2px 8px hsl(220 20% 4% / 0.3);
    transition: transform var(--t-spring), box-shadow var(--t-base), border-color var(--t-fast);
  }
  .veil-swatch:hover { transform: translateY(-2px) scale(1.08); box-shadow: 0 6px 14px hsl(220 20% 4% / 0.35); }
  .veil-swatch.active {
    border-color: var(--veil-text-primary);
    box-shadow: 0 0 0 3px var(--veil-bg-elevated), 0 0 0 5px var(--veil-text-primary), 0 4px 14px hsl(220 20% 4% / 0.4);
    transform: scale(1.1);
  }
  .veil-swatch-inner {
    position: absolute;
    inset: 5px;
    border-radius: var(--radius-full);
    background: var(--swatch);
    box-shadow: inset 0 1px 2px hsl(0 0% 100% / 0.25), inset 0 -2px 4px hsl(0 0% 0% / 0.2);
  }
  .veil-select {
    background: var(--veil-bg-surface);
    color: var(--veil-text-primary);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-base);
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .veil-custom-color { margin-top: var(--space-4); }
  .veil-custom-color-row { display: flex; align-items: center; gap: var(--space-2); }
  .veil-custom-hex { flex: 1; min-width: 0; font-family: var(--font-mono); text-transform: lowercase; }
  .veil-color-native {
    width: 44px;
    height: 36px;
    padding: 2px;
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    background: var(--veil-bg-surface);
    cursor: pointer;
    flex-shrink: 0;
  }
  .veil-custom-hint { margin-top: var(--space-2); }
</style>

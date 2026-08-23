<script module lang="ts">
  let lastStudioTab: 'editor' | 'ai' | 'media' | 'import-export' | null = (() => {
    try {
      const v = localStorage.getItem('veilanon-studio-tab') as 'editor' | 'ai' | 'media' | 'import-export' | null;
      if (v && ['editor','ai','media','import-export'].includes(v)) return v;
    } catch {}
    return null;
  })();
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore, type BgPlaylistItem } from '$lib/stores/ui';
  import { settingsApi } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import { sanitizeCss, validateMediaUrl, MAX_CSS_LENGTH } from '$lib/utils/css-sanitizer';
  import {
    getStarterCssTemplate,
    generateAiThemePrompt,
    exportThemeJson,
    importThemeJson,
    saveNamedTheme,
    getSavedThemes,
    deleteSavedTheme,
    generateThemeId,
    renameSavedTheme,
    setActiveThemeName,
    getActiveThemeName,
    type ThemeExportData,
    type SavedTheme,
  } from '$lib/utils/theme-apply';

  const ui = $derived($uiStore);
  const activePlaylist = $derived(ui.playlists.find(p => p.id === ui.activePlaylistId) ?? null);

  type StudioTab = NonNullable<typeof lastStudioTab>;
  let activeStudioTab = $state<StudioTab>(lastStudioTab ?? 'editor');

  function switchStudioTab(tab: StudioTab) {
    activeStudioTab = tab;
    lastStudioTab = tab;
    try { localStorage.setItem('veilanon-studio-tab', tab); } catch {}
  }

  // Editor State
  let customCssInput = $state('');
  let customThemeNameInput = $state('Kişisel Tema');
  let isCustomCssActive = $state(false);
  let sanitizerWarnings = $state<string[]>([]);
  let debounceTimeout: ReturnType<typeof setTimeout> | null = null;

  // Media State
  let mediaUrlInput = $state('');
  let detectedMediaType = $state<'image' | 'video' | 'page' | 'unknown'>('unknown');
  let isScrapingUrl = $state(false);
  let scrapeResults = $state<Array<{url: string; mediaType: string; source: string; poster?: string | null}>>([]);
  let bgOpacityInput = $state(0.26);
  let messageBlurInput = $state(8);
  let mediaError = $state<string | null>(null);
  let playlistImportModalOpen = $state(false);
  let playlistImportText = $state('');
  let lastAutoScrapedUrl = $state('');
  let autoScrapeToken = 0;
  let autoScrapeTimer: ReturnType<typeof setTimeout> | null = null;

  // Saved Themes State
  let savedThemesList = $state<Array<{id: string; name: string; savedAt: string}>>([]);
  let activeThemeName = $state<string | null>(null);
  let showSaveDialog = $state(false);
  let saveThemeName = $state('');

  // AI Prompt State
  let aiIdeaInput = $state('');

  // JSON Import State
  let jsonImportModalOpen = $state(false);
  let jsonImportText = $state('');
  let jsonImportError = $state<string | null>(null);

  onMount(() => {
    customCssInput = ui.customCss || '';
    isCustomCssActive = ui.customCssEnabled;
    if (ui.customBgVideo) {
      mediaUrlInput = ui.customBgVideo;
      detectedMediaType = 'video';
    } else if (ui.customBgImage) {
      mediaUrlInput = ui.customBgImage;
      detectedMediaType = 'image';
    }
    bgOpacityInput = ui.customBgOpacity ?? 0.26;
    messageBlurInput = ui.messageBackdropBlur ?? 8;
    customThemeNameInput = ui.customThemeName || 'Kişisel Tema';
    activeThemeName = getActiveThemeName();
    refreshSavedThemes();
  });

  function filenameFromUrl(url: string): string {
    try {
      return decodeURIComponent(new URL(url).pathname.split('/').pop() || '');
    } catch {
      return '';
    }
  }

  function addCurrentMediaToPlaylist() {
    const url = mediaUrlInput.trim();
    if (!url || !activePlaylist) return;
    const type = (detectedMediaType === 'video' || detectedMediaType === 'unknown') ? 'video' : 'image';
    uiStore.addToPlaylist(activePlaylist.id, {
      id: crypto.randomUUID(),
      url,
      type,
      title: filenameFromUrl(url) || `Medya ${activePlaylist.items.length + 1}`,
    });
    toastStore.success(`"${activePlaylist.name}" listesine eklendi.`);
  }

  async function handleCreatePlaylist() {
    const name = await uiStore.promptInput('Yeni arka plan listesi için bir ad girin', {
      title: 'Yeni Liste',
      placeholder: 'örn. uzay, arabalar…',
      confirmLabel: 'Oluştur',
    });
    if (name === null || !name.trim()) return;
    uiStore.addPlaylist(name);
    toastStore.success(`"${name.trim()}" listesi oluşturuldu.`);
  }

  async function handleRenamePlaylist() {
    if (!activePlaylist) return;
    const result = await uiStore.promptInput(`"${activePlaylist.name}" listesinin adını düzenle`, {
      title: 'Listeyi Yeniden Adlandır',
      defaultValue: activePlaylist.name,
      placeholder: 'Liste adı',
    });
    if (result === null || !result.trim()) return;
    uiStore.renamePlaylist(activePlaylist.id, result);
    toastStore.success('Liste adı güncellendi.');
  }

  async function handleDeletePlaylist() {
    if (!activePlaylist) return;
    const ok = await uiStore.confirm(
      `"${activePlaylist.name}" listesini ve içindeki ${activePlaylist.items.length} medyayı silmek istediğinize emin misiniz?`,
      { title: 'Listeyi Sil', danger: true, confirmLabel: 'Sil' }
    );
    if (!ok) return;
    uiStore.deletePlaylist(activePlaylist.id);
    toastStore.info('Liste silindi.');
  }

  async function handleExportPlaylist() {
    if (!activePlaylist || activePlaylist.items.length === 0) {
      toastStore.info('Aktif liste henüz boş.');
      return;
    }
    const jsonStr = JSON.stringify(activePlaylist.items, null, 2);
    const fileName = `veilanon-playlist-${activePlaylist.name.toLowerCase().replace(/\s+/g, '-')}.json`;
    if ('__TAURI_INTERNALS__' in window) {
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const path = await save({ title: 'Playlist dosyasını kaydet', defaultPath: fileName, filters: [{ name: 'JSON', extensions: ['json'] }] });
        if (!path) { toastStore.info('Kaydetme iptal edildi.'); return; }
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('write_text_file_user', { path, contents: jsonStr });
        toastStore.success(`Playlist kaydedildi: ${path}`);
        return;
      } catch {}
    }
    const blob = new Blob([jsonStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = fileName;
    a.click();
    URL.revokeObjectURL(url);
    toastStore.success('Playlist dosyası indirildi.');
  }

  function handleImportPlaylist() {
    if (!playlistImportText.trim() || !activePlaylist) return;
    try {
      let items: Array<{ id: string; url: string; type: string; title?: string }> = [];
      if (playlistImportText.trim().startsWith('[')) {
        items = JSON.parse(playlistImportText);
      } else {
        items = playlistImportText
          .split('\n')
          .map(l => l.trim())
          .filter(Boolean)
          .map((url, i) => ({
            id: crypto.randomUUID(),
            url,
            type: detectMediaType(url) === 'video' ? 'video' : 'image',
            title: `Medya ${i + 1}`,
          }));
      }
      if (Array.isArray(items) && items.length > 0) {
        const normalized: BgPlaylistItem[] = items.map(it => ({
          id: typeof it.id === 'string' && it.id ? it.id : crypto.randomUUID(),
          url: String(it.url),
          type: it.type === 'video' ? 'video' : 'image',
          title: it.title,
        }));
        uiStore.setPlaylists(
          ui.playlists.map(p =>
            p.id === activePlaylist.id
              ? { ...p, items: [...p.items.filter(existing => !normalized.some(n => n.url === existing.url)), ...normalized] }
              : p
          )
        );
        playlistImportModalOpen = false;
        playlistImportText = '';
        toastStore.success(`${normalized.length} medya "${activePlaylist.name}" listesine aktarıldı.`);
      }
    } catch {
      toastStore.error('Geçersiz playlist formatı.');
    }
  }

  // Debounced live update for CSS preview
  function handleCssChange(newCss: string) {
    customCssInput = newCss;
    const sanitized = sanitizeCss(newCss);
    sanitizerWarnings = sanitized.warnings;

    if (debounceTimeout) clearTimeout(debounceTimeout);
    if (isCustomCssActive) {
      debounceTimeout = setTimeout(() => {
        uiStore.setCustomCss(newCss);
      }, 400);
    }
  }

  function handleToggleLive(enabled: boolean) {
    isCustomCssActive = enabled;
    uiStore.toggleCustomCss(enabled);
    if (enabled) {
      uiStore.setCustomCss(customCssInput);
    }
    saveSettings({ customCssEnabled: enabled });
  }

  async function saveCustomTheme() {
    const sanitized = sanitizeCss(customCssInput);
    uiStore.setCustomCss(sanitized.safe);
    uiStore.setCustomThemeName(customThemeNameInput);
    uiStore.toggleCustomCss(isCustomCssActive);

    await saveSettings({
      customCss: sanitized.safe,
      customCssEnabled: isCustomCssActive,
      customThemeName: customThemeNameInput,
    });

    toastStore.success('Kişisel tema kaydedildi.');
  }

  function loadTemplate() {
    const template = getStarterCssTemplate();
    customCssInput = template;
    handleCssChange(template);
    toastStore.info('Başlangıç CSS şablonu yüklendi.');
  }

  async function resetCustomTheme() {
    const ok = await uiStore.confirm(
      'Kişisel CSS stillerini ve medya arka planını sıfırlamak istediğinize emin misiniz? Seçili hazır temanız korunacaktır.',
      { title: 'Kişisel Temayı Sıfırla', danger: true, confirmLabel: 'Sıfırla' }
    );
    if (!ok) return;

    uiStore.resetCustomLayer();
    customCssInput = '';
    isCustomCssActive = false;
    mediaUrlInput = '';
    detectedMediaType = 'unknown';
    scrapeResults = [];
    bgOpacityInput = 0.26;
    customThemeNameInput = 'Kişisel Tema';
    sanitizerWarnings = [];

    await saveSettings({
      customCss: '',
      customCssEnabled: false,
      customBgImage: '',
      customBgVideo: '',
      customBgOpacity: 0.26,
      customThemeName: 'Kişisel Tema',
    });

    setActiveThemeName(null);
    activeThemeName = null;
    toastStore.info('Kişisel tema katmanı sıfırlandı.');
  }

  // Media Handlers
  function applyCurrentMedia() {
    const url = mediaUrlInput.trim();
    if (!url) return;
    const check = validateMediaUrl(url);
    if (!check.isValid) {
      mediaError = check.error;
      return;
    }
    mediaError = null;
    if (detectedMediaType === 'video' || detectedMediaType === 'unknown') {
      uiStore.setCustomBackground('', url, bgOpacityInput);
      saveSettings({ customBgImage: '', customBgVideo: url, customBgOpacity: bgOpacityInput });
    } else {
      uiStore.setCustomBackground(url, '', bgOpacityInput);
      saveSettings({ customBgImage: url, customBgVideo: '', customBgOpacity: bgOpacityInput });
    }
    toastStore.success('Arka plan medyası uygulandı.');
  }

  function detectMediaType(url: string): 'image' | 'video' | 'page' | 'unknown' {
    const lower = url.toLowerCase();
    if (/\.(mp4|webm|mov|avi|mkv|ogv)(\?|$)/i.test(lower)) return 'video';
    if (/\.(jpg|jpeg|png|gif|webp|svg|bmp|tiff|avif)(\?|$)/i.test(lower)) return 'image';
    if (lower.startsWith('data:video/')) return 'video';
    if (lower.startsWith('data:image/')) return 'image';
    if (lower.startsWith('blob:')) return 'video';
    if (lower.startsWith('http://') || lower.startsWith('https://')) {
      const path = new URL(url).pathname;
      const lastSegment = path.split('/').pop() || '';
      if (!lastSegment.includes('.')) return 'page';
      if (!/\.(mp4|webm|mov|jpg|jpeg|png|gif|webp|svg)(\?|$)/i.test(lower)) return 'page';
    }
    return 'unknown';
  }

  function handleMediaUrlChange(e: Event) {
    const val = (e.currentTarget as HTMLInputElement).value;
    mediaUrlInput = val;
    detectedMediaType = detectMediaType(val);
    scrapeResults = [];
    scheduleAutoScrape(val);
  }

  // Debounced auto-scrape for pasted page links (no manual click needed)
  function scheduleAutoScrape(val: string) {
    autoScrapeToken += 1;
    const token = autoScrapeToken;
    if (autoScrapeTimer) clearTimeout(autoScrapeTimer);
    autoScrapeTimer = setTimeout(() => {
      if (token !== autoScrapeToken) return;
      const trimmed = val.trim();
      if (
        detectedMediaType === 'page' &&
        /^https?:\/\//i.test(trimmed) &&
        trimmed.length >= 8 &&
        trimmed !== lastAutoScrapedUrl
      ) {
        lastAutoScrapedUrl = trimmed;
        handleScrapeUrl();
      }
    }, 800);
  }

  async function handleScrapeUrl() {
    if (!mediaUrlInput.trim()) return;
    isScrapingUrl = true;
    scrapeResults = [];
    mediaError = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<{success: boolean; media_urls: Array<{url: string; media_type: string; source: string; poster?: string | null}>; title?: string; error?: string}>('scrape_url', { url: mediaUrlInput.trim() });
      if (result.success && result.media_urls.length > 0) {
        scrapeResults = result.media_urls.map(m => ({ url: m.url, mediaType: m.media_type, source: m.source, poster: m.poster ?? undefined }));
        toastStore.success(`${result.media_urls.length} medya bulundu${result.title ? ': ' + result.title : ''}`);
      } else {
        mediaError = result.error || 'Bu sayfada medya bulunamadı.';
      }
    } catch (err) {
      mediaError = 'URL arama başarısız: ' + String(err);
    } finally {
      isScrapingUrl = false;
    }
  }

  // Apply a scraped candidate directly without touching the URL input field.
  function applyScrapedDirect(url: string, mediaType: 'image' | 'video') {
    const check = validateMediaUrl(url);
    if (!check.isValid) {
      mediaError = check.error;
      return;
    }
    mediaError = null;
    if (mediaType === 'video') {
      uiStore.setCustomBackground('', url, bgOpacityInput);
      saveSettings({ customBgImage: '', customBgVideo: url, customBgOpacity: bgOpacityInput });
    } else {
      uiStore.setCustomBackground(url, '', bgOpacityInput);
      saveSettings({ customBgImage: url, customBgVideo: '', customBgOpacity: bgOpacityInput });
    }
    toastStore.success('Arka plan medyası uygulandı.');
  }

  // Safe hostname extraction for result cards
  function getHostname(url: string): string {
    try {
      return new URL(url).hostname;
    } catch {
      return url.length > 24 ? url.slice(0, 24) + '…' : url;
    }
  }

  // Add a scraped candidate directly to the active background playlist
  function addScrapedMediaToPlaylist(item: {url: string; mediaType: string}) {
    if (!activePlaylist) return;
    let filename = '';
    try {
      const path = new URL(item.url).pathname;
      filename = decodeURIComponent(path.split('/').pop() || '');
    } catch {
      filename = '';
    }
    uiStore.addToPlaylist(activePlaylist.id, {
      id: crypto.randomUUID(),
      url: item.url,
      type: item.mediaType === 'video' ? 'video' : 'image',
      title: filename || `Medya ${activePlaylist.items.length + 1}`,
    });
    toastStore.success(`"${activePlaylist.name}" listesine eklendi.`);
  }

  function removeMedia() {
    mediaUrlInput = '';
    detectedMediaType = 'unknown';
    scrapeResults = [];
    mediaError = null;
    lastAutoScrapedUrl = '';
    autoScrapeToken += 1;
    if (autoScrapeTimer) {
      clearTimeout(autoScrapeTimer);
      autoScrapeTimer = null;
    }
    uiStore.setCustomBackground('', '', bgOpacityInput);
    saveSettings({ customBgImage: '', customBgVideo: '' });
    toastStore.info('Arka plan medyası kaldırıldı.');
  }

  // Named Theme Save / Load
  function refreshSavedThemes() {
    savedThemesList = getSavedThemes().map(t => ({ id: t.id, name: t.name, savedAt: t.savedAt }));
    activeThemeName = getActiveThemeName();
  }

  async function handleSaveNamedTheme() {
    const name = saveThemeName.trim();
    if (!name) return;
    const accent = localStorage.getItem('veilanon-accent') || null;

    const existing = getSavedThemes().find(t => t.name.toLowerCase() === name.toLowerCase());
    if (existing) {
      const ok = await uiStore.confirm(
        `"${name}" adıyla bir tema zaten var. Üzerine yazılsın mı?`,
        { title: 'Üzerine Yaz', danger: true, confirmLabel: 'Üzerine Yaz' }
      );
      if (!ok) return;
    }

    const theme: SavedTheme = {
      id: existing ? existing.id : generateThemeId(),
      name,
      presetThemeId: ui.presetThemeId,
      customCss: customCssInput,
      customCssEnabled: isCustomCssActive,
      accentColor: accent,
      customBgImage: ui.customBgImage,
      customBgVideo: ui.customBgVideo,
      customBgOpacity: ui.customBgOpacity,
      savedAt: new Date().toISOString(),
    };
    saveNamedTheme(theme);
    setActiveThemeName(theme.name);
    activeThemeName = theme.name;
    refreshSavedThemes();
    showSaveDialog = false;
    saveThemeName = '';
    toastStore.success(`"${theme.name}" teması kaydedildi.`);
  }

  function handleLoadSavedTheme(id: string) {
    const theme = getSavedThemes().find(t => t.id === id);
    if (!theme) return;
    customCssInput = theme.customCss;
    isCustomCssActive = theme.customCssEnabled;
    customThemeNameInput = theme.name;
    uiStore.setCustomCss(theme.customCss);
    uiStore.toggleCustomCss(theme.customCssEnabled);
    uiStore.setCustomThemeName(theme.name);
    uiStore.setPresetTheme(theme.presetThemeId);
    uiStore.setCustomBackground(theme.customBgImage, theme.customBgVideo, theme.customBgOpacity);
    if (theme.accentColor) uiStore.setAccentColor(theme.accentColor);
    saveSettings({
      customCss: theme.customCss, customCssEnabled: theme.customCssEnabled,
      customThemeName: theme.name, presetThemeId: theme.presetThemeId,
      customBgImage: theme.customBgImage, customBgVideo: theme.customBgVideo,
      customBgOpacity: theme.customBgOpacity, accentColor: theme.accentColor,
    });
    setActiveThemeName(theme.name);
    activeThemeName = theme.name;
    toastStore.success(`"${theme.name}" teması yüklendi.`);
  }

  async function handleDeleteSavedTheme(id: string, name: string) {
    const ok = await uiStore.confirm(`"${name}" temasını silmek istediğinize emin misiniz?`, { title: 'Temayı Sil', danger: true, confirmLabel: 'Sil' });
    if (!ok) return;
    deleteSavedTheme(id);
    refreshSavedThemes();
    toastStore.info(`"${name}" silindi.`);
  }

  async function handleRenameSavedTheme(theme: { id: string; name: string }) {
    const result = await uiStore.promptInput(`"${theme.name}" temasının adını düzenle`, {
      title: 'Yeniden Adlandır',
      defaultValue: theme.name,
      placeholder: 'Tema adı',
    });
    if (result === null || !result.trim()) return;
    const ok = renameSavedTheme(theme.id, result);
    if (!ok) {
      toastStore.error('Tema adı güncellenemedi.');
      return;
    }
    refreshSavedThemes();
    toastStore.success('Tema adı güncellendi.');
  }

  // AI Prompt Copy
  async function copyAiPrompt() {
    const prompt = generateAiThemePrompt(aiIdeaInput);
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard) {
        await navigator.clipboard.writeText(prompt);
      }
      toastStore.success('Güvenli AI tema istemi panoya kopyalandı!');
    } catch {
      toastStore.error('Panoya kopyalama başarısız oldu.');
    }
  }

  async function copyCurrentCss() {
    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard) {
        await navigator.clipboard.writeText(customCssInput);
      }
      toastStore.success('CSS kodu panoya kopyalandı.');
    } catch {
      toastStore.error('Kopyalama başarısız oldu.');
    }
  }

  // JSON Export / Import
  async function handleExportJson() {
    const data: ThemeExportData = {
      version: 1,
      name: customThemeNameInput,
      presetThemeId: ui.presetThemeId,
      customCss: customCssInput,
      customCssEnabled: isCustomCssActive,
      // Export the APPLIED background (source of truth), not the raw input field.
      customBgImage: ui.customBgImage || '',
      customBgVideo: ui.customBgVideo || '',
      customBgOpacity: bgOpacityInput,
    };
    const jsonStr = exportThemeJson(data);
    const fileName = `${customThemeNameInput.toLowerCase().replace(/\s+/g, '-')}-theme.json`;

    // Browser preview (no Tauri runtime): fall back to Blob download.
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
      const blob = new Blob([jsonStr], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName;
      a.click();
      URL.revokeObjectURL(url);
      toastStore.success('Tema dosyası indirildi (.json).');
      return;
    }

    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        title: 'Tema dosyasını kaydet',
        defaultPath: fileName,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!path) {
        toastStore.info('Kaydetme iptal edildi.');
        return;
      }
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('write_text_file_user', { path, contents: jsonStr });
      toastStore.success(`Tema dosyası kaydedildi: ${path}`);
    } catch (err) {
      toastStore.error('Tema dosyası kaydedilemedi: ' + String(err));
    }
  }

  function handleImportJsonSubmit() {
    jsonImportError = null;
    if (!jsonImportText.trim()) {
      jsonImportError = 'Lütfen geçerli bir JSON metni yapıştırın.';
      return;
    }

    const { data, error } = importThemeJson(jsonImportText);
    if (error || !data) {
      jsonImportError = error || 'Bilinmeyen import hatası.';
      return;
    }

    customThemeNameInput = data.name;
    customCssInput = data.customCss;
    isCustomCssActive = data.customCssEnabled;
    if (data.customBgVideo) {
      mediaUrlInput = data.customBgVideo;
      detectedMediaType = 'video';
    } else if (data.customBgImage) {
      mediaUrlInput = data.customBgImage;
      detectedMediaType = 'image';
    } else {
      mediaUrlInput = '';
      detectedMediaType = 'unknown';
    }
    bgOpacityInput = data.customBgOpacity;

    uiStore.setCustomThemeName(data.name);
    uiStore.setCustomCss(data.customCss);
    uiStore.toggleCustomCss(data.customCssEnabled);
    uiStore.setCustomBackground(data.customBgImage, data.customBgVideo, data.customBgOpacity);
    if (data.presetThemeId) {
      uiStore.setPresetTheme(data.presetThemeId);
    }

    saveSettings({
      customThemeName: data.name,
      customCss: data.customCss,
      customCssEnabled: data.customCssEnabled,
      customBgImage: data.customBgImage,
      customBgVideo: data.customBgVideo,
      customBgOpacity: data.customBgOpacity,
      presetThemeId: data.presetThemeId,
    });

    jsonImportModalOpen = false;
    jsonImportText = '';
    toastStore.success(`"${data.name}" teması başarıyla içe aktarıldı!`);
  }

  async function handleFileUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    try {
      const text = await file.text();
      jsonImportText = text;
      handleImportJsonSubmit();
    } catch {
      toastStore.error('Dosya okunamadı.');
    } finally {
      // Reset so re-selecting the same file fires onchange again.
      input.value = '';
    }
  }

  async function saveSettings(patch: Record<string, any>) {
    try {
      await settingsApi.update(patch as any);
    } catch {
      /* best effort */
    }
  }
</script>

<div class="veil-theme-studio">
  <!-- Studio Header Tabs -->
  <div class="veil-studio-nav">
    <button
      type="button"
      class="veil-studio-tab"
      class:active={activeStudioTab === 'editor'}
      onclick={() => switchStudioTab('editor')}
    >
      <Icon name="code" size={14} />
      <span>CSS Editörü</span>
    </button>
    <button
      type="button"
      class="veil-studio-tab"
      class:active={activeStudioTab === 'ai'}
      onclick={() => switchStudioTab('ai')}
    >
      <Icon name="sparkle" size={14} />
      <span>AI Tema Sihirbazı</span>
    </button>
    <button
      type="button"
      class="veil-studio-tab"
      class:active={activeStudioTab === 'media'}
      onclick={() => switchStudioTab('media')}
    >
      <Icon name="film" size={14} />
      <span>Medya Arka Planı</span>
      {#if ui.customBgImage || ui.customBgVideo}
        <span class="dot-active"></span>
      {/if}
    </button>
    <button
      type="button"
      class="veil-studio-tab"
      class:active={activeStudioTab === 'import-export'}
      onclick={() => switchStudioTab('import-export')}
    >
      <Icon name="download" size={14} />
      <span>İçe / Dışa Aktar</span>
    </button>
  </div>

  <!-- Tab 1: CSS Editor -->
  {#if activeStudioTab === 'editor'}
    <div class="veil-editor-pane">
      <div class="veil-editor-toolbar">
        <div class="toolbar-left">
          <input
            type="text"
            class="veil-input veil-theme-name-input"
            bind:value={customThemeNameInput}
            placeholder="Tema Adı"
            maxlength={40}
            aria-label="Kişisel Tema Adı"
          />
          <div class="live-toggle-wrapper">
            <Toggle
              checked={isCustomCssActive}
              onChange={handleToggleLive}
              label="Canlı Önizleme"
            />
            <span class="live-toggle-label">{isCustomCssActive ? 'Aktif' : 'Pasif'}</span>
          </div>
        </div>

        <div class="toolbar-right">
          <button type="button" class="btn btn-secondary btn-xs" onclick={loadTemplate} title="Örnek şablon yükle">
            <Icon name="file" size={12} />
            <span>Şablon Yükle</span>
          </button>
          <button type="button" class="btn btn-secondary btn-xs" onclick={copyCurrentCss} title="CSS Kodunu Kopyala">
            <Icon name="copy" size={12} />
            <span>Kopyala</span>
          </button>
          <button type="button" class="btn btn-danger btn-xs" onclick={resetCustomTheme} title="Özel stilleri temizle">
            <Icon name="trash" size={12} />
            <span>Sıfırla</span>
          </button>
          <button type="button" class="btn btn-primary btn-xs" onclick={saveCustomTheme}>
            <Icon name="check" size={12} />
            <span>Kaydet</span>
          </button>
        </div>
      </div>

      {#if sanitizerWarnings.length > 0}
        <div class="veil-sanitizer-warnings" role="alert">
          <Icon name="warning" size={14} />
          <div class="warning-list">
            {#each sanitizerWarnings as w}
              <div>{w}</div>
            {/each}
          </div>
        </div>
      {/if}

      <div class="veil-code-editor-box">
        <textarea
          class="veil-css-textarea"
          bind:value={customCssInput}
          oninput={(e) => handleCssChange((e.currentTarget as HTMLTextAreaElement).value)}
          placeholder={`/* Buraya kendi CSS kurallarınızı yazın */\n:root {\n  --veil-brand: #7c3aed;\n  --veil-bg-void: #0d0f14;\n}`}
          spellcheck="false"
          autocomplete="off"
          autocapitalize="off"
          aria-label="Kişisel CSS Kodu"
        ></textarea>
        <div class="editor-footer">
          <span class="char-count" class:limit={customCssInput.length > MAX_CSS_LENGTH - 1000}>
            {customCssInput.length.toLocaleString('tr-TR')} / {MAX_CSS_LENGTH.toLocaleString('tr-TR')} karakter
          </span>
          <span class="editor-hint">Değişiklikler anlık olarak uygulanır. Kalıcı olması için "Kaydet"e basın.</span>
        </div>
      </div>

      <div class="saved-themes-section">
        <div class="saved-themes-header">
          <h4>Kayıtlı Temalar</h4>
          <button type="button" class="btn btn-primary btn-xs" onclick={() => { showSaveDialog = true; saveThemeName = customThemeNameInput; }}>
            <Icon name="download" size={12} />
            <span>Bu Temayı Kaydet</span>
          </button>
        </div>
        {#if savedThemesList.length === 0}
          <p class="saved-themes-empty">Henüz kaydedilmiş tema yok.</p>
        {:else}
          <div class="saved-themes-list">
            {#each savedThemesList as theme}
              <div class="saved-theme-row">
                <div class="saved-theme-info">
                  <div class="saved-theme-name-row">
                    <span class="saved-theme-name">{theme.name}</span>
                    {#if theme.name === activeThemeName}
                      <span class="dot-active" title="Aktif tema"></span>
                    {/if}
                  </div>
                  <span class="saved-theme-date">{new Date(theme.savedAt).toLocaleDateString('tr-TR')}</span>
                </div>
                <div class="saved-theme-actions">
                  <button type="button" class="btn btn-secondary btn-xs" onclick={() => handleLoadSavedTheme(theme.id)} title="Temayı Yükle">
                    <Icon name="download" size={12} />
                    <span>Yükle</span>
                  </button>
                  <button type="button" class="btn btn-secondary btn-xs" onclick={() => handleRenameSavedTheme(theme)} title="Temayı Yeniden Adlandır">
                    <Icon name="edit" size={12} />
                  </button>
                  <button type="button" class="btn btn-danger btn-xs" onclick={() => handleDeleteSavedTheme(theme.id, theme.name)} title="Temayı Sil">
                    <Icon name="trash" size={12} />
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

  <!-- Tab 2: AI Prompt Wizard -->
  {:else if activeStudioTab === 'ai'}
    <div class="veil-ai-pane">
      <div class="veil-ai-header">
        <div class="ai-badge">
          <Icon name="sparkle" size={14} />
          <span>Yapay Zeka Tema İstemi Üretici</span>
        </div>
        <h4>İstediğiniz temayı tarif edin, AI sizin için tam uyumlu CSS üretsin</h4>
        <p class="veil-settings-row-desc">
          Gizliliğiniz için verileriniz harici sunuculara otomatik iletilmez. Buradan kopyalayacağınız optimize edilmiş istemi tercih ettiğiniz herhangi bir yapay zekaya (ChatGPT, Claude, Gemini, vb.) yapıştırıp anında tema ürettirebilirsiniz.
        </p>
      </div>

      <div class="veil-ai-input-group">
        <label class="veil-form-label" for="ai-idea-input">Tema Fikriniz veya Konseptiniz (İsteğe bağlı):</label>
        <textarea
          id="ai-idea-input"
          class="veil-input veil-ai-idea-textarea"
          bind:value={aiIdeaInput}
          placeholder="Örnek: Cyberpunk gece kulübü, neon pembe vurgular, derin zift siyahı paneller ve mor hafif parıltı..."
          rows={3}
        ></textarea>
      </div>

      <div class="veil-ai-actions">
        <button type="button" class="btn btn-primary" onclick={copyAiPrompt}>
          <Icon name="copy" size={16} />
          <span>AI İstemini Panoya Kopyala</span>
        </button>
      </div>

      <div class="veil-ai-steps">
        <div class="step-card">
          <div class="step-num">1</div>
          <div class="step-body">
            <strong>İstemi Kopyala</strong>
            <span>Yukarıdaki butona basarak güvenlik kuralları ve token sözleşmesi içeren hazır promptu kopyalayın.</span>
          </div>
        </div>
        <div class="step-card">
          <div class="step-num">2</div>
          <div class="step-body">
            <strong>AI'a Gönderin</strong>
            <span>İstemi dilediğiniz yapay zekaya yapıştırın ve ürettiği CSS kodunu alın.</span>
          </div>
        </div>
        <div class="step-card">
          <div class="step-num">3</div>
          <div class="step-body">
            <strong>CSS Editörüne Yapıştırın</strong>
            <span>"CSS Editörü" sekmesine yapıştırıp "Kaydet"e basın; tema anında güvenli filtreden geçerek uygulanır.</span>
          </div>
        </div>
      </div>
    </div>

  <!-- Tab 3: Media Background -->
  {:else if activeStudioTab === 'media'}
    <div class="veil-media-pane">
      <div class="veil-media-header">
        <h4>Arka Plan Görseli veya Videosu</h4>
        <p class="veil-settings-row-desc">
          Tek bir URL ile görsel veya video ekleyin; türü otomatik algılanır. Bir web sayfası linki yapıştırırsanız medya bağlantılarını otomatik olarak tarar.
        </p>
      </div>

      {#if mediaError}
        <div class="veil-form-error" role="alert">
          <Icon name="warning" size={14} />
          <span>{mediaError}</span>
        </div>
      {/if}

      <div class="veil-media-form">
        <div class="veil-form-group">
          <label class="veil-form-label" for="media-url-input">Medya URL'si:</label>
          <div class="veil-media-input-row">
            <input
              id="media-url-input"
              type="url"
              class="veil-input"
              value={mediaUrlInput}
              oninput={handleMediaUrlChange}
              placeholder="https://example.com/wallpaper.jpg, .mp4, veya sayfa linki"
            />
            {#if detectedMediaType !== 'unknown'}
              <span class="media-type-badge" class:badge-video={detectedMediaType === 'video'} class:badge-image={detectedMediaType === 'image'} class:badge-page={detectedMediaType === 'page'}>
                {detectedMediaType === 'video' ? 'Video' : detectedMediaType === 'image' ? 'Görsel' : 'Sayfa'}
              </span>
            {/if}
          </div>
          {#if detectedMediaType === 'page'}
            <button type="button" class="btn btn-secondary btn-xs" onclick={handleScrapeUrl} disabled={isScrapingUrl} style="align-self: flex-start; margin-top: 4px;">
              {#if isScrapingUrl}
                <span class="spinner-tiny"></span>
                <span>Taranıyor…</span>
              {:else}
                <Icon name="search" size={12} />
                <span>Linkten Medya Bul</span>
              {/if}
            </button>
          {/if}
        </div>

        {#if isScrapingUrl || scrapeResults.length > 0}
          <div class="scrape-results">
            {#if isScrapingUrl}
              <div class="scrape-results-header">
                <Icon name="search" size={13} />
                <span>Sayfa taranıyor…</span>
              </div>
              <div class="scrape-grid">
                {#each [0, 1, 2, 3, 4, 5] as _sk, i (i)}
                  <div class="scrape-card scrape-skeleton" aria-hidden="true">
                    <div class="scrape-thumb"></div>
                    <span class="skeleton-line"></span>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="scrape-results-header">
                <Icon name="search" size={13} />
                <span>{scrapeResults.length} medya bulundu — uygulamak istediğine tıkla</span>
              </div>
              <div class="scrape-grid">
                {#each scrapeResults as item (item.url)}
                  <div class="scrape-card">
                    <button
                      type="button"
                      class="scrape-thumb"
                      onclick={() => applyScrapedDirect(item.url, item.mediaType === 'video' ? 'video' : 'image')}
                      title={item.url}
                    >
                      {#if item.mediaType === 'video'}
                        {#if item.poster}
                          <img src={item.poster} alt="" loading="lazy" referrerpolicy="no-referrer" />
                        {:else}
                          <span class="scrape-play-tile"><Icon name="play" size={22} /></span>
                        {/if}
                      {:else}
                        <img src={item.url} alt="" loading="lazy" referrerpolicy="no-referrer" />
                      {/if}
                      <span class="media-type-badge badge-small" class:badge-video={item.mediaType === 'video'} class:badge-image={item.mediaType !== 'video'}>
                        {item.mediaType === 'video' ? 'Video' : 'Görsel'}
                      </span>
                    </button>
                    <span class="scrape-host" title={item.url}>{getHostname(item.url)}</span>
                    <div class="scrape-actions">
                      <button type="button" class="btn btn-primary btn-xs" onclick={() => applyScrapedDirect(item.url, item.mediaType === 'video' ? 'video' : 'image')}>
                        Uygula
                      </button>
                      <button type="button" class="btn btn-secondary btn-xs" onclick={() => addScrapedMediaToPlaylist(item)}>
                        Playliste Ekle
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <div class="veil-form-group">
          <div class="opacity-header">
            <label class="veil-form-label" for="bg-opacity-slider">Arka Plan Opaklığı:</label>
            <span class="opacity-val">{Math.round(bgOpacityInput * 100)}%</span>
          </div>
          <input
            id="bg-opacity-slider"
            type="range"
            min="0"
            max="1.0"
            step="0.01"
            class="veil-range-slider"
            style={`--range-pct: ${Math.round(bgOpacityInput * 100)}%`}
            bind:value={bgOpacityInput}
            oninput={() => {
              const img = detectedMediaType === 'image' ? mediaUrlInput : '';
              const vid = detectedMediaType === 'video' || detectedMediaType === 'unknown' ? mediaUrlInput : '';
              uiStore.setCustomBackground(img, vid, bgOpacityInput);
            }}
          />
        </div>

        <div class="veil-form-group">
          <div class="opacity-header">
            <label class="veil-form-label" for="bg-blur-slider">Mesaj Arka Planı Bulanıklığı (Backdrop Blur):</label>
            <span class="opacity-val">{messageBlurInput}px</span>
          </div>
          <input
            id="bg-blur-slider"
            type="range"
            min="0"
            max="30"
            step="1"
            class="veil-range-slider"
            style={`--range-pct: ${Math.round((messageBlurInput / 30) * 100)}%`}
            bind:value={messageBlurInput}
            oninput={() => {
              uiStore.setMessageBlur(messageBlurInput);
            }}
          />
          <span class="veil-custom-hint">Arka plandaki medya varken mesajların daha rahat okunması için bulanıklık şiddetini ayarlar.</span>
        </div>

        <div class="veil-media-actions">
          <button type="button" class="btn btn-secondary btn-sm" onclick={removeMedia} disabled={!ui.customBgImage && !ui.customBgVideo}>
            <Icon name="trash" size={14} />
            <span>Medyayı Kaldır</span>
          </button>
          <button type="button" class="btn btn-secondary btn-sm" onclick={addCurrentMediaToPlaylist} disabled={!mediaUrlInput.trim()}>
            <Icon name="plus" size={14} />
            <span>Playliste Ekle</span>
          </button>
          <button type="button" class="btn btn-primary btn-sm" onclick={applyCurrentMedia} disabled={!mediaUrlInput.trim()}>
            <Icon name="check" size={14} />
            <span>Uygula & Kaydet</span>
          </button>
        </div>

        <!-- Playlist Section -->
        <div class="veil-playlist-section">
          <div class="veil-playlist-header">
            <div class="veil-playlist-title">
              <Icon name="film" size={15} />
              <span>Arka Plan Listeleri</span>
            </div>
            <div class="veil-playlist-tools">
              {#if activePlaylist && activePlaylist.items.length > 0}
                <button type="button" class="btn btn-ghost btn-xs" onclick={() => uiStore.advancePlayback()} title="Sonraki Medyaya Geç">
                  <Icon name="arrow-right" size={13} />
                  <span>Sonraki</span>
                </button>
                <button type="button" class="btn btn-ghost btn-xs" onclick={handleExportPlaylist} title="Aktif Listeyi Dışa Aktar">
                  <Icon name="download" size={13} />
                  <span>Dışa Aktar</span>
                </button>
                <button type="button" class="btn btn-ghost btn-xs" onclick={() => (playlistImportModalOpen = true)} title="Listeye İçe Aktar">
                  <Icon name="upload" size={13} />
                  <span>İçe Aktar</span>
                </button>
              {/if}
            </div>
          </div>

          <div class="veil-playlist-chips">
            {#each ui.playlists as pl (pl.id)}
              <button
                type="button"
                class="playlist-chip"
                class:active={pl.id === ui.activePlaylistId}
                onclick={() => uiStore.setActivePlaylist(pl.id)}
                title="{pl.name} ({pl.items.length} medya)"
              >
                {pl.name}
                <span class="chip-count">{pl.items.length}</span>
              </button>
            {/each}
            <button type="button" class="playlist-chip chip-new" onclick={handleCreatePlaylist} title="Yeni Liste Oluştur">
              <Icon name="plus" size={12} />
              <span>Yeni Liste</span>
            </button>
          </div>

          {#if activePlaylist}
            <div class="veil-playlist-actions-row">
              <span class="actions-row-label">"{activePlaylist.name}" listesi:</span>
              <button
                type="button"
                class="btn btn-secondary btn-xs"
                onclick={handleRenamePlaylist}
                title="Listeyi Yeniden Adlandır"
              >
                <Icon name="edit" size={12} />
                <span>Yeniden Adlandır</span>
              </button>
              <button
                type="button"
                class="btn btn-danger btn-xs"
                onclick={handleDeletePlaylist}
                disabled={ui.playlists.length <= 1}
                title={ui.playlists.length <= 1 ? 'Son liste silinemez' : 'Listeyi Sil'}
              >
                <Icon name="trash" size={12} />
                <span>Sil</span>
              </button>

              <div class="mode-segmented" role="group" aria-label="Oynatma modu">
                <button
                  type="button"
                  class="mode-btn"
                  class:active={ui.playbackMode === 'sequential'}
                  onclick={() => uiStore.setPlaybackMode('sequential')}
                  title="Sırayla oynat"
                >Sırayla</button>
                <button
                  type="button"
                  class="mode-btn"
                  class:active={ui.playbackMode === 'shuffle'}
                  onclick={() => uiStore.setPlaybackMode('shuffle')}
                  title="Karışık oynat"
                >Karışık</button>
                <button
                  type="button"
                  class="mode-btn"
                  class:active={ui.playbackMode === 'timed'}
                  onclick={() => uiStore.setPlaybackMode('timed')}
                  title="Zamanlı oynat"
                >Zamanlı</button>
              </div>

              {#if ui.playbackMode === 'timed'}
                <select
                  class="interval-select"
                  value={String(ui.playbackIntervalSec)}
                  onchange={(e) => uiStore.setPlaybackIntervalSec(Number((e.currentTarget as HTMLSelectElement).value))}
                  aria-label="Zamanlı oynatma aralığı"
                >
                  <option value="30">30 sn</option>
                  <option value="60">1 dk</option>
                  <option value="300">5 dk</option>
                  <option value="900">15 dk</option>
                </select>
              {/if}
            </div>

            {#if activePlaylist.items.length > 0}
              <div class="veil-playlist-grid">
                {#each activePlaylist.items as item (item.id)}
                  {@const isCurrent = ui.customBgImage === item.url || ui.customBgVideo === item.url}
                  <div class="veil-playlist-card" class:active={isCurrent}>
                    <div class="veil-playlist-info">
                      <span class="media-type-badge badge-small" class:badge-video={item.type === 'video'} class:badge-image={item.type === 'image'}>
                        {item.type === 'video' ? 'Video' : 'Görsel'}
                      </span>
                      <div class="veil-playlist-meta">
                        <span class="veil-playlist-title-text" title={item.title || filenameFromUrl(item.url)}>
                          {item.title || filenameFromUrl(item.url)}
                        </span>
                        <span class="veil-playlist-url" title={item.url}>{item.url}</span>
                      </div>
                    </div>
                    <div class="veil-playlist-card-actions">
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        onclick={() => {
                          mediaUrlInput = item.url;
                          detectedMediaType = item.type;
                          if (item.type === 'video') {
                            uiStore.setCustomBackground('', item.url, bgOpacityInput);
                            saveSettings({ customBgImage: '', customBgVideo: item.url, customBgOpacity: bgOpacityInput });
                          } else {
                            uiStore.setCustomBackground(item.url, '', bgOpacityInput);
                            saveSettings({ customBgImage: item.url, customBgVideo: '', customBgOpacity: bgOpacityInput });
                          }
                        }}
                        title="Bu Medyayı Seç"
                      >
                        {isCurrent ? 'Aktif' : 'Seç'}
                      </button>
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        onclick={async () => {
                          const result = await uiStore.promptInput('Medya adını düzenle', {
                            title: 'Yeniden Adlandır',
                            defaultValue: item.title || filenameFromUrl(item.url),
                            placeholder: 'Medya adı',
                          });
                          if (result === null) return;
                          uiStore.renamePlaylistItem(item.id, result);
                          toastStore.success('Medya adı güncellendi.');
                        }}
                        title="Medya Adını Düzenle"
                      >
                        <Icon name="edit" size={12} />
                      </button>
                      <button
                        type="button"
                        class="btn btn-ghost btn-xs text-danger"
                        onclick={() => uiStore.removePlaylistItem(activePlaylist.id, item.id)}
                        title="Playlistten Kaldır"
                      >
                        <Icon name="trash" size={12} />
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="veil-playlist-empty">Bu liste boş. Yukarıdaki "Playliste Ekle" ile medya ekleyin.</p>
            {/if}
          {/if}
        </div>
      </div>
    </div>

  <!-- Tab 4: Import / Export -->
  {:else if activeStudioTab === 'import-export'}
    <div class="veil-impexp-pane">
      <div class="impexp-grid">
        <!-- Export Card -->
        <div class="impexp-card">
          <div class="impexp-icon">
            <Icon name="download" size={24} />
          </div>
          <h4>Temayı Dışa Aktar</h4>
          <p class="veil-settings-row-desc">
            Kişisel CSS kodunuzu, tema adınızı ve medya ayarlarınızı içeren sürüm kontrollü bir <code>.json</code> tema dosyası oluşturup indirin.
          </p>
          <button type="button" class="btn btn-secondary" onclick={handleExportJson}>
            <Icon name="download" size={14} />
            <span>Tema Dosyası İndir (.json)</span>
          </button>
        </div>

        <!-- Import Card -->
        <div class="impexp-card">
          <div class="impexp-icon">
            <Icon name="upload" size={24} />
          </div>
          <h4>Temayı İçe Aktar</h4>
          <p class="veil-settings-row-desc">
            Daha önce oluşturulmuş veya topluluk tarafından paylaşılmış bir <code>.json</code> tema dosyasını yükleyin veya metin olarak yapıştırın.
          </p>
          <div class="import-buttons">
            <label class="btn btn-secondary" style="cursor: pointer;">
              <Icon name="upload" size={14} />
              <span>Dosya Seç (.json)</span>
              <input type="file" accept=".json" class="visually-hidden" onchange={handleFileUpload} />
            </label>
            <button type="button" class="btn btn-secondary" onclick={() => (jsonImportModalOpen = true)}>
              <Icon name="copy" size={14} />
              <span>JSON Yapıştır</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Modal: JSON Paste Import -->
{#if jsonImportModalOpen}
  <div
    class="veil-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="import-json-title"
    tabindex="-1"
    onclick={() => (jsonImportModalOpen = false)}
    onkeydown={(e) => { if (e.key === 'Escape') jsonImportModalOpen = false; }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="veil-modal veil-modal-md"
      role="document"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="veil-modal-header">
        <h3 id="import-json-title">Tema JSON'ı İçe Aktar</h3>
        <button type="button" class="btn-icon" onclick={() => (jsonImportModalOpen = false)} aria-label="Kapat">
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="veil-modal-body" style="display:flex; flex-direction:column; gap:12px;">
        {#if jsonImportError}
          <div class="veil-form-error" role="alert">
            <Icon name="warning" size={14} />
            <span>{jsonImportError}</span>
          </div>
        {/if}
        <p class="veil-settings-row-desc" style="margin:0;">
          Dışa aktarılan <code>.json</code> dosyasının içeriğini buraya yapıştırın veya <em>Dosya Seç</em> ile yükleyin.
        </p>
        <textarea
          class="veil-input veil-import-textarea"
          bind:value={jsonImportText}
          placeholder={`{\n  "version": 1,\n  "name": "Kişisel Tema",\n  "customCss": ":root { ... }"\n}`}
          rows={12}
          spellcheck="false"
          style="min-height:220px; display:block; width:100%; resize:vertical; font-family:var(--font-mono,monospace); font-size:12px; line-height:1.5;"
        ></textarea>
      </div>

      <div class="veil-modal-footer">
        <button type="button" class="btn btn-ghost" onclick={() => (jsonImportModalOpen = false)}>
          İptal
        </button>
        <button type="button" class="btn btn-primary" onclick={handleImportJsonSubmit} disabled={!jsonImportText.trim()}>
          İçe Aktar & Uygula
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Modal: Playlist Import -->
{#if playlistImportModalOpen}
  <div
    class="veil-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="import-playlist-title"
    tabindex="-1"
    onclick={() => (playlistImportModalOpen = false)}
    onkeydown={(e) => { if (e.key === 'Escape') playlistImportModalOpen = false; }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="veil-modal veil-modal-md"
      role="document"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="veil-modal-header">
        <h3 id="import-playlist-title">Arka Plan Playlisti İçe Aktar</h3>
        <button type="button" class="btn-icon" onclick={() => (playlistImportModalOpen = false)} aria-label="Kapat">
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="veil-modal-body">
        <p class="veil-settings-row-desc">
          JSON formatında bir playlist veya her satırında bir görsel/video linki bulunan metin yapıştırın.
        </p>
        <textarea
          class="veil-input veil-import-textarea"
          bind:value={playlistImportText}
          placeholder={`https://example.com/bg1.mp4\nhttps://example.com/wallpaper.jpg`}
          rows={8}
          spellcheck="false"
        ></textarea>
      </div>

      <div class="veil-modal-footer">
        <button type="button" class="btn btn-ghost" onclick={() => (playlistImportModalOpen = false)}>
          İptal
        </button>
        <button type="button" class="btn btn-primary" onclick={handleImportPlaylist} disabled={!playlistImportText.trim()}>
          İçe Aktar
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showSaveDialog}
  <div
    class="veil-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="save-theme-title"
    tabindex="-1"
    onclick={() => (showSaveDialog = false)}
    onkeydown={(e) => { if (e.key === 'Escape') showSaveDialog = false; }}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="veil-modal veil-modal-md"
      role="document"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="veil-modal-header">
        <h3 id="save-theme-title">Temayı Kaydet</h3>
        <button type="button" class="btn-icon" onclick={() => (showSaveDialog = false)} aria-label="Kapat">
          <Icon name="x" size={16} />
        </button>
      </div>

      <div class="veil-modal-body">
        <div class="veil-form-group">
          <label class="veil-form-label" for="save-theme-name">Tema Adı:</label>
          <input
            id="save-theme-name"
            type="text"
            class="veil-input"
            bind:value={saveThemeName}
            placeholder="Tema adı girin…"
            maxlength={50}
            onkeydown={(e) => { if (e.key === 'Enter') handleSaveNamedTheme(); }}
          />
        </div>
      </div>

      <div class="veil-modal-footer">
        <button type="button" class="btn btn-ghost" onclick={() => (showSaveDialog = false)}>
          İptal
        </button>
        <button type="button" class="btn btn-primary" onclick={handleSaveNamedTheme} disabled={!saveThemeName.trim()}>
          Kaydet
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .veil-theme-studio {
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 0.75rem);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl, 1rem);
    padding: var(--space-3, 0.75rem);
    overflow: hidden;
  }

  .veil-studio-nav {
    display: flex;
    align-items: center;
    gap: var(--space-1, 0.25rem);
    background: var(--veil-bg-surface);
    padding: 3px;
    border-radius: var(--radius-lg, 0.75rem);
    border: 1px solid var(--veil-border-subtle);
    flex-wrap: wrap;
  }

  .veil-studio-tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: var(--text-xs, 12px);
    font-weight: 600;
    color: var(--veil-text-secondary);
    background: transparent;
    border: none;
    border-radius: var(--radius-md, 0.5rem);
    cursor: pointer;
    transition: all var(--t-fast, 150ms ease);
    position: relative;
  }

  .veil-studio-tab:hover {
    color: var(--veil-text-primary);
    background: var(--veil-bg-elevated);
  }

  .veil-studio-tab.active {
    background: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }

  .dot-active {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--veil-success, #10b981);
    box-shadow: 0 0 6px var(--veil-success, #10b981);
  }

  .veil-editor-pane,
  .veil-ai-pane,
  .veil-media-pane,
  .veil-impexp-pane {
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 0.75rem);
    padding-top: var(--space-1, 0.25rem);
  }

  .veil-editor-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3, 0.75rem);
    flex-wrap: wrap;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: var(--space-3, 0.75rem);
  }

  .veil-theme-name-input {
    width: 140px;
    font-size: var(--text-xs, 12px);
    padding: 4px 8px;
    height: 30px;
  }

  .live-toggle-wrapper {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .live-toggle-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-secondary);
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.5rem);
  }

  .btn-xs {
    height: 30px;
    padding: 0 10px;
    font-size: 11px;
    gap: 4px;
  }

  .veil-sanitizer-warnings {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    background: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: var(--radius-md, 0.5rem);
    padding: var(--space-2, 0.5rem) var(--space-3, 0.75rem);
    color: #f59e0b;
    font-size: 12px;
  }

  .warning-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .veil-code-editor-box {
    display: flex;
    flex-direction: column;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg, 0.75rem);
    overflow: hidden;
  }

  .veil-css-textarea {
    width: 100%;
    height: 220px;
    background: transparent;
    border: none;
    color: var(--veil-text-primary);
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.5;
    padding: var(--space-3, 0.75rem);
    resize: vertical;
    outline: none;
    tab-size: 2;
  }

  .editor-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px var(--space-3, 0.75rem);
    background: var(--veil-bg-void);
    border-top: 1px solid var(--veil-border-subtle);
    font-size: 11px;
    color: var(--veil-text-muted);
  }

  .char-count.limit {
    color: var(--veil-danger, #ef4444);
    font-weight: 700;
  }

  /* AI Wizard Styles */
  .veil-ai-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .ai-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--veil-brand);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .veil-ai-header h4 {
    margin: 0;
    font-size: var(--text-md, 14px);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .veil-ai-idea-textarea {
    font-family: var(--font-sans);
    resize: vertical;
    font-size: var(--text-xs, 12px);
  }

  .veil-ai-steps {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: var(--space-3, 0.75rem);
    margin-top: var(--space-2, 0.5rem);
  }

  .step-card {
    display: flex;
    gap: var(--space-2, 0.5rem);
    padding: var(--space-3, 0.75rem);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg, 0.75rem);
  }

  .step-num {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .step-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 11px;
  }

  .step-body strong {
    color: var(--veil-text-primary);
  }

  .step-body span {
    color: var(--veil-text-secondary);
    line-height: 1.35;
  }

  /* Media Background Styles */
  .veil-media-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 0.75rem);
  }

  .veil-form-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .opacity-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .opacity-val {
    font-size: 12px;
    font-weight: 700;
    color: var(--veil-brand);
  }

  .veil-range-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    border-radius: 999px;
    background: linear-gradient(to right, var(--veil-brand) 0%, var(--veil-brand) var(--range-pct, 26%), var(--veil-bg-overlay) var(--range-pct, 26%), var(--veil-bg-overlay) 100%);
    outline: none;
    cursor: pointer;
    transition: filter 150ms ease;
  }
  .veil-range-slider:hover { filter: brightness(1.08); }
  .veil-range-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--veil-brand);
    border: 3px solid var(--veil-bg-elevated);
    box-shadow: 0 2px 8px rgba(0,0,0,0.35), 0 0 0 1px color-mix(in srgb, var(--veil-brand) 40%, transparent);
    cursor: pointer;
    transition: transform 150ms cubic-bezier(0.16,1,0.3,1), box-shadow 150ms ease;
  }
  .veil-range-slider::-webkit-slider-thumb:hover { transform: scale(1.12); box-shadow: 0 4px 12px rgba(0,0,0,0.4), 0 0 0 2px var(--veil-brand); }
  .veil-range-slider:active::-webkit-slider-thumb { transform: scale(0.98); }
  .veil-range-slider::-moz-range-thumb {
    width: 18px; height: 18px; border-radius: 50%; background: var(--veil-brand); border: 3px solid var(--veil-bg-elevated);
    box-shadow: 0 2px 8px rgba(0,0,0,0.35); cursor: pointer;
  }
  .veil-range-slider::-moz-range-track { height: 6px; border-radius: 999px; background: var(--veil-bg-overlay); }

  .veil-media-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2, 0.5rem);
    margin-top: var(--space-2, 0.5rem);
  }

  /* Import / Export Styles */
  .impexp-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-4, 1rem);
  }

  .impexp-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-2, 0.5rem);
    padding: var(--space-5, 1.25rem);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl, 1rem);
  }

  .impexp-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-xl, 1rem);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--veil-brand);
  }

  .import-buttons {
    display: flex;
    align-items: center;
    gap: var(--space-2, 0.5rem);
    flex-wrap: wrap;
    justify-content: center;
  }

  .veil-import-textarea {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    tab-size: 2;
    min-height: 220px !important;
    display: block;
    width: 100%;
    background: var(--veil-bg-void) !important;
    border: 1px solid var(--veil-border) !important;
    color: var(--veil-text-primary);
  }
  .veil-import-textarea:focus {
    border-color: var(--veil-brand) !important;
    box-shadow: 0 0 0 3px hsl(262 72% 60% / 0.18);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }

  .veil-media-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .veil-media-input-row .veil-input {
    flex: 1;
    min-width: 0;
  }

  .media-type-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-radius: 999px;
    background: var(--veil-bg-overlay);
    color: var(--veil-text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .media-type-badge.badge-video {
    background: rgba(139, 92, 246, 0.15);
    color: #a78bfa;
  }

  .media-type-badge.badge-image {
    background: rgba(16, 185, 129, 0.15);
    color: #34d399;
  }

  .media-type-badge.badge-page {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
  }

  .media-type-badge.badge-small {
    font-size: 9px;
    padding: 1px 6px;
  }

  .spinner-tiny {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--veil-text-muted);
    border-top-color: var(--veil-brand);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .scrape-results {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .scrape-results-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-secondary);
  }

  .scrape-results-header :global(svg) {
    color: var(--veil-brand);
    flex-shrink: 0;
  }

  .scrape-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(128px, 1fr));
    gap: 10px;
    max-height: 300px;
    overflow-y: auto;
    padding: 2px;
  }

  .scrape-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md, 0.5rem);
    transition:
      transform var(--t-fast, 150ms ease),
      border-color var(--t-fast, 150ms ease),
      box-shadow var(--t-fast, 150ms ease);
  }

  .scrape-card:hover {
    transform: translateY(-2px);
    border-color: var(--veil-brand);
    box-shadow:
      0 0 0 1px var(--veil-brand),
      0 6px 14px rgba(0, 0, 0, 0.3);
  }

  .scrape-thumb {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 16 / 10;
    overflow: hidden;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm, 0.375rem);
    background: var(--veil-bg-elevated);
    cursor: pointer;
  }

  .scrape-thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .scrape-thumb .media-type-badge {
    position: absolute;
    top: 5px;
    left: 5px;
    z-index: 1;
  }

  .scrape-play-tile {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--veil-bg-overlay);
    color: var(--veil-text-muted);
  }

  .scrape-host {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    color: var(--veil-text-muted);
  }

  .scrape-actions {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .scrape-actions .btn {
    min-width: 0;
    width: 100%;
  }

  /* Skeleton tiles while scraping */
  .scrape-skeleton .scrape-thumb,
  .skeleton-line {
    animation: veil-pulse 1.2s ease-in-out infinite;
  }

  .skeleton-line {
    height: 8px;
    border-radius: 999px;
    background: var(--veil-bg-overlay);
  }

  @keyframes veil-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .scrape-card,
    .scrape-card:hover {
      transform: none;
      transition: none;
    }
    .scrape-skeleton .scrape-thumb,
    .skeleton-line {
      animation: none;
    }
  }

  .saved-themes-section {
    margin-top: var(--space-3, 0.75rem);
    padding-top: var(--space-3, 0.75rem);
    border-top: 1px solid var(--veil-border-subtle);
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 0.5rem);
  }

  .saved-themes-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .saved-themes-header h4 {
    margin: 0;
    font-size: var(--text-sm, 13px);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .saved-themes-empty {
    font-size: 12px;
    color: var(--veil-text-muted);
    margin: 0;
    padding: var(--space-2, 0.5rem) 0;
  }

  .saved-themes-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 200px;
    overflow-y: auto;
  }

  .saved-theme-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2, 0.5rem);
    padding: 6px 8px;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md, 0.5rem);
  }

  .saved-theme-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .saved-theme-name-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .saved-theme-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--veil-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .saved-theme-date {
    font-size: 10px;
    color: var(--veil-text-muted);
  }

  .saved-theme-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  /* Playlist Styles */
  .veil-playlist-section {
    margin-top: var(--space-3, 0.75rem);
    padding: var(--space-3, 0.75rem);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg, 0.75rem);
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 0.5rem);
  }

  .veil-playlist-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .veil-playlist-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-xs, 12px);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .veil-playlist-tools {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .veil-playlist-grid {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 180px;
    overflow-y: auto;
  }

  .veil-playlist-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 6px 8px;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
    transition: border-color var(--t-base, 0.15s ease);
  }

  .veil-playlist-card.active {
    border-color: var(--veil-brand);
    background: color-mix(in srgb, var(--veil-brand) 8%, var(--veil-bg-elevated));
  }

  .veil-playlist-info {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }

  .veil-playlist-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }

  .veil-playlist-title-text {
    font-size: 11px;
    font-weight: 700;
    color: var(--veil-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .veil-playlist-url {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--veil-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .veil-playlist-card-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .veil-playlist-chips {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .playlist-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    color: var(--veil-text-secondary);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: 999px;
    cursor: pointer;
    transition:
      color var(--t-fast, 150ms ease),
      border-color var(--t-fast, 150ms ease),
      background var(--t-fast, 150ms ease);
  }

  .playlist-chip:hover {
    color: var(--veil-text-primary);
    border-color: var(--veil-border);
  }

  .playlist-chip.active {
    background: var(--veil-brand);
    border-color: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
  }

  .playlist-chip.active .chip-count {
    background: rgba(255, 255, 255, 0.22);
    color: var(--veil-brand-foreground, #fff);
  }

  .chip-count {
    min-width: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--veil-bg-overlay);
    color: var(--veil-text-secondary);
    font-size: 10px;
    font-weight: 700;
    text-align: center;
  }

  .playlist-chip.chip-new {
    border-style: dashed;
    color: var(--veil-brand);
  }

  .veil-playlist-actions-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .actions-row-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-secondary);
    margin-right: 2px;
  }

  .mode-segmented {
    display: inline-flex;
    align-items: center;
    padding: 2px;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md, 0.5rem);
    margin-left: auto;
  }

  .mode-btn {
    padding: 3px 9px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    color: var(--veil-text-secondary);
    background: transparent;
    border: none;
    border-radius: calc(var(--radius-md, 0.5rem) - 2px);
    cursor: pointer;
    transition:
      color var(--t-fast, 150ms ease),
      background var(--t-fast, 150ms ease);
  }

  .mode-btn:hover {
    color: var(--veil-text-primary);
  }

  .mode-btn.active {
    background: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
  }

  .interval-select {
    height: 26px;
    padding: 0 6px;
    font-size: 11px;
    font-family: var(--font-sans);
    color: var(--veil-text-primary);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md, 0.5rem);
    cursor: pointer;
  }

  .veil-playlist-empty {
    margin: 0;
    padding: var(--space-2, 0.5rem) 0;
    font-size: 12px;
    color: var(--veil-text-muted);
  }
</style>

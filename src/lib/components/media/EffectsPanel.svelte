<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { effectsStore } from '$lib/effects/store';
  import { addPlugin, addPythonPlugin, removePlugin, downloadSamplePlugin } from '$lib/effects/plugin';
  import type { EffectCategory, EffectParam, ActiveEffect } from '$lib/effects/types';
  import { getEffect, BUILTIN_EFFECTS } from '$lib/effects/effects';
  import { getPlugins } from '$lib/effects/plugin';
  import Icon from '../ui/Icon.svelte';
  import type { IconName } from '../ui/Icon.svelte';
  import { toastStore } from '$lib/stores/notifications';

  const effects = $derived($effectsStore);
  const activeEffects = $derived(effects.activeEffects);
  const activeCount = $derived(activeEffects.length);
  const category = $derived(effects.selectedCategory);

  let selectedParamEffectId = $state<string | null>(null);
  let searchQuery = $state('');
  let pluginFileInput = $state<HTMLInputElement | null>(null);
  let pluginName = $state('');
  let pluginAuthor = $state('');
  let pluginDesc = $state('');
  let showAddPlugin = $state(false);
  let showDiagnostics = $state(true);
  let isUploading = $state(false);
  let isDraggingFile = $state(false);

  let diagnostics = $state<ReturnType<typeof effectsStore.getDiagnostics> | null>(null);
  let diagInterval: ReturnType<typeof setInterval> | null = null;

  function refreshDiagnostics() {
    diagnostics = effectsStore.getDiagnostics();
  }

  let gridWrapEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (effects.panelOpen && gridWrapEl && effects.gridScrollTop > 0) {
      requestAnimationFrame(() => {
        if (gridWrapEl) {
          gridWrapEl.scrollTop = effects.gridScrollTop;
        }
      });
    }
  });

  function handleGridScroll(e: UIEvent) {
    const el = e.currentTarget as HTMLDivElement;
    if (el) {
      effectsStore.setScrollTop(el.scrollTop);
    }
  }

  $effect(() => {
    if (effects.panelOpen) {
      refreshDiagnostics();
      if (!diagInterval) {
        diagInterval = setInterval(refreshDiagnostics, 800);
      }
    } else {
      if (diagInterval) {
        clearInterval(diagInterval);
        diagInterval = null;
      }
    }
  });

  // Keep selectedParamEffectId valid
  $effect(() => {
    if (activeEffects.length > 0) {
      if (!selectedParamEffectId || !activeEffects.some(e => e.effectId === selectedParamEffectId)) {
        selectedParamEffectId = activeEffects[activeEffects.length - 1].effectId;
      }
    } else {
      selectedParamEffectId = null;
    }
  });

  onDestroy(() => {
    if (diagInterval) {
      clearInterval(diagInterval);
      diagInterval = null;
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && effects.panelOpen) {
      effectsStore.closePanel();
    }
  }

  const allAvailableEffects = $derived.by(() => {
    const builtins = BUILTIN_EFFECTS.filter(e => e.id !== 'custom').map(e => ({
      id: e.id,
      name: e.name,
      nameTr: e.nameTr,
      category: e.category,
      thumbnail: e.thumbnail,
      icon: e.icon as IconName,
      isPlugin: false,
    }));
    const plugins = getPlugins().map(p => ({
      id: 'plugin-' + p.manifest.id,
      name: p.manifest.name,
      nameTr: p.manifest.name,
      category: p.manifest.category,
      thumbnail: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
      icon: 'puzzle' as IconName,
      isPlugin: true,
    }));
    return [...builtins, ...plugins];
  });

  const categoryCounts = $derived.by(() => {
    const counts: Record<string, number> = { all: allAvailableEffects.length, face: 0, hand: 0, body: 0, gesture: 0, custom: 0 };
    for (const eff of allAvailableEffects) {
      if (counts[eff.category] !== undefined) {
        counts[eff.category]++;
      }
    }
    return counts;
  });

  const categories = $derived<Array<{ id: EffectCategory | 'all'; label: string; icon: IconName; count: number }>>([
    { id: 'all', label: 'Tümü', icon: 'sparkles', count: categoryCounts.all },
    { id: 'face', label: 'Yüz', icon: 'user', count: categoryCounts.face },
    { id: 'hand', label: 'El', icon: 'hand', count: categoryCounts.hand },
    { id: 'body', label: 'Vücut', icon: 'activity', count: categoryCounts.body },
    { id: 'gesture', label: 'Jest & FX', icon: 'zap', count: categoryCounts.gesture },
    { id: 'custom', label: 'Özel', icon: 'puzzle', count: categoryCounts.custom },
  ]);

  const filteredEffects = $derived.by(() => {
    let list = allAvailableEffects;
    if (category !== 'all') {
      list = list.filter(e => e.category === category);
    }
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      list = list.filter(e => e.nameTr.toLowerCase().includes(q) || e.name.toLowerCase().includes(q));
    }
    return list;
  });

  function isEffectActive(effectId: string): boolean {
    return activeEffects.some(e => e.effectId === effectId);
  }

  function toggleEffect(effectId: string) {
    effectsStore.toggleEffect(effectId);
  }

  function deactivateAll() {
    effectsStore.deactivateAllEffects();
    toastStore.info('Tüm kamera efektleri kapatıldı');
  }

  const selectedEffectObj = $derived(
    selectedParamEffectId ? getEffect(selectedParamEffectId) : null
  );

  const selectedActiveEffect = $derived(
    selectedParamEffectId ? activeEffects.find(e => e.effectId === selectedParamEffectId) : null
  );

  function getParamValue(param: EffectParam): number | string | boolean {
    if (!selectedActiveEffect) return param.default;
    return selectedActiveEffect.params[param.name] ?? param.default;
  }

  function onParamChange(paramName: string, value: number | string | boolean) {
    if (!selectedParamEffectId) return;
    effectsStore.updateParams(selectedParamEffectId, { [paramName]: value });
  }

  function resetSelectedParams() {
    if (!selectedParamEffectId) return;
    effectsStore.resetParams(selectedParamEffectId);
    toastStore.info('Efekt ayarları varsayılana sıfırlandı');
  }

  async function processFile(file: File) {
    isUploading = true;
    try {
      const isPython = file.name.endsWith('.py');
      const isJs = file.name.endsWith('.js');

      if (!isPython && !isJs) {
        toastStore.error('Sadece .js ve .py script dosyaları destekleniyor');
        return;
      }

      const baseName = file.name.replace(/\.(js|py)$/, '');
      let result;
      if (isPython) {
        result = await addPythonPlugin(file, pluginName || baseName, pluginAuthor || 'Kullanıcı', pluginDesc || 'Kullanıcı Python plugini');
      } else {
        result = await addPlugin(file, pluginName || baseName, pluginAuthor || 'Kullanıcı', pluginDesc || 'Kullanıcı plugini');
      }

      if (result.success) {
        toastStore.success(`"${result.plugin!.manifest.name}" eklendi`);
        showAddPlugin = false;
        pluginName = '';
        pluginAuthor = '';
        pluginDesc = '';
        if (pluginFileInput) pluginFileInput.value = '';
      } else {
        toastStore.error(result.error ?? 'Plugin eklenemedi');
      }
    } finally {
      isUploading = false;
      isDraggingFile = false;
    }
  }

  async function handlePluginUpload() {
    const file = pluginFileInput?.files?.[0];
    if (!file || isUploading) return;
    await processFile(file);
  }

  function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    isDraggingFile = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) {
      processFile(file);
    }
  }

  function handleRemovePlugin(pluginId: string) {
    removePlugin(pluginId);
    effectsStore.deactivateEffect('plugin-' + pluginId);
    toastStore.success('Plugin kaldırıldı');
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if effects.panelOpen}
  <div class="veil-fx-backdrop" onclick={() => effectsStore.closePanel()} aria-hidden="true"></div>

  <div class="veil-fx-panel" role="dialog" aria-label="Kamera Efektleri">
    <!-- Header -->
    <div class="veil-fx-header">
      <div class="veil-fx-header-left">
        <div class="veil-fx-sparkle-badge">
          <Icon name="sparkle" size={16} />
        </div>
        <div class="veil-fx-title-wrap">
          <div class="veil-fx-title-row">
            <h3 class="veil-fx-title">Kamera Efektleri</h3>
            {#if activeCount > 0}
              <span class="veil-fx-active-badge">{activeCount} Aktif</span>
            {/if}
          </div>
          <span class="veil-fx-count-pill">{filteredEffects.length} efekt mevcut</span>
        </div>
      </div>
      <div class="veil-fx-header-right">
        {#if activeCount > 0}
          <button
            class="veil-fx-clear-btn"
            onclick={deactivateAll}
            title="Tüm seçili efektleri kaldır"
          >
            <Icon name="trash" size={13} />
            <span>Tümünü Kapat</span>
          </button>
        {/if}
        <button
          class="veil-fx-icon-btn"
          class:active={showDiagnostics}
          onclick={() => { showDiagnostics = !showDiagnostics; if (showDiagnostics) refreshDiagnostics(); }}
          title="Canlı Motor Tanı Ekranı"
          aria-label="Tanı Ekranını Aç"
        >
          <Icon name="cpu" size={15} />
        </button>
        <button
          class="veil-fx-icon-btn"
          onclick={() => effectsStore.closePanel()}
          title="Kapat (Esc)"
          aria-label="Kapat"
        >
          <Icon name="x" size={16} />
        </button>
      </div>
    </div>

    <!-- Live Telemetry HUD Bar -->
    {#if showDiagnostics}
      <div class="veil-fx-hud-bar">
        <div class="veil-fx-hud-status">
          <span
            class="veil-fx-status-dot"
            class:running={diagnostics?.status === 'running'}
            class:idle={diagnostics?.status === 'idle_ready'}
            class:loading={diagnostics?.status === 'loading_models'}
            class:offline={diagnostics?.status === 'camera_off' || diagnostics?.status === 'offline'}
            class:error={diagnostics?.status === 'error'}
          ></span>
          <span class="veil-fx-status-text" class:error={diagnostics?.status === 'error'}>
            {#if diagnostics?.status === 'running'}
              Efekt Motoru Aktif ({activeCount} Katman)
            {:else if diagnostics?.status === 'loading_models'}
              MediaPipe Modelleri Hazırlanıyor...
            {:else if diagnostics?.status === 'idle_ready'}
              Hazır — Efekt Seçin
            {:else if diagnostics?.status === 'error'}
              <span class="veil-fx-error-chip">
                <Icon name="warning" size={12} />
                {diagnostics.error || 'Motor Hatası'}
              </span>
            {:else}
              Kamera Beklemede • GPU Hazır
            {/if}
          </span>
        </div>

        <div class="veil-fx-hud-metrics">
          <span class="veil-fx-metric-pill fps">
            <strong>{diagnostics?.fps && diagnostics.fps > 0 ? diagnostics.fps : '60'}</strong> FPS
          </span>
          {#if diagnostics?.videoSize && diagnostics.videoSize.width > 0}
            <span class="veil-fx-metric-pill">
              {diagnostics.videoSize.width}×{diagnostics.videoSize.height}
            </span>
          {:else}
            <span class="veil-fx-metric-pill">
              WebGL 2.0
            </span>
          {/if}
          <span class="veil-fx-metric-pill model">
            Yüz: {diagnostics?.models.face === 'loaded' ? '✓' : diagnostics?.models.face === 'loading' ? '⏳' : 'Hazır'}
          </span>
          <span class="veil-fx-metric-pill model">
            El: {diagnostics?.models.hand === 'loaded' ? '✓' : diagnostics?.models.hand === 'loading' ? '⏳' : 'Hazır'}
          </span>
        </div>
      </div>
    {/if}

    <!-- Search Bar -->
    <div class="veil-fx-search-bar">
      <Icon name="search" size={14} class="veil-fx-search-icon" />
      <input
        type="text"
        placeholder="Efekt ara... (ör. Matrix, Hayalet, Vizör, Altın Taç)"
        bind:value={searchQuery}
        class="veil-fx-search-input"
      />
      {#if searchQuery}
        <button class="veil-fx-search-clear" onclick={() => (searchQuery = '')} aria-label="Aramayı Temizle">
          <Icon name="x" size={12} />
        </button>
      {/if}
    </div>

    <!-- Visibility Selector -->
    <div class="veil-fx-visibility">
      <div class="veil-fx-vis-header">
        <span class="veil-fx-vis-label">Yayın Modu</span>
        <span class="veil-fx-vis-hint">
          {effects.visibility === 'self' ? 'Sadece kendi ekranında görünür' : 'Görüşmedeki herkese yayınlanır'}
        </span>
      </div>
      <div class="veil-fx-vis-toggle">
        <button
          class="veil-fx-vis-btn"
          class:active={effects.visibility === 'self'}
          onclick={() => effectsStore.setVisibility('self')}
        >
          <Icon name="user" size={13} />
          Sadece Ben
        </button>
        <button
          class="veil-fx-vis-btn"
          class:active={effects.visibility === 'broadcast'}
          onclick={() => effectsStore.setVisibility('broadcast')}
        >
          <Icon name="radio" size={13} />
          Herkes
        </button>
      </div>
    </div>

    <!-- Category Filters -->
    <div class="veil-fx-categories">
      {#each categories as cat}
        <button
          class="veil-fx-cat-btn"
          class:active={category === cat.id}
          onclick={() => effectsStore.setCategory(cat.id)}
        >
          <Icon name={cat.icon} size={13} />
          <span>{cat.label}</span>
          <span class="veil-fx-cat-count">{cat.count}</span>
        </button>
      {/each}
    </div>

    <!-- Active Effect Parameters Control Accordion -->
    {#if activeCount > 0}
      <div class="veil-fx-active-params-section">
        <div class="veil-fx-params-header">
          <div class="veil-fx-params-title-row">
            <Icon name="sliders" size={14} />
            <span class="veil-fx-params-heading">Aktif Efekt Ayarları</span>
          </div>
          {#if selectedEffectObj && selectedEffectObj.params.length > 0}
            <button class="veil-fx-reset-params-btn" onclick={resetSelectedParams} title="Varsayılana Sıfırla">
              <Icon name="refresh-cw" size={12} />
              Sıfırla
            </button>
          {/if}
        </div>

        <!-- Active Effect Selector Tabs (if multiple) -->
        {#if activeCount > 1}
          <div class="veil-fx-active-tabs">
            {#each activeEffects as act}
              {@const eff = getEffect(act.effectId)}
              <div
                class="veil-fx-active-tab"
                class:selected={selectedParamEffectId === act.effectId}
                onclick={() => (selectedParamEffectId = act.effectId)}
                role="button"
                tabindex="0"
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') selectedParamEffectId = act.effectId; }}
              >
                <span class="veil-fx-tab-dot"></span>
                <span>{eff?.nameTr || act.effectId}</span>
                <button
                  class="veil-fx-tab-close"
                  onclick={(e) => { e.stopPropagation(); effectsStore.deactivateEffect(act.effectId); }}
                  title="Kaldır"
                >
                  ×
                </button>
              </div>
            {/each}
          </div>
        {/if}

        <!-- Parameter Controls -->
        {#if selectedEffectObj && selectedEffectObj.params.length > 0}
          <div class="veil-fx-param-grid">
            {#each selectedEffectObj.params as param}
              <div class="veil-fx-param-row">
                <div class="veil-fx-param-label-wrap">
                  <label for="param-{param.name}" class="veil-fx-param-label">{param.label}</label>
                  {#if param.type === 'number'}
                    <span class="veil-fx-param-val">{getParamValue(param)}</span>
                  {/if}
                </div>

                {#if param.type === 'number'}
                  <div class="veil-fx-slider-wrap">
                    <input
                      id="param-{param.name}"
                      type="range"
                      min={param.min ?? 0}
                      max={param.max ?? 100}
                      step={param.step ?? 1}
                      value={getParamValue(param) as number}
                      oninput={(e) => onParamChange(param.name, parseFloat(e.currentTarget.value))}
                      class="veil-fx-slider"
                    />
                  </div>
                {:else if param.type === 'color'}
                  <div class="veil-fx-color-wrap">
                    <input
                      id="param-{param.name}"
                      type="color"
                      value={getParamValue(param) as string}
                      oninput={(e) => onParamChange(param.name, e.currentTarget.value)}
                      class="veil-fx-color-input"
                    />
                    <span class="veil-fx-color-hex">{getParamValue(param)}</span>
                  </div>
                {:else if param.type === 'boolean'}
                  <label class="veil-fx-toggle-switch">
                    <input
                      type="checkbox"
                      checked={getParamValue(param) as boolean}
                      onchange={(e) => onParamChange(param.name, e.currentTarget.checked)}
                    />
                    <span class="veil-fx-toggle-slider"></span>
                  </label>
                {:else if param.type === 'select'}
                  <select
                    id="param-{param.name}"
                    value={getParamValue(param) as string}
                    onchange={(e) => onParamChange(param.name, e.currentTarget.value)}
                    class="veil-fx-select"
                  >
                    {#each param.options ?? [] as opt}
                      <option value={opt}>{opt}</option>
                    {/each}
                  </select>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="veil-fx-no-params">
            <span>Bu efekt için ek ayar bulunmuyor.</span>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Effects Grid -->
    <div class="veil-fx-grid-wrap" bind:this={gridWrapEl} onscroll={handleGridScroll}>
      {#if filteredEffects.length === 0}
        <div class="veil-fx-empty">
          <Icon name="search" size={28} />
          <p>Eşleşen efekt bulunamadı</p>
          <button class="veil-fx-empty-btn" onclick={() => { searchQuery = ''; effectsStore.setCategory('all'); }}>
            Filtreleri Temizle
          </button>
        </div>
      {:else}
        <div class="veil-fx-grid">
          {#each filteredEffects as eff (eff.id)}
            {@const active = isEffectActive(eff.id)}
            <button
              class="veil-fx-card"
              class:active
              onclick={() => toggleEffect(eff.id)}
              aria-label={eff.nameTr}
              aria-pressed={active}
            >
              <!-- Card Thumbnail Box -->
              <div class="veil-fx-thumb" style="background: {eff.thumbnail}">
                <div class="veil-fx-thumb-icon">
                  <Icon name={eff.icon} size={22} />
                </div>

                <!-- Selection Checkmark Badge -->
                {#if active}
                  <div class="veil-fx-card-check">
                    <Icon name="check" size={12} strokeWidth={2.8} />
                  </div>
                {/if}

                {#if eff.isPlugin}
                  <span class="veil-fx-plugin-tag">Plugin</span>
                {/if}
              </div>

              <!-- Card Name & Category Footer -->
              <div class="veil-fx-card-meta">
                <span class="veil-fx-card-name" title={eff.nameTr}>{eff.nameTr}</span>
                <span class="veil-fx-card-cat">{eff.category}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Plugin Add / Drop Area -->
    <div class="veil-fx-footer">
      {#if !showAddPlugin}
        <button
          class="veil-fx-add-plugin-btn"
          onclick={() => (showAddPlugin = true)}
        >
          <Icon name="plus" size={14} />
          <span>Özel Efekt (Plugin) Yükle</span>
        </button>
      {:else}
        <div
          class="veil-fx-upload-box"
          class:dragging={isDraggingFile}
          ondragover={(e) => { e.preventDefault(); isDraggingFile = true; }}
          ondragleave={() => (isDraggingFile = false)}
          ondrop={handleFileDrop}
          role="region"
          aria-label="Plugin Yükleme Alanı"
        >
          <div class="veil-fx-upload-header">
            <span class="veil-fx-upload-title">Yeni Efekt Plugini (.js veya .py)</span>
            <button class="veil-fx-upload-close" onclick={() => (showAddPlugin = false)}>
              <Icon name="x" size={13} />
            </button>
          </div>

          <div class="veil-fx-upload-inputs">
            <input
              type="text"
              placeholder="Efekt Adı (ör. Neon Kalpler)"
              bind:value={pluginName}
              class="veil-fx-input"
            />
            <input
              type="text"
              placeholder="Geliştirici Adı"
              bind:value={pluginAuthor}
              class="veil-fx-input"
            />
          </div>

          <div
            class="veil-fx-file-dropzone"
            onclick={() => pluginFileInput?.click()}
            role="button"
            tabindex="0"
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') pluginFileInput?.click(); }}
          >
            <Icon name="upload" size={20} />
            <span>Dosyayı buraya sürükle veya seç</span>
            <input
              type="file"
              accept=".js,.py"
              bind:this={pluginFileInput}
              onchange={handlePluginUpload}
              class="veil-fx-hidden-input"
            />
          </div>

          <div class="veil-fx-upload-actions">
            <button class="veil-fx-sample-btn" onclick={() => downloadSamplePlugin('javascript')}>
              <Icon name="download" size={12} />
              Örnek Şablon İndir
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .veil-fx-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(4px);
    z-index: 998;
  }

  .veil-fx-panel {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(540px, 94vw);
    max-height: 88vh;
    background: var(--veil-bg-elevated, #12131a);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 16px;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.65), 0 0 0 1px rgba(255, 255, 255, 0.05);
    z-index: 999;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--veil-text, #f1f5f9);
    font-family: inherit;
  }

  /* Header */
  .veil-fx-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
  }

  .veil-fx-header-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .veil-fx-sparkle-badge {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    background: linear-gradient(135deg, rgba(139, 92, 246, 0.25), rgba(6, 182, 212, 0.25));
    border: 1px solid rgba(139, 92, 246, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #a855f7;
  }

  .veil-fx-title-wrap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .veil-fx-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .veil-fx-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .veil-fx-active-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 999px;
    background: #10b981;
    color: #042f1a;
  }

  .veil-fx-count-pill {
    font-size: 11px;
    color: var(--veil-text-muted, #94a3b8);
  }

  .veil-fx-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .veil-fx-clear-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 9px;
    font-size: 11px;
    font-weight: 500;
    color: #f87171;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-fx-clear-btn:hover {
    background: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.4);
  }

  .veil-fx-icon-btn {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--veil-text-muted, #94a3b8);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-fx-icon-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--veil-text, #f1f5f9);
  }

  .veil-fx-icon-btn.active {
    background: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.4);
    color: #c084fc;
  }

  /* Live HUD Bar */
  .veil-fx-hud-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 18px;
    background: rgba(0, 0, 0, 0.35);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 11px;
  }

  .veil-fx-hud-status {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .veil-fx-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #64748b;
  }

  .veil-fx-status-dot.running {
    background: #10b981;
    box-shadow: 0 0 8px #10b981;
  }

  .veil-fx-status-dot.idle {
    background: #8b5cf6;
  }

  .veil-fx-status-dot.loading {
    background: #f59e0b;
    box-shadow: 0 0 6px #f59e0b;
  }

  .veil-fx-status-dot.error {
    background: var(--veil-danger, hsl(0, 72%, 62%));
  }

  .veil-fx-status-text.error {
    color: var(--veil-warning, hsl(38, 92%, 50%));
  }

  .veil-fx-error-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 2px 8px;
    border: 1px solid var(--veil-border-subtle, hsl(220, 13%, 16%));
    border-radius: 6px;
    background: color-mix(in srgb, var(--veil-danger, hsl(0, 72%, 62%)) 10%, transparent);
    color: var(--veil-text, #f1f5f9);
    font-size: 11px;
    line-height: 1.4;
  }

  .veil-fx-error-chip :global(svg) {
    flex-shrink: 0;
    color: var(--veil-warning, hsl(38, 92%, 50%));
  }

  .veil-fx-status-text {
    font-weight: 500;
    color: var(--veil-text, #f1f5f9);
  }

  .veil-fx-hud-metrics {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .veil-fx-metric-pill {
    padding: 2px 7px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--veil-text-muted, #94a3b8);
    font-family: monospace;
    font-size: 10px;
  }

  .veil-fx-metric-pill.fps {
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.1);
  }

  .veil-fx-metric-pill.model {
    color: #a78bfa;
  }

  /* Search Bar */
  .veil-fx-search-bar {
    position: relative;
    padding: 10px 18px 4px;
    display: flex;
    align-items: center;
  }

  :global(.veil-fx-search-icon) {
    position: absolute;
    left: 28px;
    color: var(--veil-text-muted, #64748b);
    pointer-events: none;
  }

  .veil-fx-search-input {
    width: 100%;
    padding: 8px 32px 8px 32px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 10px;
    color: var(--veil-text, #f1f5f9);
    font-size: 12px;
    outline: none;
    transition: all 0.15s ease;
  }

  .veil-fx-search-input:focus {
    background: rgba(255, 255, 255, 0.07);
    border-color: rgba(139, 92, 246, 0.5);
    box-shadow: 0 0 0 2px rgba(139, 92, 246, 0.15);
  }

  .veil-fx-search-clear {
    position: absolute;
    right: 28px;
    background: transparent;
    border: none;
    color: var(--veil-text-muted, #64748b);
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  /* Visibility */
  .veil-fx-visibility {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 18px;
    gap: 12px;
  }

  .veil-fx-vis-header {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .veil-fx-vis-label {
    font-size: 12px;
    font-weight: 500;
  }

  .veil-fx-vis-hint {
    font-size: 10px;
    color: var(--veil-text-muted, #64748b);
  }

  .veil-fx-vis-toggle {
    display: flex;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 2px;
    gap: 2px;
  }

  .veil-fx-vis-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    font-size: 11px;
    font-weight: 500;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--veil-text-muted, #94a3b8);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-fx-vis-btn.active {
    background: rgba(139, 92, 246, 0.25);
    color: #c084fc;
  }

  /* Categories */
  .veil-fx-categories {
    display: flex;
    gap: 4px;
    padding: 3px 18px 8px;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
  }

  .veil-fx-categories::-webkit-scrollbar {
    display: none;
  }

  .veil-fx-cat-btn {
    display: inline-flex;
    align-items: center;
    gap: 4.5px;
    padding: 4.5px 8px;
    font-size: 11px;
    font-weight: 500;
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    color: var(--veil-text-muted, #94a3b8);
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .veil-fx-cat-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--veil-text, #f1f5f9);
  }

  .veil-fx-cat-btn.active {
    background: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.4);
    color: #d8b4fe;
  }

  .veil-fx-cat-count {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--veil-text-muted, #94a3b8);
  }

  /* Active Parameters Section */
  .veil-fx-active-params-section {
    background: rgba(0, 0, 0, 0.28);
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    padding: 10px 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .veil-fx-params-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .veil-fx-params-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #c084fc;
  }

  .veil-fx-params-heading {
    font-size: 12px;
    font-weight: 600;
  }

  .veil-fx-reset-params-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--veil-text-muted, #94a3b8);
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .veil-fx-reset-params-btn:hover {
    color: var(--veil-text, #f1f5f9);
  }

  .veil-fx-active-tabs {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 2px;
  }

  .veil-fx-active-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 11px;
    color: var(--veil-text-muted, #94a3b8);
    cursor: pointer;
  }

  .veil-fx-active-tab.selected {
    background: rgba(139, 92, 246, 0.2);
    border-color: rgba(139, 92, 246, 0.35);
    color: #e9d5ff;
  }

  .veil-fx-tab-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
  }

  .veil-fx-tab-close {
    border: none;
    background: transparent;
    color: #f87171;
    cursor: pointer;
    font-size: 13px;
    line-height: 1;
    padding: 0;
    margin-left: 2px;
  }

  .veil-fx-param-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .veil-fx-param-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .veil-fx-param-label-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 130px;
  }

  .veil-fx-param-label {
    font-size: 11px;
    color: var(--veil-text-muted, #cbd5e1);
  }

  .veil-fx-param-val {
    font-size: 10px;
    font-family: monospace;
    color: #38bdf8;
  }

  .veil-fx-slider-wrap {
    flex: 1;
    display: flex;
    align-items: center;
  }

  .veil-fx-slider {
    width: 100%;
    accent-color: #8b5cf6;
    height: 4px;
    cursor: pointer;
  }

  .veil-fx-color-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .veil-fx-color-input {
    width: 26px;
    height: 26px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 6px;
    padding: 0;
    background: transparent;
    cursor: pointer;
  }

  .veil-fx-color-hex {
    font-size: 10px;
    font-family: monospace;
    color: var(--veil-text-muted, #94a3b8);
  }

  .veil-fx-select {
    padding: 4px 8px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--veil-text, #f1f5f9);
    font-size: 11px;
    outline: none;
  }

  .veil-fx-toggle-switch {
    position: relative;
    display: inline-block;
    width: 32px;
    height: 18px;
  }

  .veil-fx-toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .veil-fx-toggle-slider {
    position: absolute;
    inset: 0;
    background-color: rgba(255, 255, 255, 0.15);
    border-radius: 18px;
    transition: 0.2s;
    cursor: pointer;
  }

  .veil-fx-toggle-slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: white;
    border-radius: 50%;
    transition: 0.2s;
  }

  .veil-fx-toggle-switch input:checked + .veil-fx-toggle-slider {
    background-color: #8b5cf6;
  }

  .veil-fx-toggle-switch input:checked + .veil-fx-toggle-slider:before {
    transform: translateX(14px);
  }

  .veil-fx-no-params {
    font-size: 11px;
    color: var(--veil-text-muted, #64748b);
    padding: 2px 0;
  }

  /* Grid Wrap */
  .veil-fx-grid-wrap {
    flex: 1;
    overflow-y: auto;
    padding: 12px 18px;
    min-height: 220px;
    max-height: 380px;
  }

  .veil-fx-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .veil-fx-card {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 128px;
    min-height: 128px;
    max-height: 128px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 6px;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease, transform 0.1s ease;
    box-sizing: border-box;
    position: relative;
    overflow: hidden;
    user-select: none;
  }

  .veil-fx-card:hover {
    background: rgba(255, 255, 255, 0.06);
    border-color: rgba(255, 255, 255, 0.18);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .veil-fx-card:active {
    transform: scale(0.98);
  }

  .veil-fx-card.active {
    background: rgba(139, 92, 246, 0.16);
    border-color: #8b5cf6;
    box-shadow: inset 0 0 0 1px rgba(139, 92, 246, 0.5), 0 4px 16px rgba(139, 92, 246, 0.25);
  }

  .veil-fx-thumb {
    width: 100%;
    height: 72px;
    min-height: 72px;
    max-height: 72px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
    flex-shrink: 0;
  }

  .veil-fx-thumb-icon {
    color: #ffffff;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.6));
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .veil-fx-card-check {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #10b981;
    color: #ffffff;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5), 0 0 8px rgba(16, 185, 129, 0.5);
    z-index: 2;
  }

  .veil-fx-plugin-tag {
    position: absolute;
    bottom: 4px;
    left: 4px;
    font-size: 8px;
    font-weight: 700;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.7);
    color: #a78bfa;
    letter-spacing: 0.5px;
  }

  .veil-fx-card-meta {
    display: flex;
    flex-direction: column;
    padding: 6px 2px 2px;
    gap: 1px;
    min-width: 0;
    flex: 1;
    justify-content: center;
  }

  .veil-fx-card-name {
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--veil-text, #f1f5f9);
    line-height: 1.2;
  }

  .veil-fx-card-cat {
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    font-weight: 500;
    color: var(--veil-text-muted, #94a3b8);
  }

  .veil-fx-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 0;
    gap: 10px;
    color: var(--veil-text-muted, #64748b);
  }

  .veil-fx-empty-btn {
    font-size: 11px;
    padding: 5px 12px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--veil-text, #f1f5f9);
    cursor: pointer;
  }

  /* Footer & Plugins */
  .veil-fx-footer {
    padding: 10px 18px 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.015);
  }

  .veil-fx-add-plugin-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px dashed rgba(255, 255, 255, 0.15);
    color: var(--veil-text-muted, #94a3b8);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-fx-add-plugin-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(139, 92, 246, 0.4);
    color: #c084fc;
  }

  .veil-fx-upload-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
  }

  .veil-fx-upload-box.dragging {
    border-color: #8b5cf6;
    background: rgba(139, 92, 246, 0.1);
  }

  .veil-fx-upload-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .veil-fx-upload-title {
    font-size: 12px;
    font-weight: 600;
  }

  .veil-fx-upload-close {
    border: none;
    background: transparent;
    color: var(--veil-text-muted, #94a3b8);
    cursor: pointer;
  }

  .veil-fx-upload-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .veil-fx-input {
    padding: 6px 10px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    color: var(--veil-text, #f1f5f9);
    font-size: 11px;
    outline: none;
  }

  .veil-fx-file-dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 14px;
    border: 1px dashed rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    cursor: pointer;
    font-size: 11px;
    color: var(--veil-text-muted, #94a3b8);
  }

  .veil-fx-file-dropzone:hover {
    border-color: #8b5cf6;
    color: var(--veil-text, #f1f5f9);
  }

  .veil-fx-hidden-input {
    display: none;
  }

  .veil-fx-upload-actions {
    display: flex;
    justify-content: flex-end;
  }

  .veil-fx-sample-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--veil-text-muted, #94a3b8);
    background: transparent;
    border: none;
    cursor: pointer;
  }

  .veil-fx-sample-btn:hover {
    color: #38bdf8;
  }
</style>

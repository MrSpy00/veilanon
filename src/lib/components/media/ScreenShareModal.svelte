<script lang="ts">
  import { mediaStore, SCREEN_SHARE_PRESETS, type ScreenShareOptions } from '$lib/stores/media';
  import Icon from '../ui/Icon.svelte';
  import Toggle from '../ui/Toggle.svelte';

  let {
    open = false,
    onClose,
  }: {
    open?: boolean;
    onClose?: () => void;
  } = $props();

  let selectedResolution = $state<'1080p' | '720p' | '480p'>('1080p');
  let selectedFps = $state<60 | 45 | 30 | 15>(60);
  let shareAudio = $state(true);
  let starting = $state(false);

  const resMap: Record<string, { width: number; height: number }> = {
    '1080p': { width: 1920, height: 1080 },
    '720p': { width: 1280, height: 720 },
    '480p': { width: 854, height: 480 },
  };

  async function startShare() {
    starting = true;
    try {
      const res = resMap[selectedResolution];
      const opts: ScreenShareOptions = {
        resolution: res,
        frameRate: selectedFps,
        audio: shareAudio,
      };
      await mediaStore.startScreenShare(opts);
      onClose?.();
    } catch {
      // toast shown in store
    } finally {
      starting = false;
    }
  }

  function onOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget && !starting) onClose?.();
  }
</script>

{#if open}
  <div class="veil-overlay" role="presentation" onclick={onOverlayClick}>
    <div class="veil-modal veil-ss-modal" role="dialog" aria-modal="true" aria-labelledby="ss-modal-title">
      <div class="veil-ss-header">
        <div class="veil-ss-icon-wrap">
          <Icon name="monitor" size={24} />
        </div>
        <div>
          <h3 id="ss-modal-title" class="veil-ss-title">Ekran Paylaşımı</h3>
          <p class="veil-ss-subtitle">Yayın çözünürlüğü ve kare hızını belirleyin.</p>
        </div>
      </div>

      <div class="veil-ss-body">
        <!-- Resolution Section -->
        <div class="veil-ss-section">
          <div class="veil-ss-label">Çözünürlük</div>
          <div class="veil-ss-grid-3">
            <button
              type="button"
              class="veil-ss-opt"
              class:active={selectedResolution === '1080p'}
              onclick={() => (selectedResolution = '1080p')}
            >
              <span class="veil-ss-opt-title">1080p</span>
              <span class="veil-ss-opt-desc">Full HD (1920x1080)</span>
            </button>
            <button
              type="button"
              class="veil-ss-opt"
              class:active={selectedResolution === '720p'}
              onclick={() => (selectedResolution = '720p')}
            >
              <span class="veil-ss-opt-title">720p</span>
              <span class="veil-ss-opt-desc">HD (1280x720)</span>
            </button>
            <button
              type="button"
              class="veil-ss-opt"
              class:active={selectedResolution === '480p'}
              onclick={() => (selectedResolution = '480p')}
            >
              <span class="veil-ss-opt-title">480p</span>
              <span class="veil-ss-opt-desc">SD (854x480)</span>
            </button>
          </div>
        </div>

        <!-- FPS Section -->
        <div class="veil-ss-section">
          <div class="veil-ss-label">Kare Hızı (FPS)</div>
          <div class="veil-ss-grid-4">
            {#each [60, 45, 30, 15] as fps}
              <button
                type="button"
                class="veil-ss-opt"
                class:active={selectedFps === fps}
                onclick={() => (selectedFps = fps as 60 | 45 | 30 | 15)}
              >
                <span class="veil-ss-opt-title">{fps} FPS</span>
                <span class="veil-ss-opt-desc">{fps >= 60 ? 'Ultra Akıcı' : fps >= 45 ? 'Akıcı' : fps >= 30 ? 'Standart' : 'Tasarruf'}</span>
              </button>
            {/each}
          </div>
        </div>

        <!-- Audio Toggle Section -->
        <div class="veil-ss-toggle-row">
          <div class="veil-ss-toggle-info">
            <span class="veil-ss-toggle-title">Sistem Sesini Dahil Et</span>
            <span class="veil-ss-toggle-desc">Uygulama ve oyun seslerini katılımcılara iletir.</span>
          </div>
          <Toggle checked={shareAudio} onChange={(v) => (shareAudio = v)} />
        </div>
      </div>

      <div class="veil-ss-actions">
        <button class="btn btn-secondary" onclick={onClose} disabled={starting}>İptal</button>
        <button class="btn btn-primary" onclick={startShare} disabled={starting}>
          <Icon name="monitor" size={14} />
          <span>{starting ? 'Başlatılıyor…' : `Yayını Başlat (${selectedResolution} @ ${selectedFps}fps)`}</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .veil-ss-modal {
    max-width: 520px;
    padding: var(--space-5);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-2xl);
  }
  .veil-ss-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-5);
  }
  .veil-ss-icon-wrap {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-xl);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .veil-ss-title {
    font-size: var(--text-lg);
    font-weight: 700;
    margin: 0;
    color: var(--veil-text-primary);
  }
  .veil-ss-subtitle {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin: 2px 0 0;
  }
  .veil-ss-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .veil-ss-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-ss-label {
    font-size: var(--text-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-ss-grid-3 {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
  }
  .veil-ss-grid-4 {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-2);
  }
  .veil-ss-opt {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-3) var(--space-2);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all var(--t-fast);
    text-align: center;
  }
  .veil-ss-opt:hover {
    background: var(--veil-bg-overlay);
    border-color: var(--veil-border-focus);
  }
  .veil-ss-opt.active {
    background: var(--veil-brand-subtle);
    border-color: var(--veil-brand);
    box-shadow: inset 0 0 0 1px var(--veil-brand);
  }
  .veil-ss-opt-title {
    font-size: var(--text-sm);
    font-weight: 700;
    color: var(--veil-text-primary);
  }
  .veil-ss-opt.active .veil-ss-opt-title {
    color: var(--veil-brand);
  }
  .veil-ss-opt-desc {
    font-size: 10px;
    color: var(--veil-text-muted);
    margin-top: 2px;
  }
  .veil-ss-toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
  }
  .veil-ss-toggle-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-ss-toggle-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
  }
  .veil-ss-toggle-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-ss-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>

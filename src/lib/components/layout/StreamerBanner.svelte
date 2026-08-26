<script lang="ts">
  import { streamerMode, type StreamerPreset } from '$lib/stores/streamerMode';
  import Icon from '$lib/components/ui/Icon.svelte';

  let { onOpenSettings, onToggle }: { onOpenSettings?: () => void; onToggle?: () => void } = $props();

  const presetLabels: Record<StreamerPreset, string> = {
    max_privacy: 'Maksimum Gizlilik',
    streamer_balanced: 'Dengeli Yayıncı',
    minimal: 'Minimal',
    custom: 'Özel Ayarlar',
  };

  function handleDisable() {
    if (onToggle) {
      onToggle();
    } else {
      streamerMode.setEnabled(false);
    }
  }
</script>

{#if $streamerMode.enabled}
  <div class="streamer-banner" role="status" aria-live="polite">
    <div class="banner-left">
      <span class="live-dot" aria-hidden="true"></span>
      <Icon name="broadcast" size={16} class="banner-icon" />
      <span class="banner-title">YAYINCI MODU AKTİF</span>
      <span class="banner-badge">{presetLabels[$streamerMode.preset]}</span>
    </div>

    <div class="banner-center">
      <span class="banner-tip">Hassas bilgiler sansürlendi (Stil: <strong>{$streamerMode.maskStyle}</strong>)</span>
    </div>

    <div class="banner-right">
      {#if onOpenSettings}
        <button type="button" class="banner-btn secondary" onclick={onOpenSettings} title="Yayıncı Modu Ayarlarını Aç">
          <Icon name="settings" size={14} />
          <span>Ayarlar</span>
        </button>
      {/if}
      <button type="button" class="banner-btn danger" onclick={handleDisable} title="Yayıncı Modunu Kapat">
        <Icon name="x" size={14} />
        <span>Devre Dışı Bırak</span>
      </button>
    </div>
  </div>
{/if}

<style>
  .streamer-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 16px;
    background: linear-gradient(90deg, var(--veil-brand), color-mix(in srgb, var(--veil-brand) 85%, var(--veil-bg-elevated) 15%));
    color: var(--veil-brand-foreground, #ffffff);
    font-size: 12px;
    font-weight: 500;
    z-index: 900;
    box-shadow: 0 4px 14px var(--veil-theme-glow, rgba(0, 0, 0, 0.2));
    border-bottom: 1px solid var(--veil-brand-border, rgba(255, 255, 255, 0.2));
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    animation: slideDown 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideDown {
    from {
      transform: translateY(-100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .banner-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .live-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--veil-danger, #ef4444);
    box-shadow: 0 0 8px var(--veil-danger, #ef4444);
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0% {
      transform: scale(0.95);
      opacity: 0.8;
    }
    50% {
      transform: scale(1.25);
      opacity: 1;
    }
    100% {
      transform: scale(0.95);
      opacity: 0.8;
    }
  }

  :global(.banner-icon) {
    animation: rotateSlow 8s linear infinite;
  }

  .banner-title {
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 11.5px;
    color: var(--veil-brand-foreground, #ffffff);
  }

  .banner-badge {
    background: color-mix(in srgb, var(--veil-brand-foreground, #ffffff) 15%, transparent);
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    border: 1px solid color-mix(in srgb, var(--veil-brand-foreground, #ffffff) 25%, transparent);
    color: var(--veil-brand-foreground, #ffffff);
  }

  .banner-center {
    display: none;
  }

  @media (min-width: 768px) {
    .banner-center {
      display: flex;
      align-items: center;
      opacity: 0.95;
      color: var(--veil-brand-foreground, #ffffff);
    }
  }

  .banner-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .banner-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: var(--radius-sm, 4px);
    font-size: 11px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .banner-btn.secondary {
    background: color-mix(in srgb, var(--veil-brand-foreground, #ffffff) 18%, transparent);
    color: var(--veil-brand-foreground, #ffffff);
    border: 1px solid color-mix(in srgb, var(--veil-brand-foreground, #ffffff) 30%, transparent);
  }

  .banner-btn.secondary:hover {
    background: color-mix(in srgb, var(--veil-brand-foreground, #ffffff) 28%, transparent);
  }

  .banner-btn.danger {
    background: var(--veil-danger, #ef4444);
    color: #ffffff;
  }

  .banner-btn.danger:hover {
    filter: brightness(1.15);
    transform: translateY(-1px);
  }
</style>

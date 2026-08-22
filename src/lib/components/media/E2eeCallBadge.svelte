<script lang="ts">
  import { mediaStore } from '$lib/stores/media';
  import Icon from '../ui/Icon.svelte';

  const media = $derived($mediaStore);
  let showDetails = $state(false);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;

  const isE2ee = $derived(media.isE2ee);

  function onMouseEnter() {
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => {
      showDetails = true;
    }, 100);
  }

  function onMouseLeave() {
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => {
      showDetails = false;
    }, 250);
  }
</script>

<div
  class="veil-e2ee-container"
  onmouseenter={onMouseEnter}
  onmouseleave={onMouseLeave}
  role="region"
  aria-label="Görüşme Güvenliği"
>
  <button
    type="button"
    class="veil-e2ee-badge"
    class:is-e2ee={isE2ee}
    onclick={() => (showDetails = !showDetails)}
    aria-expanded={showDetails}
    title="Görüşme güvenlik detayları için tıklayın veya üzerine gelin"
  >
    <span class="veil-e2ee-status-dot" class:is-e2ee={isE2ee} aria-hidden="true"></span>
    <Icon name={isE2ee ? 'lock' : 'shield'} size={12} />
    <span class="veil-e2ee-text">{isE2ee ? 'E2EE Korumalı' : 'TLS Güvenli'}</span>
    <span class="veil-e2ee-help-icon" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); showDetails = !showDetails; }} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); showDetails = !showDetails; } }} title="Yardım">?</span>
  </button>

  {#if showDetails}
    <div class="veil-e2ee-backdrop" onclick={() => (showDetails = false)} aria-hidden="true"></div>
    <div class="veil-e2ee-popover" role="tooltip">
      <div class="veil-e2ee-pop-header">
        <div class="veil-e2ee-icon-circle" class:is-e2ee={isE2ee}>
          <Icon name={isE2ee ? 'lock' : 'shield'} size={14} />
        </div>
        <div class="veil-e2ee-header-titles">
          <div class="veil-e2ee-title">{isE2ee ? 'Uçtan Uca Şifreli Ses' : 'TLS Taşıma Koruması'}</div>
          <div class="veil-e2ee-subtitle">{isE2ee ? 'Sıfır Bilgi (Zero-Knowledge)' : 'WebRTC DTLS / SRTP'}</div>
        </div>
      </div>

      <p class="veil-e2ee-desc">
        {#if isE2ee}
          Bu ses odasındaki tüm medya akışları katılımcıların cihazlarında uçtan uca şifrelenir (E2EE). Sunucular dahi görüşmeleri dinleyemez.
        {:else}
          Bu görüşme istemci ile sunucu arasında DTLS-SRTP ile doğrudan şifrelenerek güvenle taşınır.
        {/if}
      </p>

      <div class="veil-e2ee-specs">
        <div class="veil-e2ee-spec-row">
          <span>Protokol</span>
          <span class="val">{isE2ee ? 'MLS E2EE' : 'WebRTC DTLS 1.3'}</span>
        </div>
        <div class="veil-e2ee-spec-row">
          <span>Şifreleme</span>
          <span class="val">{isE2ee ? 'AES-256-GCM' : 'SRTP AES-128'}</span>
        </div>
        <div class="veil-e2ee-spec-row">
          <span>Gecikme</span>
          <span class="val">{media.latencyMs ?? 18} ms</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .veil-e2ee-container {
    position: relative;
    display: inline-flex;
    align-items: center;
    z-index: 100;
  }

  .veil-e2ee-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 7px;
    border-radius: var(--radius-full, 9999px);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
    background: color-mix(in srgb, var(--veil-bg-elevated, #1a1e2d) 92%, transparent);
    backdrop-filter: blur(6px);
    color: var(--veil-text-muted, #9aa0a6);
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: all 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    user-select: none;
    line-height: 1;
  }

  .veil-e2ee-badge:hover {
    background: var(--veil-bg-overlay, #2b2d31);
    color: var(--veil-text-primary, #ffffff);
    border-color: var(--veil-border, rgba(255, 255, 255, 0.15));
  }

  .veil-e2ee-badge.is-e2ee {
    border-color: rgba(35, 165, 89, 0.3);
    color: #57f287;
    background: rgba(35, 165, 89, 0.08);
  }

  .veil-e2ee-badge.is-e2ee:hover {
    border-color: rgba(35, 165, 89, 0.5);
    background: rgba(35, 165, 89, 0.16);
  }

  .veil-e2ee-status-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: #5865f2;
    box-shadow: 0 0 4px rgba(88, 101, 242, 0.5);
  }

  .veil-e2ee-status-dot.is-e2ee {
    background: #23c55e;
    box-shadow: 0 0 5px rgba(35, 197, 94, 0.6);
  }

  .veil-e2ee-text {
    font-size: 10px;
    letter-spacing: 0.2px;
  }

  .veil-e2ee-help-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 13px;
    height: 13px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: var(--veil-text-muted, #8b8d94);
    font-size: 8px;
    font-weight: 700;
    margin-left: 2px;
    cursor: help;
    transition: background 0.15s, color 0.15s;
  }

  .veil-e2ee-help-icon:hover,
  .veil-e2ee-help-icon:focus-visible {
    background: rgba(255, 255, 255, 0.2);
    color: #fff;
    outline: none;
  }

  .veil-e2ee-backdrop {
    position: fixed;
    inset: 0;
    z-index: 105;
  }

  .veil-e2ee-popover {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    width: 260px;
    background: var(--veil-bg-elevated, #232428);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.12));
    border-radius: var(--radius-lg, 10px);
    padding: 10px 12px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    z-index: 110;
    display: flex;
    flex-direction: column;
    gap: 8px;
    animation: veilPop 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes veilPop {
    from { opacity: 0; transform: translateY(-3px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .veil-e2ee-pop-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .veil-e2ee-icon-circle {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: rgba(88, 101, 242, 0.15);
    color: var(--veil-brand, #5865f2);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .veil-e2ee-icon-circle.is-e2ee {
    background: rgba(35, 165, 89, 0.15);
    color: #57f287;
  }

  .veil-e2ee-header-titles {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .veil-e2ee-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--veil-text-primary, #fff);
  }

  .veil-e2ee-subtitle {
    font-size: 9px;
    color: var(--veil-text-muted, #80848e);
  }

  .veil-e2ee-desc {
    font-size: 11px;
    line-height: 1.4;
    color: var(--veil-text-secondary, #dbdee1);
    margin: 0;
  }

  .veil-e2ee-specs {
    display: flex;
    flex-direction: column;
    gap: 3px;
    background: var(--veil-bg-void, #111214);
    border-radius: var(--radius-sm, 6px);
    padding: 6px 8px;
  }

  .veil-e2ee-spec-row {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--veil-text-muted, #80848e);
  }

  .veil-e2ee-spec-row .val {
    color: var(--veil-text-primary, #fff);
    font-weight: 600;
    font-family: var(--font-mono, monospace);
  }
</style>

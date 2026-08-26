<script lang="ts">
  import { authStore } from '$lib/stores/auth';
  import { uiStore } from '$lib/stores/ui';
  import { mediaStore } from '$lib/stores/media';
  import { spaceStore } from '$lib/stores/spaces';
  import UserStatusMenu from '../ui/UserStatusMenu.svelte';
  import Icon from '../ui/Icon.svelte';
  import ScreenShareModal from '../media/ScreenShareModal.svelte';

  const auth = $derived($authStore);
  const ui = $derived($uiStore);
  const media = $derived($mediaStore);
  const spaces = $derived($spaceStore);

  let audioPopOpen = $state(false);
  let screenShareModalOpen = $state(false);
  let masterVolume = $state(100);

  const channelName = $derived(
    (function() {
      if (!media.channelId) return 'Ses Kanalı';
      const ch = Object.values(spaces.channelsBySpace).flat().find(c => c.id === media.channelId)
        ?? spaces.dmChannels.find(c => c.id === media.channelId);
      if (ch?.name && ch.name.length === 36 && ch.name.includes('-')) return 'Ses Kanalı';
      return ch?.name ?? 'Ses Kanalı';
    })()
  );

  function navigateToCallChannel() {
    if (!media.channelId) return;
    const dm = spaces.dmChannels.find(d => d.id === media.channelId);
    if (dm) {
      uiStore.navigateDm(dm.id);
    } else {
      for (const spaceId in spaces.channelsBySpace) {
        const ch = spaces.channelsBySpace[spaceId].find(c => c.id === media.channelId);
        if (ch) {
          uiStore.navigate(spaceId, ch.id);
          return;
        }
      }
    }
  }
</script>

<div class="veil-bottom-bar" role="region" aria-label="Kullanıcı ve ses paneli">
  <!-- Voice connection panel (Discord-style) — only while connected -->
  {#if media.isInCall && media.channelId}
    <div class="veil-voice-connection" role="status" aria-label="Ses bağlantısı">
      <button
        type="button"
        class="veil-vc-left"
        onclick={navigateToCallChannel}
        title="Görüşme Odasına Git"
      >
        <span
          class="veil-vc-signal"
          class:good={media.connectionState === 'connected' && (media.latencyMs === null || media.latencyMs < 150)}
          class:mid={media.connectionState === 'connected' && media.latencyMs !== null && media.latencyMs >= 150}
          aria-hidden="true"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 20h.01"/><path d="M7 20v-4"/><path d="M12 20v-8"/><path d="M17 20V8"/><path d="M22 4v16"/></svg>
        </span>
        <div class="veil-vc-meta">
          <span class="veil-vc-channel-name"><Icon name="volume" size={12} /> {channelName}</span>
          <span class="veil-vc-status">
            {#if media.connectionState === 'reconnecting'}
              yeniden bağlanıyor…
            {:else if media.connectionState === 'connecting'}
              bağlanıyor…
            {:else}
              {media.latencyMs ?? 20} ms · Ses Bağlandı
            {/if}
            {#if media.isE2ee} · E2EE{/if}
          </span>
        </div>
      </button>

      <div class="veil-vc-actions">
        <!-- Camera Toggle -->
        <button
          class="btn-icon veil-vc-btn"
          class:active={media.isCameraOn}
          class:danger={!media.isCameraOn}
          title={media.isCameraOn ? 'Kamerayı Kapat' : 'Kamerayı Aç'}
          aria-label="Kamerayı aç/kapat"
          aria-pressed={media.isCameraOn}
          onclick={() => mediaStore.toggleCamera()}
        >
          <Icon name={media.isCameraOn ? 'camera' : 'camera-off'} size={14} />
        </button>

        <!-- Screen Share Toggle -->
        <button
          class="btn-icon veil-vc-btn"
          class:active={media.isScreenSharing}
          class:danger={!media.isScreenSharing}
          title={media.isScreenSharing ? 'Ekran Paylaşımını Durdur' : 'Ekran Paylaş'}
          aria-label="Ekran paylaşımı"
          aria-pressed={media.isScreenSharing}
          onclick={() => {
            if (media.isScreenSharing) {
              mediaStore.stopScreenShare();
            } else {
              screenShareModalOpen = true;
            }
          }}
        >
          <Icon name="monitor" size={14} />
        </button>

        <!-- Audio Quick Settings Popover Trigger -->
        <button
          class="btn-icon veil-vc-btn"
          class:active={audioPopOpen}
          title="Ses Düzeyi & Ayarları"
          aria-label="Ses düzeyi ve ayarları"
          onclick={() => (audioPopOpen = !audioPopOpen)}
        >
          <Icon name="volume" size={14} />
        </button>

        <!-- Leave Call Button -->
        <button
          class="btn-icon veil-vc-btn veil-vc-leave"
          title="Kanaldan ayrıl"
          aria-label="Ses kanalından ayrıl"
          onclick={() => mediaStore.leaveVoice()}
        >
          <Icon name="phone-off" size={14} />
        </button>
      </div>

      {#if audioPopOpen}
        <div class="veil-audio-pop-backdrop" onclick={() => (audioPopOpen = false)} aria-hidden="true"></div>
        <div class="veil-audio-pop" role="dialog" aria-label="Genel ses düzeyi">
          <div class="veil-audio-pop-header">
            <span class="veil-audio-pop-title">Genel Ses Düzeyi</span>
            <span class="veil-audio-pop-val">%{masterVolume}</span>
          </div>
          <div class="veil-audio-pop-row">
            <Icon name={masterVolume === 0 ? 'volume-x' : 'volume'} size={15} />
            <input
              type="range"
              min="0"
              max="100"
              bind:value={masterVolume}
              oninput={(e) => {
                const v = Number((e.target as HTMLInputElement).value);
                mediaStore.setMasterVolume(v / 100);
              }}
              class="veil-slider"
              aria-label="Genel ses düzeyi"
            />
          </div>
          <button
            class="btn btn-secondary btn-sm"
            style="width:100%; font-size: var(--text-xs); margin-top:2px;"
            onclick={() => { audioPopOpen = false; uiStore.openModal('settings', { tab: 'audio-video' }); }}
          >
            <Icon name="settings" size={13} />
            Gelişmiş Ses Ayarları
          </button>
        </div>
      {/if}
    </div>

    <ScreenShareModal
      open={screenShareModalOpen}
      onClose={() => (screenShareModalOpen = false)}
    />
  {/if}

  <!-- User status & action panel -->
  <div class="veil-user-panel" role="contentinfo">
    <UserStatusMenu
      name={auth.identity?.displayName ?? 'veilanon'}
      username={auth.identity?.username ?? ''}
      avatarHash={auth.identity?.avatarHash ?? null}
      presence={ui.presence}
      placement="up"
      showLabel
      actions={[
        {
          icon: 'user',
          label: 'Profili Gör',
          onClick: () => uiStore.openModal('user-profile', {
            userId: auth.identity?.id ?? '',
            username: auth.identity?.username ?? '',
            displayName: auth.identity?.displayName ?? '',
            avatarHash: auth.identity?.avatarHash ?? null,
            onlineStatus: ui.presence,
          }),
        },
        {
          icon: 'settings',
          label: 'Ayarlar',
          onClick: () => uiStore.openModal('settings'),
        },
        {
          icon: 'logout',
          label: 'Oturumu Kapat',
          danger: true,
          onClick: async () => {
            const ok = await uiStore.confirm(
              'Bu cihazda oturumu kapatmak istediğine emin misin? Mesajların cihazında şifreli kalır.',
              { title: 'Oturumu Kapat', confirmLabel: 'Kapat', danger: true }
            );
            if (ok) {
              try { await authStore.signOut(); } catch { /* store handles state */ }
            }
          },
        },
      ]}
    />
    <div class="veil-user-panel-btns">
      <button
        class="btn-icon veil-panel-btn"
        class:active={!media.isMuted}
        title={media.isMuted ? 'Mikrofonu aç' : 'Mikrofonu kapat'}
        aria-label={media.isMuted ? 'Mikrofonu aç' : 'Mikrofonu kapat'}
        aria-pressed={!media.isMuted}
        onclick={() => mediaStore.toggleMute()}
      >
        <span class="veil-mic-btn" class:muted={media.isMuted}>
          <Icon name={media.isMuted ? 'mic-off' : 'mic'} size={18} />
        </span>
      </button>
      <button
        class="btn-icon veil-panel-btn"
        class:active={media.isDeafened}
        title={media.isDeafened ? 'Kulaklığı aç' : 'Kulaklığı kapat'}
        aria-label={media.isDeafened ? 'Kulaklığı aç' : 'Kulaklığı kapat'}
        aria-pressed={media.isDeafened}
        onclick={() => mediaStore.toggleDeafen()}
      >
        <span class="veil-mic-btn" class:muted={media.isDeafened}>
          <Icon name={media.isDeafened ? 'volume-x' : 'volume'} size={18} />
        </span>
      </button>
      <button
        class="btn-icon veil-panel-btn"
        title="Ayarlar"
        aria-label="Ayarlar"
        onclick={() => uiStore.openModal('settings')}
      >
        <Icon name="settings" size={18} />
      </button>
    </div>
  </div>
</div>

<style>
  .veil-bottom-bar {
    grid-area: bottom-user;
    display: flex;
    flex-direction: column;
    background: var(--veil-bg-void);
    border-top: 1px solid var(--veil-border-subtle);
    border-right: 1px solid var(--veil-border-subtle);
    z-index: 50;
    width: 100%;
    box-sizing: border-box;
  }

  .veil-voice-connection {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--veil-bg-elevated) 92%, var(--veil-brand));
    border-bottom: 1px solid var(--veil-border-subtle);
    flex-shrink: 0;
    gap: var(--space-2);
  }

  .veil-vc-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    flex: 1;
    background: transparent;
    border: none;
    padding: 2px 4px;
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;
  }
  .veil-vc-left:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .veil-vc-signal { color: var(--veil-text-muted); flex-shrink: 0; }
  .veil-vc-signal.good { color: var(--veil-success); }
  .veil-vc-signal.mid { color: var(--veil-warning); }
  .veil-vc-meta { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .veil-vc-channel-name {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-vc-status {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-variant-numeric: tabular-nums;
  }
  .veil-vc-actions { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
  .veil-vc-btn { width: 28px; height: 28px; }
  .veil-vc-btn.active { background: var(--veil-brand-subtle); color: var(--veil-brand); }
  .veil-vc-leave:hover { background: var(--veil-danger); color: #fff; }

  .veil-audio-pop-backdrop {
    position: fixed;
    inset: 0;
    z-index: 250;
  }
  .veil-audio-pop {
    position: fixed;
    bottom: 108px;
    left: 8px;
    width: 240px;
    background: color-mix(in srgb, var(--veil-bg-elevated) 96%, transparent);
    backdrop-filter: blur(16px);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    padding: var(--space-3) var(--space-4);
    box-shadow: var(--shadow-2xl);
    z-index: 260;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .veil-audio-pop-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .veil-audio-pop-title {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--veil-text-muted);
  }
  .veil-audio-pop-val {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
    font-variant-numeric: tabular-nums;
  }
  .veil-audio-pop-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
  }
  .veil-audio-pop-row .veil-slider {
    flex: 1;
    height: 5px;
    accent-color: var(--veil-brand);
    border-radius: var(--radius-full);
    cursor: pointer;
  }

  .veil-user-panel {
    flex-shrink: 0;
    min-height: 56px;
    background: var(--veil-bg-void);
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    width: 100%;
    box-sizing: border-box;
    overflow: visible;
  }
  .veil-user-panel-btns { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
</style>

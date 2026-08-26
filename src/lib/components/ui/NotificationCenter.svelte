<script lang="ts">
  import { notificationStore, type NotificationItem, type NotificationCategory } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import Avatar from './Avatar.svelte';
  import Icon, { type IconName } from './Icon.svelte';

  let { open = false, onClose }: { open: boolean; onClose: () => void } = $props();

  type TabFilter = 'all' | 'mentions' | 'requests' | 'system';
  let activeTab = $state<TabFilter>('all');

  const notifications = $derived($notificationStore);
  const unreadCount = $derived(notifications.filter(n => !n.read).length);

  const filteredNotifications = $derived(
    notifications.filter(item => {
      if (activeTab === 'mentions') return item.type === 'mention';
      if (activeTab === 'requests') return item.type === 'friend_request' || item.type === 'space_invite';
      if (activeTab === 'system') return item.type === 'system' || item.type === 'call';
      return true;
    })
  );

  function formatRelativeTime(ts: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - ts;
    if (diff < 60) return 'Az önce';
    if (diff < 3600) return `${Math.floor(diff / 60)} dk önce`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} sa önce`;
    const date = new Date(ts * 1000);
    return date.toLocaleDateString('tr-TR', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
  }

  function getCategoryIcon(type: NotificationCategory): IconName {
    switch (type) {
      case 'mention': return 'sparkle';
      case 'friend_request': return 'users';
      case 'space_invite': return 'shield';
      case 'call': return 'volume';
      case 'system': return 'settings';
      default: return 'chat';
    }
  }

  async function handleNotificationClick(item: NotificationItem) {
    notificationStore.markAsRead(item.id);

    if (item.channelId) {
      if (item.spaceId) {
        uiStore.navigate(item.spaceId, item.channelId);
      } else {
        uiStore.navigateDm(item.channelId);
      }
      onClose();
    } else if (item.type === 'friend_request') {
      uiStore.navigate(null, null);
      onClose();
    } else if (item.type === 'space_invite' && item.data?.inviteCode) {
      try {
        const space = await spaceStore.redeem(item.data.inviteCode);
        toastStore.success(`"${space.name}" topluluğuna katıldın!`);
        uiStore.navigate(space.id, null);
      } catch {
        toastStore.error('Davet kodu geçersiz veya süresi dolmuş.');
      }
      onClose();
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={open ? onKeydown : undefined} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="veil-notif-backdrop" onclick={onClose}>
    <div class="veil-notif-panel veil-pop-in" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="notif-panel-title" tabindex="-1">
      <header class="veil-notif-header">
        <div class="veil-notif-title-row">
          <div class="veil-notif-title-wrap">
            <Icon name="bell" size={18} />
            <h3 id="notif-panel-title">Bildirimler</h3>
            {#if unreadCount > 0}
              <span class="veil-notif-badge">{unreadCount}</span>
            {/if}
          </div>
          <div class="veil-notif-actions">
            {#if unreadCount > 0}
              <button
                class="btn btn-ghost btn-xs"
                onclick={() => notificationStore.markAllAsRead()}
                title="Tümünü okundu işaretle"
              >
                Tümünü Oku
              </button>
            {/if}
            {#if notifications.length > 0}
              <button
                class="btn btn-ghost btn-xs text-muted"
                onclick={() => notificationStore.clearAll()}
                title="Tüm bildirimleri temizle"
              >
                Temizle
              </button>
            {/if}
            <button class="btn-icon btn-icon-sm" onclick={onClose} aria-label="Kapat">
              <Icon name="x" size={16} />
            </button>
          </div>
        </div>

        <div class="veil-notif-tabs" role="tablist">
          <button
            class="veil-notif-tab"
            class:active={activeTab === 'all'}
            role="tab"
            aria-selected={activeTab === 'all'}
            onclick={() => (activeTab = 'all')}
          >
            Tümü
          </button>
          <button
            class="veil-notif-tab"
            class:active={activeTab === 'mentions'}
            role="tab"
            aria-selected={activeTab === 'mentions'}
            onclick={() => (activeTab = 'mentions')}
          >
            Bahsetmeler
          </button>
          <button
            class="veil-notif-tab"
            class:active={activeTab === 'requests'}
            role="tab"
            aria-selected={activeTab === 'requests'}
            onclick={() => (activeTab = 'requests')}
          >
            İstekler & Davetler
          </button>
          <button
            class="veil-notif-tab"
            class:active={activeTab === 'system'}
            role="tab"
            aria-selected={activeTab === 'system'}
            onclick={() => (activeTab = 'system')}
          >
            Sistem
          </button>
        </div>
      </header>

      <div class="veil-notif-body">
        {#if filteredNotifications.length === 0}
          <div class="veil-notif-empty">
            <div class="veil-notif-empty-icon" aria-hidden="true">
              <Icon name="bell" size={36} />
            </div>
            <p class="veil-notif-empty-title">Bildirim yok</p>
            <span class="veil-notif-empty-desc">
              {activeTab === 'mentions'
                ? 'Henüz bir mesajda senden bahsedilmedi.'
                : activeTab === 'requests'
                ? 'Bekleyen arkadaşlık isteği veya davet yok.'
                : 'Her şey güncel! Yeni bir aktivite olduğunda burada görünecek.'}
            </span>
          </div>
        {:else}
          <div class="veil-notif-list">
            {#each filteredNotifications as item (item.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="veil-notif-item"
                class:unread={!item.read}
                role="button"
                tabindex="0"
                onclick={() => handleNotificationClick(item)}
              >
                <div class="veil-notif-avatar-col">
                  {#if item.username}
                    <Avatar hash={item.avatarHash} name={item.username} size="md" />
                  {:else}
                    <div class="veil-notif-icon-circle {item.type}">
                      <Icon name={getCategoryIcon(item.type)} size={16} />
                    </div>
                  {/if}
                  {#if !item.read}
                    <span class="veil-notif-unread-dot" aria-hidden="true"></span>
                  {/if}
                </div>

                <div class="veil-notif-content-col">
                  <div class="veil-notif-meta-row">
                    <span class="veil-notif-item-title">{item.title}</span>
                    <span class="veil-notif-time">{formatRelativeTime(item.timestamp)}</span>
                  </div>
                  <p class="veil-notif-body-text">{item.body}</p>
                </div>

                <button
                  class="btn-icon btn-icon-xs veil-notif-item-del"
                  onclick={(e) => {
                    e.stopPropagation();
                    notificationStore.remove(item.id);
                  }}
                  title="Bildirimi sil"
                  aria-label="Bildirimi sil"
                >
                  <Icon name="x" size={13} />
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .veil-notif-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding: 3.5rem 1rem 1rem;
  }
  .veil-notif-panel {
    width: 100%;
    max-width: 420px;
    max-height: calc(100vh - 5rem);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-2xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: veilModalPop 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes veilModalPop {
    from { opacity: 0; transform: scale(0.96) translateY(-8px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }
  .veil-notif-header {
    padding: var(--space-4) var(--space-4) var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    background: var(--veil-bg-surface);
  }
  .veil-notif-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .veil-notif-title-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .veil-notif-title-wrap h3 {
    margin: 0;
    font-size: var(--text-base);
    font-weight: 700;
  }
  .veil-notif-badge {
    padding: 0 6px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    color: #fff;
    font-size: var(--text-xs);
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .veil-notif-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .veil-notif-tabs {
    display: flex;
    gap: var(--space-1);
    background: var(--veil-bg-elevated);
    padding: 2px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--veil-border-subtle);
  }
  .veil-notif-tab {
    flex: 1;
    text-align: center;
    padding: 5px 8px;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-muted);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-notif-tab:hover {
    color: var(--veil-text-primary);
  }
  .veil-notif-tab.active {
    background: var(--veil-bg-surface);
    color: var(--veil-text-primary);
    box-shadow: var(--shadow-sm);
  }
  .veil-notif-body {
    flex: 1;
    overflow-y: auto;
    min-height: 280px;
    max-height: 520px;
  }
  .veil-notif-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-10) var(--space-6);
    text-align: center;
    color: var(--veil-text-muted);
  }
  .veil-notif-empty-icon {
    width: 64px;
    height: 64px;
    border-radius: var(--radius-full);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: var(--space-3);
  }
  .veil-notif-empty-title {
    font-size: var(--text-md);
    font-weight: 700;
    color: var(--veil-text-primary);
    margin: 0 0 var(--space-1);
  }
  .veil-notif-empty-desc {
    font-size: var(--text-xs);
    line-height: var(--leading-relaxed);
    max-width: 280px;
  }
  .veil-notif-list {
    display: flex;
    flex-direction: column;
  }
  .veil-notif-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--veil-border-subtle);
    cursor: pointer;
    transition: background var(--t-fast);
    position: relative;
  }
  .veil-notif-item:hover {
    background: var(--veil-bg-surface);
  }
  .veil-notif-item.unread {
    background: color-mix(in srgb, var(--veil-brand) 6%, transparent);
  }
  .veil-notif-item.unread:hover {
    background: color-mix(in srgb, var(--veil-brand) 12%, transparent);
  }
  .veil-notif-avatar-col {
    position: relative;
    flex-shrink: 0;
  }
  .veil-notif-icon-circle {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--veil-bg-surface);
    color: var(--veil-brand);
    border: 1px solid var(--veil-border);
  }
  .veil-notif-icon-circle.mention {
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
  }
  .veil-notif-icon-circle.friend_request {
    background: color-mix(in srgb, var(--veil-success) 15%, transparent);
    color: var(--veil-success);
  }
  .veil-notif-icon-circle.space_invite {
    background: color-mix(in srgb, var(--veil-warning) 15%, transparent);
    color: var(--veil-warning);
  }
  .veil-notif-unread-dot {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 9px;
    height: 9px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    border: 2px solid var(--veil-bg-elevated);
  }
  .veil-notif-content-col {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-notif-meta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .veil-notif-item-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-notif-time {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    flex-shrink: 0;
  }
  .veil-notif-body-text {
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    margin: 0;
    line-height: var(--leading-normal);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .veil-notif-item-del {
    opacity: 0;
    transition: opacity var(--t-fast);
    align-self: center;
    color: var(--veil-text-muted);
  }
  .veil-notif-item:hover .veil-notif-item-del {
    opacity: 1;
  }
  .veil-notif-item-del:hover {
    color: var(--veil-danger);
  }
</style>

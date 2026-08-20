<script lang="ts">
  import { onMount } from 'svelte';
  import { spaceStore } from '$lib/stores/spaces';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore } from '$lib/stores/notifications';
  import Avatar from '../ui/Avatar.svelte';
  import Icon from '../ui/Icon.svelte';
  import ContextMenu from '../ui/ContextMenu.svelte';
  import type { ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';

  const spaces = $derived($spaceStore);
  const ui = $derived($uiStore);

  const dms = $derived(spaces.dmChannels);

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);

  let unlisten: (() => void) | null = null;
  let dmReloadTimer: ReturnType<typeof setTimeout> | null = null;

  function debouncedLoadDms() {
    if (dmReloadTimer) clearTimeout(dmReloadTimer);
    dmReloadTimer = setTimeout(() => spaceStore.loadDms(), 300);
  }

  onMount(() => {
    spaceStore.loadDms();
    if ('__TAURI_INTERNALS__' in window) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen('channels:changed', () => {
          debouncedLoadDms();
        });
        listen('presence:changed', () => {
          debouncedLoadDms();
        });
        listen<{ type: string }>('veilanon:broadcast', (e) => {
          if (e.payload?.type === 'presence') {
            debouncedLoadDms();
          }
        });
      }).catch(() => {});
    }
  });

  function openDmMenu(e: MouseEvent, dm: { id: string; name: string }) {
    e.preventDefault();
    e.stopPropagation();
    menuItems = [
      {
        label: 'Sohbeti Aç',
        icon: 'chat',
        onClick: () => uiStore.navigateDm(dm.id),
      },
      {
        label: 'DM Bağlantısını Kopyala',
        icon: 'link',
        onClick: async () => {
          await copyText(`https://veilanon.com/dm/${dm.id}`);
          toastStore.success('DM bağlantısı kopyalandı.');
        },
      },
      {
        label: 'Kanal ID\'sini Kopyala',
        icon: 'copy',
        onClick: async () => {
          await copyText(dm.id);
          toastStore.success('ID kopyalandı.');
        },
      },
    ];
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }
</script>

<div class="veil-dm-list">
  {#if dms.length === 0}
    <div class="veil-dm-empty">
      <div class="veil-dm-empty-icon" aria-hidden="true">
        <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 11.5a8.5 8.5 0 0 1-8.5 8.5c-1.5 0-3-.4-4.2-1.1L3 20l1.1-5.3a8.5 8.5 0 1 1 16.9-3.2Z"/>
          <path d="M8 11.5h.01M12 11.5h.01M16 11.5h.01"/>
        </svg>
      </div>
      <p class="veil-dm-empty-title">Gelen kutusu bomboş… kriket sesleri geliyor 🦗</p>
      <p class="veil-dm-empty-hint">Bir arkadaşını seçip ilk mesajı at, dedikodunun veya projenin fitilini ateşle! 🚀</p>
    </div>
  {:else}
    {#each dms as dm (dm.id)}
      <button
        class="veil-dm-row"
        class:active={ui.activeDmId === dm.id}
        onclick={() => uiStore.navigateDm(dm.id)}
        oncontextmenu={(e) => openDmMenu(e, dm)}
      >
        <Avatar
          name={dm.name}
          hash={dm.avatarHash}
          presence={dm.onlineStatus as 'online' | 'away' | 'dnd' | 'offline' | 'invisible' | null | undefined}
          size="sm"
        />
        <div class="veil-dm-info">
          <div class="veil-dm-name">{dm.name}</div>
          {#if dm.unreadCount > 0}
            <span class="veil-badge veil-dm-badge">{dm.unreadCount > 99 ? '99+' : dm.unreadCount}</span>
          {/if}
        </div>
        {#if dm.isE2ee}
          <Icon name="lock" size={12} />
        {/if}
      </button>
    {/each}
  {/if}
</div>

{#if menuOpen}
  <ContextMenu
    x={menuX}
    y={menuY}
    items={menuItems}
    onClose={() => (menuOpen = false)}
  />
{/if}

<style>
  .veil-dm-list { display: flex; flex-direction: column; gap: 2px; }
  .veil-dm-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--veil-text-muted);
  }
  .veil-dm-empty-icon {
    color: var(--veil-brand);
    background: var(--veil-brand-subtle);
    width: 72px;
    height: 72px;
    border-radius: var(--radius-2xl);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: var(--space-1);
  }
  .veil-dm-empty-title { font-weight: 600; color: var(--veil-text-secondary); }
  .veil-dm-empty-hint { font-size: var(--text-sm); line-height: var(--leading-relaxed); max-width: 34ch; }
  .veil-dm-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    cursor: pointer;
    color: var(--veil-text-secondary);
    transition: background var(--t-fast), color var(--t-fast);
    text-align: left;
  }
  .veil-dm-row:hover { background: var(--veil-channel-hover); color: var(--veil-text-primary); }
  .veil-dm-row.active { background: var(--veil-channel-active); color: var(--veil-text-primary); font-weight: 500; }
  .veil-dm-info { flex: 1; min-width: 0; display: flex; align-items: center; gap: var(--space-2); }
  .veil-dm-name { font-size: var(--text-base); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .veil-dm-badge { flex-shrink: 0; }
</style>

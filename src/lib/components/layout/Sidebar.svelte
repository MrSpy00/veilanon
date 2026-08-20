<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import AppLogo from '../ui/AppLogo.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Icon from '../ui/Icon.svelte';
  import ContextMenu, { type ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import { spaceApi, type SpaceInfo } from '$lib/api/tauri';

  interface ServerFolder {
    id: string;
    name: string;
    color: string;
    spaceIds: string[];
    expanded: boolean;
  }

  const spaces = $derived($spaceStore.spaces);
  const ui = $derived($uiStore);

  const activeSpace = $derived(spaces.find(s => s.id === ui.activeSpaceId) ?? null);
  const unreadDms = $derived($spaceStore.dmChannels.reduce((sum, dm) => sum + (dm.unreadCount || 0), 0));

  let folders = $state<ServerFolder[]>([]);
  let dragOverFolderId = $state<string | null>(null);
  let draggingSpaceId = $state<string | null>(null);

  onMount(() => {
    try {
      const stored = localStorage.getItem('veil_server_folders');
      if (stored) folders = JSON.parse(stored);
    } catch { /* defaults */ }
  });

  function saveFolders(next: ServerFolder[]) {
    folders = next;
    try {
      localStorage.setItem('veil_server_folders', JSON.stringify(next));
    } catch { /* storage full */ }
  }

  const spaceFolderMap = $derived(
    new Map(folders.flatMap(f => f.spaceIds.map(id => [id, f.id])))
  );

  const looseSpaces = $derived(
    spaces.filter(s => !spaceFolderMap.has(s.id))
  );

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);

  function toggleFolder(folderId: string) {
    saveFolders(folders.map(x => x.id === folderId ? { ...x, expanded: !x.expanded } : x));
  }

  function getFolderSpaces(folder: ServerFolder): SpaceInfo[] {
    return folder.spaceIds
      .map(id => spaces.find(s => s.id === id))
      .filter((s): s is SpaceInfo => Boolean(s));
  }

  async function createFolderForSpace(sp: SpaceInfo) {
    const name = await uiStore.promptInput('Klasör Adı:', {
      title: 'Sunucu Klasörü Oluştur',
      placeholder: 'Örn: Topluluklar, Oyunlar',
      confirmLabel: 'Oluştur',
    });
    if (!name) return;
    const newFolder: ServerFolder = {
      id: crypto.randomUUID(),
      name,
      color: 'var(--veil-brand)',
      spaceIds: [sp.id],
      expanded: true,
    };
    saveFolders([...folders, newFolder]);
    toastStore.success(`"${sp.name}" yeni klasöre eklendi.`);
  }

  function addSpaceToFolder(sp: SpaceInfo, folderId: string) {
    saveFolders(folders.map(f => {
      if (f.id === folderId) {
        return { ...f, spaceIds: [...f.spaceIds.filter(id => id !== sp.id), sp.id] };
      }
      return { ...f, spaceIds: f.spaceIds.filter(id => id !== sp.id) };
    }));
    toastStore.success(`"${sp.name}" klasöre taşındı.`);
  }

  function removeSpaceFromFolder(sp: SpaceInfo) {
    saveFolders(
      folders.map(f => ({
        ...f,
        spaceIds: f.spaceIds.filter(id => id !== sp.id),
      })).filter(f => f.spaceIds.length > 0)
    );
    toastStore.success(`"${sp.name}" klasörden çıkarıldı.`);
  }

  async function leaveOrDeleteSpace(sp: SpaceInfo) {
    if (sp.isOwner) {
      const ok = await uiStore.confirm(
        `"${sp.name}" topluluğunu kalıcı olarak silmek istediğine emin misin? Bu işlem tüm kanalları ve mesajları yok eder.`,
        { title: 'Topluluğu Sil', confirmLabel: 'Sil', danger: true }
      );
      if (!ok) return;
      try {
        await spaceApi.delete(sp.id);
        await spaceStore.loadSpaces();
        uiStore.navigate(null, null);
        toastStore.success('Topluluk silindi.');
      } catch (err) {
        toastStore.error(`Silinemedi: ${String(err).replace(/^Error:\s*/, '')}`);
      }
    } else {
      const ok = await uiStore.confirm(
        `"${sp.name}" topluluğundan ayrılmak istediğine emin misin?`,
        { title: 'Topluluktan Ayrıl', confirmLabel: 'Ayrıl', danger: true }
      );
      if (!ok) return;
      try {
        await spaceApi.leave(sp.id);
        await spaceStore.loadSpaces();
        uiStore.navigate(null, null);
        toastStore.success('Topluluktan ayrıldın.');
      } catch (err) {
        toastStore.error(`Ayrılamadı: ${String(err).replace(/^Error:\s*/, '')}`);
      }
    }
  }

  function openSpaceMenu(e: MouseEvent, sp: SpaceInfo) {
    e.preventDefault();
    e.stopPropagation();
    const currentFolderId = spaceFolderMap.get(sp.id);
    const canManage = sp.isOwner || (sp.myRoles && sp.myRoles.length > 0);
    const items: ContextMenuItem[] = [];

    if (canManage) {
      items.push(
        {
          label: 'Sunucu Ayarları',
          icon: 'settings',
          onClick: () => uiStore.openModal('channel-settings', { spaceId: sp.id }),
        },
        {
          label: 'Kanal Oluştur',
          icon: 'plus',
          onClick: () => uiStore.openModal('create-channel', { spaceId: sp.id }),
        },
      );
    }

    items.push(
      {
        label: 'Davet Bağlantısı',
        icon: 'link',
        onClick: () => uiStore.openModal('invite', { spaceId: sp.id }),
      },
      {
        label: 'Sunucu Bağlantısını Kopyala',
        icon: 'link',
        onClick: async () => {
          await copyText(`https://veilanon.com/server/${sp.id}`);
          toastStore.success('Sunucu bağlantısı kopyalandı.');
        },
      },
      {
        label: 'Sunucu ID\'sini Kopyala',
        icon: 'copy',
        onClick: async () => {
          await copyText(sp.id);
          toastStore.success('Sunucu ID\'si kopyalandı.');
        },
      },
      { label: '', separator: true },
    );

    if (currentFolderId) {
      items.push({
        label: 'Klasörden Çıkar',
        icon: 'x',
        onClick: () => removeSpaceFromFolder(sp),
      });
    } else {
      items.push({
        label: 'Yeni Klasör Oluştur',
        icon: 'plus',
        onClick: () => void createFolderForSpace(sp),
      });
      if (folders.length > 0) {
        for (const f of folders) {
          items.push({
            label: `"${f.name}" Klasörüne Ekle`,
            icon: 'shield',
            onClick: () => addSpaceToFolder(sp, f.id),
          });
        }
      }
    }

    items.push(
      { label: '', separator: true },
      {
        label: sp.isOwner ? 'Sunucuyu Sil' : 'Sunucudan Ayrıl',
        icon: 'logout',
        danger: true,
        onClick: () => void leaveOrDeleteSpace(sp),
      },
    );

    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  function openFolderMenu(e: MouseEvent, f: ServerFolder) {
    e.preventDefault();
    e.stopPropagation();
    menuItems = [
      {
        label: 'Klasör Adını Değiştir',
        icon: 'edit',
        onClick: async () => {
          const name = await uiStore.promptInput('Klasör Adı:', {
            title: 'Klasörü Yeniden Adlandır',
            defaultValue: f.name,
          });
          if (name?.trim()) {
            saveFolders(folders.map(fold => fold.id === f.id ? { ...fold, name: name.trim() } : fold));
            toastStore.success('Klasör adı güncellendi.');
          }
        },
      },
      {
        label: 'Klasör Rengini Değiştir',
        icon: 'sparkle',
        onClick: async () => {
          const color = await uiStore.promptInput('Renk Kodu (HEX):', {
            title: 'Klasör Rengi',
            defaultValue: f.color || '#5865f2',
          });
          if (color?.trim()) {
            saveFolders(folders.map(fold => fold.id === f.id ? { ...fold, color: color.trim() } : fold));
            toastStore.success('Klasör rengi güncellendi.');
          }
        },
      },
      { label: '', separator: true },
      {
        label: 'Klasörü Dağıt',
        icon: 'trash',
        danger: true,
        onClick: () => {
          saveFolders(folders.filter(fold => fold.id !== f.id));
          toastStore.success(`"${f.name}" klasörü dağıtıldı.`);
        },
      },
    ];
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }
</script>

<nav class="veil-sidebar" aria-label="Topluluklar">
  <!-- veilanon logo / home button -->
  <button
    class="veil-sidebar-logo"
    title="veilanon — Ana Menü"
    aria-label="Ana Menü"
    onclick={() => uiStore.navigate(null, null)}
  >
    <AppLogo size={48} radius={20} alt="veilanon" />
  </button>

  <div class="veil-sidebar-divider" role="separator"></div>

  <!-- Server Folders -->
  {#each folders as folder (folder.id)}
    {@const folderSpaces = getFolderSpaces(folder)}
    {#if folderSpaces.length > 0}
      <div class="veil-folder-wrap">
        <button
          type="button"
          class="veil-folder-icon"
          class:expanded={folder.expanded}
          style="--folder-bg: {folder.color};"
          title="{folder.name} ({folderSpaces.length} topluluk)"
          aria-label="{folder.name} klasörü"
          aria-expanded={folder.expanded}
          onclick={() => toggleFolder(folder.id)}
          oncontextmenu={(e) => openFolderMenu(e, folder)}
        >
          {#if !folder.expanded}
            <div class="veil-folder-grid">
              {#each folderSpaces.slice(0, 4) as sp (sp.id)}
                <div class="veil-folder-mini-avatar">
                  {sp.name.slice(0, 1).toUpperCase()}
                </div>
              {/each}
            </div>
          {:else}
            <Icon name="shield" size={20} />
          {/if}
        </button>

        {#if folder.expanded}
          <div class="veil-folder-children" style="--folder-bg: {folder.color};">
            {#each folderSpaces as space (space.id)}
              <button
                class="veil-space-icon"
                class:active={ui.activeSpaceId === space.id}
                title={space.name}
                aria-label={space.name}
                aria-pressed={ui.activeSpaceId === space.id}
                onclick={() => void uiStore.navigateSpace(space.id)}
                oncontextmenu={(e) => openSpaceMenu(e, space)}
              >
                <Avatar name={space.name} hash={space.iconHash} size="lg" />
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  {/each}

  <!-- Loose (non-foldered) Spaces -->
  {#each looseSpaces as space (space.id)}
    <button
      class="veil-space-icon"
      class:active={ui.activeSpaceId === space.id}
      title={space.name}
      aria-label={space.name}
      aria-pressed={ui.activeSpaceId === space.id}
      onclick={() => void uiStore.navigateSpace(space.id)}
      oncontextmenu={(e) => openSpaceMenu(e, space)}
    >
      <Avatar name={space.name} hash={space.iconHash} size="lg" />
    </button>
  {/each}

  <!-- Add space button -->
  <button
    class="veil-space-icon"
    title="Yeni Topluluk Oluştur"
    aria-label="Yeni topluluk oluştur"
    onclick={() => uiStore.openModal('create-space')}
    style="color: var(--veil-success);"
  >
    <Icon name="plus" size={22} />
  </button>
</nav>

<ContextMenu open={menuOpen} x={menuX} y={menuY} items={menuItems} onClose={() => (menuOpen = false)} />

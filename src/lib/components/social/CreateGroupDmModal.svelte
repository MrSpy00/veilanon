<script lang="ts">
  import { friendsStore } from '$lib/stores/friends';
  import { spaceStore } from '$lib/stores/spaces';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore } from '$lib/stores/notifications';
  import { dmApi, type FriendInfo } from '$lib/api/tauri';
  import Avatar from '../ui/Avatar.svelte';
  import Icon from '../ui/Icon.svelte';

  const friends = $derived($friendsStore);
  const acceptedFriends = $derived(friends.friends.filter(f => f.status === 'friends'));

  let groupName = $state('');
  let searchQuery = $state('');
  let selectedIds = $state<string[]>([]);
  let creating = $state(false);

  const filteredFriends = $derived(
    acceptedFriends.filter(f => {
      const q = searchQuery.trim().toLowerCase();
      if (!q) return true;
      return (
        f.username.toLowerCase().includes(q) ||
        (f.displayName && f.displayName.toLowerCase().includes(q))
      );
    })
  );

  function toggleFriend(id: string) {
    if (selectedIds.includes(id)) {
      selectedIds = selectedIds.filter(x => x !== id);
    } else {
      if (selectedIds.length >= 9) {
        toastStore.error('Bir grupta en fazla 10 kişi (sen dahil) olabilir.');
        return;
      }
      selectedIds = [...selectedIds, id];
    }
  }

  async function createGroup() {
    if (selectedIds.length === 0 || creating) return;
    creating = true;
    try {
      const name = groupName.trim() || undefined;
      const channel = await dmApi.createGroup({
        name,
        memberIds: selectedIds,
      });
      await spaceStore.loadDms();
      toastStore.success(`Grup sohbeti oluşturuldu: ${channel.name}`);
      uiStore.closeModal();
      uiStore.navigateDm(channel.id);
    } catch (err) {
      toastStore.error(`Grup oluşturulamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      creating = false;
    }
  }
</script>

<div class="veil-group-dm-modal">
  <div class="veil-group-dm-header">
    <p class="veil-group-dm-desc">
      Arkadaşlarını seçerek şifreli bir grup sohbeti başlatabilirsin (en fazla 9 arkadaş).
    </p>
  </div>

  <div class="veil-form-group">
    <label for="group-dm-name" class="veil-form-label">Grup Adı (İsteğe Bağlı)</label>
    <input
      id="group-dm-name"
      class="veil-input"
      bind:value={groupName}
      placeholder="Örn: Gizli Konsey, Proje Ekibi…"
      maxlength={45}
    />
  </div>

  <div class="veil-form-group">
    <label for="group-dm-search" class="veil-form-label">
      Arkadaş Seç ({selectedIds.length}/9)
    </label>
    <div class="veil-search-wrap">
      <Icon name="search" size={14} />
      <input
        id="group-dm-search"
        class="veil-input"
        bind:value={searchQuery}
        placeholder="Arkadaş ara…"
      />
    </div>
  </div>

  <div class="veil-friends-picker">
    {#if filteredFriends.length === 0}
      <div class="veil-picker-empty">
        <span>{searchQuery ? 'Eşleşen arkadaş bulunamadı.' : 'Henüz eklenmiş bir arkadaşın yok.'}</span>
      </div>
    {:else}
      {#each filteredFriends as f (f.userId)}
        {@const selected = selectedIds.includes(f.userId)}
        <button
          type="button"
          class="veil-friend-picker-row"
          class:selected
          onclick={() => toggleFriend(f.userId)}
        >
          <div class="veil-friend-picker-avatar">
            <Avatar name={f.displayName || f.username} hash={f.avatarHash} presence={f.onlineStatus} size="sm" />
          </div>
          <div class="veil-friend-picker-info">
            <span class="veil-friend-picker-name">{f.displayName || f.username}</span>
            <span class="veil-friend-picker-user">@{f.username}</span>
          </div>
          <div class="veil-friend-picker-check" class:checked={selected}>
            {#if selected}
              <Icon name="check" size={12} />
            {/if}
          </div>
        </button>
      {/each}
    {/if}
  </div>

  <div class="veil-group-dm-footer">
    <button type="button" class="btn btn-secondary" onclick={() => uiStore.closeModal()} disabled={creating}>
      İptal
    </button>
    <button
      type="button"
      class="btn btn-primary"
      onclick={createGroup}
      disabled={selectedIds.length === 0 || creating}
    >
      {creating ? 'Oluşturuluyor…' : 'Grup Sohbeti Başlat'}
    </button>
  </div>
</div>

<style>
  .veil-group-dm-modal {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-2) 0;
  }
  .veil-group-dm-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    line-height: 1.45;
  }
  .veil-form-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-form-label {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .veil-search-wrap :global(svg) {
    position: absolute;
    left: var(--space-3);
    color: var(--veil-text-muted);
    pointer-events: none;
  }
  .veil-search-wrap .veil-input {
    padding-left: var(--space-8);
  }
  .veil-friends-picker {
    max-height: 220px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    background: var(--veil-bg-base);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    padding: var(--space-2);
  }
  .veil-picker-empty {
    padding: var(--space-6);
    text-align: center;
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-friend-picker-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: transparent;
    border: 1px solid transparent;
    color: var(--veil-text-primary);
    cursor: pointer;
    text-align: left;
    transition: background var(--t-fast);
  }
  .veil-friend-picker-row:hover {
    background: var(--veil-bg-surface);
  }
  .veil-friend-picker-row.selected {
    background: var(--veil-bg-overlay);
    border-color: var(--veil-border-subtle);
  }
  .veil-friend-picker-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .veil-friend-picker-name {
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-friend-picker-user {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-friend-picker-check {
    width: 18px;
    height: 18px;
    border-radius: var(--radius-sm);
    border: 2px solid var(--veil-border);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--t-fast);
  }
  .veil-friend-picker-check.checked {
    background: var(--veil-brand);
    border-color: var(--veil-brand);
    color: #fff;
  }
  .veil-group-dm-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
</style>

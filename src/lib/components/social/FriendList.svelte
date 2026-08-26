<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { friendsStore } from '$lib/stores/friends';
  import { spaceStore } from '$lib/stores/spaces';
  import { uiStore } from '$lib/stores/ui';
  import { authStore } from '$lib/stores/auth';
  import { dmApi } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import { listen } from '@tauri-apps/api/event';
  import Avatar from '../ui/Avatar.svelte';
  import Icon from '../ui/Icon.svelte';
  import ContextMenu from '../ui/ContextMenu.svelte';
  import type { ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import type { FriendInfo } from '$lib/api/tauri';

  type FriendTab = 'online' | 'all' | 'pending' | 'blocked' | 'add';
  let activeTab = $state<FriendTab>('online');
  let searchQuery = $state('');
  let newUsername = $state('');
  let adding = $state(false);
  let refreshing = $state(false);

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);

  const friendsState = $derived($friendsStore);
  const allList = $derived(friendsState.friends);
  const auth = $derived($authStore);

  function isAccepted(s: string | null | undefined): boolean {
    const v = (s ?? '').toString().trim().toLowerCase();
    return v === 'friends' || v === 'accepted' || v === 'friend' || v === 'approved';
  }
  const onlineFriends = $derived(
    allList.filter(f => isAccepted(f.status) && f.onlineStatus !== 'offline' && f.onlineStatus !== 'invisible')
  );
  const acceptedFriends = $derived(
    allList.filter(f => isAccepted(f.status))
  );
  const incomingRequests = $derived(
    allList.filter(f => f.status === 'pending_incoming')
  );
  const outgoingRequests = $derived(
    allList.filter(f => f.status === 'pending_outgoing')
  );
  const pendingTotal = $derived(incomingRequests.length + outgoingRequests.length);
  const blockedList = $derived(
    allList.filter(f => f.status === 'blocked')
  );

  const filteredList = $derived(
    (() => {
      const q = searchQuery.trim().toLowerCase().replace(/^@/, '');
      let base: FriendInfo[] = [];
      if (activeTab === 'online') base = onlineFriends;
      else if (activeTab === 'all') base = acceptedFriends;
      else if (activeTab === 'blocked') base = blockedList;
      else return [];

      if (!q) return base;
      return base.filter(
        f => f.username.toLowerCase().includes(q) || (f.displayName && f.displayName.toLowerCase().includes(q))
      );
    })()
  );

  let friendsLoadDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  function debouncedFriendsLoad() {
    if (friendsLoadDebounceTimer) clearTimeout(friendsLoadDebounceTimer);
    friendsLoadDebounceTimer = setTimeout(() => {
      void friendsStore.load();
    }, 1000);
  }

  const unlistenFns: Array<() => void> = [];

  onMount(async () => {
    // Sayfa açıldığında arkadaş listesini yenile
    await friendsStore.load();
    listen('friends:changed', () => {
      debouncedFriendsLoad();
    }).then(u => unlistenFns.push(u));

    listen('presence:changed', () => {
      debouncedFriendsLoad();
    }).then(u => unlistenFns.push(u));

    listen('user:updated', () => {
      debouncedFriendsLoad();
    }).then(u => unlistenFns.push(u));

    listen('veilanon:broadcast', (e: any) => {
      const p = e.payload;
      if (p?.type === 'friend_request') {
        const isTarget = !p.target_id || p.target_id === auth.identity?.id;
        const isSender = p.sender_id === auth.identity?.id;
        if (isTarget || isSender) {
          debouncedFriendsLoad();
          if (isTarget && p.sender_username && p.action === 'incoming') {
            void toastStore.notifyFriendRequest({
              username: p.sender_username,
              displayName: p.sender_display_name || p.sender_username,
            });
          }
        }
      }
    }).then(u => unlistenFns.push(u));
  });

  onDestroy(() => {
    if (friendsLoadDebounceTimer) clearTimeout(friendsLoadDebounceTimer);
    for (const u of unlistenFns) {
      try { u(); } catch { /* ignore */ }
    }
  });

  async function refresh() {
    refreshing = true;
    try {
      await friendsStore.load();
      toastStore.success('Arkadaş listesi güncellendi.');
    } catch {
      toastStore.error('Güncelleme başarısız oldu.');
    } finally {
      refreshing = false;
    }
  }

  async function handleAddFriend() {
    const raw = newUsername.trim().replace(/^@/, '');
    if (!raw || adding) return;
    adding = true;
    try {
      await friendsStore.add(raw);
      newUsername = '';
      toastStore.success(`@${raw} kullanıcısına arkadaşlık isteği gönderildi!`);
      activeTab = 'pending';
    } catch (err) {
      const msg = String(err).replace(/^Error:\s*/, '');
      toastStore.error(msg || 'İstek gönderilemedi. Kullanıcı adını kontrol edin.');
    } finally {
      adding = false;
    }
  }

  async function copyMyProfileLink() {
    if (!auth.identity?.username) return;
    await copyText(`https://veilanon.com/u/${auth.identity.username}`);
    toastStore.success('Profil bağlantın kopyalandı.');
  }

  async function acceptRequest(f: FriendInfo) {
    try {
      await friendsStore.accept(f.userId);
      toastStore.success(`${f.displayName || f.username} ile artık arkadaşsınız! 🎉`);
    } catch {
      toastStore.error('İstek kabul edilirken bir hata oluştu.');
    }
  }

  async function rejectRequest(f: FriendInfo) {
    try {
      await friendsStore.reject(f.userId);
      toastStore.success('Arkadaşlık isteği reddedildi.');
    } catch {
      toastStore.error('İstek reddedilemedi.');
    }
  }

  async function cancelRequest(f: FriendInfo) {
    try {
      await friendsStore.cancel(f.userId);
      toastStore.success('Arkadaşlık isteği iptal edildi.');
    } catch {
      toastStore.error('İstek iptal edilemedi.');
    }
  }

  async function removeFriend(f: FriendInfo) {
    const ok = await uiStore.confirm(
      `${f.displayName || f.username} kullanıcısını arkadaş listenizden çıkarmak istediğinize emin misiniz?`,
      { title: 'Arkadaşı Çıkar', confirmLabel: 'Çıkar', danger: true }
    );
    if (!ok) return;
    try {
      await friendsStore.remove(f.userId);
      toastStore.success(`${f.displayName || f.username} arkadaş listesinden çıkarıldı.`);
    } catch {
      toastStore.error('Arkadaş listeden çıkarılamadı.');
    }
  }

  async function blockUser(f: FriendInfo) {
    const ok = await uiStore.confirm(
      `${f.displayName || f.username} kullanıcısını engellemek istiyor musunuz? Engellenen kullanıcılar size mesaj veya istek gönderemez.`,
      { title: 'Kullanıcıyı Engelle', confirmLabel: 'Engelle', danger: true }
    );
    if (!ok) return;
    try {
      await friendsStore.block(f.userId);
      toastStore.success('Kullanıcı engellendi.');
    } catch {
      toastStore.error('Kullanıcı engellenemedi.');
    }
  }

  async function unblockUser(f: FriendInfo) {
    try {
      await friendsStore.unblock(f.userId);
      toastStore.success('Engel kaldırıldı.');
    } catch {
      toastStore.error('Engel kaldırılamadı.');
    }
  }

  async function openDm(f: FriendInfo) {
    try {
      const channel = await dmApi.open(f.userId);
      await spaceStore.loadDms();
      uiStore.navigateDm(channel.id);
    } catch (err) {
      const msg = String(err);
      if (msg.includes('arkadaşlarınızla') || msg.includes('aynı sunucudaki') || msg.includes('engelliyor')) {
        toastStore.error(msg.replace(/^Error:\s*/, ''));
      } else {
        toastStore.error('Mesajlaşma kanalı açılamadı.');
      }
    }
  }

  function openProfile(f: FriendInfo) {
    uiStore.openModal('user-profile', {
      userId: f.userId,
      username: f.username,
      displayName: f.displayName,
      onlineStatus: f.onlineStatus,
    });
  }

  function openFriendMenu(e: MouseEvent, f: FriendInfo) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: 'Profili Gör', icon: 'user', onClick: () => openProfile(f) },
    ];

    if (f.status === 'blocked') {
      items.push({
        label: 'Engeli Kaldır',
        icon: 'shield',
        onClick: () => void unblockUser(f),
      });
    } else {
      items.push(
        { label: 'Mesaj Gönder (DM)', icon: 'chat', onClick: () => void openDm(f) },
        { label: '', separator: true },
        {
          label: 'Profil Bağlantısını Kopyala',
          icon: 'link',
          onClick: async () => {
            await copyText(`https://veilanon.com/u/${f.username}`);
            toastStore.success('Profil bağlantısı kopyalandı.');
          },
        },
        {
          label: 'Kullanıcı Adını Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(`@${f.username}`);
            toastStore.success('Kullanıcı adı kopyalandı.');
          },
        },
        { label: '', separator: true },
        {
          label: 'Arkadaşlıktan Çıkar',
          icon: 'x',
          danger: true,
          onClick: () => void removeFriend(f),
        },
        {
          label: 'Engelle',
          icon: 'x',
          danger: true,
          onClick: () => void blockUser(f),
        },
      );
    }

    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }
</script>

<div class="veil-friends-manager">
  <!-- Sekme Başlığı -->
  <header class="veil-friends-subnav">
    <div class="veil-friends-subnav-left">
      <div class="veil-friends-icon-badge" aria-hidden="true">
        <Icon name="users" size={18} />
      </div>
      <span class="veil-friends-title">Arkadaşlar</span>
      <div class="veil-friends-separator" aria-hidden="true"></div>

      <button
        class="veil-subnav-tab"
        class:active={activeTab === 'online'}
        onclick={() => (activeTab = 'online')}
      >
        <span>Çevrimiçi</span>
        {#if onlineFriends.length > 0}
          <span class="veil-tab-count">{onlineFriends.length}</span>
        {/if}
      </button>

      <button
        class="veil-subnav-tab"
        class:active={activeTab === 'all'}
        onclick={() => (activeTab = 'all')}
      >
        <span>Tümü</span>
        <span class="veil-tab-count">{friendsState.loading && allList.length === 0 ? '…' : acceptedFriends.length}</span>
      </button>

      <button
        class="veil-subnav-tab"
        class:active={activeTab === 'pending'}
        onclick={() => (activeTab = 'pending')}
      >
        <span>Bekleyenler</span>
        {#if pendingTotal > 0}
          <span class="veil-tab-badge" class:has-incoming={incomingRequests.length > 0}>
            {incomingRequests.length > 0 ? incomingRequests.length : pendingTotal}
          </span>
        {/if}
      </button>

      <button
        class="veil-subnav-tab"
        class:active={activeTab === 'blocked'}
        onclick={() => (activeTab = 'blocked')}
      >
        <span>Engellenenler</span>
        {#if blockedList.length > 0}
          <span class="veil-tab-count">{blockedList.length}</span>
        {/if}
      </button>

      <button
        class="veil-subnav-tab veil-tab-add"
        class:active={activeTab === 'add'}
        onclick={() => (activeTab = 'add')}
      >
        <Icon name="plus" size={14} />
        <span>Arkadaş Ekle</span>
      </button>
    </div>

    <div class="veil-friends-subnav-right">
      <button
        class="btn-icon"
        class:spin={refreshing}
        title="Yenile / Eşitle"
        aria-label="Yenile"
        onclick={refresh}
        disabled={refreshing}
      >
        <Icon name="refresh-cw" size={16} />
      </button>
    </div>
  </header>

  <!-- Sekme İçeriği -->
  <div class="veil-friends-content">
    {#if activeTab === 'add'}
      <!-- ARKADAŞ EKLE PANELİ -->
      <div class="veil-add-friend-panel">
        <div class="veil-add-header">
          <h3>Arkadaş Ekle</h3>
          <p>Veilanon kullanıcı adını girerek arkadaşlık isteği gönderin.</p>
        </div>

        <div class="veil-add-input-box" class:focused={adding}>
          <div class="veil-add-prefix">@</div>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="veil-add-input"
            bind:value={newUsername}
            placeholder="kullanici_adi"
            maxlength={32}
            autocomplete="off"
            autofocus
            onkeydown={(e) => { if (e.key === 'Enter') handleAddFriend(); }}
          />
          <button
            class="btn btn-primary"
            onclick={handleAddFriend}
            disabled={!newUsername.trim() || adding}
          >
            {adding ? 'Gönderiliyor…' : 'Arkadaşlık İsteği Gönder'}
          </button>
        </div>

        <div class="veil-add-tips">
          <div class="veil-tip-item">
            <Icon name="shield" size={16} />
            <span>Kullanıcı adları büyük/küçük harfe duyarsızdır.</span>
          </div>
          <div class="veil-tip-item">
            <Icon name="lock" size={16} />
            <span>Arkadaş olduktan sonra uçtan uca şifreli (E2EE) 1:1 mesajlaşabilirsiniz.</span>
          </div>
        </div>

        {#if auth.identity?.username}
          <div class="veil-add-share-box">
            <div class="veil-add-share-label">Veya arkadaşlarına kendi profil bağlantını gönder:</div>
            <div class="veil-add-share-row">
              <code>veilanon.com/u/{auth.identity.username}</code>
              <button class="btn btn-secondary btn-sm" onclick={copyMyProfileLink} title="Profil bağlantısını kopyala">
                <Icon name="copy" size={13} />
                Bağlantıyı Kopyala
              </button>
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'pending'}
      <!-- BEKLEYEN İSTEKLER PANELİ -->
      <div class="veil-pending-manager">
        <!-- Gelen İstekler -->
        <div class="veil-pending-section">
          <div class="veil-pending-header">
            <span>Gelen İstekler — {incomingRequests.length}</span>
          </div>
          {#if incomingRequests.length === 0}
            <p class="veil-section-empty">Gelen arkadaşlık isteği yok.</p>
          {:else}
            <div class="veil-friends-grid">
              {#each incomingRequests as req (req.userId)}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="veil-friend-card" oncontextmenu={(e) => openFriendMenu(e, req)}>
                  <div class="veil-card-left" onclick={() => openProfile(req)} role="button" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter') openProfile(req); }}>
                    <Avatar name={req.displayName || req.username} hash={req.avatarHash} size="md" presence={req.onlineStatus === 'invisible' ? 'offline' : (req.onlineStatus as any || 'offline')} />
                    <div class="veil-card-details">
                      <span class="veil-card-title">{req.displayName || req.username}</span>
                      <span class="veil-card-sub">@{req.username} · Gelen İstek</span>
                    </div>
                  </div>
                  <div class="veil-card-actions">
                    <button class="btn btn-primary btn-sm" onclick={() => acceptRequest(req)} title="Kabul Et">
                      <Icon name="check" size={15} />
                      <span>Kabul Et</span>
                    </button>
                    <button class="btn btn-ghost btn-sm veil-btn-danger" onclick={() => rejectRequest(req)} title="Reddet">
                      <Icon name="x" size={15} />
                      <span>Reddet</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <div class="veil-friends-divider" role="separator"></div>

        <!-- Giden İstekler -->
        <div class="veil-pending-section">
          <div class="veil-pending-header">
            <span>Giden İstekler — {outgoingRequests.length}</span>
          </div>
          {#if outgoingRequests.length === 0}
            <p class="veil-section-empty">Giden arkadaşlık isteği yok.</p>
          {:else}
            <div class="veil-friends-grid">
              {#each outgoingRequests as req (req.userId)}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="veil-friend-card" oncontextmenu={(e) => openFriendMenu(e, req)}>
                  <div class="veil-card-left" onclick={() => openProfile(req)} role="button" tabindex="0" onkeydown={(e) => { if (e.key === 'Enter') openProfile(req); }}>
                    <Avatar name={req.displayName || req.username} hash={req.avatarHash} size="md" presence={req.onlineStatus === 'invisible' ? 'offline' : (req.onlineStatus as any || 'offline')} />
                    <div class="veil-card-details">
                      <span class="veil-card-title">{req.displayName || req.username}</span>
                      <span class="veil-card-sub">@{req.username} · Yanıt Bekleniyor</span>
                    </div>
                  </div>
                  <div class="veil-card-actions">
                    <button class="btn btn-ghost btn-sm" onclick={() => cancelRequest(req)} title="İsteği İptal Et">
                      <Icon name="x" size={15} />
                      <span>İptal Et</span>
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

    {:else}
      <!-- ÇEVRİMİÇİ, TÜMÜ VEYA ENGELLENENLER LİSTESİ -->
      <div class="veil-list-container">
        <!-- Arama Çubuğu -->
        <div class="veil-search-bar">
          <Icon name="search" size={15} />
          <input
            class="veil-search-input"
            bind:value={searchQuery}
            placeholder="Ara..."
            maxlength={40}
          />
          {#if searchQuery}
            <button class="veil-search-clear" onclick={() => (searchQuery = '')} aria-label="Aramayı Temizle">
              <Icon name="x" size={14} />
            </button>
          {/if}
        </div>

        <div class="veil-list-header">
          <span>
            {activeTab === 'online' ? `Çevrimiçi — ${filteredList.length}` : activeTab === 'all' ? `Tüm Arkadaşlar — ${filteredList.length}` : `Engellenenler — ${filteredList.length}`}
          </span>
        </div>

        {#if friendsState.loading && allList.length === 0}
          <div class="veil-empty-state">
            <p>Arkadaşlar yükleniyor…</p>
          </div>
        {:else if filteredList.length === 0}
          <div class="veil-empty-state">
            <div class="veil-empty-state-icon">
              {#if activeTab === 'online'}
                <Icon name="users" size={44} />
              {:else if activeTab === 'blocked'}
                <Icon name="shield" size={44} />
              {:else}
                <Icon name="users" size={44} />
              {/if}
            </div>
            <p class="veil-empty-title">
              {#if searchQuery}
                Eşleşen arkadaş bulunamadı
              {:else if activeTab === 'online'}
                Şu anda çevrimiçi arkadaşın yok
              {:else if activeTab === 'blocked'}
                Engellenen kullanıcı yok
              {:else}
                Henüz arkadaş listen boş
              {/if}
            </p>
            <span class="veil-empty-hint">
              {#if activeTab === 'online'}
                Arkadaşların bağlandığında burada listelenir.
              {:else if activeTab === 'all'}
                Yukarıdaki <strong>Arkadaş Ekle</strong> sekmesinden kullanıcı adı ile istek gönderebilirsin.
              {/if}
            </span>
          </div>
        {:else}
          <div class="veil-friends-grid">
            {#each filteredList as f (f.userId)}
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="veil-friend-card" oncontextmenu={(e) => openFriendMenu(e, f)}>
                <div
                  class="veil-card-left"
                  onclick={() => openProfile(f)}
                  role="button"
                  tabindex="0"
                  onkeydown={(e) => { if (e.key === 'Enter') openProfile(f); }}
                >
                  <Avatar
                    name={f.displayName || f.username}
                    hash={f.avatarHash}
                    size="md"
                    presence={activeTab === 'blocked' ? 'offline' : (f.onlineStatus === 'invisible' ? 'offline' : (f.onlineStatus as any))}
                  />
                  <div class="veil-card-details">
                    <span class="veil-card-title">{f.displayName || f.username}</span>
                    <span class="veil-card-sub">
                      @{f.username}
                      {#if activeTab !== 'blocked'}
                        · <span class="veil-status-text" class:online={f.onlineStatus === 'online'}>
                          {f.onlineStatus === 'online' ? 'Çevrimiçi' : f.onlineStatus === 'away' ? 'Uzakta' : f.onlineStatus === 'dnd' ? 'Rahatsız Etmeyin' : 'Çevrimdışı'}
                        </span>
                      {/if}
                    </span>
                  </div>
                </div>

                <div class="veil-card-actions">
                  {#if activeTab === 'blocked'}
                    <button class="btn btn-secondary btn-sm" onclick={() => unblockUser(f)} title="Engeli Kaldır">
                      <Icon name="check" size={15} />
                      <span>Engeli Kaldır</span>
                    </button>
                  {:else}
                    <button class="btn-icon" onclick={() => openDm(f)} title="Mesaj Gönder" aria-label="Mesaj Gönder">
                      <Icon name="chat" size={17} />
                    </button>
                    <button class="btn-icon" onclick={() => openProfile(f)} title="Profili Görüntüle" aria-label="Profil">
                      <Icon name="users" size={17} />
                    </button>
                    <button class="btn-icon veil-btn-danger" onclick={() => removeFriend(f)} title="Arkadaşlıktan Çıkar" aria-label="Arkadaşlıktan Çıkar">
                      <Icon name="x" size={17} />
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
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
  .veil-friends-manager {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    gap: var(--space-4);
  }

  .veil-friends-subnav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--veil-border-subtle);
    gap: var(--space-2);
  }

  .veil-friends-subnav-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .veil-friends-icon-badge {
    color: var(--veil-text-muted);
    display: flex;
    align-items: center;
  }

  .veil-friends-title {
    font-size: var(--text-md);
    font-weight: 700;
    color: var(--veil-text-primary);
  }

  .veil-friends-separator {
    width: 1px;
    height: 18px;
    background: var(--veil-border);
    margin: 0 var(--space-2);
  }

  .veil-subnav-tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 6px 10px;
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all var(--t-fast);
  }

  .veil-subnav-tab:hover {
    background: var(--veil-bg-overlay);
    color: var(--veil-text-primary);
  }

  .veil-subnav-tab.active {
    background: var(--veil-bg-overlay);
    color: var(--veil-text-primary);
  }

  .veil-tab-add {
    background: var(--veil-brand);
    color: #fff !important;
  }

  .veil-tab-add:hover {
    background: var(--veil-brand-hover, #6d58ff);
  }

  .veil-tab-count {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin-left: 2px;
  }

  .veil-tab-badge {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-surface);
    color: var(--veil-text-secondary);
    font-size: var(--text-xs);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
  }

  .veil-tab-badge.has-incoming {
    background: var(--veil-danger);
    color: #fff;
  }

  .veil-friends-content {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  /* ARKADAŞ EKLE */
  .veil-add-friend-panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4) 0;
    max-width: 600px;
  }

  .veil-add-header h3 {
    font-size: var(--text-lg);
    font-weight: 700;
    margin-bottom: var(--space-1);
  }

  .veil-add-header p {
    font-size: var(--text-sm);
    color: var(--veil-text-muted);
  }

  .veil-add-input-box {
    display: flex;
    align-items: center;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-3);
    gap: var(--space-2);
    transition: border-color var(--t-fast);
  }

  .veil-add-input-box:focus-within {
    border-color: var(--veil-brand);
  }

  .veil-add-prefix {
    color: var(--veil-text-muted);
    font-weight: 700;
    font-size: var(--text-md);
  }

  .veil-add-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--veil-text-primary);
    font-size: var(--text-md);
    outline: none;
  }

  .veil-add-tips {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .veil-tip-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .veil-add-share-box {
    margin-top: var(--space-4);
    padding: var(--space-4);
    background: var(--veil-bg-void);
    border: 1px dashed var(--veil-brand-border);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .veil-add-share-label {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-weight: 500;
  }

  .veil-add-share-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .veil-add-share-row code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--veil-brand);
    font-weight: 600;
  }

  /* ARAMA VE LİSTE */
  .veil-list-container {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    color: var(--veil-text-muted);
  }

  .veil-search-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--veil-text-primary);
    font-size: var(--text-sm);
    outline: none;
  }

  .veil-search-clear {
    background: transparent;
    border: none;
    color: var(--veil-text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
  }

  .veil-list-header {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
    padding: var(--space-2) 0;
  }

  .veil-friends-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .veil-friend-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    transition: background var(--t-fast), border-color var(--t-fast);
  }

  .veil-friend-card:hover {
    background: var(--veil-bg-overlay);
    border-color: var(--veil-border);
  }

  .veil-card-left {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex: 1;
    min-width: 0;
    cursor: pointer;
    background: none;
    border: none;
    text-align: left;
  }

  .veil-card-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .veil-card-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-card-sub {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .veil-status-text.online {
    color: var(--veil-success, #22c55e);
  }

  .veil-card-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .veil-btn-danger:hover {
    color: var(--veil-danger) !important;
  }

  /* BEKLEYEN LİSTE */
  .veil-pending-manager {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .veil-pending-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .veil-pending-header {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }

  .veil-section-empty {
    font-size: var(--text-sm);
    color: var(--veil-text-muted);
    padding: var(--space-3) 0;
  }

  .veil-friends-divider {
    height: 1px;
    background: var(--veil-border-subtle);
    margin: var(--space-2) 0;
  }

  .veil-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--veil-text-muted);
  }

  .veil-empty-state-icon {
    color: var(--veil-text-disabled);
    margin-bottom: var(--space-2);
  }

  .veil-empty-title {
    font-weight: 600;
    color: var(--veil-text-secondary);
  }

  .veil-empty-hint {
    font-size: var(--text-sm);
    max-width: 40ch;
  }
</style>

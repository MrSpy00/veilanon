<script lang="ts">
  /**
   * Home — the app's main menu: friends, direct messages and spaces in one
   * tabbed surface, Discord-home style.
   */
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { friendsStore } from '$lib/stores/friends';
  import { spaceStore } from '$lib/stores/spaces';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore, unreadNotificationCount } from '$lib/stores/notifications';
  import NotificationCenter from '../ui/NotificationCenter.svelte';
  import { dmApi, spaceApi, type SpaceInfo, type FriendInfo } from '$lib/api/tauri';
  import Avatar from '../ui/Avatar.svelte';
  import BannerImage from '../ui/BannerImage.svelte';
  import Icon from '../ui/Icon.svelte';
  import FriendList from '../social/FriendList.svelte';
  import DmList from '../social/DmList.svelte';

  type Tab = 'friends' | 'dms' | 'spaces';
  let tab = $state<Tab>('friends');
  let notifOpen = $state(false);

  const friends = $derived($friendsStore);
  const spaces = $derived($spaceStore);

  let newUsername = $state('');
  let adding = $state(false);
  let inviteCode = $state('');
  let redeeming = $state(false);

  // Açık Toplulukları Keşfet
  let discoverSearch = $state('');
  let discoverSpaces = $state<SpaceInfo[]>([]);
  let searchingPublic = $state(false);
  let joiningSpaceId = $state<string | null>(null);

  const unlistens: Array<() => void> = [];

  onMount(() => {
    friendsStore.load();
    spaceStore.loadSpaces();
    spaceStore.loadDms();
    searchPublicSpaces();

    listen('spaces:changed', () => {
      searchPublicSpaces();
      spaceStore.loadSpaces();
    }).then(u => unlistens.push(u)).catch(() => {});

    listen('space:deleted', () => {
      searchPublicSpaces();
      spaceStore.loadSpaces();
    }).then(u => unlistens.push(u)).catch(() => {});

    listen('channels:changed', () => {
      spaceStore.loadDms();
    }).then(u => unlistens.push(u)).catch(() => {});
  });

  onDestroy(() => {
    for (const u of unlistens) {
      u();
    }
  });

  async function searchPublicSpaces() {
    searchingPublic = true;
    try {
      discoverSpaces = await spaceApi.searchPublic(discoverSearch.trim() || undefined);
    } catch {
      // ignore
    } finally {
      searchingPublic = false;
    }
  }

  async function joinPublicSpace(sp: SpaceInfo) {
    if (joiningSpaceId) return;
    joiningSpaceId = sp.id;
    try {
      const space = await spaceStore.joinPublic(sp.customLink || sp.id);
      toastStore.success(`Topluluğa katıldın: ${space.name}`);
      void uiStore.navigateSpace(space.id);
    } catch (err) {
      toastStore.error(`Topluluğa katılınamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      joiningSpaceId = null;
    }
  }

  async function addFriend() {
    const username = newUsername.trim();
    if (!username || adding) return;
    adding = true;
    try {
      await friendsStore.add(username);
      newUsername = '';
      toastStore.success('Arkadaşlık isteği gönderildi.');
    } catch {
      toastStore.error('İstek gönderilemedi. Kullanıcı adını kontrol et.');
    } finally {
      adding = false;
    }
  }

  async function accept(f: FriendInfo) {
    try {
      await friendsStore.accept(f.userId);
      toastStore.success(`${f.displayName || f.username} artık arkadaşın.`);
    } catch {
      toastStore.error('İstek kabul edilemedi.');
    }
  }

  async function startDm(f: FriendInfo) {
    try {
      const channel = await dmApi.open(f.userId);
      await spaceStore.loadDms();
      uiStore.navigateDm(channel.id);
    } catch {
      toastStore.error('DM kanalı açılamadı.');
    }
  }

  async function redeemInvite() {
    const raw = inviteCode.trim();
    if (!raw || redeeming) return;
    redeeming = true;
    try {
      // Davet linki veya kodu yapıştırıldıysa normalize et
      let code = raw;
      if (raw.includes('code=')) {
        code = new URLSearchParams(raw.split('?')[1] ?? '').get('code') ?? raw;
      } else {
        const cleaned = raw
          .replace(/^https?:\/\//i, '')
          .replace(/^veilanon:\/\//i, '')
          .replace(/^(?:www\.)?veilanon\.(?:com|com\.tr|online|info)\//i, '');
        const segs = cleaned.split('/').filter(Boolean);
        if (segs.length > 1 && ['invite', 'join', 'c', 'server', 'space'].includes(segs[0].toLowerCase())) {
          code = segs[1].split(/[?#]/)[0];
        } else if (segs.length === 1) {
          code = segs[0].split(/[?#]/)[0];
        }
      }
      code = code.trim().replace(/^@/, '');
      const space = await spaceStore.redeem(code);
      toastStore.success(`Topluluğa katıldın: ${space.name}`);
      inviteCode = '';
      void uiStore.navigateSpace(space.id);
    } catch {
      toastStore.error('Davet kodu veya bağlantısı geçersiz ya da süresi dolmuş.');
    } finally {
      redeeming = false;
    }
  }

  const tabs: Array<{ id: Tab; label: string; icon: 'users' | 'chat' | 'compass' }> = [
    { id: 'friends', label: 'Arkadaşlar', icon: 'users' },
    { id: 'dms', label: 'Direkt Mesajlar', icon: 'chat' },
    { id: 'spaces', label: 'Topluluklar & Keşfet', icon: 'compass' },
  ];

  const pending = $derived(
    friends.friends.filter(f => f.status === 'pending_incoming' || f.status === 'pending_outgoing')
  );
  const accepted = $derived(friends.friends.filter(f => f.status === 'friends'));
</script>

<div class="veil-home">
  <header class="veil-home-header">
    <div class="veil-home-brand" aria-hidden="true">
      <Icon name="chat" size={22} />
    </div>
    <h2>Ana Menü</h2>
    <nav class="veil-home-tabs" aria-label="Ana menü sekmeleri">
      {#each tabs as t (t.id)}
        <button
          class="veil-home-tab"
          class:active={tab === t.id}
          role="tab"
          aria-selected={tab === t.id}
          onclick={() => (tab = t.id)}
        >
          <Icon name={t.icon} size={15} />
          {t.label}
          {#if t.id === 'friends' && pending.length > 0}
            <span class="veil-home-tab-badge">{pending.length}</span>
          {/if}
        </button>
      {/each}
      <button
        class="veil-home-notif-btn"
        title="Bildirimler"
        aria-label="Bildirimler"
        aria-expanded={notifOpen}
        onclick={() => (notifOpen = !notifOpen)}
      >
        <Icon name="bell" size={15} />
        {#if $unreadNotificationCount > 0}
          <span class="veil-home-notif-badge">{$unreadNotificationCount > 99 ? '99+' : $unreadNotificationCount}</span>
        {/if}
      </button>
    </nav>
  </header>

  <NotificationCenter open={notifOpen} onClose={() => (notifOpen = false)} />

  <div class="veil-home-body">
    {#if tab === 'friends'}
      <section class="veil-home-panel veil-home-panel-flush" aria-label="Arkadaşlar">
        <FriendList />
      </section>

    {:else if tab === 'dms'}
      <section class="veil-home-panel" aria-label="Direkt mesajlar">
        <div class="veil-home-panel-title-row">
          <h3 class="veil-home-panel-title">Direkt Mesajlar</h3>
          <button class="btn btn-secondary btn-sm" onclick={() => (tab = 'friends')} title="Yeni mesaj için arkadaş seç">
            <Icon name="plus" size={14} />
            Yeni
          </button>
        </div>
        <DmList />
      </section>

    {:else}
      <section class="veil-home-panel" aria-label="Topluluklar">
        <div class="veil-home-panel-title-row">
          <h3 class="veil-home-panel-title">Toplulukların</h3>
          <button class="btn btn-primary btn-sm" onclick={() => uiStore.openModal('create-space')}>
            <Icon name="plus" size={14} />
            Topluluk Oluştur
          </button>
        </div>

        {#if spaces.spaces.length === 0}
          <div class="veil-home-empty">
            <div class="veil-home-empty-icon"><Icon name="sparkle" size={40} /></div>
            <p>Henüz bir topluluğun yok</p>
            <span>Kendi topluluğunu oluştur ya da aşağıdaki açık topluluklara göz at.</span>
          </div>
        {:else}
          <div class="veil-my-spaces-grid">
            {#each spaces.spaces as space (space.id)}
              <button
                class="veil-space-row"
                onclick={() => void uiStore.navigateSpace(space.id)}
              >
                <span class="veil-space-row-icon" aria-hidden="true">
                  <Avatar name={space.name} hash={space.iconHash} size="md" />
                </span>
                <span class="veil-space-row-info">
                  <span class="veil-space-row-name">{space.name}</span>
                  <span class="veil-space-row-meta">{space.memberCount} üye{space.isOwner ? ' · sahibi' : ''}</span>
                </span>
                <Icon name="arrow-right" size={14} />
              </button>
            {/each}
          </div>
        {/if}

        <div class="veil-home-divider" role="separator"></div>

        <div class="veil-home-section-header">
          <div class="veil-home-section-title">
            <Icon name="compass" size={16} />
            <span>Açık Toplulukları Keşfet</span>
          </div>
          <div class="veil-home-search-wrap">
            <input
              type="search"
              class="veil-input veil-home-search"
              placeholder="Topluluk ara…"
              bind:value={discoverSearch}
              oninput={() => { searchPublicSpaces(); }}
            />
          </div>
        </div>

        {#if searchingPublic}
          <div class="veil-home-loading">
            <div class="veil-spinner"></div>
            <span>Açık topluluklar taranıyor…</span>
          </div>
        {:else if discoverSpaces.length === 0}
          <div class="veil-home-empty">
            <div class="veil-home-empty-icon"><Icon name="compass" size={40} /></div>
            <p>Açık topluluk bulunamadı</p>
            <span>Topluluk ayarlarından kendi topluluğunu herkese açık hale getirebilirsin.</span>
          </div>
        {:else}
          <div class="veil-communities-grid">
            {#each discoverSpaces as sp (sp.id)}
              {@const isJoined = spaces.spaces.some(s => s.id === sp.id)}
              <div class="veil-community-card">
                <div class="veil-community-card-banner">
                  {#if sp.bannerHash}
                    <BannerImage hash={sp.bannerHash} alt="" class="veil-community-banner-img" />
                  {/if}
                </div>

                <div class="veil-community-card-body">
                  <div class="veil-community-card-avatar-wrap">
                    <Avatar name={sp.name} hash={sp.iconHash} size="lg" />
                  </div>

                  <div class="veil-community-card-info">
                    <span class="veil-community-name">{sp.name}</span>
                    <span class="veil-community-members">
                      <span class="veil-online-dot"></span>
                      {sp.memberCount || 1} üye
                    </span>
                  </div>

                  <p class="veil-community-desc">
                    {sp.description || 'Bu topluluk gizlilik odaklı sohbet ve paylaşımlar için açılmıştır.'}
                  </p>

                  <div class="veil-community-card-footer">
                    {#if sp.customLink}
                      <span class="veil-community-link-tag">/{sp.customLink}</span>
                    {/if}
                    {#if isJoined}
                      <button
                        class="btn btn-secondary btn-sm"
                        onclick={() => void uiStore.navigateSpace(sp.id)}
                      >
                        <Icon name="check" size={13} />
                        Görüntüle
                      </button>
                    {:else}
                      <button
                        class="btn btn-primary btn-sm"
                        onclick={() => joinPublicSpace(sp)}
                        disabled={joiningSpaceId === sp.id}
                      >
                        {joiningSpaceId === sp.id ? 'Katılıyor…' : 'Katıl'}
                      </button>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <div class="veil-home-divider" role="separator"></div>

        <div class="veil-home-section-label">Davet Linki veya Kodu ile Katıl</div>
        <div class="veil-home-add">
          <input
            class="veil-input veil-mono-input"
            bind:value={inviteCode}
            placeholder="veilanon.com/join/… veya davet kodu"
            aria-label="Davet linki veya kodu"
            maxlength={120}
            autocomplete="off"
            onkeydown={(e) => { if (e.key === 'Enter') redeemInvite(); }}
          />
          <button class="btn btn-secondary" onclick={redeemInvite} disabled={!inviteCode.trim() || redeeming}>
            {redeeming ? 'Katılıyor…' : 'Katıl'}
          </button>
        </div>
      </section>
    {/if}
  </div>
</div>

<style>
  .veil-home {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    max-width: 100%;
    width: 100%;
    margin: 0;
    box-sizing: border-box;
  }
  .veil-home-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: color-mix(in srgb, var(--veil-bg-surface, #1e1f22) 65%, transparent);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl, 14px);
  }
  .veil-home-brand {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-xl);
    background: transparent;
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    flex-shrink: 0;
  }
  .veil-home-header h2 { font-size: var(--text-xl); font-weight: 700; letter-spacing: var(--tracking-tight); }
  .veil-home-tabs {
    margin-left: auto;
    display: flex;
    gap: var(--space-1);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    padding: 3px;
  }
  .veil-home-tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
    font-weight: 600;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-home-tab:hover { color: var(--veil-text-primary); }
  .veil-home-tab.active { background: var(--veil-brand); color: #fff; }
  .veil-home-tab-badge {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: var(--veil-danger);
    color: #fff;
    font-size: var(--text-xs);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .veil-home-notif-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2);
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    color: var(--veil-text-muted);
    cursor: pointer;
    position: relative;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-home-notif-btn:hover {
    background: var(--veil-bg-overlay);
    color: var(--veil-text-primary);
  }
  .veil-home-notif-badge {
    position: absolute;
    top: -2px;
    right: -2px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 2px solid var(--veil-bg-surface);
  }
  .veil-home-body { display: flex; flex-direction: column; }
  .veil-home-panel {
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-home-panel-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--veil-border-subtle);
  }
  .veil-home-panel-title {
    font-size: var(--text-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-home-add { display: flex; gap: var(--space-2); }
  .veil-home-add .veil-input { flex: 1; min-width: 0; }
  .veil-home-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-8) var(--space-4);
    text-align: center;
    color: var(--veil-text-muted);
  }
  .veil-home-empty-icon { color: var(--veil-text-disabled); margin-bottom: var(--space-1); }
  .veil-home-empty p { font-weight: 600; color: var(--veil-text-secondary); }
  .veil-home-empty span { font-size: var(--text-sm); max-width: 38ch; line-height: var(--leading-relaxed); }
  .veil-space-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    cursor: pointer;
    color: var(--veil-text-secondary);
    transition: background var(--t-fast);
    text-align: left;
  }
  .veil-space-row:hover { background: var(--veil-bg-overlay); color: var(--veil-text-primary); }
  .veil-space-row-icon {
    width: 42px;
    height: 42px;
    border-radius: var(--radius-lg);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    font-weight: 700;
    font-size: var(--text-md);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    overflow: hidden;
  }
  .veil-space-row-info { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .veil-space-row-name { font-weight: 600; color: var(--veil-text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .veil-space-row-meta { font-size: var(--text-xs); color: var(--veil-text-muted); }
  .veil-home-divider { height: 1px; background: var(--veil-border-subtle); margin: var(--space-3) 0; }

  /* ── My Spaces Grid ─────────────────────────────────────────── */
  .veil-my-spaces-grid {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  /* ── Discover Public Communities ────────────────────────────── */
  .veil-discover-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-discover-heading {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .veil-discover-sub {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .veil-discover-search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    color: var(--veil-text-muted);
    transition: border-color var(--t-fast);
  }

  .veil-discover-search-bar:focus-within {
    border-color: var(--veil-brand);
  }

  .veil-discover-input {
    flex: 1;
    border: none !important;
    background: transparent !important;
    padding: 0 !important;
    font-size: var(--text-sm);
    box-shadow: none !important;
  }

  .veil-discover-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--space-3);
    margin-top: var(--space-1);
  }

  .veil-community-card {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: all var(--t-fast);
  }

  .veil-community-card:hover {
    border-color: var(--veil-border-focus);
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
  }

  .veil-community-card-banner {
    position: relative;
    width: 100%;
    height: 72px;
    background: var(--veil-bg-surface);
  }

  :global(.veil-community-banner-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .veil-community-banner-placeholder {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(120% 160% at 15% 0%, var(--veil-brand-subtle), transparent 55%),
      linear-gradient(160deg, var(--veil-bg-surface), var(--veil-bg-void));
  }

  .veil-community-avatar-wrap {
    position: absolute;
    bottom: -16px;
    left: var(--space-3);
    z-index: 2;
  }

  .veil-community-avatar-wrap :global(.veil-avatar) {
    border: 2px solid var(--veil-bg-elevated);
    box-shadow: 0 2px 8px rgba(0,0,0,0.5);
  }

  .veil-community-card-body {
    padding: var(--space-4) var(--space-3) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    flex: 1;
  }

  .veil-community-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .veil-community-name {
    font-size: var(--text-sm);
    font-weight: 700;
    color: var(--veil-text-primary);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-community-members-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 600;
    color: var(--veil-text-muted);
    background: var(--veil-bg-surface);
    padding: 2px 6px;
    border-radius: var(--radius-full);
    flex-shrink: 0;
  }

  .veil-community-desc {
    font-size: 11px;
    color: var(--veil-text-muted);
    line-height: var(--leading-relaxed);
    margin: 0;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    flex: 1;
  }

  .veil-community-card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--veil-border-subtle);
    margin-top: auto;
  }

  .veil-community-link-tag {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--veil-brand);
    background: var(--veil-brand-subtle);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    max-width: 100px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>

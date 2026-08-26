<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { uiStore } from '$lib/stores/ui';
  import { authStore } from '$lib/stores/auth';
  import { friendsStore } from '$lib/stores/friends';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import { dmApi, socialApi, memberApi, roleApi, type UserProfileInfo, type MemberInfo, type RoleInfo } from '$lib/api/tauri';
  import { streamerMode, maskUserId } from '$lib/stores/streamerMode';
  import { open } from '@tauri-apps/plugin-dialog';
  import { identityApi } from '$lib/api/tauri';
  import Avatar, { cacheAvatar } from '../ui/Avatar.svelte';
  import BannerImage, { cacheBanner, removeBannerCache } from '../ui/BannerImage.svelte';
  import Icon from '../ui/Icon.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import { readLocalImageAsDataUrl } from '$lib/utils/image-loader';
  import ImageCropModal from '../ui/ImageCropModal.svelte';
  import BannerCropModal from '../ui/BannerCropModal.svelte';

  const ui = $derived($uiStore);

  interface ProfileData {
    userId: string;
    username: string;
    displayName?: string;
    avatarHash?: string | null;
    bannerHash?: string | null;
    onlineStatus?: string;
  }

  const profile = $derived((ui.modalData as ProfileData | null) ?? null);

  let full = $state<UserProfileInfo | null>(null);
  let loading = $state(false);

  type ProfileTab = 'overview' | 'mutual_friends' | 'mutual_spaces';
  let activeTab = $state<ProfileTab>('overview');

  interface MutualFriend {
    userId: string;
    username: string;
    displayName: string;
    avatarHash?: string | null;
    onlineStatus: string;
  }

  interface MutualSpace {
    id: string;
    name: string;
    iconHash?: string | null;
    memberCount: number;
    description?: string | null;
  }

  let mutualFriends = $state<MutualFriend[]>([]);
  let mutualSpaces = $state<MutualSpace[]>([]);
  let mutualLoading = $state(false);

  // Sunucuya özel roller: profil, aktif topluluğun üye listesinden çözülür.
  let serverRoles = $state<RoleInfo[]>([]);
  let allSpaceRoles = $state<RoleInfo[]>([]);
  let canManageRoles = $state(false);
  let showRolePicker = $state(false);
  let serverRoleLoading = $state(false);

  const localFriend = $derived.by(() => {
    if (!profile) return null;
    const list = $friendsStore.friends;
    return list.find(
      f => (profile.userId && f.userId === profile.userId) ||
           (full?.userId && f.userId === full.userId) ||
           (profile.username && f.username.toLowerCase() === profile.username.toLowerCase()) ||
           (full?.username && f.username.toLowerCase() === full.username.toLowerCase())
    ) ?? null;
  });
  const status = $derived.by(() => {
    const lf = localFriend?.status;
    const ff = full?.friendStatus as string | undefined;
    const norm = (s: string | undefined) => (s === 'accepted' ? 'friends' : s);
    const a = norm(lf);
    const b = norm(ff);
    // Prefer authoritative positive status from either source.
    // Priority: friends > blocked > pending_incoming > pending_outgoing > none
    // Use server-provided full.friendStatus as primary when available
    if (b === 'friends' || a === 'friends') return 'friends';
    if (b === 'blocked' || a === 'blocked') return 'blocked';
    if (b === 'pending_incoming' || a === 'pending_incoming') return 'pending_incoming';
    if (b === 'pending_outgoing' || a === 'pending_outgoing') return 'pending_outgoing';
    // Hala loading ise ve full henüz gelmediyse 'none' gösterme
    if ($friendsStore.loading && !full && !localFriend) {
      return 'loading' as any;
    }
    // full.friendStatus varsa onu tercih et (sunucu side source of truth)
    if (ff && ff !== 'none') return ff as any;
    return (a ?? b ?? 'none') as any;
  });
  const isSelf = $derived(!!profile && profile.userId === $authStore.identity?.id);
  const onlineStatus = $derived(
    (isSelf ? $uiStore.presence : (localFriend?.onlineStatus ?? full?.onlineStatus ?? profile?.onlineStatus ?? 'offline')) as 'online' | 'away' | 'dnd' | 'offline' | 'invisible'
  );

  const PRESENCE_LABELS: Record<string, string> = {
    online: 'Çevrimiçi',
    away: 'Boşta',
    dnd: 'Rahatsız Etme',
    offline: 'Çevrimdışı',
    invisible: 'Görünmez',
  };

  function formatJoinDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString('tr-TR', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  }

  async function loadServerRoles() {
    const spaceId = $uiStore.activeSpaceId;
    if (!spaceId || !profile) return;
    serverRoleLoading = true;
    try {
      const currentSpace = $spaceStore.spaces.find(s => s.id === spaceId);
      const myId = $authStore.identity?.id;
      const isOwner = !!(currentSpace && currentSpace.ownerId === myId);

      const [members, roles] = await Promise.all([
        memberApi.list(spaceId).catch(() => [] as MemberInfo[]),
        roleApi.list(spaceId).catch(() => [] as RoleInfo[]),
      ]);
      allSpaceRoles = roles;

      const myMember = members.find(m => m.userId === myId);
      const myRoles = roles.filter(r => myMember?.roleIds.includes(r.id));
      const hasPerm = myRoles.some(r => r.permissions.includes('manage_roles') || r.permissions.includes('administrator'));
      canManageRoles = isOwner || hasPerm;

      const member = members.find(m => m.userId === profile!.userId);
      if (member) {
        serverRoles = roles.filter(r => member.roleIds.includes(r.id));
      } else {
        serverRoles = [];
      }
    } catch {
      serverRoles = [];
    } finally {
      serverRoleLoading = false;
    }
  }

  async function loadMutuals() {
    if (!profile || isSelf) return;
    mutualLoading = true;
    try {
      const [friends, spaces] = await Promise.all([
        invoke<MutualFriend[]>('get_mutual_friends', { userId: profile.userId }).catch(() => []),
        invoke<MutualSpace[]>('get_mutual_spaces', { userId: profile.userId }).catch(() => []),
      ]);
      mutualFriends = friends;
      mutualSpaces = spaces;
    } catch {
      mutualFriends = [];
      mutualSpaces = [];
    } finally {
      mutualLoading = false;
    }
  }

  async function toggleRole(roleId: string) {
    const spaceId = $uiStore.activeSpaceId;
    if (!spaceId || !profile || !canManageRoles) return;
    const currentIds = serverRoles.map(r => r.id);
    let nextIds: string[];
    if (currentIds.includes(roleId)) {
      nextIds = currentIds.filter(id => id !== roleId);
    } else {
      nextIds = [...currentIds, roleId];
    }
    try {
      await memberApi.update({ spaceId, userId: profile.userId, roleIds: nextIds });
      const [members, roles] = await Promise.all([
        memberApi.list(spaceId).catch(() => [] as MemberInfo[]),
        roleApi.list(spaceId).catch(() => [] as RoleInfo[]),
      ]);
      allSpaceRoles = roles;
      const member = members.find(m => m.userId === profile!.userId);
      serverRoles = member ? roles.filter(r => member.roleIds.includes(r.id)) : [];
      toastStore.success('Roller güncellendi.');
    } catch (err) {
      toastStore.error(`Rol güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  onMount(async () => {
    // Arkadaş listesini önce yükle — profil durumu bundan bağımlı
    // Paralel olarak hem arkadaşları hem profili yükle
    const friendsPromise = friendsStore.load();
    if (profile) {
      loading = true;
      const profilePromise = socialApi.getUserProfile(profile.userId).then((p) => {
        full = p;
      }).catch(() => {});
      await Promise.allSettled([friendsPromise, profilePromise]);
      loading = false;
      void loadServerRoles();
      if (!isSelf) {
        void loadMutuals();
      }
    } else {
      await friendsPromise;
    }

    // Broadcast eventlerini dinle: profil/banner/arkadaşlık güncellendiğinde yenile
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const unsubPromise = listen<Record<string, unknown>>('veilanon:broadcast', (event) => {
          const actual = ((event.payload?.payload || event.payload) as Record<string, unknown>) || {};
          const type = actual.type as string;
          if (type === 'profile_updated' || type === 'avatar_updated' || type === 'banner_updated' || type === 'user_updated') {
            const targetUserId = (actual.user_id || actual.id) as string | undefined;
            if (profile && targetUserId === profile.userId) {
              // Profil sahibi kendisiyse authStore'dan al, değilse API'den çek
              if (isSelf) {
                void authStore.refreshRemoteProfile();
              } else {
                void socialApi.getUserProfile(profile.userId).then((p) => { full = p; }).catch(() => {});
              }
            }
            void friendsStore.load();
          }
          if (type === 'friend_accepted' || type === 'friend_removed' || type === 'friend_request') {
            void friendsStore.load();
          }
        });
        // Cleanup: unsubPromise'i handle et, onMount return type ile uyumlu olsun
        void unsubPromise.then((unsub) => { unsub(); });
      } catch { /* ignore */ }
    }
  });

  $effect(() => {
    const pid = profile?.userId;
    if (pid) {
      // Paralel yükle: hem arkadaşları hem profili
      const fp = friendsStore.load();
      const pp = socialApi.getUserProfile(pid).then((p) => { full = p; }).catch(() => {});
      Promise.allSettled([fp, pp]).then(() => {
        void loadServerRoles();
        if (!isSelf) void loadMutuals();
      });
    }
  });

  const displayName = $derived(
    isSelf
      ? ($authStore.identity?.displayName || $authStore.identity?.username || '')
      : (full?.displayName ?? profile?.displayName ?? profile?.username ?? '')
  );
  const username = $derived(
    isSelf
      ? ($authStore.identity?.username || '')
      : (full?.username ?? profile?.username ?? '')
  );
  const avatarHash = $derived(
    isSelf
      ? ($authStore.identity?.avatarHash ?? null)
      : (full?.avatarHash ?? profile?.avatarHash ?? null)
  );
  const bio = $derived(
    isSelf
      ? ($authStore.identity?.bio ?? full?.bio ?? null)
      : (full?.bio ?? null)
  );
  const customStatus = $derived(
    isSelf
      ? ($authStore.identity?.customStatus ?? full?.customStatus ?? null)
      : (full?.customStatus ?? null)
  );
  const bannerHash = $derived(
    isSelf
      ? ($authStore.identity?.bannerHash ?? null)
      : (full?.bannerHash ?? profile?.bannerHash ?? (localFriend as any)?.bannerHash ?? null)
  );

  async function addFriend() {
    if (!profile || isSelf || profile.userId === $authStore.identity?.id) return;
    try {
      await friendsStore.add(username);
      full = full ? { ...full, friendStatus: 'pending_outgoing' } : full;
      toastStore.success('Arkadaşlık isteği gönderildi.');
    } catch {
      toastStore.error('İstek gönderilemedi.');
    }
  }

  async function accept() {
    if (!profile) return;
    try {
      await friendsStore.accept(profile.userId);
      full = full ? { ...full, friendStatus: 'friends' } : full;
      toastStore.success('Arkadaşlık isteği kabul edildi.');
    } catch {
      toastStore.error('İşlem başarısız.');
    }
  }

  async function block() {
    if (!profile) return;
    const ok = await uiStore.confirm(`${displayName} adlı kullanıcıyı engellemek istediğine emin misin?`, {
      title: 'Kullanıcıyı Engelle',
      confirmLabel: 'Engelle',
      danger: true,
    });
    if (!ok) return;
    try {
      await friendsStore.block(profile.userId);
      full = full ? { ...full, friendStatus: 'blocked' } : full;
      toastStore.success('Kullanıcı engellendi.');
    } catch {
      toastStore.error('İşlem başarısız.');
    }
  }

  async function unblock() {
    if (!profile) return;
    try {
      await friendsStore.unblock(profile.userId);
      full = full ? { ...full, friendStatus: 'none' } : full;
      toastStore.success('Engel kaldırıldı.');
    } catch {
      toastStore.error('İşlem başarısız.');
    }
  }

  async function reject() {
    if (!profile) return;
    try {
      await friendsStore.reject(profile.userId);
      toastStore.info('İstek reddedildi.');
    } catch {
      toastStore.error('İşlem başarısız.');
    }
  }

  async function cancelOutgoing() {
    if (!profile) return;
    try {
      await friendsStore.remove(profile.userId);
      toastStore.info('İstek iptal edildi.');
    } catch {
      toastStore.error('İşlem başarısız.');
    }
  }

  async function startDm() {
    if (!profile) return;
    try {
      const channel = await dmApi.open(profile.userId);
      await spaceStore.loadDms();
      uiStore.closeModal();
      uiStore.navigateDm(channel.id);
    } catch (err) {
      const msg = String(err).replace(/^Error:\s*/, '');
      toastStore.error(msg || 'DM kanalı açılamadı.');
    }
  }

  async function copyProfileLink() {
    await copyText(`https://veilanon.com/u/${username}`);
    toastStore.success('Profil linki kopyalandı.');
  }

  function openFriendProfile(f: MutualFriend) {
    uiStore.openModal('user-profile', {
      userId: f.userId,
      username: f.username,
      displayName: f.displayName,
      avatarHash: f.avatarHash,
      onlineStatus: f.onlineStatus,
    });
  }

  let avatarCropSrc = $state<string | null>(null);
  let bannerCropSrc = $state<string | null>(null);
  let profileEditBusy = $state(false);

  async function changeAvatarDirect() {
    if (profileEditBusy || !isSelf) return;
    const selected = await open({
      title: 'Profil fotoğrafı seç',
      multiple: false,
      filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    });
    if (!selected || typeof selected !== 'string') return;
    avatarCropSrc = await readLocalImageAsDataUrl(selected);
  }

  async function handleAvatarCropSave(croppedDataUrl: string) {
    avatarCropSrc = null;
    profileEditBusy = true;
    try {
      const hash = await identityApi.setAvatar(croppedDataUrl);
      cacheAvatar(hash, croppedDataUrl);
      authStore.updateIdentity({ avatarHash: hash });
      toastStore.success('Profil fotoğrafı güncellendi.');
    } catch {
      toastStore.error('Profil fotoğrafı güncellenemedi.');
    } finally {
      profileEditBusy = false;
    }
  }

  async function changeBannerDirect() {
    if (profileEditBusy || !isSelf) return;
    const selected = await open({
      title: 'Profil bannerı seç',
      multiple: false,
      filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    });
    if (!selected || typeof selected !== 'string') return;
    bannerCropSrc = await readLocalImageAsDataUrl(selected);
  }

  async function handleBannerCropSave(croppedDataUrl: string) {
    bannerCropSrc = null;
    profileEditBusy = true;
    try {
      // Eski banner hash'ini cache'den temizle
      const oldHash = $authStore.identity?.bannerHash;
      if (oldHash) removeBannerCache(oldHash);
      const hash = await identityApi.setBanner(croppedDataUrl);
      cacheBanner(hash, croppedDataUrl);
      authStore.updateIdentity({ bannerHash: hash });
      toastStore.success('Profil bannerı güncellendi.');
    } catch {
      toastStore.error('Profil bannerı güncellenemedi.');
    } finally {
      profileEditBusy = false;
    }
  }

  function openSpace(s: MutualSpace) {
    uiStore.closeModal();
    void uiStore.navigateSpace(s.id);
  }
</script>

{#if profile}
  <div class="veil-user-profile">
    <!-- Header Banner -->
    <div class="veil-up-banner" class:placeholder={!bannerHash}>
      {#if bannerHash}
        <BannerImage hash={bannerHash} alt="" class="veil-up-banner-img" />
      {/if}
      <div class="veil-up-banner-overlay"></div>
      {#if isSelf}
        <button
          type="button"
          class="veil-up-banner-edit-btn"
          onclick={changeBannerDirect}
          disabled={profileEditBusy}
          title="Bannerı Değiştir / Konumlandır"
        >
          <Icon name="image" size={13} />
          <span>{bannerHash ? 'Bannerı Değiştir' : 'Banner Ekle'}</span>
        </button>
      {/if}
    </div>

    <!-- Body content cleanly below banner -->
    <div class="veil-up-body">
      <!-- Avatar row & Primary ID -->
      <div class="veil-up-avatar-section">
        <div class="veil-up-avatar-wrapper">
          <Avatar name={displayName} size="2xl" hash={avatarHash} presence={onlineStatus} />
          {#if isSelf}
            <button
              type="button"
              class="veil-up-avatar-edit-btn"
              onclick={changeAvatarDirect}
              disabled={profileEditBusy}
              title="Fotoğrafı Değiştir / Konumlandır"
            >
              <Icon name="user" size={16} />
            </button>
          {/if}
        </div>
        
        <div class="veil-up-primary-actions">
          {#if isSelf}
            <button class="btn btn-secondary btn-sm" onclick={() => { uiStore.closeModal(); uiStore.openModal('settings'); }}>
              <Icon name="settings" size={14} />
              <span>Profili Düzenle</span>
            </button>
            <button class="btn btn-secondary btn-sm" onclick={copyProfileLink} title="Profili paylaş">
              <Icon name="link" size={14} />
            </button>
          {:else if status === 'friends'}
            <button class="btn btn-primary btn-sm" onclick={startDm}>
              <Icon name="chat" size={14} />
              <span>Mesaj</span>
            </button>
            <button class="btn btn-secondary btn-sm" onclick={copyProfileLink} title="Profili paylaş">
              <Icon name="link" size={14} />
            </button>
            <button class="btn btn-danger btn-sm" onclick={block} title="Engelle">
              <Icon name="x" size={14} />
            </button>
          {:else if status === 'pending_incoming'}
            <button class="btn btn-primary btn-sm" onclick={accept}>
              <Icon name="check" size={14} />
              <span>Kabul Et</span>
            </button>
            <button class="btn btn-secondary btn-sm" onclick={startDm}>
              <Icon name="chat" size={14} />
              <span>Mesaj</span>
            </button>
            <button class="btn btn-danger btn-sm" onclick={reject} title="İsteği Reddet">
              <Icon name="x" size={14} />
            </button>
          {:else if status === 'pending_outgoing'}
            <button class="btn btn-secondary btn-sm" disabled>İstek Gönderildi</button>
            <button class="btn btn-primary btn-sm" onclick={startDm}>
              <Icon name="chat" size={14} />
              <span>Mesaj</span>
            </button>
            <button class="btn btn-secondary btn-sm" onclick={cancelOutgoing} title="İsteği İptal Et">
              <Icon name="x" size={14} />
            </button>
          {:else if status === 'blocked'}
            <button class="btn btn-secondary btn-sm" onclick={unblock}>Engeli Kaldır</button>
          {:else if status === 'loading'}
            <div class="veil-spinner veil-spinner-sm" style="margin: 4px;"></div>
          {:else}
            <button class="btn btn-primary btn-sm" onclick={addFriend}>
              <Icon name="plus" size={14} />
              <span>Arkadaş Ekle</span>
            </button>
            <button class="btn btn-secondary btn-sm" onclick={startDm}>
              <Icon name="chat" size={14} />
              <span>Mesaj</span>
            </button>
            <button class="btn btn-secondary btn-sm" onclick={copyProfileLink} title="Profili paylaş">
              <Icon name="link" size={14} />
            </button>
          {/if}
        </div>
      </div>

      <!-- Identity Details Card -->
      <div class="veil-up-card veil-up-identity-card">
        <div class="veil-up-name-row">
          <h2 class="veil-user-profile-name">{displayName}</h2>
          <span class="veil-up-presence-pill {onlineStatus}">
            <span class="veil-up-dot {onlineStatus}" aria-hidden="true"></span>
            {PRESENCE_LABELS[onlineStatus] ?? 'Çevrimdışı'}
          </span>
          <span class="veil-up-e2ee-badge" title="Tüm iletişim uçtan uca şifreli (E2EE) olarak korunur">
            <Icon name="lock" size={11} />
            <span>E2EE</span>
          </span>
        </div>
        <p class="veil-user-profile-tag" data-streamer-mask="id" data-auto-protect="secret">
          @{$streamerMode.enabled && $streamerMode.hideUserIds ? maskUserId(username) : username}
        </p>

        {#if customStatus}
          <div class="veil-up-custom-status">
            <Icon name="chat" size={13} />
            <span>{customStatus}</span>
          </div>
        {/if}

        {#if full?.createdAt && (isSelf || full.showJoinDate !== false)}
          <div class="veil-up-join">
            <Icon name="calendar" size={13} />
            <span>Katılım: {formatJoinDate(full.createdAt)}</span>
          </div>
        {/if}
      </div>

      <!-- Tab Navigation -->
      {#if !isSelf}
        <div class="veil-up-tabs" role="tablist">
          <button
            type="button"
            class="veil-up-tab"
            class:active={activeTab === 'overview'}
            onclick={() => (activeTab = 'overview')}
            role="tab"
            aria-selected={activeTab === 'overview'}
          >
            <Icon name="shield" size={14} />
            <span>Genel Bakış</span>
          </button>
          <button
            type="button"
            class="veil-up-tab"
            class:active={activeTab === 'mutual_friends'}
            onclick={() => (activeTab = 'mutual_friends')}
            role="tab"
            aria-selected={activeTab === 'mutual_friends'}
          >
            <Icon name="users" size={14} />
            <span>Ortak Arkadaşlar</span>
            {#if mutualFriends.length > 0}
              <span class="veil-up-tab-count">{mutualFriends.length}</span>
            {/if}
          </button>
          <button
            type="button"
            class="veil-up-tab"
            class:active={activeTab === 'mutual_spaces'}
            onclick={() => (activeTab = 'mutual_spaces')}
            role="tab"
            aria-selected={activeTab === 'mutual_spaces'}
          >
            <Icon name="hash" size={14} />
            <span>Ortak Sunucular</span>
            {#if mutualSpaces.length > 0}
              <span class="veil-up-tab-count">{mutualSpaces.length}</span>
            {/if}
          </button>
        </div>
      {/if}

      <!-- Tab Contents -->
      {#if activeTab === 'overview' || isSelf}
        <!-- Bio -->
        {#if bio}
          <div class="veil-up-card veil-up-bio">
            <div class="veil-up-card-label">Hakkımda</div>
            <p class="veil-up-bio-text">{bio}</p>
          </div>
        {/if}

        <!-- Server Roles -->
        {#if serverRoles.length > 0 || canManageRoles}
          <div class="veil-up-card veil-up-roles">
            <div class="veil-up-roles-head">
              <div class="veil-up-card-label">Bu Topluluktaki Rolleri</div>
              {#if canManageRoles && allSpaceRoles.length > 0}
                <button
                  type="button"
                  class="veil-up-add-role-btn"
                  onclick={() => (showRolePicker = !showRolePicker)}
                  title="Rolleri Yönet"
                >
                  <Icon name="shield" size={12} />
                  <span>{showRolePicker ? 'Kapat' : 'Rol Yönet'}</span>
                </button>
              {/if}
            </div>

            <div class="veil-up-roles-list">
              {#each serverRoles as role (role.id)}
                <span
                  class="veil-up-role"
                  class:colored={!!role.color}
                  style={role.color ? `--role-color:${role.color}` : ''}
                  title={role.name}
                >
                  <span class="veil-up-role-dot" style={role.color ? `background:${role.color}` : ''}></span>
                  <span>{role.name}</span>
                  {#if canManageRoles}
                    <button
                      type="button"
                      class="veil-up-role-remove"
                      title="{role.name} rolünü kaldır"
                      onclick={() => toggleRole(role.id)}
                    >
                      <Icon name="x" size={10} />
                    </button>
                  {/if}
                </span>
              {/each}
              {#if serverRoles.length === 0}
                <span class="veil-up-noroles">Bu sunucuda henüz rol atanmamış</span>
              {/if}
            </div>

            {#if showRolePicker && canManageRoles}
              <div class="veil-up-role-picker">
                <div class="veil-up-picker-title">Sunucu Rolleri</div>
                <div class="veil-up-picker-grid">
                  {#each allSpaceRoles as role (role.id)}
                    {@const isAssigned = serverRoles.some(r => r.id === role.id)}
                    <button
                      type="button"
                      class="veil-up-picker-item"
                      class:assigned={isAssigned}
                      onclick={() => toggleRole(role.id)}
                    >
                      <span
                        class="veil-up-picker-dot"
                        style={role.color ? `background:${role.color}` : ''}
                      ></span>
                      <span class="veil-up-picker-name">{role.name}</span>
                      {#if isAssigned}
                        <Icon name="check" size={13} />
                      {/if}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else if serverRoleLoading}
          <div class="veil-up-card veil-up-roles">
            <div class="veil-up-card-label">Bu Topluluktaki Rolleri</div>
            <div class="veil-spinner veil-spinner-sm" style="margin:0.5rem 0;"></div>
          </div>
        {/if}

      {:else if activeTab === 'mutual_friends'}
        <div class="veil-up-card">
          <div class="veil-up-card-label">Ortak Arkadaşlar ({mutualFriends.length})</div>
          {#if mutualLoading}
            <div class="veil-spinner veil-spinner-sm" style="margin:1rem auto;"></div>
          {:else if mutualFriends.length === 0}
            <p class="veil-up-empty-text">Ortak arkadaş bulunmuyor.</p>
          {:else}
            <div class="veil-up-mutual-list">
              {#each mutualFriends as friend (friend.userId)}
                <button type="button" class="veil-up-mutual-item" onclick={() => openFriendProfile(friend)}>
                  <Avatar name={friend.displayName} size="md" hash={friend.avatarHash} presence={friend.onlineStatus as any} />
                  <div class="veil-up-mutual-info">
                    <span class="veil-up-mutual-name">{friend.displayName}</span>
                    <span class="veil-up-mutual-tag">@{friend.username}</span>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>

      {:else if activeTab === 'mutual_spaces'}
        <div class="veil-up-card">
          <div class="veil-up-card-label">Ortak Sunucular ({mutualSpaces.length})</div>
          {#if mutualLoading}
            <div class="veil-spinner veil-spinner-sm" style="margin:1rem auto;"></div>
          {:else if mutualSpaces.length === 0}
            <p class="veil-up-empty-text">Ortak topluluk bulunmuyor.</p>
          {:else}
            <div class="veil-up-mutual-list">
              {#each mutualSpaces as space (space.id)}
                <button type="button" class="veil-up-mutual-item" onclick={() => openSpace(space)}>
                  <div class="veil-up-space-icon">
                    <Avatar name={space.name} hash={space.iconHash} size="md" />
                  </div>
                  <div class="veil-up-mutual-info">
                    <span class="veil-up-mutual-name">{space.name}</span>
                    <span class="veil-up-mutual-tag">{space.memberCount} üye{space.description ? ` · ${space.description}` : ''}</span>
                  </div>
                  <Icon name="arrow-right" size={14} />
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <button class="btn btn-ghost veil-close-profile" onclick={() => uiStore.closeModal()}>Kapat</button>
    </div>
  </div>

  {#if avatarCropSrc}
    <ImageCropModal
      src={avatarCropSrc}
      shape="circle"
      aspectRatio={1}
      title="Profil Fotoğrafını Ayarla"
      onSave={handleAvatarCropSave}
      onClose={() => { avatarCropSrc = null; }}
    />
  {/if}

  {#if bannerCropSrc}
    <BannerCropModal
      src={bannerCropSrc}
      aspectRatio={3}
      title="Profil Bannerını Ayarla"
      hasAvatarPreview={true}
      avatarName={displayName}
      avatarHash={avatarHash}
      onSave={handleBannerCropSave}
      onClose={() => { bannerCropSrc = null; }}
    />
  {/if}
{:else}
  <p class="veil-empty-inline">Profil bulunamadı.</p>
{/if}

<style>
  .veil-user-profile {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-width: 580px;
    background: var(--veil-bg-elevated);
    border-radius: var(--radius-2xl);
    border: 1px solid var(--veil-border);
    overflow: hidden;
    box-shadow: var(--shadow-2xl);
  }

  .veil-up-banner {
    position: relative;
    width: 100%;
    height: 150px;
    background:
      radial-gradient(120% 160% at 15% 0%, var(--veil-brand-subtle), transparent 60%),
      linear-gradient(160deg, #181926, #0e0f17);
    overflow: hidden;
  }

  :global(.veil-up-banner-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    display: block;
  }

  .veil-up-banner-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, transparent 60%, rgba(12, 14, 20, 0.7));
    pointer-events: none;
  }

  .veil-up-banner-edit-btn {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: rgba(15, 17, 23, 0.75);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: var(--radius-full);
    color: #fff;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    z-index: 10;
  }
  .veil-up-banner-edit-btn:hover {
    background: rgba(15, 17, 23, 0.92);
    border-color: var(--veil-brand);
    transform: translateY(-1px);
  }

  .veil-up-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: 0 var(--space-5) var(--space-5);
    background: var(--veil-bg-elevated);
  }

  .veil-up-avatar-section {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    margin-top: -50px;
    position: relative;
    z-index: 20;
  }

  .veil-up-avatar-wrapper {
    position: relative;
    border-radius: var(--radius-full);
    display: inline-flex;
  }

  .veil-up-avatar-wrapper :global(.veil-avatar) {
    border: 5px solid var(--veil-bg-elevated);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    background: var(--veil-bg-surface);
    overflow: visible;
  }

  .veil-up-avatar-wrapper :global(.veil-presence) {
    border-color: var(--veil-bg-elevated);
    z-index: 30;
    bottom: 2px;
    right: 2px;
  }

  .veil-up-avatar-edit-btn {
    position: absolute;
    bottom: 4px;
    right: 4px;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    color: var(--veil-brand-foreground, #fff);
    border: 3px solid var(--veil-bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    transition: all 0.2s ease;
    z-index: 35;
  }
  .veil-up-avatar-edit-btn:hover {
    transform: scale(1.1);
    background: var(--veil-brand-hover, var(--veil-brand));
  }

  .veil-up-primary-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: 4px;
  }

  .veil-up-card {
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    padding: var(--space-3) var(--space-4);
  }

  .veil-up-card-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--veil-text-muted);
    margin-bottom: var(--space-2);
  }

  .veil-up-name-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .veil-user-profile-name {
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: var(--tracking-tight);
    color: var(--veil-text-primary);
  }

  .veil-up-presence-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-muted);
  }
  .veil-up-presence-pill.online { color: var(--veil-success); border-color: hsl(142 71% 45% / 0.3); }
  .veil-up-presence-pill.away { color: var(--veil-warning); border-color: hsl(38 92% 50% / 0.3); }
  .veil-up-presence-pill.dnd { color: var(--veil-danger); border-color: hsl(0 84% 60% / 0.3); }

  .veil-up-e2ee-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: var(--radius-full);
    background: color-mix(in srgb, var(--veil-brand) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--veil-brand) 30%, transparent);
    color: var(--veil-brand);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.05em;
  }

  .veil-user-profile-tag {
    font-size: var(--text-sm);
    color: var(--veil-text-muted);
    font-family: var(--font-mono);
    margin-top: 2px;
  }

  .veil-up-custom-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-xs);
    color: var(--veil-text-primary);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
    padding: 4px 10px;
    margin-top: var(--space-2);
    font-weight: 500;
  }

  .veil-up-join {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin-top: var(--space-2);
  }

  .veil-up-tabs {
    display: flex;
    gap: var(--space-1);
    border-bottom: 1px solid var(--veil-border-subtle);
    padding-bottom: var(--space-1);
  }

  .veil-up-tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--veil-text-secondary);
    font-size: var(--text-xs);
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-up-tab:hover {
    color: var(--veil-text-primary);
    background: var(--veil-bg-surface);
  }

  .veil-up-tab.active {
    color: var(--veil-brand);
    background: color-mix(in srgb, var(--veil-brand) 12%, transparent);
  }

  .veil-up-tab-count {
    padding: 1px 5px;
    background: var(--veil-bg-elevated);
    border-radius: var(--radius-full);
    font-size: 10px;
  }

  .veil-up-bio-text {
    font-size: var(--text-sm);
    color: var(--veil-text-secondary);
    line-height: var(--leading-relaxed);
    user-select: text;
    word-break: break-word;
  }

  .veil-up-roles-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .veil-up-add-role-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-brand);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    background: var(--veil-brand-subtle);
    border: 1px solid var(--veil-brand-border);
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .veil-up-add-role-btn:hover {
    background: color-mix(in srgb, var(--veil-brand) 25%, transparent);
  }

  .veil-up-roles-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .veil-up-role {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    font-size: var(--text-xs);
    font-weight: 600;
    border-radius: var(--radius-full);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    color: var(--veil-text-secondary);
  }
  .veil-up-role.colored {
    color: var(--role-color, var(--veil-brand));
    border-color: color-mix(in srgb, var(--role-color, var(--veil-brand)) 40%, transparent);
    background: color-mix(in srgb, var(--role-color, var(--veil-brand)) 12%, transparent);
  }

  .veil-up-role-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-text-muted);
  }

  .veil-up-role-remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    padding: 2px;
    margin-left: 2px;
    color: inherit;
    opacity: 0.6;
    cursor: pointer;
    border-radius: var(--radius-full);
  }
  .veil-up-role-remove:hover { opacity: 1; background: rgba(255, 255, 255, 0.15); }

  .veil-up-noroles, .veil-up-empty-text {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-style: italic;
    padding: var(--space-2) 0;
  }

  .veil-up-role-picker {
    margin-top: var(--space-2);
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
  }
  .veil-up-picker-title {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--veil-text-muted);
    margin-bottom: var(--space-2);
  }
  .veil-up-picker-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .veil-up-picker-item {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 10px;
    font-size: var(--text-xs);
    font-weight: 500;
    border-radius: var(--radius-md);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    color: var(--veil-text-secondary);
    cursor: pointer;
  }
  .veil-up-picker-item.assigned {
    background: var(--veil-brand-subtle);
    border-color: var(--veil-brand-border);
    color: var(--veil-brand);
    font-weight: 600;
  }
  .veil-up-picker-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-text-muted);
  }

  .veil-up-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-offline);
  }
  .veil-up-dot.online { background: var(--veil-online); }
  .veil-up-dot.away { background: var(--veil-away); }
  .veil-up-dot.dnd { background: var(--veil-dnd); }

  .veil-up-mutual-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    max-height: 200px;
    overflow-y: auto;
  }

  .veil-up-mutual-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;
  }
  .veil-up-mutual-item:hover {
    background: var(--veil-bg-elevated);
  }

  .veil-up-space-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--veil-text-secondary);
  }

  .veil-up-mutual-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .veil-up-mutual-name {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
  }
  .veil-up-mutual-tag {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-family: var(--font-mono);
  }

  .veil-close-profile {
    margin-top: var(--space-2);
    align-self: center;
  }
</style>

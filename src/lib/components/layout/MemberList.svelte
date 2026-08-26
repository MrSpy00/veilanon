<script lang="ts" module>
  const memberCache = new Map<string, MemberInfo[]>();
  const roleCache = new Map<string, RoleInfo[]>();
</script>

<script lang="ts">
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { friendsStore } from '$lib/stores/friends';
  import { authStore } from '$lib/stores/auth';
  import { memberApi, roleApi, dmApi, type MemberInfo, type RoleInfo, type PresenceStatus } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import { permissionsStore } from '$lib/stores/permissions';
  import Avatar from '../ui/Avatar.svelte';
  import ContextMenu, { type ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';

  const ui = $derived($uiStore);
  const auth = $derived($authStore);

  let members = $state<MemberInfo[]>([]);
  let roles = $state<RoleInfo[]>([]);
  let loading = $state(false);

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);

  const spaceId = $derived(ui.activeSpaceId);
  const activeDmChannel = $derived($spaceStore.dmChannels.find(d => d.id === ui.activeDmId) ?? null);
  const isGroupDm = $derived(activeDmChannel?.channelType === 'group_dm');
  const targetId = $derived(spaceId || (isGroupDm ? ui.activeDmId : null));
  const isOwner = $derived(spaceId ? ($spaceStore.spaces.find(s => s.id === spaceId)?.isOwner ?? false) : false);

  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  $effect(() => {
    const id = targetId;
    if (!id) {
      members = [];
      roles = [];
      return;
    }
    const cachedMembers = memberCache.get(id);
    const cachedRoles = spaceId ? roleCache.get(id) : [];
    if (cachedMembers && cachedMembers.length > 0) {
      members = cachedMembers;
      if (cachedRoles) roles = cachedRoles;
      loading = false;
    } else {
      loading = true;
    }
    let cancelled = false;
    void memberApi.list(id).then((m) => {
      if (!cancelled) {
        members = m;
        memberCache.set(id, m);
      }
    }).catch(() => {
      if (!cancelled && !cachedMembers) members = [];
    }).finally(() => {
      if (!cancelled) loading = false;
    });
    if (spaceId) {
      void roleApi.list(id).then((r) => {
        if (!cancelled) {
          roles = r;
          roleCache.set(id, r);
        }
      }).catch(() => {});
    } else {
      roles = [];
    }
    return () => { cancelled = true; };
  });

  onMount(() => {
    const unlisteners: Array<Promise<() => void>> = [];

    unlisteners.push(
      listen('roles:changed', (e: { payload?: { spaceId?: string } }) => {
        if (targetId && (!e.payload?.spaceId || e.payload.spaceId === targetId)) {
          if (spaceId) roleApi.list(spaceId).then((r) => (roles = r)).catch(() => {});
          memberApi.list(targetId).then((m) => (members = m)).catch(() => {});
          reloadMembersImmediate();
        }
      })
    );

    unlisteners.push(
      listen('members:changed', (e: { payload?: { spaceId?: string } }) => {
        if (targetId && (!e.payload?.spaceId || e.payload.spaceId === targetId)) {
          memberApi.list(targetId).then((m) => (members = m)).catch(() => {});
        }
      })
    );

    unlisteners.push(
      listen('presence:changed', (e: any) => {
        const p = e.payload;
        if (!p) return;
        const uid = p.user_id || p.userId;
        const status = p.status || 'online';
        if (uid) {
          batchPresenceUpdate(uid, status);
        } else {
          reloadMembers();
        }
      })
    );

    unlisteners.push(
      listen('veilanon:presence', () => {
        reloadMembers();
      })
    );

    unlisteners.push(
      listen('user:updated', (e: any) => {
        const p = e.payload;
        if (p?.id) {
          members = members.map((m) =>
            m.userId === p.id
              ? {
                  ...m,
                  displayName: p.display_name ?? m.displayName,
                  username: p.username ?? m.username,
                  avatarHash: p.avatar_hash !== undefined ? p.avatar_hash : m.avatarHash,
                }
              : m
          );
        }
        reloadMembers();
      })
    );

    unlisteners.push(
      listen('space:updated', (e: any) => {
        if (spaceId && (!e.payload?.spaceId || e.payload.spaceId === spaceId)) {
          reloadMembers();
        }
      })
    );

    unlisteners.push(
      listen('veilanon:broadcast', (e: any) => {
        const payload = e.payload;
        if (!payload) return;
        if (payload.type === 'presence' && payload.user_id && payload.status) {
          batchPresenceUpdate(payload.user_id, payload.status);
        } else if (
          payload.type === 'profile_updated' ||
          payload.type === 'avatar_updated' ||
          payload.type === 'user_updated'
        ) {
          const uid = payload.user_id || payload.id;
          if (uid) {
            members = members.map((m) =>
              m.userId === uid
                ? {
                    ...m,
                    displayName: payload.display_name ?? m.displayName,
                    username: payload.username ?? m.username,
                    avatarHash: payload.avatar_hash !== undefined ? payload.avatar_hash : m.avatarHash,
                  }
                : m
            );
          }
          reloadMembers();
        }
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  });

  const roleById = $derived(new Map(roles.map(r => [r.id, r])));

  // Kendi satırını canlı kimlikle senkronla: ayarlardan görünen ad/avatar
  // değişince üye listesi de anında güncellenir.
  const me = $derived($authStore.identity);
  const friendsList = $derived($friendsStore.friends);

  const activeMembers = $derived(members.filter(m => isMemberActive(m)));
  const offlineMembers = $derived(members.filter(m => !isMemberActive(m)));

  function isMemberActive(m: MemberInfo): boolean {
    const p = myPresence(m);
    return p === 'online' || p === 'away' || p === 'dnd';
  }

  function memberRoleColor(m: MemberInfo): string | null {
    if (!m.roleIds || m.roleIds.length === 0) return null;
    for (const role of roles) {
      if (m.roleIds.includes(role.id) && role.color) {
        return role.color;
      }
    }
    for (const rid of m.roleIds) {
      const r = roleById.get(rid);
      if (r?.color) return r.color;
    }
    return null;
  }

  function myDisplayName(m: MemberInfo): string {
    return m.userId === me?.id && me?.displayName ? me.displayName : (m.displayName || m.username);
  }

  function myAvatar(m: MemberInfo): string | null | undefined {
    return m.userId === me?.id && me?.avatarHash !== undefined ? me.avatarHash : m.avatarHash;
  }

  function myPresence(m: MemberInfo): 'online' | 'away' | 'dnd' | 'offline' | 'invisible' {
    if (m.userId === me?.id) return ui.presence;
    return m.onlineStatus === 'invisible' ? 'offline' : m.onlineStatus;
  }

  function getFriendStatus(userId: string): string {
    const friend = friendsList.find(f => f.userId === userId);
    return friend?.status ?? 'none';
  }

  function isFriend(userId: string): boolean {
    const s = getFriendStatus(userId);
    return s === 'friends' || s === 'accepted';
  }

  function openProfile(m: MemberInfo) {
    uiStore.openModal('user-profile', {
      userId: m.userId,
      username: m.username,
      displayName: m.displayName,
      avatarHash: m.avatarHash,
      onlineStatus: m.onlineStatus,
    });
  }

  async function openDm(m: MemberInfo) {
    try {
      const channel = await dmApi.open(m.userId);
      await spaceStore.loadDms();
      uiStore.navigateDm(channel.id);
    } catch {
      toastStore.error('DM kanalı açılamadı.');
    }
  }

  async function addFriend(m: MemberInfo) {
    try {
      await friendsStore.add(m.username);
      toastStore.success('Arkadaşlık isteği gönderildi.');
    } catch {
      toastStore.error('İstek gönderilemedi.');
    }
  }

  const myHighestRolePosition = $derived.by(() => {
    if (isOwner) return Infinity;
    const myMember = members.find(m => m.userId === me?.id);
    if (!myMember) return 0;
    let highest = 0;
    for (const rid of myMember.roleIds) {
      const r = roleById.get(rid);
      if (r && r.position > highest) highest = r.position;
    }
    return highest;
  });

  function getMemberHighestRolePosition(m: MemberInfo): number {
    let highest = 0;
    for (const rid of m.roleIds) {
      const r = roleById.get(rid);
      if (r && r.position > highest) highest = r.position;
    }
    return highest;
  }

  function canManageTarget(m: MemberInfo): boolean {
    if (m.userId === me?.id) return false;
    if (isOwner) return true;
    const myRank = myHighestRolePosition;
    const targetRank = getMemberHighestRolePosition(m);
    return myRank > targetRank;
  }

  function openMemberMenu(e: MouseEvent, m: MemberInfo) {
    e.preventDefault();
    e.stopPropagation();
    const isSelf = m.userId === me?.id;
    const items: ContextMenuItem[] = [];

    if (isSelf) {
      items.push(
        { label: 'Profilimi Gör', icon: 'user', onClick: () => openProfile(m) },
        { label: 'Ayarlar', icon: 'settings', onClick: () => uiStore.openModal('settings') },
        { label: '', separator: true },
        {
          label: 'Profil Bağlantısını Kopyala',
          icon: 'link',
          onClick: async () => {
            await copyText(`https://veilanon.com/u/${m.username}`);
            toastStore.success('Profil bağlantısı kopyalandı.');
          },
        },
        {
          label: 'Kullanıcı Adını Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(`@${m.username}`);
            toastStore.success('Kullanıcı adı kopyalandı.');
          },
        },
        {
          label: 'Kullanıcı ID\'sini Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(m.userId);
            toastStore.success('Kullanıcı ID\'si kopyalandı.');
          },
        },
      );
    } else {
      const friendStatus = getFriendStatus(m.userId);
      const isAlreadyFriend = friendStatus === 'friends' || friendStatus === 'accepted';
      const hasPendingRequest = friendStatus === 'pending_incoming' || friendStatus === 'pending_outgoing';

      items.push(
        { label: 'Profili Gör', icon: 'user', onClick: () => openProfile(m) },
        { label: 'Mesaj Gönder (DM)', icon: 'chat', onClick: () => void openDm(m) },
      );

      if (isAlreadyFriend) {
        items.push({ label: 'Zaten Arkadaş', icon: 'check', disabled: true });
      } else if (hasPendingRequest) {
        items.push({ label: 'İstek Bekleniyor', icon: 'clock', disabled: true });
      } else {
        items.push({ label: 'Arkadaş Ekle', icon: 'plus', onClick: () => void addFriend(m) });
      }

      items.push(
        { label: '', separator: true },
        {
          label: 'Profil Bağlantısını Kopyala',
          icon: 'link',
          onClick: async () => {
            await copyText(`https://veilanon.com/u/${m.username}`);
            toastStore.success('Profil bağlantısı kopyalandı.');
          },
        },
        {
          label: 'Kullanıcı Adını Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(`@${m.username}`);
            toastStore.success('Kullanıcı adı kopyalandı.');
          },
        },
        {
          label: 'Kullanıcı ID\'sini Kopyala',
          icon: 'copy',
          onClick: async () => {
            await copyText(m.userId);
            toastStore.success('Kullanıcı ID\'si kopyalandı.');
          },
        },
      );
      const effective = $permissionsStore;
      const canManageRank = canManageTarget(m);

      if (effective.isOwner || (canManageRank && (effective.has('manage_roles') || effective.has('timeout_members') || effective.has('kick_members') || effective.has('ban_members')))) {
        items.push({ label: '', separator: true });

        if (effective.isOwner || (canManageRank && effective.has('manage_roles'))) {
          items.push({
            label: 'Rolleri Yönet…',
            icon: 'shield',
            onClick: () => {
              if (spaceId) uiStore.openModal('channel-settings', { spaceId, tab: 'members' });
            },
          });
        }
        if (effective.isOwner || (canManageRank && effective.has('timeout_members'))) {
          items.push({
            label: 'Sustur…',
            icon: 'moon',
            onClick: () => void timeoutMember(m),
          });
        }
        if (effective.isOwner || (canManageRank && effective.has('kick_members'))) {
          items.push({ label: 'Sunucudan At (Kick)', icon: 'logout', onClick: () => void kickMember(m) });
        }
        if (effective.isOwner || (canManageRank && effective.has('ban_members'))) {
          items.push({ label: 'Sunucudan Yasakla (Ban)', icon: 'x', danger: true, onClick: () => void banMember(m) });
        }
      }
    }
    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  async function kickMember(m: MemberInfo) {
    if (!spaceId) return;
    const ok = await uiStore.confirm(
      `${m.displayName || m.username} kullanıcısını topluluktan atmak istiyor musun?`,
      { title: 'Üyeyi At', confirmLabel: 'At', danger: true }
    );
    if (!ok) return;
    try {
      await memberApi.kick({ spaceId, userId: m.userId });
      toastStore.success('Üye atıldı.');
      reloadMembers();
    } catch (err) {
      toastStore.error(`Atılamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function banMember(m: MemberInfo) {
    if (!spaceId) return;
    const reason = await uiStore.promptInput(
      `Yasaklama nedeni (isteğe bağlı):`,
      { title: `${m.displayName || m.username} kullanıcısını yasakla`, confirmLabel: 'Yasakla' }
    );
    if (reason === null) return;
    const ok = await uiStore.confirm(
      `${m.displayName || m.username} kullanıcısı yasaklanacak ve davetle bile geri dönemeyecek.`,
      { title: 'Yasakla', confirmLabel: 'Yasakla', danger: true }
    );
    if (!ok) return;
    try {
      await memberApi.ban({ spaceId, userId: m.userId, reason: reason.trim() || null });
      toastStore.success('Üye yasaklandı.');
      reloadMembers();
    } catch (err) {
      toastStore.error(`Yasaklanamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function timeoutMember(m: MemberInfo) {
    if (!spaceId) return;
    const minutes = await uiStore.promptInput(
      `Susturma süresi (dakika, 0 = kaldır):`,
      { title: `${m.displayName || m.username} kullanıcısını sustur`, confirmLabel: 'Sustur', defaultValue: '30' }
    );
    if (minutes === null) return;
    const n = Number(minutes.trim());
    if (!Number.isFinite(n) || n < 0) {
      toastStore.error('Geçerli bir dakika değeri gir.');
      return;
    }
    const until = n === 0 ? null : Math.floor(Date.now() / 1000) + n * 60;
    try {
      await memberApi.timeout({ spaceId, userId: m.userId, until });
      toastStore.success(n === 0 ? 'Susturma kaldırıldı.' : `${n} dakikalık susturma uygulandı.`);
    } catch (err) {
      toastStore.error(`Susturulamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  // Group members by role / presence
  const memberGroups = $derived.by(() => {
    const groups: Array<{ name: string; color?: string | null; count: number; members: MemberInfo[] }> = [];
    const assigned = new Set<string>();

    // 1. Custom Space Roles (sorted by position descending)
    const customRoles = [...roles]
      .filter((r) => !r.isDefault && r.name !== '@everyone')
      .sort((a, b) => b.position - a.position);

    for (const role of customRoles) {
      const roleMembers = activeMembers.filter(
        (m) => m.roleIds.includes(role.id) && !assigned.has(m.userId)
      );
      if (roleMembers.length > 0) {
        roleMembers.forEach((m) => assigned.add(m.userId));
        groups.push({
          name: role.name,
          color: role.color,
          count: roleMembers.length,
          members: roleMembers,
        });
      }
    }

    // 2. Remaining active members
    const unassignedActive = activeMembers.filter((m) => !assigned.has(m.userId));
    if (unassignedActive.length > 0) {
      groups.push({
        name: 'Çevrimiçi',
        count: unassignedActive.length,
        members: unassignedActive,
      });
    }

    // 3. Offline members
    if (offlineMembers.length > 0) {
      groups.push({
        name: 'Çevrimdışı',
        count: offlineMembers.length,
        members: offlineMembers,
      });
    }

    return groups;
  });

  let reloadTimer: ReturnType<typeof setTimeout> | null = null;
  let presenceBatchTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingPresenceUpdates = new Map<string, string>();

  async function reloadMembersImmediate() {
    const id = targetId;
    if (!id) return;
    try {
      if (spaceId) {
        const [m, r] = await Promise.all([memberApi.list(id), roleApi.list(id)]);
        members = m;
        roles = r;
      } else {
        const m = await memberApi.list(id);
        members = m;
        roles = [];
      }
    } catch { /* keep previous list */ }
  }

  function reloadMembers() {
    if (reloadTimer) clearTimeout(reloadTimer);
    reloadTimer = setTimeout(reloadMembersImmediate, 500);
  }

  function batchPresenceUpdate(userId: string, status: string) {
    pendingPresenceUpdates.set(userId, status);
    if (presenceBatchTimer) clearTimeout(presenceBatchTimer);
    presenceBatchTimer = setTimeout(() => {
      const updates = pendingPresenceUpdates;
      pendingPresenceUpdates = new Map();
      if (updates.size > 0) {
        members = members.map((m) => {
          const newStatus = updates.get(m.userId);
          return newStatus ? { ...m, onlineStatus: newStatus as PresenceStatus } : m;
        });
      }
    }, 150);
  }
</script>

<aside class="veil-member-list" aria-label="Üyeler">
  <div class="veil-member-header">Üyeler — {members.length}</div>
  <div class="veil-member-scroll">
    {#if loading && members.length === 0}
      <div class="veil-member-loading"><div class="veil-spinner veil-spinner-sm"></div></div>
    {:else}
      {#each memberGroups as group (group.name)}
        <div class="veil-member-category" style={group.color ? `color:${group.color};` : ''}>
          {group.name} — {group.count}
        </div>
        {#each group.members as m (m.userId)}
          {@const roleColor = memberRoleColor(m)}
          <button
            class="veil-member-row"
            class:offline={!isMemberActive(m)}
            onclick={() => openProfile(m)}
            oncontextmenu={(e) => openMemberMenu(e, m)}
          >
            <Avatar name={myDisplayName(m)} size="sm" hash={myAvatar(m)} presence={myPresence(m)} />
            <div class="veil-member-info">
              <span class="veil-member-name" style={roleColor ? `color:${roleColor};font-weight:600;` : ''}>
                {myDisplayName(m)}
              </span>
              {#if m.roleIds.length > 0}
                <div class="veil-member-roles">
                  {#each m.roleIds.slice(0, 3) as rid (rid)}
                    {@const role = roleById.get(rid)}
                    {#if role && role.name !== '@everyone'}
                      <span
                        class="veil-member-role-badge"
                        style={role.color ? `--role-color:${role.color}; border-color:${role.color};` : ''}
                        title={role.name}
                      >
                        <span class="veil-role-dot" style={role.color ? `background:${role.color};` : ''}></span>
                        {role.name}
                      </span>
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
          </button>
        {/each}
      {/each}

      {#if members.length === 0 && !loading}
        <div class="veil-member-empty">
          Bu toplulukta henüz üye yok. Davet linki paylaşarak arkadaşlarını davet et!
        </div>
      {/if}
    {/if}
  </div>
</aside>

<ContextMenu open={menuOpen} x={menuX} y={menuY} items={menuItems} onClose={() => (menuOpen = false)} />

<style>
  .veil-member-list {
    grid-area: members;
    width: var(--member-list-width);
    background: var(--veil-channel-bg);
    border-left: 1px solid var(--veil-border-subtle);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .veil-member-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 var(--space-4);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
    border-bottom: 1px solid var(--veil-border-subtle);
    flex-shrink: 0;
  }
  .veil-member-scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
  }
  .veil-member-category {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
    padding: var(--space-3) var(--space-2) var(--space-2);
  }
  .veil-member-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    cursor: pointer;
    color: var(--veil-text-secondary);
    text-align: left;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-member-row:hover { background: var(--veil-channel-hover); color: var(--veil-text-primary); }
  .veil-member-name {
    font-size: var(--text-sm);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-member-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-member-row.offline { opacity: 0.55; }
  .veil-member-roles { display: inline-flex; flex-wrap: wrap; gap: 4px; }
  .veil-member-role-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    font-size: 10px;
    font-weight: 600;
    border-radius: var(--radius-full);
    background: var(--veil-bg-surface);
    color: var(--veil-text-secondary);
    border: 1px solid var(--veil-border-subtle);
  }
  .veil-role-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--veil-brand);
  }
  .veil-member-empty {
    font-size: var(--text-sm);
    color: var(--veil-text-muted);
    padding: var(--space-3) var(--space-2);
    line-height: var(--leading-relaxed);
  }
  .veil-member-loading {
    display: flex;
    justify-content: center;
    padding: var(--space-6) 0;
  }
</style>

/**
 * veilanon — Permissions Store
 * Reactive store for currently active space/channel permissions and hierarchy checks.
 */

import { derived, writable, get } from 'svelte/store';
import { uiStore } from '$lib/stores/ui';
import { spaceStore } from '$lib/stores/spaces';
import { authStore } from '$lib/stores/auth';
import {
  computeEffectivePermissions,
  getMemberHighestRolePosition,
  canActorManageTarget as utilCanActorManageTarget,
  canActorManageRole as utilCanActorManageRole,
  type EffectivePermissions,
} from '$lib/utils/permissions';
import { roleApi, memberApi, channelApi, type RoleInfo, type MemberInfo, type ChannelOverrideItem } from '$lib/api/tauri';

interface PermissionsCacheState {
  spaceId: string | null;
  channelId: string | null;
  roles: RoleInfo[];
  members: MemberInfo[];
  channelOverrides: Record<string, ChannelOverrideItem[]>; // keyed by channelId
  loading: boolean;
}

function createPermissionsStore() {
  const cache = writable<PermissionsCacheState>({
    spaceId: null,
    channelId: null,
    roles: [],
    members: [],
    channelOverrides: {},
    loading: false,
  });

  // Track active space / channel to refresh
  uiStore.subscribe(async (ui) => {
    const spaceId = ui.activeSpaceId;
    const channelId = ui.activeChannelId;

    if (!spaceId) {
      cache.set({
        spaceId: null,
        channelId: null,
        roles: [],
        members: [],
        channelOverrides: {},
        loading: false,
      });
      return;
    }

    cache.update((c) => ({ ...c, spaceId, channelId, loading: true }));

    try {
      const [roles, members] = await Promise.all([
        roleApi.list(spaceId).catch(() => []),
        memberApi.list(spaceId).catch(() => []),
      ]);

      let overrides: ChannelOverrideItem[] = [];
      if (channelId) {
        overrides = await channelApi.getOverrides(channelId).catch(() => []);
      }

      cache.update((c) => ({
        ...c,
        spaceId,
        channelId,
        roles,
        members,
        channelOverrides: channelId ? { ...c.channelOverrides, [channelId]: overrides } : c.channelOverrides,
        loading: false,
      }));
    } catch {
      cache.update((c) => ({ ...c, loading: false }));
    }
  });

  const effective = derived(
    [cache, spaceStore, authStore, uiStore],
    ([$cache, $spaces, $auth, $ui]): EffectivePermissions => {
      const spaceId = $ui.activeSpaceId;
      const channelId = $ui.activeChannelId;
      const myId = $auth.identity?.id;

      if (!spaceId || !myId) {
        // DM or no active space -> unrestricted
        return {
          has: () => true,
          isOwner: true,
          isAdmin: true,
          isTimedOut: false,
          timeoutRemainingSeconds: 0,
          allEnabledIds: [],
        };
      }

      const space = $spaces.spaces.find((s) => s.id === spaceId);
      const isOwner = (space?.isOwner ?? false) || (space?.ownerId === myId);
      const myMember = $cache.members.find((m) => m.userId === myId);
      const defaultRole = $cache.roles.find((r) => r.isDefault || r.name === '@everyone');
      const userRoles = $cache.roles.filter(
        (r) => (defaultRole && r.id === defaultRole.id) || (myMember?.roleIds?.includes(r.id))
      );
      const channelOverrides = channelId ? $cache.channelOverrides[channelId] ?? [] : [];

      return computeEffectivePermissions({
        isOwner,
        userId: myId,
        userRoles,
        allRoles: $cache.roles,
        channelOverrides,
        timeoutUntil: (myMember as any)?.timeoutUntil ?? null,
      });
    }
  );

  return {
    subscribe: effective.subscribe,
    cache: { subscribe: cache.subscribe },

    async refresh(spaceId?: string, channelId?: string) {
      const ui = get(uiStore);
      const sid = spaceId ?? ui.activeSpaceId;
      const cid = channelId ?? ui.activeChannelId;
      if (!sid) return;

      try {
        const [roles, members] = await Promise.all([
          roleApi.list(sid).catch(() => []),
          memberApi.list(sid).catch(() => []),
        ]);

        let overrides: ChannelOverrideItem[] = [];
        if (cid) {
          overrides = await channelApi.getOverrides(cid).catch(() => []);
        }

        cache.update((c) => ({
          ...c,
          spaceId: sid,
          channelId: cid,
          roles,
          members,
          channelOverrides: cid ? { ...c.channelOverrides, [cid]: overrides } : c.channelOverrides,
          loading: false,
        }));
      } catch {
        // ignore
      }
    },

    getMemberHighestRole(member: MemberInfo | null | undefined, isOwner = false) {
      const c = get(cache);
      return getMemberHighestRolePosition(member, c.roles, isOwner);
    },

    canManageTarget(targetUserId: string, targetIsOwner = false, requiredPerm?: string) {
      const c = get(cache);
      const auth = get(authStore);
      const ui = get(uiStore);
      const spaces = get(spaceStore);

      const myId = auth.identity?.id;
      if (!myId) return false;
      const spaceId = ui.activeSpaceId;
      const space = spaces.spaces.find((s) => s.id === spaceId);
      const isActorOwner = (space?.isOwner ?? false) || (space?.ownerId === myId);

      return utilCanActorManageTarget(
        myId,
        targetUserId,
        c.roles,
        c.members,
        isActorOwner,
        targetIsOwner,
        requiredPerm
      );
    },

    canManageRole(targetRole: RoleInfo | { position?: number; permissions?: string[] } | null) {
      const c = get(cache);
      const auth = get(authStore);
      const ui = get(uiStore);
      const spaces = get(spaceStore);

      const myId = auth.identity?.id;
      if (!myId) return false;
      const spaceId = ui.activeSpaceId;
      const space = spaces.spaces.find((s) => s.id === spaceId);
      const isActorOwner = (space?.isOwner ?? false) || (space?.ownerId === myId);

      return utilCanActorManageRole(myId, targetRole, c.roles, c.members, isActorOwner);
    },
  };
}

export const permissionsStore = createPermissionsStore();

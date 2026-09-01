/**
 * Spaces (communities) store
 */
import { writable, get } from 'svelte/store';
import { spaceApi, channelApi, dmApi, inviteApi, type ChannelType, type SpaceInfo } from '$lib/api/tauri';
import { authStore } from '$lib/stores/auth';
export type { ChannelType };

export interface Space {
  id: string;
  name: string;
  iconHash: string | null;
  ownerId: string;
  memberCount: number;
  isOwner: boolean;
  myRoles: string[];
  bannerHash?: string | null;
  description?: string | null;
  customLink?: string | null;
}

export interface Channel {
  id: string;
  spaceId: string | null;
  name: string;
  channelType: 'text' | 'voice' | 'category' | 'announcement' | 'forum' | 'dm' | 'group_dm';
  position: number;
  isNsfw: boolean;
  isE2ee: boolean;
  unreadCount: number;
  mentioned: boolean;
  lastMessageId: string | null;
  avatarHash?: string | null;
  peerId?: string | null;
  onlineStatus?: string | null;
}

interface SpaceState {
  spaces: Space[];
  channelsBySpace: Record<string, Channel[]>;
  dmChannels: Channel[];
  loading: boolean;
}

function createSpaceStore() {
  const { subscribe, update, set } = writable<SpaceState>({
    spaces: [],
    channelsBySpace: {},
    dmChannels: [],
    loading: false,
  });

  let dmLoadInFlight = false;
  // Dedup maps: prevent concurrent identical fetches to reduce Supabase egress
  const spacesInFlight = new Map<string, Promise<Space[]>>();
  const channelsInFlight = new Map<string, Promise<Channel[]>>();

  /** 1:1 DM adı gerçek değilse (boş / "Direkt Mesaj" / UUID / kendi nick) doğrudur. */
  function isDmPlaceholder(name: string | null | undefined, selfName: string): boolean {
    const t = (name ?? '').trim();
    if (!t || t === 'Direkt Mesaj') return true;
    if (t.length === 36 && t.includes('-')) return true;
    if (selfName && t.toLowerCase() === selfName.toLowerCase()) return true;
    return false;
  }

  function authIdentitySelfName(): string {
    try {
      const auth = get(authStore);
      const d = auth.identity?.displayName?.trim();
      const u = auth.identity?.username?.trim();
      if (d) return d;
      if (u) return u;
    } catch { /* store may not be ready */ }
    return '';
  }

  return {
    subscribe,

    // ── Loading ─────────────────────────────────────────────
    async loadSpaces() {
      const key = 'all';
      const existing = spacesInFlight.get(key);
      if (existing) return existing;

      update(s => ({ ...s, loading: true }));
      const promise = spaceApi.list()
        .then((spaces) => {
          update(s => ({ ...s, spaces, loading: false }));
          return spaces;
        })
        .catch(() => {
          update(s => ({ ...s, loading: false }));
          return [] as Space[];
        })
        .finally(() => {
          spacesInFlight.delete(key);
        });

      spacesInFlight.set(key, promise);
      return promise;
    },

    async loadChannels(spaceId: string) {
      const existing = channelsInFlight.get(spaceId);
      if (existing) return existing;

      const promise = channelApi.list(spaceId)
        .then((channels) => {
          this.setChannels(spaceId, channels);
          return channels;
        })
        .catch(() => [] as Channel[])
        .finally(() => {
          channelsInFlight.delete(spaceId);
        });

      channelsInFlight.set(spaceId, promise);
      return promise;
    },

    async loadDms() {
      if (dmLoadInFlight) return;
      dmLoadInFlight = true;
      try {
        const dms = await dmApi.list();
        this.setDmChannels(dms);
      } catch {
        // Ignore — UI keeps previous state.
      } finally {
        dmLoadInFlight = false;
      }
    },

    refreshDms() {
      void this.loadDms();
    },

    // ── Mutations ───────────────────────────────────────────
    async createSpace(name: string, iconHash?: string | null): Promise<SpaceInfo> {
      const space = await spaceApi.create({ name, iconHash: iconHash ?? null });
      update(s => ({ ...s, spaces: [...s.spaces, space] }));
      return space;
    },

    async createChannel(spaceId: string, name: string, channelType: ChannelType, position?: number, e2ee = false) {
      const channel = await channelApi.create({ spaceId, name, channelType, position, e2ee });
      update(s => ({
        ...s,
        channelsBySpace: {
          ...s.channelsBySpace,
          [spaceId]: [...(s.channelsBySpace[spaceId] ?? []), channel],
        },
      }));
      return channel;
    },

    async invite(spaceId: string, maxUses?: number | null, expiresAt?: number | null) {
      return inviteApi.create({ spaceId, maxUses: maxUses ?? null, expiresAt: expiresAt ?? null });
    },

    async redeem(code: string): Promise<SpaceInfo> {
      try {
        const space = await inviteApi.redeem(code);
        update(s => ({
          ...s,
          spaces: s.spaces.some(sp => sp.id === space.id)
            ? s.spaces.map(sp => (sp.id === space.id ? space : sp))
            : [...s.spaces, space],
        }));
        await Promise.all([
          this.loadChannels(space.id).catch(() => []),
          this.loadSpaces().catch(() => {}),
        ]);
        if (typeof window !== 'undefined') {
          window.dispatchEvent(new CustomEvent('spaces:changed'));
        }
        return space;
      } catch (err) {
        await this.loadSpaces().catch(() => {});
        const refreshed = get({ subscribe }).spaces;
        const cleanCode = code.trim().replace(/^@/, '').toLowerCase();
        const found = refreshed.find(s =>
          s.id.toLowerCase() === cleanCode ||
          s.customLink?.toLowerCase() === cleanCode ||
          cleanCode.includes(s.id.toLowerCase()) ||
          (s.customLink && cleanCode.includes(s.customLink.toLowerCase()))
        );
        if (found) {
          await this.loadChannels(found.id).catch(() => []);
          return found;
        }
        throw err;
      }
    },

    async joinPublic(spaceIdOrLink: string): Promise<SpaceInfo> {
      try {
        const space = await spaceApi.joinPublic(spaceIdOrLink);
        update(s => ({
          ...s,
          spaces: s.spaces.some(sp => sp.id === space.id)
            ? s.spaces.map(sp => (sp.id === space.id ? space : sp))
            : [...s.spaces, space],
        }));
        await Promise.all([
          this.loadChannels(space.id).catch(() => []),
          this.loadSpaces().catch(() => {}),
        ]);
        if (typeof window !== 'undefined') {
          window.dispatchEvent(new CustomEvent('spaces:changed'));
        }
        return space;
      } catch (err) {
        await this.loadSpaces().catch(() => {});
        const refreshed = get({ subscribe }).spaces;
        const trimmed = spaceIdOrLink.trim().replace(/^@/, '').toLowerCase();
        const found = refreshed.find(s =>
          s.id.toLowerCase() === trimmed ||
          s.customLink?.toLowerCase() === trimmed ||
          trimmed.includes(s.id.toLowerCase()) ||
          (s.customLink && trimmed.includes(s.customLink.toLowerCase()))
        );
        if (found) {
          await this.loadChannels(found.id).catch(() => {});
          return found;
        }
        throw err;
      }
    },

    async transferOwnership(spaceId: string, newOwnerId: string): Promise<SpaceInfo> {
      const space = await spaceApi.transferOwnership(spaceId, newOwnerId);
      update(s => ({
        ...s,
        spaces: s.spaces.map(sp => (sp.id === space.id ? { ...sp, ...space } : sp)),
      }));
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('spaces:changed'));
      }
      return space;
    },

    async deleteSpace(spaceId: string): Promise<void> {
      await spaceApi.delete(spaceId);
      update(s => ({
        ...s,
        spaces: s.spaces.filter(sp => sp.id !== spaceId),
      }));
    },

    async leaveSpace(spaceId: string): Promise<void> {
      await spaceApi.leave(spaceId);
      update(s => ({
        ...s,
        spaces: s.spaces.filter(sp => sp.id !== spaceId),
      }));
    },

    applySpace(space: SpaceInfo) {
      update(s => ({
        ...s,
        spaces: s.spaces.map(sp => (sp.id === space.id ? { ...sp, ...space } : sp)),
      }));
    },

    addSpace(space: Space) {
      update(s => ({ ...s, spaces: [...s.spaces, space] }));
    },

    setChannels(spaceId: string, channels: Channel[]) {
      update(s => ({
        ...s,
        channelsBySpace: { ...s.channelsBySpace, [spaceId]: channels },
      }));
    },

    setDmChannels(channels: Channel[]) {
      update(s => {
        const selfName = authIdentitySelfName();
        const isPlaceholder = (name: string | null | undefined) =>
          isDmPlaceholder(name, selfName);
        if (s.dmChannels.length === channels.length) {
          let identical = true;
          for (let i = 0; i < channels.length; i++) {
            const a = s.dmChannels[i];
            const b = channels[i];
            if (a.id !== b.id || a.unreadCount !== b.unreadCount || a.mentioned !== b.mentioned) { identical = false; break; }
            const aIsPlaceholder = isPlaceholder(a.name);
            const bIsPlaceholder = isPlaceholder(b.name);
            if (!aIsPlaceholder && !bIsPlaceholder && a.name !== b.name) { identical = false; break; }
            if (a.avatarHash !== b.avatarHash || a.onlineStatus !== b.onlineStatus) { identical = false; break; }
          }
          if (identical) return s;
        }
        // Merge: keep existing good name if incoming is generic placeholder or UUID
        const merged = channels.map(nc => {
          const existing = s.dmChannels.find(e => e.id === nc.id);
          const ncIsPlaceholder = isPlaceholder(nc.name);
          if (existing && existing.name && !isPlaceholder(existing.name) && ncIsPlaceholder) {
            return { ...nc, name: existing.name, avatarHash: existing.avatarHash ?? nc.avatarHash };
          }
          // Also prefer existing name if it's more descriptive (longer, not UUID)
          if (existing && ncIsPlaceholder && existing.name.length > 2) {
            return { ...nc, name: existing.name, avatarHash: existing.avatarHash ?? nc.avatarHash };
          }
          return nc;
        });
        return { ...s, dmChannels: merged };
      });
    },

    markRead(channelId: string) {
      update(s => {
        const updated = { ...s };
        for (const spaceId in updated.channelsBySpace) {
          updated.channelsBySpace[spaceId] = updated.channelsBySpace[spaceId].map(ch =>
            ch.id === channelId ? { ...ch, unreadCount: 0, mentioned: false } : ch
          );
        }
        updated.dmChannels = updated.dmChannels.map(ch =>
          ch.id === channelId ? { ...ch, unreadCount: 0, mentioned: false } : ch
        );
        return updated;
      });
    },

    incrementUnread(channelId: string, isMention = false) {
      update(s => {
        const updated = { ...s };
        for (const spaceId in updated.channelsBySpace) {
          updated.channelsBySpace[spaceId] = updated.channelsBySpace[spaceId].map(ch =>
            ch.id === channelId
              ? {
                  ...ch,
                  unreadCount: (ch.unreadCount || 0) + 1,
                  mentioned: ch.mentioned || isMention,
                }
              : ch
          );
        }
        updated.dmChannels = updated.dmChannels.map(ch =>
          ch.id === channelId
            ? {
                ...ch,
                unreadCount: (ch.unreadCount || 0) + 1,
                mentioned: ch.mentioned || isMention,
              }
            : ch
        );
        return updated;
      });
    },

    reset() {
      set({
        spaces: [],
        channelsBySpace: {},
        dmChannels: [],
        loading: false,
      });
    },
  };
}

export const spaceStore = createSpaceStore();

/**
 * Spaces (communities) store
 */
import { writable } from 'svelte/store';
import { spaceApi, channelApi, dmApi, inviteApi, type ChannelType, type SpaceInfo } from '$lib/api/tauri';
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

  return {
    subscribe,

    // ── Loading ─────────────────────────────────────────────
    async loadSpaces() {
      update(s => ({ ...s, loading: true }));
      try {
        const spaces = await spaceApi.list();
        update(s => ({ ...s, spaces, loading: false }));
      } catch {
        update(s => ({ ...s, loading: false }));
      }
    },

    async loadChannels(spaceId: string) {
      try {
        const channels = await channelApi.list(spaceId);
        this.setChannels(spaceId, channels);
        return channels;
      } catch {
        return [];
      }
    },

    async loadDms() {
      try {
        const dms = await dmApi.list();
        this.setDmChannels(dms);
      } catch {
        // Ignore — UI keeps previous state.
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
      const space = await inviteApi.redeem(code);
      update(s => ({
        ...s,
        spaces: s.spaces.some(sp => sp.id === space.id)
          ? s.spaces.map(sp => (sp.id === space.id ? space : sp))
          : [...s.spaces, space],
      }));
      return space;
    },

    async joinPublic(spaceIdOrLink: string): Promise<SpaceInfo> {
      const space = await spaceApi.joinPublic(spaceIdOrLink);
      update(s => ({
        ...s,
        spaces: s.spaces.some(sp => sp.id === space.id)
          ? s.spaces.map(sp => (sp.id === space.id ? space : sp))
          : [...s.spaces, space],
      }));
      return space;
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
      update(s => ({ ...s, dmChannels: channels }));
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

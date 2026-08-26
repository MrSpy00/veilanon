<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { authStore } from '$lib/stores/auth';
  import { mediaStore } from '$lib/stores/media';
  import { toastStore } from '$lib/stores/notifications';
  import { channelNameFor } from '$lib/utils/channel';
  import { channelApi } from '$lib/api/tauri';
  import { permissionsStore } from '$lib/stores/permissions';
  import { streamerMode, maskDmText } from '$lib/stores/streamerMode';
  import Avatar from '../ui/Avatar.svelte';
  import BannerImage from '../ui/BannerImage.svelte';
  import Icon, { type IconName } from '../ui/Icon.svelte';
  import ContextMenu, { type ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';

  /** Mobile overlay state — controlled by AppLayout */
  let { open = false }: { open?: boolean } = $props();

  const ui = $derived($uiStore);
  const spaces = $derived($spaceStore);
  const auth = $derived($authStore);
  const media = $derived($mediaStore);

  const currentSpace = $derived(spaces.spaces.find(s => s.id === ui.activeSpaceId) ?? null);
  const channels = $derived(
    ui.activeSpaceId
      ? (spaces.channelsBySpace[ui.activeSpaceId] ?? [])
      : []
  );

  // Channel type icon
  function channelIcon(type: string): IconName {
    switch (type) {
      case 'voice': return 'volume';
      case 'announcement': return 'megaphone';
      case 'forum': return 'chat';
      default: return 'hash';
    }
  }

  const channelName = $derived(
    (function() {
      const name = channelNameFor(spaces.channelsBySpace, spaces.dmChannels, ui.activeSpaceId, media.channelId);
      if (name && name.length === 36 && name.includes('-')) return 'Ses Kanalı';
      return name || 'Ses Kanalı';
    })()
  );

  let joiningChannelId = $state<string | null>(null);
  interface VoiceUserEntry {
    id: string;
    name: string;
    avatarHash: string | null;
    isMuted?: boolean;
    isDeafened?: boolean;
    isVideoOn?: boolean;
    isScreenSharing?: boolean;
    isSpeaking?: boolean;
  }
  let remoteVoiceUsers = $state<Record<string, VoiceUserEntry[]>>({});

  onMount(() => {
    const unlisten = listen('veilanon:broadcast', (e: any) => {
      const p = e.payload;
      if (p?.type === 'request_voice_presence' || p?.action === 'query_voice_presence') {
        if (media.isInCall && media.channelId) {
          void invoke('broadcast_voice_state', {
            input: {
              channel_id: media.channelId,
              is_muted: media.isMuted,
              is_deafened: media.isDeafened,
              is_camera_on: media.isCameraOn,
              is_screen_sharing: media.isScreenSharing,
              is_speaking: media.isSpeaking,
            },
          }).catch(() => {});
        }
        return;
      }

      if (p?.type === 'voice_presence') {
        const cid = p.channel_id;
        const uid = p.user_id;
        const name = p.display_name || p.username || 'Kullanıcı';
        const avatarHash = p.avatar_hash || null;
        if (!uid) return;
        if ((p.action === 'join' || p.action === 'state') && cid) {
          const list = remoteVoiceUsers[cid] ?? [];
          const existingIdx = list.findIndex((u) => u.id === uid);
          const entry: VoiceUserEntry = {
            id: uid,
            name,
            avatarHash,
            isMuted: p.is_muted ?? false,
            isDeafened: p.is_deafened ?? false,
            isVideoOn: p.is_camera_on ?? false,
            isScreenSharing: p.is_screen_sharing ?? false,
            isSpeaking: p.is_speaking ?? false,
          };
          if (existingIdx !== -1) {
            const copy = [...list];
            copy[existingIdx] = { ...copy[existingIdx], ...entry };
            remoteVoiceUsers = { ...remoteVoiceUsers, [cid]: copy };
          } else {
            remoteVoiceUsers = {
              ...remoteVoiceUsers,
              [cid]: [...list, entry],
            };
          }
        } else if (p.action === 'leave') {
          const updated: Record<string, VoiceUserEntry[]> = {};
          for (const c in remoteVoiceUsers) {
            updated[c] = (remoteVoiceUsers[c] ?? []).filter((u) => u.id !== uid);
          }
          remoteVoiceUsers = updated;
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  });

  // Query voice presence when switching spaces so we immediately see who is in voice
  $effect(() => {
    if (ui.activeSpaceId) {
      void invoke('request_voice_presence').catch(() => {});
    }
  });

  // Broadcast local voice state to space members when active in a channel
  $effect(() => {
    if (media.isInCall && media.channelId) {
      void invoke('broadcast_voice_state', {
        input: {
          channel_id: media.channelId,
          is_muted: media.isMuted,
          is_deafened: media.isDeafened,
          is_camera_on: media.isCameraOn,
          is_screen_sharing: media.isScreenSharing,
          is_speaking: media.isSpeaking,
        },
      }).catch(() => {});
    }
  });

  async function handleVoiceClick(ch: { id: string; name: string; channelType: string }) {
    uiStore.navigate(ui.activeSpaceId, ch.id);
    if (media.isInCall && media.channelId === ch.id) {
      return;
    }
    joiningChannelId = ch.id;
    try {
      if (media.isInCall) {
        await mediaStore.switchVoiceChannel(ch.id);
      } else {
        await mediaStore.joinVoice(ch.id);
      }
    } catch {
      // error handled smoothly
    } finally {
      joiningChannelId = null;
    }
  }

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);

  async function deleteChannel(ch: { id: string; name: string }) {
    const ok = await uiStore.confirm(
      `"${ch.name}" kanalını ve tüm içeriklerini silmek istediğine emin misin?`,
      { title: 'Kanalı Sil', confirmLabel: 'Sil', danger: true }
    );
    if (!ok) return;
    try {
      await channelApi.delete(ch.id);
      if (currentSpace) {
        await spaceStore.loadChannels(currentSpace.id);
      }
      toastStore.success('Kanal silindi.');
    } catch (err) {
      toastStore.error(`Kanal silinemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  function openChannelMenu(e: MouseEvent, ch: { id: string; name: string; channelType: string }) {
    e.preventDefault();
    e.stopPropagation();
    const perms = $permissionsStore;
    const canManageChannel = perms.isOwner || perms.has('manage_channels');

    const items: ContextMenuItem[] = [
      {
        label: ch.channelType === 'voice' ? 'Sese Bağlan' : 'Kanalı Aç',
        icon: ch.channelType === 'voice' ? 'volume' : 'chat',
        onClick: () => {
          if (ch.channelType === 'voice') {
            void handleVoiceClick(ch);
          } else {
            uiStore.navigate(ui.activeSpaceId, ch.id);
          }
        },
      },
    ];

    if (canManageChannel) {
      items.push({
        label: 'Kanalı Düzenle & İzinler',
        icon: 'settings',
        onClick: () => {
          uiStore.openModal('channel-edit', { spaceId: currentSpace?.id, channelId: ch.id });
        },
      });
    }

    items.push(
      { label: '', separator: true },
      {
        label: 'Kanal Bağlantısını Kopyala',
        icon: 'link',
        onClick: async () => {
          await copyText(`https://veilanon.com/channel/${ch.id}`);
          toastStore.success('Kanal bağlantısı kopyalandı.');
        },
      },
      {
        label: 'Kanal ID\'sini Kopyala',
        icon: 'copy',
        onClick: async () => {
          await copyText(ch.id);
          toastStore.success('Kanal ID\'si kopyalandı.');
        },
      },
    );

    if (ch.channelType === 'voice') {
      items.push(
        { label: '', separator: true },
        {
          label: media.isInCall && media.channelId === ch.id ? 'Kanaldan Ayrıl' : 'Sese Katıl',
          icon: media.isInCall && media.channelId === ch.id ? 'phone-off' : 'volume',
          onClick: () => {
            if (media.isInCall && media.channelId === ch.id) {
              void mediaStore.leaveVoice();
            } else {
              void handleVoiceClick(ch);
            }
          },
        },
      );
    }

    if (canManageChannel) {
      items.push(
        { label: '', separator: true },
        {
          label: 'Kanalı Sil',
          icon: 'x',
          danger: true,
          onClick: () => void deleteChannel(ch),
        },
      );
    }

    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

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

  // Collapsible categories state
  let collapsedCategories = $state<Record<string, boolean>>({});
  function toggleCategory(catKey: string) {
    collapsedCategories[catKey] = !collapsedCategories[catKey];
  }

  // Audio quick popover state in voice bar
  let audioPopOpen = $state(false);
  let masterVolume = $state(100);

  function openVoiceParticipantMenu(e: MouseEvent, p: { id: string; name: string; avatarHash?: string | null }) {
    e.preventDefault();
    e.stopPropagation();
    let currentVol = Math.round(mediaStore.getParticipantVolume(p.id) * 100);
    menuItems = [
      {
        label: 'Kullanıcı Sesi',
        icon: 'volume',
        isSlider: true,
        sliderValue: currentVol,
        sliderMin: 0,
        sliderMax: 200,
        onSliderChange: (val: number) => {
          currentVol = val;
          mediaStore.setParticipantVolume(p.id, val / 100);
        },
      },
      {
        label: currentVol === 0 ? 'Sesi Aç' : 'Kullanıcıyı Sustur',
        icon: currentVol === 0 ? 'volume' : 'volume-x',
        onClick: () => {
          const next = currentVol === 0 ? 100 : 0;
          currentVol = next;
          mediaStore.setParticipantVolume(p.id, next / 100);
        },
      },
      { label: '', separator: true },
      {
        label: 'Profili Gör',
        icon: 'user',
        onClick: () => {
          uiStore.openModal('user-profile', {
            userId: p.id,
            username: p.name,
            displayName: p.name,
            avatarHash: p.avatarHash,
          });
        },
      },
      {
        label: 'Kullanıcı Adını Kopyala',
        icon: 'copy',
        onClick: async () => {
          await copyText(`@${p.name}`);
          toastStore.success('Kullanıcı adı kopyalandı.');
        },
      },
    ];
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  function openVoiceSelfMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    menuItems = [
      {
        label: media.isMuted ? 'Mikrofonu Aç' : 'Mikrofonu Kapat',
        icon: media.isMuted ? 'mic-off' : 'mic',
        onClick: () => void mediaStore.toggleMute(),
      },
      {
        label: media.isDeafened ? 'Kulaklığı Aç' : 'Kulaklığı Kapat',
        icon: media.isDeafened ? 'volume-x' : 'volume',
        onClick: () => void mediaStore.toggleDeafen(),
      },
      {
        label: media.isCameraOn ? 'Kamerayı Kapat' : 'Kamerayı Aç',
        icon: media.isCameraOn ? 'camera' : 'video-off',
        onClick: () => void mediaStore.toggleCamera(),
      },
      { label: '', separator: true },
      {
        label: 'Ses Ayarları',
        icon: 'settings',
        onClick: () => uiStore.openModal('settings'),
      },
    ];
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  const canManageSpace = $derived(
    $permissionsStore.isOwner ||
    $permissionsStore.has('manage_space') ||
    $permissionsStore.has('manage_roles') ||
    $permissionsStore.has('manage_channels')
  );
</script>

<aside class="veil-channels" class:open>
  <!-- Space name header -->
  {#if currentSpace}
    <div class="veil-space-hero-wrap">
      {#if currentSpace.bannerHash}
        <div class="veil-channel-banner-container">
          <BannerImage hash={currentSpace.bannerHash} alt="" class="veil-channel-banner" />
        </div>
      {/if}
      <div class="veil-channel-header">
        {#if canManageSpace}
          <button
            type="button"
            class="veil-space-header-btn"
            onclick={() => {
              if (currentSpace) {
                uiStore.openModal('channel-settings', { spaceId: currentSpace.id });
              }
            }}
            title="Topluluk Ayarları"
          >
            {#if currentSpace.iconHash}
              <Avatar hash={currentSpace.iconHash} name={currentSpace.name} size="sm" />
            {/if}
            <span class="veil-space-header-name">{currentSpace.name}</span>
          </button>
          <button
            type="button"
            class="btn-icon veil-space-settings-btn"
            title="Topluluk Ayarları"
            aria-label="Topluluk ayarları"
            onclick={() => {
              if (currentSpace) {
                uiStore.openModal('channel-settings', { spaceId: currentSpace.id });
              }
            }}
          >
            <Icon name="cog" size={16} />
          </button>
        {:else}
          {#if currentSpace.iconHash}
            <Avatar hash={currentSpace.iconHash} name={currentSpace.name} size="sm" />
          {/if}
          <span class="veil-space-header-name">{currentSpace.name}</span>
        {/if}
      </div>
    </div>
  {:else}
    <div class="veil-channel-header veil-dm-header-bar" role="banner">
      <span class="veil-space-header-name">Direkt Mesajlar</span>
      <button
        class="btn-icon veil-space-settings-btn"
        title="Yeni Grup Sohbeti Oluştur"
        aria-label="Yeni Grup Sohbeti"
        onclick={() => uiStore.openModal('create-group-dm')}
      >
        <Icon name="plus" size={15} />
      </button>
    </div>
  {/if}

  <!-- Channel scroll area -->
  <div class="veil-channel-scroll">
    {#if ui.activeSpaceId}
      <!-- Text channels (text, announcement, forum) -->
      <div
        class="veil-category"
        role="button"
        tabindex="0"
        onclick={() => toggleCategory('text')}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleCategory('text'); }}
      >
        <div class="veil-category-left">
          <span class="veil-category-caret" class:expanded={!collapsedCategories['text']} aria-hidden="true">
            <Icon name="arrow-right" size={11} />
          </span>
          <span>Metin Kanalları</span>
        </div>
        {#if $permissionsStore.isOwner || $permissionsStore.has('manage_channels')}
          <button
            type="button"
            class="veil-category-action"
            title="Kanal Oluştur"
            aria-label="Metin Kanalı Oluştur"
            onclick={(e) => {
              e.stopPropagation();
              uiStore.openModal('create-channel', { spaceId: currentSpace?.id, defaultType: 'text' });
            }}
          >
            <Icon name="plus" size={14} />
          </button>
        {/if}
      </div>

      {#if !collapsedCategories['text']}
        {#each channels.filter(c => c.channelType === 'text' || c.channelType === 'announcement' || c.channelType === 'forum') as ch (ch.id)}
          <button
            class="veil-channel-item"
            class:active={ui.activeChannelId === ch.id}
            class:unread={ch.unreadCount > 0}
            class:mentioned={ch.mentioned}
            aria-current={ui.activeChannelId === ch.id ? 'page' : undefined}
            onclick={() => uiStore.navigate(ui.activeSpaceId, ch.id)}
            oncontextmenu={(e) => openChannelMenu(e, ch)}
          >
            <span class="veil-channel-icon" aria-hidden="true"><Icon name={channelIcon(ch.channelType)} size={16} /></span>
            <span class="veil-channel-name">{ch.name}</span>
            <span class="veil-channel-badges">
              {#if ch.mentioned}
                <span class="veil-badge" aria-label="Mention">@</span>
              {:else if ch.unreadCount > 0}
                <span class="veil-badge" aria-label="{ch.unreadCount} unread">
                  {ch.unreadCount > 99 ? '99+' : ch.unreadCount}
                </span>
              {/if}
              {#if ch.isE2ee}
                <span class="veil-e2ee-mini" title="Uçtan uca şifreli" aria-label="End-to-end encrypted"><Icon name="lock" size={10} /></span>
              {/if}
            </span>
          </button>
        {/each}
      {/if}

      <!-- Voice channels -->
      {#if channels.filter(c => c.channelType === 'voice').length > 0}
        <div
          class="veil-category"
          role="button"
          tabindex="0"
          onclick={() => toggleCategory('voice')}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleCategory('voice'); }}
        >
          <div class="veil-category-left">
            <span class="veil-category-caret" class:expanded={!collapsedCategories['voice']} aria-hidden="true">
              <Icon name="arrow-right" size={11} />
            </span>
            <span>Ses Kanalları</span>
          </div>
          {#if $permissionsStore.isOwner || $permissionsStore.has('manage_channels')}
            <button
              type="button"
              class="veil-category-action"
              title="Ses Kanalı Oluştur"
              aria-label="Ses Kanalı Oluştur"
              onclick={(e) => {
                e.stopPropagation();
                uiStore.openModal('create-channel', { spaceId: currentSpace?.id, defaultType: 'voice' });
              }}
            >
              <Icon name="plus" size={14} />
            </button>
          {/if}
        </div>

        {#if !collapsedCategories['voice']}
          {#each channels.filter(c => c.channelType === 'voice') as ch (ch.id)}
            <div class="veil-voice-channel">
              <button
                class="veil-channel-item veil-voice-item"
                class:active={ui.activeChannelId === ch.id}
                class:connected={media.isInCall && media.channelId === ch.id}
                class:joining={joiningChannelId === ch.id}
                aria-pressed={media.isInCall && media.channelId === ch.id}
                onclick={() => handleVoiceClick(ch)}
                oncontextmenu={(e) => openChannelMenu(e, ch)}
              >
                <span class="veil-channel-icon" aria-hidden="true">
                  {#if joiningChannelId === ch.id}
                    <div class="veil-spinner veil-spinner-sm"></div>
                  {:else}
                    <Icon name="volume" size={16} />
                  {/if}
                </span>
                <span class="veil-channel-name">{ch.name}</span>
                {#if media.isInCall && media.channelId === ch.id}
                  <span class="veil-live-badge" title="Bağlısın" aria-label="Bağlısın"><span class="veil-live-dot" aria-hidden="true"></span></span>
                {/if}
              </button>
              {#if media.isInCall && media.channelId === ch.id}
                <div class="veil-voice-members" role="list" aria-label="Ses kanalındakiler">
                  {#each media.participants as p (p.id)}
                    <div
                      class="veil-voice-member"
                      role="listitem"
                      title={p.name}
                      oncontextmenu={(e) => openVoiceParticipantMenu(e, p)}
                    >
                      <Avatar name={p.name} hash={p.avatarHash} size="sm" speaking={p.isSpeaking} />
                      <span class="veil-voice-member-name" class:speaking={p.isSpeaking}>{p.name}</span>
                      <div class="veil-voice-badges">
                        {#if p.isMuted}
                          <span class="veil-voice-status-icon muted" title="Mikrofon Kapalı"><Icon name="mic-off" size={12} /></span>
                        {/if}
                        {#if p.isDeafened}
                          <span class="veil-voice-status-icon deafened" title="Kulaklık Kapalı"><Icon name="volume-x" size={12} /></span>
                        {/if}
                        {#if p.isVideoOn}
                          <span class="veil-voice-status-icon camera" title="Kamera Açık"><Icon name="camera" size={12} /></span>
                        {/if}
                        {#if p.isScreenSharing}
                          <span class="veil-voice-status-icon screen" title="Ekran Paylaşıyor"><Icon name="broadcast" size={12} /></span>
                        {/if}
                      </div>
                    </div>
                  {/each}
                  <div
                    class="veil-voice-member"
                    role="listitem"
                    oncontextmenu={openVoiceSelfMenu}
                  >
                    <Avatar
                      name={auth.identity?.displayName || auth.identity?.username || 'Sen'}
                      hash={auth.identity?.avatarHash}
                      size="sm"
                      speaking={media.isSpeaking}
                    />
                    <span class="veil-voice-member-name" class:speaking={media.isSpeaking}>
                      {auth.identity?.displayName || auth.identity?.username || 'Sen'}
                    </span>
                    <div class="veil-voice-badges">
                      {#if media.isMuted}
                        <span class="veil-voice-status-icon muted" title="Mikrofon Kapalı"><Icon name="mic-off" size={12} /></span>
                      {/if}
                      {#if media.isDeafened}
                        <span class="veil-voice-status-icon deafened" title="Kulaklık Kapalı"><Icon name="volume-x" size={12} /></span>
                      {/if}
                      {#if media.isCameraOn}
                        <span class="veil-voice-status-icon camera" title="Kamera Açık"><Icon name="camera" size={12} /></span>
                      {/if}
                      {#if media.isScreenSharing}
                        <span class="veil-voice-status-icon screen" title="Ekran Paylaşıyor"><Icon name="broadcast" size={12} /></span>
                      {/if}
                    </div>
                  </div>
                </div>
              {:else if (remoteVoiceUsers[ch.id]?.length ?? 0) > 0}
                <div class="veil-voice-members" role="list" aria-label="Ses kanalındakiler">
                  {#each remoteVoiceUsers[ch.id] as user (user.id)}
                    <div
                      class="veil-voice-member"
                      role="listitem"
                      title={user.name}
                      oncontextmenu={(e) => openVoiceParticipantMenu(e, user)}
                    >
                      <Avatar name={user.name} hash={user.avatarHash} size="sm" speaking={user.isSpeaking} />
                      <span class="veil-voice-member-name" class:speaking={user.isSpeaking}>{user.name}</span>
                      <div class="veil-voice-badges">
                        {#if user.isMuted}
                          <span class="veil-voice-status-icon muted" title="Mikrofon Kapalı"><Icon name="mic-off" size={12} /></span>
                        {/if}
                        {#if user.isDeafened}
                          <span class="veil-voice-status-icon deafened" title="Kulaklık Kapalı"><Icon name="volume-x" size={12} /></span>
                        {/if}
                        {#if user.isVideoOn}
                          <span class="veil-voice-status-icon camera" title="Kamera Açık"><Icon name="camera" size={12} /></span>
                        {/if}
                        {#if user.isScreenSharing}
                          <span class="veil-voice-status-icon screen" title="Ekran Paylaşıyor"><Icon name="broadcast" size={12} /></span>
                        {/if}
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        {/if}
      {/if}
    {:else}
      <!-- DM list -->
      {#each spaces.dmChannels as dm (dm.id)}
        {@const isGroup = dm.channelType === 'group_dm'}
        <button
          class="veil-channel-item"
          class:active={ui.activeDmId === dm.id}
          class:unread={dm.unreadCount > 0}
          onclick={() => uiStore.navigateDm(dm.id)}
          oncontextmenu={(e) => openDmMenu(e, dm)}
        >
          {#if isGroup}
            <div class="veil-dm-group-icon" aria-hidden="true">
              <Icon name="users" size={13} />
            </div>
          {:else}
            <div class="veil-dm-avatar-wrap" aria-hidden="true">
              <Avatar
                name={dm.name}
                hash={dm.avatarHash}
                presence={dm.onlineStatus === 'invisible' ? 'offline' : (dm.onlineStatus as any || 'offline')}
                size="sm"
              />
            </div>
          {/if}
          <span class="veil-channel-name" data-streamer-mask="dm">
            {$streamerMode.enabled && $streamerMode.hideDmContent ? maskDmText(dm.name) : dm.name}
          </span>
          {#if dm.unreadCount > 0}
            <span class="veil-badge">{dm.unreadCount}</span>
          {/if}
        </button>
      {/each}
    {/if}
  </div>
</aside>

<style>
  .veil-voice-channel { position: relative; }
  .veil-space-hero-wrap {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    position: relative;
    background: var(--veil-bg-elevated);
    border-bottom: 1px solid var(--veil-border-subtle);
    overflow: hidden;
  }
  .veil-channel-banner-container {
    width: 100%;
    height: 105px;
    max-height: 105px;
    position: relative;
    overflow: hidden;
    background:
      radial-gradient(120% 160% at 15% 0%, var(--veil-brand-subtle), transparent 55%),
      linear-gradient(160deg, var(--veil-bg-surface), var(--veil-bg-void));
  }
  .veil-channel-banner-container::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, var(--veil-channel-bg, #0d0f18) 0%, rgba(10, 12, 18, 0.45) 55%, transparent 100%);
    pointer-events: none;
  }
  :global(.veil-channel-banner) {
    width: 100%;
    height: 100%;
    display: block;
    overflow: hidden;
  }
  :global(.veil-channel-banner img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    display: block;
  }
  .veil-channel-header {
    height: var(--header-height);
    min-height: var(--header-height);
    max-height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 var(--space-3);
    font-weight: 700;
    font-size: var(--text-md);
    flex-shrink: 0;
    gap: var(--space-2);
    box-sizing: border-box;
    background: var(--veil-channel-bg);
  }
  .veil-space-header-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    background: transparent;
    border: none;
    font-size: var(--text-md);
    font-weight: 700;
    color: var(--veil-text-primary);
    cursor: pointer;
    flex: 1;
    min-width: 0;
    text-align: left;
    padding: 0;
  }
  .veil-channel-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--space-2) 0 var(--space-4);
  }
  .veil-voice-item.connected { color: var(--veil-success); }
  .veil-voice-item.connected .veil-channel-icon { color: var(--veil-success); }
  .veil-voice-item.joining { opacity: 0.7; }
  .veil-live-badge {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    margin-left: auto;
  }
  .veil-live-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-success);
    animation: veil-pulse 1.6s ease-in-out infinite;
  }
  @keyframes veil-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }
  .veil-voice-members {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 2px var(--space-2) var(--space-2) 30px;
  }
  .veil-voice-member {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 2px var(--space-1);
    border-radius: var(--radius-md);
    min-width: 0;
  }
  .veil-voice-member:hover { background: var(--veil-channel-hover); }
  .veil-voice-member-name {
    flex: 1;
    min-width: 0;
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--veil-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-voice-member-name.speaking { color: var(--veil-text-primary); }
  .veil-voice-badges {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    flex-shrink: 0;
  }
  .veil-voice-status-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    line-height: 1;
    opacity: 0.85;
  }
  .veil-voice-status-icon.muted { color: var(--veil-danger); }
  .veil-voice-status-icon.deafened { color: var(--veil-danger); }
  .veil-voice-status-icon.camera { color: var(--veil-success); }
  .veil-voice-status-icon.screen { color: var(--veil-brand); }

  .veil-dm-header-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .veil-dm-group-icon {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-full);
    background: var(--veil-brand-subtle, rgba(99, 102, 241, 0.15));
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .veil-dm-avatar-wrap {
    flex-shrink: 0;
  }
</style>

<ContextMenu open={menuOpen} x={menuX} y={menuY} items={menuItems} onClose={() => (menuOpen = false)} />

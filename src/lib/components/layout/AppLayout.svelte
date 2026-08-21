<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { authStore } from '$lib/stores/auth';
  import { mediaStore } from '$lib/stores/media';
  import { messageStore } from '$lib/stores/messages';
  import { friendsStore } from '$lib/stores/friends';
  import { handleDeepLink } from '$lib/utils/deeplink';
  import { streamerMode } from '$lib/stores/streamerMode';
  import Sidebar from './Sidebar.svelte';
  import StreamerBanner from './StreamerBanner.svelte';
  import ChannelList from './ChannelList.svelte';
  import MemberList from './MemberList.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import SettingsModal from '../settings/SettingsModal.svelte';

  const HomePromise = import('./Home.svelte');
  import CreateSpace from '../spaces/CreateSpace.svelte';
  import CreateChannelModal from '../spaces/CreateChannelModal.svelte';
  import CreateGroupDmModal from '../social/CreateGroupDmModal.svelte';
  import InviteModal from '../spaces/InviteModal.svelte';
  import SpaceSettings from '../spaces/SpaceSettings.svelte';
  import ChannelSettingsModal from '../spaces/ChannelSettingsModal.svelte';
  import RoleEditor from '../spaces/RoleEditor.svelte';
  import UserProfile from '../social/UserProfile.svelte';
  import VideoCall from '../media/VideoCall.svelte';
  import BottomUserBar from './BottomUserBar.svelte';
  import Modal from '../ui/Modal.svelte';
  import ConfirmDialog from '../ui/ConfirmDialog.svelte';
  import InputDialog from '../ui/InputDialog.svelte';

  const ui = $derived($uiStore);
  const media = $derived($mediaStore);
  const spaces = $derived($spaceStore);

  const activeSpace = $derived(spaces.spaces.find(s => s.id === ui.activeSpaceId) ?? null);
  const activeDmChannel = $derived(spaces.dmChannels.find(d => d.id === ui.activeDmId) ?? null);
  const isGroupDm = $derived(activeDmChannel?.channelType === 'group_dm');
  // MemberList yalnızca Sunucularda veya Grup DM'lerde açılır; 1:1 DM'de asla açılmaz.
  const canShowMembers = $derived(Boolean(ui.activeChannelId) || (Boolean(ui.activeDmId) && isGroupDm));
  const shouldRenderMembers = $derived(ui.showMemberList && canShowMembers);

  const modalData = $derived(ui.modalData as { displayName?: string; username?: string; spaceId?: string; channelId?: string } | null);
  const profileTitle = $derived(modalData?.displayName ?? modalData?.username ?? 'Profil');
  const spaceModalTitle = $derived(
    modalData?.spaceId ? spaces.spaces.find(s => s.id === modalData.spaceId)?.name : null
  );

  const hasAnyVideo = $derived(
    media.isCameraOn ||
    media.isScreenSharing ||
    media.participants.some(p => p.isVideoOn || p.isScreenSharing)
  );

  let manualStreamerDisable = $state(false);
  let userExplicitlyDisabled = $state(false);
  let wasAutoEnabled = $state(false);

  let bgVideoEl: HTMLVideoElement | undefined = $state();

  /** Background media must NEVER emit audio. Enforced at bind, metadata load and src swaps. */
  function enforceBgVideoSilent(el: HTMLVideoElement | null | undefined) {
    if (!el) return;
    el.muted = true;
    el.volume = 0;
  }

  $effect(() => {
    enforceBgVideoSilent(bgVideoEl);
  });

  $effect(() => {
    if (media.isScreenSharing && $streamerMode.autoEnableOnScreenShare && !$streamerMode.enabled && !manualStreamerDisable && !userExplicitlyDisabled) {
      wasAutoEnabled = true;
      streamerMode.setEnabled(true);
    }
  });

  $effect(() => {
    if (!media.isScreenSharing && wasAutoEnabled && $streamerMode.autoDisableOnScreenShareEnd && $streamerMode.enabled) {
      streamerMode.setEnabled(false);
    }
    if (!media.isScreenSharing) {
      manualStreamerDisable = false;
      userExplicitlyDisabled = false;
      wasAutoEnabled = false;
    }
  });

  function toggleStreamerManual() {
    if ($streamerMode.enabled) {
      userExplicitlyDisabled = true;
    }
    streamerMode.toggle();
  }

  function closeModal() {
    uiStore.closeModal();
  }

  function openStreamerSettings() {
    uiStore.setSettingsTab('streamer');
    uiStore.openModal('settings');
  }

  import { listen } from '@tauri-apps/api/event';
  import { onDestroy } from 'svelte';

  const unlistens: Array<() => void> = [];

  onMount(() => {
    spaceStore.loadSpaces();
    spaceStore.loadDms();
    void messageStore.initRealtime(() => ui.activeChannelId ?? ui.activeDmId);

    listen('spaces:changed', () => {
      void spaceStore.loadSpaces();
    }).then(u => unlistens.push(u));

    listen<{ spaceId: string }>('space:deleted', (e) => {
      void spaceStore.loadSpaces();
      if (ui.activeSpaceId === e.payload?.spaceId) {
        uiStore.navigate(null, null);
      }
    }).then(u => unlistens.push(u));

    listen('channels:changed', () => {
      if (ui.activeSpaceId) {
        void spaceStore.loadChannels(ui.activeSpaceId);
      }
      void spaceStore.loadDms();
    }).then(u => unlistens.push(u));

    listen('user:updated', () => {
      void spaceStore.loadSpaces();
      void spaceStore.loadDms();
      void friendsStore.load();
    }).then(u => unlistens.push(u));

    listen('space:updated', () => {
      void spaceStore.loadSpaces();
    }).then(u => unlistens.push(u));

    listen('roles:changed', () => {
      if (ui.activeSpaceId) {
        void spaceStore.loadChannels(ui.activeSpaceId);
      }
    }).then(u => unlistens.push(u));

    listen('members:changed', () => {
      if (ui.activeSpaceId) {
        void spaceStore.loadChannels(ui.activeSpaceId);
      }
      void spaceStore.loadDms();
    }).then(u => unlistens.push(u));

    listen('presence:changed', () => {
      void spaceStore.loadDms();
      void friendsStore.load();
    }).then(u => unlistens.push(u));

    listen('friends:changed', () => {
      void friendsStore.load();
      void spaceStore.loadDms();
    }).then(u => unlistens.push(u));
  });

  onDestroy(() => {
    for (const u of unlistens) {
      u();
    }
  });
</script>

<StreamerBanner onOpenSettings={openStreamerSettings} onToggle={toggleStreamerManual} />

{#if ui.customBgVideo || ui.customBgImage}
  <div class="veil-media-bg" aria-hidden="true">
    {#if ui.customBgVideo}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        class="veil-media-bg-content"
        src={ui.customBgVideo}
        autoplay
        muted
        loop
        playsinline
        preload="metadata"
        controlsList="nodownload noremoteplayback"
        disablePictureInPicture
        disableremoteplayback
        bind:this={bgVideoEl}
        onloadedmetadata={() => enforceBgVideoSilent(bgVideoEl)}
        style="opacity: {ui.customBgOpacity};"
        onerror={() => uiStore.clearMediaOnError()}
      ></video>
    {:else if ui.customBgImage}
      <img
        class="veil-media-bg-content"
        src={ui.customBgImage}
        alt=""
        style="opacity: {ui.customBgOpacity};"
        onerror={() => uiStore.clearMediaOnError()}
      />
    {/if}
    <div class="veil-media-scrim"></div>
  </div>
{/if}

<div
  class="veil-app"
  class:with-members={shouldRenderMembers}
  class:veil-has-custom-bg={!!(ui.customBgVideo || ui.customBgImage)}
  data-has-custom-bg={!!(ui.customBgVideo || ui.customBgImage)}
>
  <!-- Space sidebar -->
  <Sidebar />

  <!-- Channel list (conditionally visible on mobile) -->
  <ChannelList open={ui.showChannelList} />

  <!-- Bottom-left Unified User Bar (anchored to extreme bottom-left corner) -->
  <BottomUserBar />

  <!-- Main content area -->
  <main class="veil-main">
    {#if ui.activeChannelId || ui.activeDmId}
      <ChatView
        channelId={ui.activeChannelId ?? ui.activeDmId ?? ''}
        isDm={!ui.activeChannelId}
      />
    {:else if ui.activeSpaceId}
      <div class="veil-channel-loading-wrap" style="flex:1;display:flex;align-items:center;justify-content:center;height:100%;min-height:300px;">
        <div class="veil-spinner"></div>
      </div>
    {:else}
      {#await HomePromise then { default: Home }}
        <Home />
      {:catch}
        <div class="veil-home-loading"><div class="veil-spinner"></div></div>
      {/await}
    {/if}
  </main>

  <!-- Member list (rendered only for spaces and group DMs) -->
  {#if shouldRenderMembers}
    <MemberList />
  {/if}
</div>

<!-- Modals -->
{#if ui.openModal === 'settings'}
  <SettingsModal />
{:else if ui.openModal === 'create-space'}
  <Modal open title="Topluluk Oluştur" onClose={closeModal}>
    <CreateSpace />
  </Modal>
{:else if ui.openModal === 'create-channel'}
  <Modal open title="Kanal Oluştur" onClose={closeModal}>
    <CreateChannelModal />
  </Modal>
{:else if ui.openModal === 'create-group-dm'}
  <Modal open title="Yeni Grup Sohbeti" onClose={closeModal}>
    <CreateGroupDmModal />
  </Modal>
{:else if ui.openModal === 'invite'}
  <Modal open title="Davet" onClose={closeModal}>
    <InviteModal />
  </Modal>
{:else if ui.openModal === 'user-profile'}
  <Modal open title={profileTitle} onClose={closeModal}>
    <UserProfile />
  </Modal>
{:else if ui.openModal === 'channel-edit'}
  <Modal open title="Kanal Ayarları & İzinler" size="xl" onClose={closeModal}>
    <ChannelSettingsModal />
  </Modal>
{:else if ui.openModal === 'channel-settings' || ui.openModal === 'space-settings'}
  {#if modalData?.channelId}
    <Modal open title="Kanal Ayarları & İzinler" size="xl" onClose={closeModal}>
      <ChannelSettingsModal />
    </Modal>
  {:else}
    <Modal open title={spaceModalTitle ?? 'Topluluk Ayarları'} size="xl" onClose={closeModal}>
      <SpaceSettings />
    </Modal>
  {/if}
{:else if ui.openModal === 'role-editor'}
  <Modal open title="Rol" onClose={closeModal}>
    <RoleEditor />
  </Modal>
{/if}

<!-- Custom confirmation dialog — replaces browser/webview confirm() -->
<ConfirmDialog />
<!-- Custom input dialog — replaces browser/webview prompt() -->
<InputDialog />

<style>
  .veil-main { position: relative; }
</style>

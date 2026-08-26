<script lang="ts">
  import { onMount } from 'svelte';
  import { messageStore } from '$lib/stores/messages';
  import { spaceStore } from '$lib/stores/spaces';
  import { uiStore } from '$lib/stores/ui';
  import { authStore } from '$lib/stores/auth';
  import { toastStore } from '$lib/stores/notifications';
  import NotificationCenter from '../ui/NotificationCenter.svelte';
  import MessageList from './MessageList.svelte';
  import MessageInput from './MessageInput.svelte';
  import TypingIndicator from './TypingIndicator.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { mediaStore } from '$lib/stores/media';
  import { friendsStore } from '$lib/stores/friends';
  import { memberApi, type MemberInfo } from '$lib/api/tauri';
  import Avatar from '../ui/Avatar.svelte';
  import Icon from '../ui/Icon.svelte';
  import VideoCall from '../media/VideoCall.svelte';
  import type { Message } from '$lib/stores/messages';

  let { channelId, isDm = false } = $props<{ channelId: string; isDm?: boolean }>();

  const messages = $derived($messageStore);
  const auth = $derived($authStore);
  const spaces = $derived($spaceStore);
  const ui = $derived($uiStore);
  const media = $derived($mediaStore);
  const friends = $derived($friendsStore);

  const currentChannel = $derived(
    isDm
      ? spaces.dmChannels.find(c => c.id === channelId)
      : Object.values(spaces.channelsBySpace).flat().find(c => c.id === channelId)
  );

  const isGroupDm = $derived(currentChannel?.channelType === 'group_dm');
  const isVoiceChannel = $derived(currentChannel?.channelType === 'voice');
  const channelMessages = $derived(messages.byChannel[channelId] ?? []);

  let dmMembers = $state<MemberInfo[]>([]);

  $effect(() => {
    if (isDm && channelId) {
      memberApi.list(channelId).then(m => {
        dmMembers = m;
      }).catch(() => {
        dmMembers = [];
      });
    }
  });

  // 1-e-1 DM'de karşı kullanıcının bilgilerini çöz:
  const peerFriend = $derived(
    isDm && !isGroupDm && currentChannel?.name
      ? friends.friends.find(
          f =>
            f.displayName === currentChannel.name ||
            f.username === currentChannel.name ||
            `@${f.username}` === currentChannel.name
        ) ?? null
      : null
  );

  const peerMember = $derived(
    isDm && !isGroupDm && dmMembers.length > 0
      ? dmMembers.find(m => m.userId !== auth.identity?.id) ?? null
      : null
  );

  const peerDisplayName = $derived(
    peerFriend?.displayName || peerMember?.displayName || currentChannel?.name || 'Sohbet'
  );
  const peerUsername = $derived(
    peerFriend?.username || peerMember?.username || ''
  );
  const peerAvatarHash = $derived(
    peerFriend?.avatarHash || peerMember?.avatarHash || null
  );
  const peerPresence = $derived(
    (peerFriend?.onlineStatus || peerMember?.onlineStatus || 'offline') as any
  );
  const peerUserId = $derived(
    peerFriend?.userId || peerMember?.userId || ''
  );

  let loading = $state(false);
  let joiningVoice = $state(false);

  // Header panelleri: arama + sabitlenmiş mesajlar
  let searchOpen = $state(false);
  let searchQuery = $state('');
  let searchResults = $state<Message[]>([]);
  let searchBusy = $state(false);
  let searchDone = $state(false);
  let pinnedOpen = $state(false);
  let pinnedList = $state<Message[]>([]);
  let pinnedBusy = $state(false);
  let notifOpen = $state(false);

  async function runSearch() {
    const q = searchQuery.trim();
    if (q.length < 2 || searchBusy) return;
    searchBusy = true;
    searchDone = true;
    try {
      searchResults = await invoke<Message[]>('search_messages', {
        channelId: isDm ? null : channelId,
        query: q,
        limit: 25,
      });
    } catch {
      searchResults = [];
    } finally {
      searchBusy = false;
    }
  }

  async function openPinned() {
    pinnedOpen = !pinnedOpen;
    if (pinnedOpen && pinnedList.length === 0 && !pinnedBusy) {
      pinnedBusy = true;
      try {
        pinnedList = await invoke<Message[]>('get_pinned_messages', { channelId });
      } catch {
        pinnedList = [];
      } finally {
        pinnedBusy = false;
      }
    }
  }

  function goToMessage(msg: Message) {
    searchOpen = false;
    if (msg.channelId && msg.channelId !== channelId) {
      uiStore.navigate(ui.activeSpaceId, msg.channelId);
    }
    setTimeout(() => {
      document.getElementById(`msg-${msg.id}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, 250);
  }

  $effect(() => {
    if (channelId && !isVoiceChannel) {
      void messageStore.loadMessages(channelId);
      void invoke('mark_as_read', { channelId }).catch(() => {});
      spaceStore.markRead(channelId);
    }
    if (channelId && !currentChannel) {
      if (!isDm && ui.activeSpaceId) {
        void spaceStore.loadChannels(ui.activeSpaceId);
      } else if (isDm) {
        void spaceStore.loadDms();
      }
    }
  });

  onMount(async () => {
    if (!isVoiceChannel) {
      await messageStore.loadMessages(channelId);
      await invoke('mark_as_read', { channelId }).catch(() => {});
      spaceStore.markRead(channelId);
    }
    if (!currentChannel) {
      if (!isDm && ui.activeSpaceId) {
        await spaceStore.loadChannels(ui.activeSpaceId);
      } else if (isDm) {
        await spaceStore.loadDms();
      }
    }
  });

  async function handleJoinVoice(withCamera = false) {
    if (joiningVoice) return;
    joiningVoice = true;
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('Bağlantı zaman aşımına uğradı.')), 20000)
    );
    try {
      if (media.isInCall && media.channelId !== channelId) {
        await Promise.race([mediaStore.switchVoiceChannel(channelId, withCamera), timeout]);
      } else {
        await Promise.race([mediaStore.joinVoice(channelId, withCamera), timeout]);
      }
    } catch (err) {
      const msg = String(err);
      if (!msg.includes('Client initiated disconnect') && !msg.includes('cancelled')) {
        toastStore.error(`Ses kanalına katılamadı: ${msg.replace(/^Error:\s*/, '')}`);
      }
    } finally {
      joiningVoice = false;
    }
  }

  async function handleDmCall(withCamera: boolean) {
    if (media.isInCall && media.channelId === channelId) {
      if (withCamera && !media.isCameraOn) {
        await mediaStore.toggleCamera();
      }
      return;
    }
    joiningVoice = true;
    try {
      if (media.isInCall) {
        await mediaStore.switchVoiceChannel(channelId, withCamera);
      } else {
        await mediaStore.joinVoice(channelId, withCamera);
      }
      if (withCamera && !media.isCameraOn) {
        await mediaStore.toggleCamera();
      }
      // Karşı tarafa ringing sinyali — realtime broadcast ile anında iletilir.
      if (isDm && !isGroupDm) {
        void invoke('send_call_invite', {
          input: { channelId, kind: withCamera ? 'video' : 'audio' },
        }).catch(() => {});
      }
      toastStore.success(withCamera ? 'Görüntülü arama başlatıldı.' : 'Sesli arama başlatıldı.');
    } catch (err) {
      const msg = String(err);
      if (!msg.includes('Client initiated disconnect') && !msg.includes('cancelled')) {
        toastStore.error(`Arama başlatılamadı: ${msg.replace(/^Error:\s*/, '')}`);
      }
    } finally {
      joiningVoice = false;
    }
  }

  async function clearHistory() {
    if (!channelId) return;
    const ok = await uiStore.confirm(
      'Bu kanaldaki tüm mesajları silmek istediğine emin misin? Bu işlem geri alınamaz.',
      { title: 'Sohbeti Temizle', confirmLabel: 'Tümünü Sil', danger: true }
    );
    if (!ok) return;
    try {
      await messageStore.clearChannel(channelId);
      toastStore.success('Sohbet geçmişi temizlendi.');
    } catch (err) {
      toastStore.error(`Temizlenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  let callStageMode = $state<'full' | 'split'>('split');
</script>

<div class="veil-chat">
  <!-- Header -->
  <header class="veil-chat-header">
    <button
      class="btn-icon"
      onclick={() => uiStore.toggleChannelList()}
      title="Kanal listesi"
      aria-label="Toggle channel list"
      style="display: none;" 
    >☰</button>

    <div class="veil-chat-title-group">
      {#if isDm && !isGroupDm}
        <button
          type="button"
          class="veil-chat-dm-peer"
          onclick={() => {
            if (peerUserId) {
              uiStore.openModal('user-profile', {
                userId: peerUserId,
                username: peerUsername || peerDisplayName,
                displayName: peerDisplayName,
                avatarHash: peerAvatarHash,
                onlineStatus: peerPresence,
              });
            }
          }}
          title="Profili Gör"
        >
          <Avatar name={peerDisplayName} hash={peerAvatarHash} presence={peerPresence} size="sm" />
          <div class="veil-chat-dm-info">
            <span class="veil-chat-dm-name">{peerDisplayName}</span>
            {#if peerUsername}
              <span class="veil-chat-dm-tag">@{peerUsername}</span>
            {/if}
          </div>
        </button>
      {:else}
        <span class="veil-chat-title">
          {#if currentChannel?.channelType === 'voice'}
            <Icon name="volume" size={18} class="veil-chat-type-icon" />
          {:else if currentChannel?.channelType === 'announcement'}
            <Icon name="megaphone" size={18} class="veil-chat-type-icon" />
          {:else if currentChannel?.channelType === 'forum'}
            <Icon name="chat" size={18} class="veil-chat-type-icon" />
          {:else if isGroupDm}
            <Icon name="users" size={18} class="veil-chat-type-icon" />
          {:else}
            <span class="veil-chat-type-icon veil-chat-hash">#</span>
          {/if}
          {currentChannel?.name && currentChannel.name.length === 36 && currentChannel.name.includes('-') ? (currentChannel.channelType === 'voice' ? 'Ses Kanalı' : 'Kanal') : (currentChannel?.name ?? 'Kanal')}
          {#if isGroupDm && dmMembers.length > 0}
            <span class="veil-group-count">({dmMembers.length})</span>
          {/if}
        </span>
      {/if}
    </div>

    {#if currentChannel?.channelType === 'announcement'}
      <span class="veil-announcement-badge" title="Duyuru kanalı">
        Duyuru
      </span>
    {/if}

    {#if currentChannel?.isE2ee}
      <span class="veil-e2ee-badge" title="Bu kanal uçtan uca şifreli. Mesaj içeriği sunucu tarafından okunamaz." role="status">
        <Icon name="lock" size={12} />
        E2EE
      </span>
    {/if}

    <div class="veil-header-actions">
      {#if isDm}
        <button
          class="btn-icon"
          class:active={media.isInCall && media.channelId === channelId && !media.isCameraOn}
          title="Sesli Arama"
          aria-label="Sesli arama başlat"
          onclick={() => handleDmCall(false)}
        >
          <Icon name="volume" size={16} />
        </button>
        <button
          class="btn-icon"
          class:active={media.isInCall && media.channelId === channelId && media.isCameraOn}
          title="Görüntülü Arama"
          aria-label="Görüntülü arama başlat"
          onclick={() => handleDmCall(true)}
        >
          <Icon name="video" size={16} />
        </button>
      {/if}

      <div class="veil-header-panel-wrap">
        <button class="btn-icon" title="Ara" aria-label="Mesajlarda ara" aria-expanded={searchOpen} onclick={() => { searchOpen = !searchOpen; pinnedOpen = false; }}><Icon name="search" size={16} /></button>
        {#if searchOpen}
          <div class="veil-header-panel veil-pop-in" role="dialog" aria-label="Mesaj ara">
            <div class="veil-header-panel-search">
              <input
                class="veil-input"
                type="text"
                placeholder="Mesaj ara… (min 2 karakter)"
                aria-label="Arama terimi"
                autocomplete="off"
                bind:value={searchQuery}
                onkeydown={(e) => { if (e.key === 'Enter') runSearch(); }}
              />
              <button class="btn btn-primary btn-sm" onclick={runSearch} disabled={searchQuery.trim().length < 2 || searchBusy}>
                Ara
              </button>
            </div>
            <div class="veil-header-panel-results">
              {#if searchBusy}
                <div class="veil-panel-state"><div class="veil-spinner veil-spinner-sm"></div></div>
              {:else if searchDone && searchResults.length === 0}
                <p class="veil-panel-state">Sonuç bulunamadı.</p>
              {:else}
                {#each searchResults as msg (msg.id)}
                  <button class="veil-search-hit" onclick={() => goToMessage(msg)}>
                    <span class="veil-search-hit-author">{msg.senderId === auth.identity?.id ? 'Sen' : (msg.senderName ?? msg.senderId.slice(0, 8))}</span>
                    <span class="veil-search-hit-text">{msg.content ?? ''}</span>
                  </button>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </div>
      <div class="veil-header-panel-wrap">
        <button class="btn-icon" title="Sabitlenmiş mesajlar" aria-label="Sabitlenmiş mesajlar" aria-expanded={pinnedOpen} onclick={() => { openPinned(); searchOpen = false; }}><Icon name="pin" size={16} /></button>
        {#if pinnedOpen}
          <div class="veil-header-panel veil-pop-in" role="dialog" aria-label="Sabitlenmiş mesajlar">
            <div class="veil-header-panel-title">Sabitlenmiş Mesajlar</div>
            <div class="veil-header-panel-results">
              {#if pinnedBusy}
                <div class="veil-panel-state"><div class="veil-spinner veil-spinner-sm"></div></div>
              {:else if pinnedList.length === 0}
                <p class="veil-panel-state">Bu kanalda sabitlenmiş mesaj yok.</p>
              {:else}
                {#each pinnedList as msg (msg.id)}
                  <button class="veil-search-hit" onclick={() => goToMessage(msg)}>
                    <span class="veil-search-hit-pin"><Icon name="pin" size={11} /></span>
                    <span class="veil-search-hit-text">{msg.content ?? ''}</span>
                  </button>
                {/each}
              {/if}
            </div>
          </div>
        {/if}
      </div>
      {#if !isVoiceChannel && channelMessages.length > 0}
        <button
          class="btn-icon"
          title="Sohbeti Temizle"
          aria-label="Sohbet geçmişini temizle"
          onclick={clearHistory}
        ><Icon name="trash" size={16} /></button>
      {/if}
      {#if !isDm || isGroupDm}
        <button
          class="btn-icon"
          class:active={ui.showMemberList}
          title="Üye listesi"
          aria-label="Üye listesini aç/kapat"
          onclick={() => uiStore.toggleMemberList()}
        ><Icon name="users" size={16} /></button>
      {/if}
    </div>
  </header>

  <NotificationCenter open={notifOpen} onClose={() => (notifOpen = false)} />

  <!-- Content -->
  {#if isVoiceChannel}
    <div class="veil-voice-stage-full">
      {#if media.isInCall && media.channelId === channelId}
        <VideoCall />
      {:else}
        <div class="veil-voice-lobby">
          <div class="veil-voice-lobby-card">
            <div class="veil-voice-lobby-icon">
              <Icon name="volume" size={36} />
            </div>
            <h3>{currentChannel?.name && currentChannel.name.length === 36 && currentChannel.name.includes('-') ? 'Ses Kanalı' : (currentChannel?.name ?? 'Ses Kanalı')}</h3>
            <p class="veil-voice-lobby-desc">
              {#if joiningVoice}
                Ses kanalına bağlanılıyor…
              {:else}
                Bu ses kanalına bağlı değilsiniz. Diğer üyelerle sesli veya görüntülü görüşmek için odaya katılın.
              {/if}
            </p>
            <div class="veil-voice-lobby-actions">
              <button
                type="button"
                class="btn btn-primary"
                style="padding: 10px 24px; font-size: 14px; font-weight: 600; gap: 8px;"
                disabled={joiningVoice}
                onclick={() => handleJoinVoice(false)}
              >
                {#if joiningVoice}
                  <div class="veil-spinner veil-spinner-sm"></div>
                  <span>Bağlanılıyor…</span>
                {:else}
                  <Icon name="phone" size={16} />
                  <span>Sese Katıl</span>
                {/if}
              </button>
              <button
                type="button"
                class="btn btn-secondary"
                style="padding: 10px 20px; font-size: 14px; font-weight: 600; gap: 8px;"
                disabled={joiningVoice}
                onclick={() => handleJoinVoice(true)}
              >
                <Icon name="camera" size={16} />
                <span>Kamera ile Katıl</span>
              </button>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    {#if isDm && media.isInCall && media.channelId === channelId}
      {#if callStageMode === 'full'}
        <div class="veil-voice-stage-full veil-dm-full-stage">
          <div class="veil-dm-stage-header">
            <button
              class="btn btn-secondary btn-sm"
              onclick={() => (callStageMode = 'split')}
              title="Sohbeti Göster (Bölünmüş Görünüm)"
            >
              <Icon name="layout" size={14} />
              <span>Bölünmüş Görünüm</span>
            </button>
          </div>
          <VideoCall />
        </div>
      {:else}
        <div class="veil-dm-call-panel">
          <div class="veil-dm-stage-header-mini">
            <button
              class="btn-icon veil-dm-expand-btn"
              onclick={() => (callStageMode = 'full')}
              title="Tam Sahne Görünümü"
            >
              <Icon name="maximize-2" size={14} />
            </button>
          </div>
          <VideoCall />
        </div>
      {/if}
    {/if}

    {#if !isDm || !media.isInCall || media.channelId !== channelId || callStageMode === 'split'}
      <!-- Text channel view -->
      {#if loading}
        <div class="veil-loading-messages">
          <div class="veil-spinner"></div>
        </div>
      {:else if channelMessages.length === 0}
        <div class="veil-empty-state">
          <Icon name="chat" size={32} />
          <p>Henüz mesaj yok</p>
        </div>
      {:else}
        <MessageList {channelId} channelName={currentChannel?.name ?? 'kanal'} />
      {/if}

      <TypingIndicator {channelId} />

      <div class="veil-input-area">
        <MessageInput
          {channelId}
          placeholder={currentChannel ? `#${currentChannel.name} kanalına mesaj yaz` : 'Mesaj yaz...'}
        />
      </div>
    {/if}
  {/if}
</div>

<style>
  .veil-voice-stage-full {
    flex: 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    min-width: 0;
    display: flex;
    overflow: hidden;
    /* Add bottom padding so the VideoCall footer isn't hidden by the VoiceBar */
    padding-bottom: 0;
  }
  .veil-voice-lobby {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6, 1.5rem);
    background: transparent;
  }
  .veil-voice-lobby-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    max-width: 420px;
    padding: var(--space-6, 1.5rem);
    background: color-mix(in srgb, var(--veil-bg-elevated, #171b26) 80%, transparent);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.1));
    border-radius: var(--radius-xl, 1rem);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
    gap: var(--space-3, 0.75rem);
  }
  .veil-voice-lobby-icon {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: var(--veil-brand-subtle, rgba(124, 58, 237, 0.12));
    border: 1px solid var(--veil-brand-border, rgba(124, 58, 237, 0.25));
    color: var(--veil-brand, #7c3aed);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: var(--space-2, 0.5rem);
  }
  .veil-voice-lobby-card h3 {
    font-size: var(--text-lg, 1.125rem);
    font-weight: 700;
    color: var(--veil-text-primary, #f1f5f9);
    margin: 0;
  }
  .veil-voice-lobby-desc {
    font-size: var(--text-sm, 0.875rem);
    color: var(--veil-text-secondary, #94a3b8);
    line-height: 1.5;
    margin: 0;
  }
  .veil-voice-lobby-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3, 0.75rem);
    margin-top: var(--space-2, 0.5rem);
    flex-wrap: wrap;
    justify-content: center;
  }
  .veil-dm-full-stage {
    position: relative;
    flex: 1;
    width: 100%;
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .veil-dm-stage-header {
    position: absolute;
    top: 12px;
    right: 16px;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 8px;
    background: color-mix(in srgb, var(--veil-bg-elevated) 80%, transparent);
    backdrop-filter: blur(8px);
    padding: 4px 8px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--veil-border-subtle);
  }
  .veil-dm-stage-header-mini {
    position: absolute;
    top: 8px;
    right: 12px;
    z-index: 20;
  }
  .veil-dm-expand-btn {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md, 8px);
    background: var(--veil-bg-elevated, #1a1e2d);
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.1));
  }

  .veil-dm-call-panel {
    height: clamp(300px, 55vh, 75vh);
    min-height: 300px;
    border-bottom: 1px solid var(--veil-border-subtle);
    background: color-mix(in srgb, var(--veil-bg-surface) 95%, var(--veil-bg-elevated));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex-shrink: 0;
    position: relative;
    transition: height 0.3s cubic-bezier(0.2, 0, 0, 1);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35);
    border-radius: var(--radius-xl, 14px);
    margin: var(--space-2, 8px);
  }
  .veil-loading-messages {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .veil-empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
  }
  .veil-chat-title-group {
    display: flex;
    align-items: center;
    min-width: 0;
    gap: var(--space-2);
  }
  .veil-chat-dm-peer {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    background: transparent;
    border: none;
    padding: 3px 8px 3px 4px;
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: background var(--t-fast);
  }
  .veil-chat-dm-peer:hover {
    background: var(--veil-channel-hover);
  }
  .veil-chat-dm-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .veil-chat-dm-name {
    font-size: var(--text-md);
    font-weight: 700;
    color: var(--veil-text-primary);
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-chat-dm-tag {
    font-size: 11px;
    font-weight: 600;
    color: var(--veil-text-muted);
    line-height: 1.1;
  }
  .veil-group-count {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-muted);
    margin-left: 4px;
  }
  .veil-chat-type-icon {
    color: var(--veil-text-muted);
    display: inline-flex;
    flex-shrink: 0;
  }
  .veil-chat-hash {
    font-weight: 700;
    font-size: var(--text-md);
  }

  .veil-announcement-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: color-mix(in srgb, var(--veil-warning) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--veil-warning) 30%, transparent);
    color: var(--veil-warning);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .veil-header-panel-wrap { position: relative; }
  .veil-header-panel {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 70;
    width: 320px;
    max-width: calc(100vw - var(--space-4));
    background: var(--veil-bg-raised);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    box-shadow: var(--veil-shadow-menu);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .veil-header-panel-search {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
  }
  .veil-header-panel-title {
    padding: var(--space-3) var(--space-3) var(--space-1);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-header-panel-results {
    max-height: 320px;
    overflow-y: auto;
    padding: var(--space-1);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-search-hit {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--veil-text-secondary);
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-search-hit:hover { background: var(--veil-bg-overlay); color: var(--veil-text-primary); }
  .veil-search-hit-author { font-weight: 600; color: var(--veil-brand); flex-shrink: 0; }
  .veil-search-hit-pin { color: var(--veil-warning); flex-shrink: 0; display: inline-flex; }
  .veil-search-hit-text {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-panel-state {
    padding: var(--space-6) var(--space-3);
    text-align: center;
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
  }
</style>

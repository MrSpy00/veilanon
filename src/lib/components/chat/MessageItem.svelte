<script lang="ts">
  import { messageStore } from '$lib/stores/messages';
  import { toastStore } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { authStore } from '$lib/stores/auth';
  import { fileApi, messageApi, friendApi, dmApi, privacyToolsApi, type LinkPreviewResult } from '$lib/api/tauri';
  import { permissionsStore } from '$lib/stores/permissions';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { trustedDomainsStore } from '$lib/stores/trustedDomains';
  import ExternalLinkModal from '../ui/ExternalLinkModal.svelte';
  import { save } from '@tauri-apps/plugin-dialog';
  import { onDestroy, tick } from 'svelte';
  import Icon from '../ui/Icon.svelte';
  import Markdown from '../ui/Markdown.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import MediaAttachment from './MediaAttachment.svelte';
  import ContextMenu, { type ContextMenuItem } from '../ui/ContextMenu.svelte';
  import { COMMON_EMOJI, isEmoji } from '$lib/utils/emoji';
  import { copyText } from '$lib/utils/clipboard';
  import type { Message } from '$lib/stores/messages';
  import { getServerNowSec } from '$lib/stores/messages';
  import { detectDomains } from '$lib/utils/domainDetector';

  interface Attachment {
    fileId: string;
    r2Key: string;
    sizeBytes: number;
    contentKeyCiphertext?: string | null;
    mimeTypeHint?: string | null;
    fileName?: string | null;
  }

  let { message, grouped = false, isOwn = false, groupStart = false } = $props<{
    message: Message;
    grouped?: boolean;
    isOwn?: boolean;
    groupStart?: boolean;
  }>();

  const auth = $derived($authStore);
  const spaces = $derived($spaceStore);
  const ui = $derived($uiStore);
  const isOwner = $derived(ui.activeSpaceId ? (spaces.spaces.find(s => s.id === ui.activeSpaceId)?.isOwner ?? false) : false);

  const isE2eeChannel = $derived.by(() => {
    if (!message.channelId) return false;
    const isDm = spaces.dmChannels.some(d => d.id === message.channelId);
    if (isDm) return true;
    for (const sid in spaces.channelsBySpace) {
      const ch = spaces.channelsBySpace[sid].find(c => c.id === message.channelId);
      if (ch?.isE2ee) return true;
    }
    return false;
  });

  const resolvedAuthor = $derived.by(() => {
    if (isOwn || message.senderId === 'self' || message.senderId === auth.identity?.id) {
      return {
        name: auth.identity?.displayName || auth.identity?.username || 'Sen',
        username: auth.identity?.username || '',
        avatarHash: auth.identity?.avatarHash ?? null,
        roleColor: message.senderRoleColor ?? null,
      };
    }

    const rawName = message.senderName?.trim();
    const isCryptoKey = rawName && (rawName.startsWith('ec') || rawName.length > 32 || /^[a-f0-9]{32,}$/i.test(rawName));

    if (rawName && rawName !== 'self' && !isCryptoKey) {
      return {
        name: rawName,
        username: rawName,
        avatarHash: message.senderAvatarHash ?? null,
        roleColor: message.senderRoleColor ?? null,
      };
    }

    return {
      name: 'Kullanıcı',
      username: '',
      avatarHash: message.senderAvatarHash ?? null,
      roleColor: message.senderRoleColor ?? null,
    };
  });

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuItems = $state<ContextMenuItem[]>([]);
  let reactOpen = $state(false);

  // ── Inline düzenleme durumu (Discord tarzı) ────────────────────────────
  let editing = $state(false);
  let editDraft = $state('');
  let editTextarea = $state<HTMLTextAreaElement | null>(null);

  function formatTime(ts: number): string {
    return new Date(ts * 1000).toLocaleTimeString('tr-TR', {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function handleDelete() {
    const ok = await uiStore.confirm('Bu mesajı silmek istediğine emin misin?', {
      title: 'Mesajı Sil',
      confirmLabel: 'Sil',
      danger: true,
    });
    if (!ok) return;
    await messageStore.deleteMessage(message.channelId, message.id);
  }

  async function handleCopy() {
    if (message.content) {
      await copyText(message.content);
      toastStore.success('Kopyalandı');
    }
  }

  async function copyMessageLink() {
    await copyText(`https://veilanon.com/message/${message.channelId}/${message.id}`);
    toastStore.success('Mesaj bağlantısı kopyalandı.');
  }

  function startReply() {
    uiStore.setReplyTo({
      channelId: message.channelId,
      messageId: message.id,
      author: resolvedAuthor.name,
      content: (message.content ?? '').slice(0, 60),
    });
  }

  async function beginEdit() {
    // message.content null ise boş string ile başlat
    editDraft = message.content ?? '';
    editing = true;
    await tick();
    editTextarea?.focus();
    if (editDraft) {
      editTextarea?.select();
    } else {
      const len = editTextarea?.value.length ?? 0;
      editTextarea?.setSelectionRange(len, len);
    }
  }

  async function saveEdit() {
    const next = editDraft.trim();
    const originalContent = (message.content ?? '').trim();
    const hasAttachments = Array.isArray(message.attachments) && message.attachments.length > 0;

    // Hiçbir değişiklik yoksa iptal et
    if (next === originalContent) {
      cancelEdit();
      return;
    }
    // Ek yoksa boş içerik kaydedilemez; ek varsa başlık/altyazı boşaltılabilir.
    if (!next && !hasAttachments) {
      cancelEdit();
      return;
    }
    try {
      const edited = await messageApi.edit(message.id, next);
      toastStore.success('Mesaj düzenlendi.');
      messageStore.patchMessage(message.channelId, message.id, {
        content: edited?.content !== undefined ? edited.content : next,
        editedAt: edited?.editedAt ?? Math.floor(Date.now() / 1000),
      });
      editing = false;
    } catch {
      toastStore.error('Mesaj düzenlenemedi.');
    }
  }

  function cancelEdit() {
    editing = false;
    editDraft = '';
  }

  async function togglePin() {
    try {
      if (message.pinned) {
        await messageApi.unpin(message.id);
        toastStore.success('Sabitleme kaldırıldı.');
      } else {
        await messageApi.pin(message.id);
        toastStore.success('Mesaj sabitlendi.');
      }
      messageStore.patchMessage(message.channelId, message.id, { pinned: !message.pinned });
    } catch {
      toastStore.error('İşlem başarısız.');
    }
  }

  async function addReaction(emoji: string) {
    reactOpen = false;
    if (!isEmoji(emoji) && !COMMON_EMOJI.includes(emoji)) {
      const custom = await uiStore.promptInput('Tepki emojisi:', {
        title: 'Tepki Ekle',
        confirmLabel: 'Ekle',
        defaultValue: '👍',
      });
      if (!custom) return;
      emoji = custom.trim();
    }
    try {
      await messageApi.addReaction(message.id, emoji);
      messageStore.patchReaction(message.channelId, message.id, emoji, true);
    } catch {
      toastStore.error('Tepki eklenemedi.');
    }
  }

  async function removeReaction(emoji: string) {
    try {
      await messageApi.removeReaction(message.id, emoji);
      messageStore.patchReaction(message.channelId, message.id, emoji, false);
    } catch {
      toastStore.error('Tepki kaldırılamadı.');
    }
  }

  function openMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const perms = $permissionsStore;
    const canPin = perms.isOwner || perms.has('pin_messages') || perms.has('manage_messages');
    const canDelete = isOwn || perms.isOwner || perms.has('manage_messages');
    const canReact = perms.isOwner || perms.has('add_reactions');
    const canSend = perms.isOwner || perms.has('send_messages');

    const items: ContextMenuItem[] = [
      { label: 'Kopyala', icon: 'copy', onClick: handleCopy },
      { label: 'Mesaj bağlantısı', icon: 'link', onClick: copyMessageLink },
    ];

    if (canSend) {
      items.push({ label: '', separator: true });
      items.push({ label: 'Yanıtla', icon: 'arrow-left', onClick: startReply });
    }

    if (canReact) {
      items.push({ label: 'Tepki ekle', icon: 'sparkle', onClick: () => (reactOpen = !reactOpen) });
    }

    if (canPin) {
      items.push({ label: message.pinned ? 'Sabitlemeyi kaldır' : 'Sabitle', icon: 'pin', onClick: togglePin });
    }

    if (isOwn || canDelete) {
      items.push({ label: '', separator: true });
      if (isOwn) {
        items.push({ label: 'Düzenle', icon: 'edit', onClick: beginEdit });
      }
      if (canDelete) {
        items.push({ label: 'Sil', icon: 'trash', danger: true, onClick: handleDelete });
      }
    }
    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  function openUserProfile(e?: MouseEvent | KeyboardEvent) {
    e?.preventDefault();
    e?.stopPropagation();
    uiStore.openModal('user-profile', {
      userId: isOwn ? (auth.identity?.id ?? message.senderId) : message.senderId,
      username: resolvedAuthor.username || resolvedAuthor.name,
      displayName: resolvedAuthor.name,
      avatarHash: resolvedAuthor.avatarHash,
      onlineStatus: 'offline',
    });
  }

  function openUserMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const items: ContextMenuItem[] = [
      { label: 'Profili gör', icon: 'user', onClick: () => openUserProfile(e) },
      {
        label: 'Profil bağlantısını kopyala',
        icon: 'link',
        onClick: async () => {
          const u = resolvedAuthor.username || resolvedAuthor.name;
          await copyText(`https://veilanon.com/u/${u}`);
          toastStore.success('Profil bağlantısı kopyalandı.');
        },
      },
      { label: 'Etiketi kopyala', icon: 'copy', onClick: async () => { await copyText(`@${resolvedAuthor.name}`); toastStore.success('Etiket kopyalandı.'); } },
    ];
    if (!isOwn) {
      items.push(
        { label: '', separator: true },
        { label: 'DM ile devam et', icon: 'chat', onClick: () => void openDm() },
        { label: 'Engelle', icon: 'x', danger: true, onClick: () => void blockUser() },
      );
    }
    menuItems = items;
    menuX = e.clientX;
    menuY = e.clientY;
    menuOpen = true;
  }

  async function openDm() {
    try {
      const channel = await dmApi.open(message.senderId);
      await spaceStore.loadDms();
      uiStore.navigateDm(channel.id);
    } catch {
      toastStore.error('DM kanalı açılamadı.');
    }
  }

  async function blockUser() {
    try {
      await friendApi.block(message.senderId);
      toastStore.success('Kullanıcı engellendi.');
    } catch {
      toastStore.error('Engellenemedi.');
    }
  }

  async function downloadAttachment(att: Attachment) {
    const realName = att.fileName?.trim();
    const path = await save({
      title: 'Dosyayı kaydet',
      defaultPath: realName ? realName : `veilanon-dosya-${att.fileId.slice(0, 8)}.bin`,
      filters: [{ name: 'Tüm dosyalar', extensions: ['*'] }],
    });
    if (!path) return;
    try {
      await fileApi.download({ fileId: att.fileId, destinationPath: path });
      toastStore.success('Dosya indirildi ve çözüldü.');
    } catch {
      toastStore.error('Dosya indirilemedi.');
    }
  }

  const isDeleted = $derived(Boolean(message.deletedAt));
  const displayContent = $derived(isDeleted ? '[Mesaj silindi]' : (message.content ?? ''));

  // ── Kaybolan Mesaj Geri Sayım Sayacı (server-synced, absolute) ─────
  // disappearsAt is absolute server unix seconds. Both clients compute
  // remaining = disappearsAt - Date.now()/1000 so both show identical countdown.
  // No clock-skew correction — local skew <2s in practice, offset correction caused 3s divergence.
  const purgedMessageIds = (typeof window !== 'undefined' ? ((window as any).__veilPurgedIds ??= new Set<string>()) : new Set<string>());
  let countdown = $state<number | null>(null);
  let countdownTimer: ReturnType<typeof setInterval> | null = null;
  let isBurning = $state(false);
  let deleteScheduled = false;

  function nowSec(): number {
    return getServerNowSec();
  }

  function startCountdown() {
    if (!message.disappearsAt) return;
    if (purgedMessageIds.has(message.id)) {
      countdown = 0;
      isBurning = true;
      return;
    }
    if (countdownTimer) {
      clearInterval(countdownTimer);
      countdownTimer = null;
    }
    deleteScheduled = false;

    const update = () => {
      const remaining = message.disappearsAt! - nowSec();

      if (remaining <= 0) {
        countdown = 0;
        if (countdownTimer) {
          clearInterval(countdownTimer);
          countdownTimer = null;
        }
        if (!deleteScheduled && !purgedMessageIds.has(message.id)) {
          deleteScheduled = true;
          purgedMessageIds.add(message.id);
          isBurning = true;
          // Immediate local purge + backend tombstone broadcast
          messageStore.purgeExpiredLocal();
          void messageStore.deleteMessage(message.channelId, message.id).catch(() => {
            messageStore.purgeExpiredLocal();
          });
        }
      } else {
        countdown = Math.ceil(remaining);
      }
    };
    update();
    if (!deleteScheduled && (message.disappearsAt - nowSec()) > 0) {
      countdownTimer = setInterval(update, 250);
    }
  }

  $effect(() => {
    if (message.disappearsAt) {
      startCountdown();
    } else {
      if (countdownTimer) {
        clearInterval(countdownTimer);
        countdownTimer = null;
      }
      countdown = null;
      isBurning = false;
    }
    return () => {
      if (countdownTimer) {
        clearInterval(countdownTimer);
        countdownTimer = null;
      }
    };
  });

  function formatCountdown(secs: number): string {
    if (secs <= 0) return '0sn';
    if (secs < 60) return `${secs}sn`;
    if (secs < 3600) return `${Math.floor(secs / 60)}dk ${secs % 60}sn`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}sa`;
    return `${Math.floor(secs / 86400)}g`;
  }

  function getCountdownColor(secs: number): string {
    if (secs <= 5) return 'var(--veil-danger)';
    if (secs <= 30) return 'var(--veil-warning)';
    if (secs <= 300) return 'hsl(36 100% 55%)';
    return 'var(--veil-text-muted)';
  }

  // ── Link Önizleme ────────────────────────────────────────────────────────
  const detectedUrls = $derived.by<string[]>(() => {
    if (!message.content) return [];
    const domains = detectDomains(message.content);
    const unsuppressed = domains.filter(d => !d.suppressed);
    return unsuppressed.slice(0, 1).map(d => d.url);
  });

  let linkPreview = $state<LinkPreviewResult | null>(null);
  let linkPreviewLoading = $state(false);
  let linkPreviewError = $state(false);
  let linkModalOpen = $state(false);
  let selectedExternalUrl = $state('');

  function openExternalLink(url: string) {
    if (!url) return;
    if (trustedDomainsStore.shouldDirectRedirect(url)) {
      openUrl(url).catch(() => {
        window.open(url, '_blank', 'noopener,noreferrer');
      });
    } else {
      selectedExternalUrl = url;
      linkModalOpen = true;
    }
  }

  $effect(() => {
    const url: string | undefined = detectedUrls[0];
    linkPreview = null;
    linkPreviewError = false;
    if (url && !message.disappearsAt) {
      linkPreviewLoading = true;
      privacyToolsApi.fetchLinkPreview(url).then((result) => {
        if (result && result.isSafe !== false && (result.title || result.description || result.image)) {
          linkPreview = result;
        }
        linkPreviewLoading = false;
      }).catch(() => {
        linkPreviewLoading = false;
        linkPreviewError = true;
      });
    } else {
      linkPreviewLoading = false;
    }
  });

  onDestroy(() => {
    if (countdownTimer) clearInterval(countdownTimer);
  });
</script>


<article
  class="veil-message"
  class:grouped
  class:group-start={groupStart && !grouped}
  class:own={isOwn}
  class:burning={isBurning}
  id="msg-{message.id}"
  aria-label="Mesaj"
  oncontextmenu={openMenu}
>
  <!-- Avatar (hidden when grouped) -->
  <div class="veil-message-avatar" aria-hidden="true">
    {#if !grouped}
      <button
        class="veil-message-avatar-btn"
        onclick={openUserProfile}
        oncontextmenu={openUserMenu}
        title={resolvedAuthor.name}
      >
        <Avatar name={resolvedAuthor.name} hash={resolvedAuthor.avatarHash} size="md" />
      </button>
    {/if}
  </div>

  <!-- Message body -->
  <div class="veil-message-body">
    {#if !grouped}
      <div class="veil-message-header">
        <span
          class="veil-message-author"
          class:role-colored={!!resolvedAuthor.roleColor}
          style={resolvedAuthor.roleColor ? `color:${resolvedAuthor.roleColor}` : ''}
          role="button"
          tabindex="0"
          onclick={openUserProfile}
          oncontextmenu={openUserMenu}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); openUserProfile(e); } }}
        >{resolvedAuthor.name}</span>
        <time class="veil-message-time" datetime={new Date(message.createdAt * 1000).toISOString()}>
          {formatTime(message.createdAt)}
        </time>
        {#if message.editedAt}
          <span class="veil-message-edited">(düzenlendi)</span>
        {/if}
        {#if message.pinned}
          <span class="veil-message-pinned" title="Sabitlenmiş mesaj"><Icon name="pin" size={11} /></span>
        {/if}
        {#if countdown !== null && message.disappearsAt}
          <span
            class="veil-disappear-countdown"
            title="Bu mesaj kaybolacak"
            style="color:{getCountdownColor(countdown)};"
          >
            <Icon name="flame" size={11} />
            <span>{formatCountdown(countdown)}</span>
          </span>
        {/if}
        {#if isOwn}
          <span class="veil-message-status" aria-label="Status: {message.status}">
            {#if message.status === 'sending'}
              <span class="veil-status-spinner" aria-hidden="true"></span>
            {:else if message.status === 'failed'}
              <span class="veil-status-failed"><Icon name="x" size={12} /></span>
            {:else if message.status === 'read'}
              <span class="veil-status-read"><Icon name="check-double" size={13} /></span>
            {:else if message.status === 'delivered'}
              <span class="veil-status-delivered"><Icon name="check-double" size={13} /></span>
            {:else}
              <Icon name="check" size={12} />
            {/if}
          </span>
        {/if}
      </div>
    {:else}
      <time
        class="veil-message-time veil-message-time-grouped"
        datetime={new Date(message.createdAt * 1000).toISOString()}
      >
        {formatTime(message.createdAt)}
      </time>
      {#if countdown !== null && message.disappearsAt}
        <span
          class="veil-disappear-countdown veil-disappear-countdown-grouped"
          title="Bu mesaj kaybolacak"
          style="color:{getCountdownColor(countdown)};"
        >
          <Icon name="flame" size={10} />
          <span>{formatCountdown(countdown)}</span>
        </span>
      {/if}
    {/if}

    <!-- Reply bar -->
    {#if message.replyToId}
      <div class="veil-reply-bar">
        <span>↩</span>
        <span class="veil-reply-author">Yanıt</span>
        <span>bir mesaja</span>
      </div>
    {/if}

    <!-- Message content -->
    <div class="veil-message-content" class:deleted={isDeleted}>
      {#if editing}
        <div class="veil-message-edit-box">
          <textarea
            bind:this={editTextarea}
            bind:value={editDraft}
            class="veil-message-edit-input"
            maxlength="4000"
            rows={Math.min(6, Math.max(2, Math.ceil((editDraft.length || 1) / 80)))}
            aria-label="Mesaj düzenleme"
            onkeydown={(e) => {
              if (e.key === 'Escape') { e.preventDefault(); cancelEdit(); }
              if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void saveEdit(); }
            }}
          ></textarea>
          <div class="veil-message-edit-actions">
            <span class="veil-message-edit-hint">Esc ile vazgeç · Enter ile kaydet</span>
            <div class="veil-message-edit-btns">
              <button type="button" class="btn btn-ghost btn-sm" onclick={cancelEdit}>Vazgeç</button>
              <button
                type="button"
                class="btn btn-primary btn-sm"
                onclick={saveEdit}
                disabled={
                  editDraft.trim() === (message.content ?? '').trim() ||
                  (!editDraft.trim() && !(Array.isArray(message.attachments) && message.attachments.length > 0))
                }
              >Kaydet</button>
            </div>
          </div>
        </div>
      {:else if isDeleted}
        {displayContent}
      {:else if message.content}
        {#if message.content.startsWith('[Şifreli mesaj')}
          {#if isE2eeChannel}
            <div class="veil-encrypted-placeholder" role="status">
              <Icon name="lock" size={13} />
              <span>{message.content}</span>
            </div>
          {/if}
        {:else}
          <Markdown content={displayContent} />
        {/if}
      {/if}
    </div>

    <!-- Link Preview Card -->
    {#if detectedUrls.length > 0 && !isDeleted && !message.disappearsAt}
      {#if linkPreviewLoading}
        <div class="veil-link-preview-loading">
          <div class="veil-link-preview-skeleton"></div>
        </div>
      {:else if linkPreview}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="veil-link-preview-card"
          role="link"
          tabindex="0"
          aria-label="Bağlantı önizleme: {linkPreview.title || detectedUrls[0]}"
          onclick={() => openExternalLink(detectedUrls[0])}
        >
          {#if linkPreview.image}
            <img
              class="veil-link-preview-img"
              src={linkPreview.image}
              alt={linkPreview.title ?? 'Önizleme'}
              loading="lazy"
              onerror={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
            />
          {/if}
          <div class="veil-link-preview-body">
            {#if linkPreview.siteName}
              <span class="veil-link-preview-site">{linkPreview.siteName}</span>
            {/if}
            {#if linkPreview.title}
              <p class="veil-link-preview-title">{linkPreview.title}</p>
            {/if}
            {#if linkPreview.description}
              <p class="veil-link-preview-desc">{linkPreview.description.slice(0, 160)}</p>
            {/if}
            {#if detectedUrls[0]}
              <span class="veil-link-preview-url">{String(detectedUrls[0]).slice(0, 60)}{String(detectedUrls[0]).length > 60 ? '…' : ''}</span>
            {/if}
          </div>
        </div>
      {/if}
    {/if}

    <!-- Attachments (Images, Videos, Audio, Files) -->
    {#if !isDeleted && (message.attachments as Attachment[] | undefined)?.length}
      <div class="veil-message-attachments" role="group" aria-label="Ekler">
        {#each (message.attachments as Attachment[]) as att (att.fileId)}
          <MediaAttachment attachment={att} />
        {/each}
      </div>
    {/if}

    <!-- Reactions -->
    {#if message.reactions.length > 0}
      <div class="veil-reactions" role="group" aria-label="Tepkiler">
        {#each message.reactions as reaction}
          <button
            class="veil-reaction"
            class:own={reaction.userIds.includes('self')}
            aria-label="{reaction.emoji} {reaction.count}"
            title="{reaction.count} kişi tepki verdi"
            onclick={() => reaction.userIds.includes('self') ? removeReaction(reaction.emoji) : addReaction(reaction.emoji)}
          >
            {reaction.emoji}
            <span class="veil-reaction-count">{reaction.count}</span>
          </button>
        {/each}
        <button class="veil-reaction" aria-label="Tepki ekle" title="Tepki ekle" onclick={() => (reactOpen = !reactOpen)}>
          <Icon name="plus" size={12} />
        </button>
      </div>
    {/if}
  </div>

  <!-- Message actions (appear on hover) -->
  <div class="veil-message-actions" role="toolbar" aria-label="Mesaj işlemleri">
    <button class="btn-icon" onclick={handleCopy} title="Kopyala" aria-label="Mesajı kopyala"><Icon name="copy" size={14} /></button>
    <button class="btn-icon" onclick={() => (reactOpen = !reactOpen)} title="Tepki ekle" aria-label="Tepki ekle"><Icon name="sparkle" size={14} /></button>
    <button class="btn-icon" onclick={startReply} title="Yanıtla" aria-label="Mesaja yanıt ver"><span class="veil-reply-flip"><Icon name="arrow-left" size={14} /></span></button>
    {#if isOwn}
      <button class="btn-icon" onclick={beginEdit} title="Düzenle" aria-label="Mesajı düzenle"><Icon name="edit" size={14} /></button>
      <button
        class="btn-icon"
        style="color: var(--veil-danger);"
        title="Sil"
        aria-label="Mesajı sil"
        onclick={handleDelete}
      ><Icon name="trash" size={14} /></button>
    {/if}
  </div>

  <!-- Quick reaction popover -->
  {#if reactOpen}
    <div class="veil-react-pop veil-pop-in" role="menu" aria-label="Hızlı tepkiler">
      {#each COMMON_EMOJI as emoji (emoji)}
        <button type="button" role="menuitem" aria-label={emoji} onclick={() => addReaction(emoji)}>
          {emoji}
        </button>
      {/each}
      <button type="button" class="veil-react-custom" aria-label="Özel tepki" title="Özel emoji" onclick={() => addReaction('')}>
        <Icon name="plus" size={12} />
      </button>
    </div>
  {/if}
</article>

<!-- Global context menu for this message -->
<ContextMenu open={menuOpen} x={menuX} y={menuY} items={menuItems} onClose={() => (menuOpen = false)} />

<ExternalLinkModal
  open={linkModalOpen}
  url={selectedExternalUrl}
  onClose={() => { linkModalOpen = false; selectedExternalUrl = ''; }}
/>

<style>
  .veil-message-avatar-btn {
    border: none;
    background: transparent;
    padding: 0;
    margin: 0;
    cursor: pointer;
    display: block;
    border-radius: var(--radius-full);
    transition: transform var(--t-fast);
  }
  .veil-message-avatar-btn:hover { transform: scale(1.06); }
  .veil-message-author.role-colored { filter: saturate(1.05); }
  .veil-message-author.role-colored:hover { color: var(--veil-brand); filter: brightness(1.1); }
  .veil-message-time-grouped {
    position: absolute;
    top: var(--space-1);
    right: var(--space-4);
    opacity: 0;
    transition: opacity var(--t-fast);
    font-size: var(--text-xs);
    color: var(--veil-text-disabled);
    pointer-events: none;
  }
  .veil-message:hover .veil-message-time-grouped { opacity: 1; }
  .veil-message-status {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .veil-status-spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid var(--veil-border);
    border-top-color: var(--veil-brand);
    border-radius: var(--radius-full);
    animation: spin 0.7s linear infinite;
    display: inline-block;
  }
  .veil-status-failed { color: var(--veil-danger); display: inline-flex; }
  .veil-status-read { color: var(--veil-info); display: inline-flex; }
  .veil-status-delivered { display: inline-flex; }
  .veil-reply-flip { display: inline-flex; transform: scaleX(-1); }
  .veil-message-pinned { color: var(--veil-warning); display: inline-flex; }

  .veil-encrypted-placeholder {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    background: var(--veil-bg-subtle);
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    border: 1px dashed var(--veil-border-subtle);
    font-style: italic;
  }

  .veil-message-attachments {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .veil-react-pop {
    position: absolute;
    top: calc(100% + 6px);
    left: var(--space-4);
    z-index: 40;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 2px;
    padding: var(--space-2);
    background: var(--veil-bg-raised);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
  }
  .veil-react-pop button {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    font-size: 17px;
    cursor: pointer;
    transition: background var(--t-fast), transform var(--t-fast);
  }
  .veil-react-pop button:hover { background: var(--veil-bg-overlay); transform: scale(1.15); }
  .veil-react-custom {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--veil-text-muted);
  }

  /* ── Disappearing Message Countdown ─────────────────────────── */
  .veil-disappear-countdown {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-xs);
    font-weight: 600;
    font-family: var(--font-mono);
    margin-left: var(--space-2);
    padding: 1px 5px;
    border-radius: var(--radius-full);
    background: color-mix(in srgb, currentColor 10%, transparent);
    border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    transition: color var(--t-fast);
    flex-shrink: 0;
  }

  /* ── Link Preview Card ───────────────────────────────────────── */
  .veil-link-preview-loading {
    margin-top: var(--space-2);
    max-width: 420px;
  }

  .veil-link-preview-skeleton {
    height: 80px;
    border-radius: var(--radius-lg);
    background: linear-gradient(
      90deg,
      var(--veil-bg-elevated) 25%,
      var(--veil-bg-surface) 50%,
      var(--veil-bg-elevated) 75%
    );
    background-size: 200% 100%;
    animation: skeleton-shimmer 1.4s ease-in-out infinite;
  }

  @keyframes skeleton-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .veil-link-preview-card {
    margin-top: var(--space-2);
    max-width: 420px;
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    background: var(--veil-bg-elevated);
    overflow: hidden;
    cursor: pointer;
    transition: border-color var(--t-fast), box-shadow var(--t-fast), transform var(--t-fast);
    display: flex;
    flex-direction: column;
    animation: veil-pop-in 0.18s ease-out;
  }

  .veil-link-preview-card:hover {
    border-color: var(--veil-brand);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .veil-link-preview-img {
    width: 100%;
    max-height: 200px;
    object-fit: cover;
    display: block;
    border-bottom: 1px solid var(--veil-border-subtle);
    background: var(--veil-bg-surface);
  }

  .veil-link-preview-body {
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .veil-link-preview-site {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-brand);
  }

  .veil-link-preview-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
    line-height: var(--leading-snug);
    margin: 0;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .veil-link-preview-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    line-height: var(--leading-relaxed);
    margin: 0;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .veil-link-preview-url {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--veil-text-disabled);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 2px;
  }

  @keyframes veil-pop-in {
    from { opacity: 0; transform: translateY(4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }

  .veil-message-edit-box {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    background: linear-gradient(180deg, var(--veil-bg-elevated), var(--veil-bg-surface));
    border: 1px solid var(--veil-brand-border);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
    box-shadow: 0 8px 24px hsl(220 20% 4% / 0.18), 0 1px 0 hsl(0 0% 100% / 0.04) inset;
    animation: veil-pop-in 0.18s cubic-bezier(0.22,1,0.36,1);
  }
  .veil-message-edit-input {
    width: 100%;
    resize: vertical;
    min-height: 52px;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    color: var(--veil-text-primary);
    font: inherit;
    font-size: var(--text-sm);
    line-height: var(--leading-relaxed);
    padding: var(--space-2) var(--space-3);
    outline: none;
    transition: border-color var(--t-fast), box-shadow var(--t-fast), background var(--t-fast);
  }
  .veil-message-edit-input:focus {
    border-color: var(--veil-brand);
    background: var(--veil-bg-base);
    box-shadow: 0 0 0 3px var(--veil-brand-subtle), 0 2px 10px hsl(220 20% 4% / 0.12);
  }
  .veil-message-edit-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .veil-message-edit-hint {
    font-size: 11px;
    color: var(--veil-text-muted);
    letter-spacing: 0.01em;
  }
  .veil-message-edit-btns {
    display: flex;
    gap: var(--space-2);
  }

  .veil-message.burning {
    opacity: 0;
    transform: scale(0.96) translateY(-6px);
    filter: blur(6px) brightness(1.5);
    transition: opacity 0.45s cubic-bezier(0.4, 0, 1, 1), transform 0.45s ease, filter 0.45s ease;
    pointer-events: none;
  }
</style>

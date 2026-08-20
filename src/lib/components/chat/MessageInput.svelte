<script lang="ts">
  import { tick, onDestroy, onMount } from 'svelte';
  import { messageStore } from '$lib/stores/messages';
  import { authStore } from '$lib/stores/auth';
  import { toastStore } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { fileApi, messageApi, presenceApi, type GifResult } from '$lib/api/tauri';
  import { permissionsStore } from '$lib/stores/permissions';
  import { formatTimeoutRemaining } from '$lib/utils/permissions';
  import { open, type DialogFilter } from '@tauri-apps/plugin-dialog';
  import Icon, { type IconName } from '../ui/Icon.svelte';
  import EmojiGifPicker from './EmojiGifPicker.svelte';
  import { draftStore } from '$lib/stores/drafts';
  import { extractImageFromClipboard } from '$lib/utils/clipboard';
  import type { Message } from '$lib/stores/messages';

  let { channelId, placeholder = 'Mesaj yaz...' } = $props<{
    channelId: string;
    placeholder?: string;
  }>();

  const ui = $derived($uiStore);
  const auth = $derived($authStore);

  let content = $state('');
  let sending = $state(false);
  let textareaEl = $state<HTMLTextAreaElement | null>(null);
  let pickerOpen = $state(false);
  let uploadMenuOpen = $state(false);
  let pendingFiles = $state<Array<{ name: string; fileId: string; r2Key: string; sizeBytes: number; mimeTypeHint: string | null }>>([]);
  let uploading = $state(false);

  let activeTrackedChannel = $state('');

  function inferMime(fileName: string): string | null {
    const ext = fileName.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'png': return 'image/png';
      case 'jpg':
      case 'jpeg': return 'image/jpeg';
      case 'gif': return 'image/gif';
      case 'webp': return 'image/webp';
      case 'bmp': return 'image/bmp';
      case 'svg': return 'image/svg+xml';
      case 'mp4': return 'video/mp4';
      case 'webm': return 'video/webm';
      case 'mov': return 'video/quicktime';
      case 'mkv': return 'video/x-matroska';
      case 'mp3': return 'audio/mpeg';
      case 'wav': return 'audio/wav';
      case 'ogg': return 'audio/ogg';
      case 'm4a': return 'audio/mp4';
      case 'flac': return 'audio/flac';
      case 'pdf': return 'application/pdf';
      default: return null;
    }
  }

  let typingTimer: ReturnType<typeof setTimeout> | null = null;
  let typingSent = false;

  function stopTypingNow(chId: string) {
    if (typingTimer) {
      clearTimeout(typingTimer);
      typingTimer = null;
    }
    if (typingSent && chId) {
      typingSent = false;
      presenceApi.setTyping({ channelId: chId, isTyping: false }).catch(() => {});
    }
  }

  function syncDraft() {
    if (channelId) {
      draftStore.setDraft(channelId, content, ui.replyTo, pendingFiles);
    }
  }

  // Load / Switch draft when channelId changes
  $effect(() => {
    const nextChannel = channelId;
    if (nextChannel !== activeTrackedChannel) {
      // 1. Save draft and stop typing for the channel we are leaving
      if (activeTrackedChannel) {
        draftStore.setDraft(activeTrackedChannel, content, ui.replyTo, pendingFiles);
        stopTypingNow(activeTrackedChannel);
      }

      // 2. Load draft for the channel we are entering
      activeTrackedChannel = nextChannel;
      if (nextChannel) {
        const draft = draftStore.getDraft(nextChannel);
        content = draft?.content ?? '';
        pendingFiles = draft?.files ? [...draft.files] : [];
        if (draft?.replyTo) {
          uiStore.setReplyTo(draft.replyTo);
        } else {
          uiStore.clearReplyIfChannel(nextChannel);
        }
      }

      tick().then(() => {
        resizeTextarea();
      });
    }
  });

  onDestroy(() => {
    if (activeTrackedChannel) {
      draftStore.setDraft(activeTrackedChannel, content, ui.replyTo, pendingFiles);
      stopTypingNow(activeTrackedChannel);
    }
  });

  interface UploadKind {
    id: string;
    label: string;
    icon: IconName;
    filters: DialogFilter[];
  }

  const UPLOAD_KINDS: UploadKind[] = [
    { id: 'file', label: 'Dosya', icon: 'upload', filters: [{ name: 'Tüm dosyalar', extensions: ['*'] }] },
    { id: 'image', label: 'Fotoğraf', icon: 'camera', filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'] }] },
    { id: 'video', label: 'Video', icon: 'screen', filters: [{ name: 'Videolar', extensions: ['mp4', 'webm', 'mov', 'mkv'] }] },
    { id: 'audio', label: 'Ses', icon: 'mic', filters: [{ name: 'Ses kayıtları', extensions: ['mp3', 'wav', 'ogg', 'm4a', 'flac'] }] },
  ];

  function sendTyping() {
    if (!channelId) return;
    if (typingTimer) clearTimeout(typingTimer);
    if (!typingSent) {
      typingSent = true;
      presenceApi.setTyping({ channelId, isTyping: true }).catch(() => {});
    }
    typingTimer = setTimeout(() => {
      typingSent = false;
      presenceApi.setTyping({ channelId, isTyping: false }).catch(() => {});
    }, 3000);
  }

  async function send() {
    const trimmed = content.trim();
    if ((!trimmed && pendingFiles.length === 0) || sending || uploading) return;

    sending = true;
    const prev = content;
    const files = pendingFiles;
    const replyTarget = ui.replyTo && ui.replyTo.channelId === channelId ? ui.replyTo : null;
    pendingFiles = [];
    content = '';
    syncDraft();
    resizeTextarea();
    textareaEl?.focus();
    stopTypingNow(channelId);

    try {
      const attachments: Message['attachments'] = files.map(f => ({
        fileId: f.fileId,
        r2Key: f.r2Key,
        sizeBytes: f.sizeBytes,
        contentKeyCiphertext: '',
        mimeTypeHint: f.mimeTypeHint,
        fileName: f.name,
      }));
      await messageStore.sendMessage(channelId, trimmed, replyTarget?.messageId, attachments);
      draftStore.clearDraft(channelId);
      uiStore.setReplyTo(null);
    } catch (err) {
      content = prev;
      pendingFiles = files;
      syncDraft();
      const errorMsg = String(err);
      if (errorMsg.includes('Kullanıcının genel anahtarı henüz kayıtlı değil')) {
        toastStore.error('Karşının anahtarı henüz senkronize edilmemiş. Karşı taraf uygulamayı açtığında otomatik düzelecek.');
      } else if (errorMsg.includes('Mesaj çok uzun')) {
        toastStore.error('Mesaj çok uzun. En fazla 4000 karakter gönderebilirsiniz.');
      } else if (errorMsg.includes('Mesaj içeriği veya dosya eklenmelidir')) {
        toastStore.error('Mesaj içeriği boş olamaz.');
      } else {
        toastStore.error('Mesaj gönderilemedi. Yeniden deneniyor...');
        setTimeout(() => {
          if (prev.trim()) {
            content = prev;
            pendingFiles = files;
            syncDraft();
          }
        }, 2000);
      }
    } finally {
      sending = false;
      await tick();
      textareaEl?.focus();
      requestAnimationFrame(() => textareaEl?.focus());
    }
  }

  async function editLastOwnMessage() {
    const list = $messageStore.byChannel[channelId] ?? [];
    const myId = auth.identity?.id;
    const lastOwn = [...list].reverse().find(m => m.senderId === myId || m.isOwn || m.senderId === 'self');
    if (!lastOwn || !lastOwn.content) return;
    const next = await uiStore.promptInput('Mesajı düzenle:', {
      title: 'Son Mesajı Düzenle',
      confirmLabel: 'Kaydet',
      defaultValue: lastOwn.content,
    });
    if (next === null || next.trim() === lastOwn.content) return;
    try {
      const edited = await messageApi.edit(lastOwn.id, next.trim());
      messageStore.patchMessage(channelId, lastOwn.id, {
        content: edited.content,
        editedAt: edited.editedAt,
      });
      toastStore.success('Mesaj düzenlendi.');
    } catch {
      toastStore.error('Mesaj düzenlenemedi.');
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    } else if (e.key === 'Escape') {
      if (ui.replyTo) {
        e.preventDefault();
        uiStore.setReplyTo(null);
        syncDraft();
      }
      if (pickerOpen) {
        e.preventDefault();
        pickerOpen = false;
      }
    } else if (e.key === 'ArrowUp' && content === '' && pendingFiles.length === 0) {
      e.preventDefault();
      void editLastOwnMessage();
    }
  }

  function resizeTextarea() {
    if (!textareaEl) return;
    textareaEl.style.height = 'auto';
    textareaEl.style.height = `${Math.min(textareaEl.scrollHeight, window.innerHeight * 0.5)}px`;
  }

  function onInput() {
    resizeTextarea();
    syncDraft();
    if (content.trim().length > 0) {
      sendTyping();
    } else if (typingSent) {
      stopTypingNow(channelId);
    }
  }

  async function onPaste(e: ClipboardEvent) {
    const imgFile = extractImageFromClipboard(e);
    if (imgFile) {
      e.preventDefault();
      uploading = true;
      try {
        const arrayBuf = await imgFile.arrayBuffer();
        const bytes = new Uint8Array(arrayBuf);
        const fileName = imgFile.name || `clipboard-${Date.now()}.png`;
        const info = await fileApi.uploadBytes({ bytes, channelId });
        pendingFiles = [
          ...pendingFiles,
          {
            name: fileName,
            fileId: info.fileId,
            r2Key: info.r2Key ?? '',
            sizeBytes: info.sizeBytes,
            mimeTypeHint: inferMime(fileName) || imgFile.type || 'image/png',
          },
        ];
        syncDraft();
        toastStore.success('Pano görseli eklendi.');
      } catch {
        toastStore.error('Pano görseli yüklenemedi.');
      } finally {
        uploading = false;
      }
      return;
    }

    // Normal text paste: let native paste happen, then resize & sync
    setTimeout(() => {
      resizeTextarea();
      syncDraft();
      if (content.trim().length > 0) {
        sendTyping();
      }
    }, 10);
  }

  async function pickFiles(kind: (typeof UPLOAD_KINDS)[number]) {
    if (uploading) return;
    uploadMenuOpen = false;
    const selected = await open({
      title: kind.label === 'Dosya' ? 'Dosya seç' : `${kind.label} seç`,
      multiple: true,
      filters: kind.filters,
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    await uploadPaths(paths);
  }

  async function uploadPaths(paths: string[]) {
    uploading = true;
    try {
      for (const path of paths) {
        const info = await fileApi.upload({ path, channelId });
        const fileName = path.split(/[\\/]/).pop() ?? 'dosya';
        pendingFiles = [
          ...pendingFiles,
          {
            name: fileName,
            fileId: info.fileId,
            r2Key: info.r2Key ?? '',
            sizeBytes: info.sizeBytes,
            mimeTypeHint: inferMime(fileName),
          },
        ];
      }
      syncDraft();
      if (paths.length > 0 && !content.trim()) textareaEl?.focus();
    } catch {
      toastStore.error('Dosya yüklenemedi.');
    } finally {
      uploading = false;
    }
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (!files?.length) return;
    const paths: string[] = [];
    for (let i = 0; i < files.length; i++) {
      const f = files[i];
      const p = (f as any).path || (f as any).webkitRelativePath;
      if (p) paths.push(p);
    }
    if (paths.length > 0) {
      await uploadPaths(paths);
    } else {
      toastStore.info('Dosyaları eklemek için sol alttaki "+" butonunu kullanabilirsiniz.');
    }
  }

  function insertEmoji(emoji: string) {
    content = `${content}${emoji}`;
    textareaEl?.focus();
    resizeTextarea();
    syncDraft();
    sendTyping();
  }

  function insertGif(gif: GifResult) {
    content = `${content}![${gif.title || 'gif'}](${gif.url})`;
    pickerOpen = false;
    textareaEl?.focus();
    resizeTextarea();
    syncDraft();
    sendTyping();
  }

  function removePending(index: number) {
    pendingFiles = pendingFiles.filter((_, i) => i !== index);
    syncDraft();
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  const perms = $derived($permissionsStore);
  const canSend = $derived(perms.isOwner || perms.has('send_messages'));
  const canAttach = $derived(perms.isOwner || perms.has('attach_files'));
  const isTimedOut = $derived(perms.isTimedOut);
  const timeoutSeconds = $derived(perms.timeoutRemainingSeconds);

  // ── Rich Character Counter & Animated Radial Ring ────────────────────────
  const MAX_CHAR_LIMIT = 4000;
  const charCount = $derived(content.length);
  const charPercent = $derived(Math.min(100, (charCount / MAX_CHAR_LIMIT) * 100));
  const remainingChars = $derived(MAX_CHAR_LIMIT - charCount);
  const ringRadius = 7.5;
  const ringCircumference = 2 * Math.PI * ringRadius; // ~47.12
  const strokeDashoffset = $derived(
    ringCircumference - (charPercent / 100) * ringCircumference
  );

  const counterLevel = $derived.by<'normal' | 'notice' | 'warning' | 'critical'>(() => {
    if (charCount >= 3900) return 'critical';
    if (charCount >= 3400) return 'warning';
    if (charCount >= 2500) return 'notice';
    return 'normal';
  });
</script>

{#if isTimedOut}
  <div class="veil-message-box veil-message-box-locked timeout" role="alert">
    <Icon name="moon" size={18} />
    <span class="locked-text">
      Bu toplulukta susturuldunuz. Kalan süre: <strong>{formatTimeoutRemaining(timeoutSeconds)}</strong>
    </span>
  </div>
{:else if !canSend}
  <div class="veil-message-box veil-message-box-locked" role="alert">
    <Icon name="lock" size={18} />
    <span class="locked-text">Bu kanala mesaj gönderme yetkiniz bulunmuyor.</span>
  </div>
{:else}
<div class="veil-message-box" role="form" aria-label="Mesaj gir">
  {#if ui.replyTo && ui.replyTo.channelId === channelId}
    <div class="veil-reply-target">
      <span class="veil-reply-arrow">↩</span>
      <span class="veil-reply-meta">
        <span class="veil-reply-author">{ui.replyTo.author}</span>
        <span class="veil-reply-snippet">{ui.replyTo.content || 'Mesaj'}</span>
      </span>
      <button
        type="button"
        class="btn-icon veil-reply-cancel"
        aria-label="Yanıtı iptal et"
        title="Yanıtı iptal et"
        onclick={() => uiStore.setReplyTo(null)}
      >
        <Icon name="x" size={13} />
      </button>
    </div>
  {/if}

  {#if pendingFiles.length > 0}
    <div class="veil-pending-files" aria-label="Bekleyen dosyalar">
      {#each pendingFiles as file, i (file.fileId)}
        <span class="veil-pending-file" title={file.name}>
          <Icon name="upload" size={12} />
          <span class="veil-pending-name">{file.name}</span>
          <span class="veil-pending-size">{formatBytes(file.sizeBytes)}</span>
          <button
            type="button"
            class="btn-icon veil-pending-remove"
            aria-label="Dosyayı kaldır"
            onclick={() => removePending(i)}
          >
            <Icon name="x" size={12} />
          </button>
        </span>
      {/each}
    </div>
  {/if}

  <div class="veil-upload-wrap">
    <button
      class="btn-icon"
      title={canAttach ? 'Dosya ekle' : 'Dosya ekleme izniniz yok'}
      aria-label="Dosya ekle"
      aria-expanded={uploadMenuOpen}
      disabled={uploading || !canAttach}
      onclick={() => { if (canAttach) uploadMenuOpen = !uploadMenuOpen; }}
    >
      {#if uploading}
        <div class="veil-spinner veil-spinner-sm" aria-hidden="true"></div>
      {:else}
        <Icon name="plus" size={18} />
      {/if}
    </button>
    {#if uploadMenuOpen}
      <div class="veil-upload-menu veil-pop-in" role="menu" aria-label="Eklenecek tür">
        {#each UPLOAD_KINDS as kind (kind.id)}
          <button type="button" role="menuitem" onclick={() => pickFiles(kind)}>
            <Icon name={kind.icon} size={15} />
            {kind.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <textarea
    bind:this={textareaEl}
    bind:value={content}
    class="veil-message-textarea"
    {placeholder}
    rows={1}
    aria-label="Mesaj içeriği"
    aria-multiline="true"
    maxlength={4000}
    onkeydown={onKeydown}
    oninput={onInput}
    onpaste={onPaste}
    ondrop={onDrop}
  ></textarea>

  <div class="veil-input-actions">
    <!-- Rich Character Counter with Radial Progress Ring -->
    {#if charCount > 0}
      <div
        class="veil-char-counter {counterLevel}"
        class:near-limit={charCount >= 3400}
        title="{charCount} / {MAX_CHAR_LIMIT} karakter ({remainingChars} kaldı)"
        role="status"
        aria-label="Karakter sayısı: {charCount} / {MAX_CHAR_LIMIT}"
      >
        <svg class="veil-char-ring" width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
          <circle
            cx="10"
            cy="10"
            r={ringRadius}
            class="veil-ring-track"
          />
          <circle
            cx="10"
            cy="10"
            r={ringRadius}
            class="veil-ring-progress"
            style="stroke-dasharray: {ringCircumference}; stroke-dashoffset: {strokeDashoffset};"
          />
        </svg>

        {#if charCount >= 3400}
          <span class="veil-char-remaining">{remainingChars}</span>
        {:else}
          <span class="veil-char-badge">{charCount}</span>
        {/if}
      </div>
    {/if}

    <div class="veil-eg-wrap">
      <button
        class="btn-icon"
        title="Emoji ve GIF"
        aria-label="Emoji ve GIF ekle"
        aria-expanded={pickerOpen}
        onclick={() => (pickerOpen = !pickerOpen)}
      >
        <Icon name="sparkle" size={17} />
      </button>
      {#if pickerOpen}
        <EmojiGifPicker
          onPickEmoji={insertEmoji}
          onPickGif={insertGif}
          onClose={() => (pickerOpen = false)}
        />
      {/if}
    </div>
    <button
      class="btn-send"
      onclick={send}
      disabled={(!content.trim() && pendingFiles.length === 0) || sending || uploading}
      aria-label="Mesaj gönder"
      title="Gönder (Enter)"
    >
      {#if sending}
        <div class="veil-spinner" style="width:16px;height:16px;border-width:2px;"></div>
      {:else}
        <Icon name="arrow-right" size={18} />
      {/if}
    </button>
  </div>
</div>
{/if}

<style>
  .veil-pending-files {
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: calc(100% + 6px);
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .veil-pending-file {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 3px var(--space-2);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-full);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    max-width: 220px;
  }
  .veil-pending-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-pending-size { color: var(--veil-text-muted); flex-shrink: 0; }
  .veil-pending-remove { padding: 1px; }
  .veil-message-box { position: relative; }

  .veil-reply-target {
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: calc(100% + 6px);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    border-left: 3px solid var(--veil-brand);
    border-radius: var(--radius-lg);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-reply-arrow { color: var(--veil-brand); }
  .veil-reply-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }
  .veil-reply-author { font-weight: 600; color: var(--veil-brand); flex-shrink: 0; }
  .veil-reply-snippet {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-reply-cancel { padding: 2px; }

  .veil-upload-wrap { position: relative; flex-shrink: 0; }
  .veil-upload-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    z-index: 60;
    min-width: 180px;
    background: color-mix(in srgb, var(--veil-bg-raised) 92%, transparent);
    backdrop-filter: blur(14px);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-xl);
    padding: var(--space-1);
  }
  .veil-upload-menu button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-2) var(--space-3);
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--veil-text-secondary);
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
    text-align: left;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .veil-upload-menu button:hover { background: var(--veil-bg-overlay); color: var(--veil-text-primary); }

  .veil-eg-wrap { position: relative; }

  .veil-message-box-locked {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-4);
    background: color-mix(in srgb, var(--veil-bg-surface) 90%, transparent);
    border: 1px dashed var(--veil-border);
    border-radius: var(--radius-xl);
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
    user-select: none;
  }

  .veil-message-box-locked.timeout {
    background: rgba(235, 77, 75, 0.08);
    border-color: rgba(235, 77, 75, 0.3);
    color: #ff7675;
  }

  .locked-text {
    font-weight: 500;
  }

  /* ── Rich Character Counter ────────────────────────────────── */
  .veil-char-counter {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px 2px 5px;
    border-radius: var(--radius-full);
    background: color-mix(in srgb, var(--veil-bg-overlay) 60%, transparent);
    border: 1px solid var(--veil-border-subtle);
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--veil-text-muted);
    user-select: none;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    animation: char-pop-in 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    margin-right: 2px;
  }

  @keyframes char-pop-in {
    from { opacity: 0; transform: scale(0.75); }
    to { opacity: 1; transform: scale(1); }
  }

  .veil-char-ring {
    transform: rotate(-90deg);
    flex-shrink: 0;
  }

  .veil-ring-track {
    fill: none;
    stroke: color-mix(in srgb, var(--veil-border) 80%, transparent);
    stroke-width: 2.2px;
  }

  .veil-ring-progress {
    fill: none;
    stroke-width: 2.2px;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.15s ease, stroke 0.25s ease;
    stroke: var(--veil-brand);
  }

  .veil-char-badge {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--veil-text-muted);
    letter-spacing: -0.02em;
  }

  .veil-char-remaining {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  /* Counter stages */
  .veil-char-counter.normal .veil-ring-progress {
    stroke: var(--veil-brand);
  }
  .veil-char-counter.normal:hover {
    border-color: var(--veil-border);
    color: var(--veil-text-primary);
  }

  .veil-char-counter.notice {
    background: color-mix(in srgb, var(--veil-brand) 12%, transparent);
    border-color: color-mix(in srgb, var(--veil-brand) 30%, transparent);
    color: var(--veil-brand);
  }
  .veil-char-counter.notice .veil-ring-progress {
    stroke: hsl(200 90% 55%);
  }

  .veil-char-counter.warning {
    background: color-mix(in srgb, hsl(45 95% 55%) 14%, transparent);
    border-color: color-mix(in srgb, hsl(45 95% 55%) 40%, transparent);
    color: hsl(45 95% 55%);
    box-shadow: 0 0 10px hsl(45 95% 55% / 0.2);
  }
  .veil-char-counter.warning .veil-ring-progress {
    stroke: hsl(45 95% 55%);
  }

  .veil-char-counter.critical {
    background: color-mix(in srgb, hsl(0 84% 60%) 18%, transparent);
    border-color: color-mix(in srgb, hsl(0 84% 60%) 50%, transparent);
    color: hsl(0 84% 60%);
    box-shadow: 0 0 12px hsl(0 84% 60% / 0.35);
    animation: char-pop-in 0.2s cubic-bezier(0.16, 1, 0.3, 1), char-pulse 1.2s infinite ease-in-out;
  }
  .veil-char-counter.critical .veil-ring-progress {
    stroke: hsl(0 84% 60%);
  }

  @keyframes char-pulse {
    0%, 100% { transform: scale(1); box-shadow: 0 0 10px hsl(0 84% 60% / 0.3); }
    50% { transform: scale(1.05); box-shadow: 0 0 16px hsl(0 84% 60% / 0.6); }
  }
</style>

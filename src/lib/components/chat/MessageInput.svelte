<script lang="ts">
  import { tick, onDestroy, onMount } from 'svelte';
  import { messageStore } from '$lib/stores/messages';
  import { authStore } from '$lib/stores/auth';
  import { toastStore } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { fileApi, messageApi, presenceApi, privacyToolsApi, type GifResult, type LinkPreviewResult } from '$lib/api/tauri';
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
  let inputFocused = $state(false);
  let textareaEl = $state<HTMLTextAreaElement | null>(null);
  let pickerOpen = $state(false);
  let uploadMenuOpen = $state(false);
  let pendingFiles = $state<Array<{ name: string; fileId: string; r2Key: string; sizeBytes: number; mimeTypeHint: string | null; contentKeyCiphertext?: string | null }>>([]);
  let uploading = $state(false);

  // Link preview detection and sender toggle
  const URL_REGEX = /https?:\/\/[^\s<>"]+/;
  let linkPreview = $state<LinkPreviewResult | null>(null);
  let linkPreviewLoading = $state(false);
  let linkPreviewEnabled = $state(true);
  let dismissedUrl = $state<string | null>(null);
  let linkPreviewDebounce: ReturnType<typeof setTimeout> | null = null;

  const detectedUrl = $derived.by(() => {
    const match = content.match(URL_REGEX);
    return match ? match[0] : null;
  });

  $effect(() => {
    const url = detectedUrl;
    if (linkPreviewDebounce) clearTimeout(linkPreviewDebounce);
    if (!url || url === dismissedUrl) {
      linkPreview = null;
      linkPreviewLoading = false;
      return;
    }
    linkPreviewLoading = true;
    linkPreviewDebounce = setTimeout(() => {
      privacyToolsApi.fetchLinkPreview(url)
        .then((res) => {
          if (detectedUrl === url && res && res.isSafe !== false) {
            linkPreview = res;
          }
        })
        .catch(() => {
          linkPreview = null;
        })
        .finally(() => {
          linkPreviewLoading = false;
        });
    }, 350);
  });

  function dismissLinkPreview() {
    if (detectedUrl) {
      dismissedUrl = detectedUrl;
    }
    linkPreview = null;
    linkPreviewEnabled = false;
  }

  // Voice recording state
  let mediaRecorder = $state<MediaRecorder | null>(null);
  let audioChunks = $state<Blob[]>([]);
  let isRecordingVoice = $state(false);
  let voiceDuration = $state(0);
  let voiceInterval: ReturnType<typeof setInterval> | null = null;

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
    let messageToSend = trimmed;
    if (detectedUrl && !linkPreviewEnabled && !messageToSend.includes(`<${detectedUrl}>`)) {
      messageToSend = messageToSend.replace(detectedUrl, `<${detectedUrl}>`);
    }

    pendingFiles = [];
    content = '';
    linkPreview = null;
    dismissedUrl = null;
    linkPreviewEnabled = true;
    syncDraft();
    resizeTextarea();
    textareaEl?.focus();
    stopTypingNow(channelId);

    try {
      const attachments: Message['attachments'] = files.map(f => ({
        fileId: f.fileId,
        r2Key: f.r2Key,
        sizeBytes: f.sizeBytes,
        contentKeyCiphertext: f.contentKeyCiphertext ?? null,
        mimeTypeHint: f.mimeTypeHint,
        fileName: f.name,
      }));
      await messageStore.sendMessage(channelId, messageToSend, replyTarget?.messageId, attachments);
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
            contentKeyCiphertext: info.contentKeyCiphertext ?? null,
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

  const MAX_UPLOAD_BYTES = 25 * 1024 * 1024; // 25 MB — mirrors server-side MAX_FILE_SIZE

  function isTooLargeError(err: unknown): boolean {
    const msg = String(err);
    return msg.includes('FileTooLarge') || msg.includes('çok büyük');
  }

  function reportUploadOutcome(uploadedCount: number, skipped: string[], failedNames: string[]) {
    if (uploadedCount > 0) {
      toastStore.success(`${uploadedCount} dosya eklendi.`);
    }
    if (skipped.length > 0) {
      toastStore.warning(`Boyut sınırı aşıldı: ${skipped.join(', ')}`);
    }
    if (failedNames.length > 0) {
      toastStore.error(`Yüklenemedi: ${failedNames.join(', ')}`);
    }
  }

  async function uploadPaths(paths: string[]) {
    uploading = true;
    const skipped: string[] = [];
    const failedNames: string[] = [];
    let uploadedCount = 0;
    try {
      for (const path of paths) {
        const fileName = path.split(/[\\/]/).pop() ?? 'dosya';
        try {
          const info = await fileApi.upload({ path, channelId });
          pendingFiles = [
            ...pendingFiles,
            {
              name: fileName,
              fileId: info.fileId,
              r2Key: info.r2Key ?? '',
              sizeBytes: info.sizeBytes,
              mimeTypeHint: inferMime(fileName),
              contentKeyCiphertext: info.contentKeyCiphertext ?? null,
            },
          ];
          uploadedCount++;
        } catch (err) {
          // Server-side MAX_FILE_SIZE backstop surfaces as FileTooLarge
          if (isTooLargeError(err)) {
            skipped.push(fileName);
          } else {
            failedNames.push(fileName);
          }
        }
      }
      syncDraft();
      reportUploadOutcome(uploadedCount, skipped, failedNames);
      if (paths.length > 0 && !content.trim()) textareaEl?.focus();
    } finally {
      uploading = false;
    }
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (!files?.length) return;
    uploading = true;
    const skipped: string[] = [];
    const failedNames: string[] = [];
    let uploadedCount = 0;
    try {
      for (let i = 0; i < files.length; i++) {
        const f = files[i];
        // Client-side size guard — reject before any upload attempt
        if (f.size > MAX_UPLOAD_BYTES) {
          skipped.push(f.name || 'dosya');
          continue;
        }
        try {
          const p = (f as File & { path?: string }).path;
          if (p && typeof p === 'string' && p.length > 0) {
            const info = await fileApi.upload({ path: p, channelId });
            const fileName = p.split(/[\\/]/).pop() ?? f.name ?? 'dosya';
            pendingFiles = [
              ...pendingFiles,
              {
                name: fileName,
                fileId: info.fileId,
                r2Key: info.r2Key ?? '',
                sizeBytes: info.sizeBytes,
                mimeTypeHint: inferMime(fileName) || f.type || null,
                contentKeyCiphertext: info.contentKeyCiphertext ?? null,
              },
            ];
          } else {
            const arrayBuf = await f.arrayBuffer();
            const bytes = new Uint8Array(arrayBuf);
            const fileName = f.name || `dosya-${Date.now()}`;
            const info = await fileApi.uploadBytes({ bytes, channelId });
            pendingFiles = [
              ...pendingFiles,
              {
                name: fileName,
                fileId: info.fileId,
                r2Key: info.r2Key ?? '',
                sizeBytes: info.sizeBytes,
                mimeTypeHint: inferMime(fileName) || f.type || null,
                contentKeyCiphertext: info.contentKeyCiphertext ?? null,
              },
            ];
          }
          uploadedCount++;
        } catch (err) {
          const fileName = f.name || 'dosya';
          if (isTooLargeError(err)) {
            skipped.push(fileName);
          } else {
            failedNames.push(fileName);
          }
        }
      }
      syncDraft();
      reportUploadOutcome(uploadedCount, skipped, failedNames);
      if (!content.trim()) textareaEl?.focus();
    } finally {
      uploading = false;
    }
  }

  async function startVoiceRecording() {
    if (isRecordingVoice || uploading) return;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      audioChunks = [];
      const mimeType = MediaRecorder.isTypeSupported('audio/webm')
        ? 'audio/webm'
        : MediaRecorder.isTypeSupported('audio/mp4')
        ? 'audio/mp4'
        : 'audio/ogg';
      const recorder = new MediaRecorder(stream, { mimeType });
      recorder.ondataavailable = (e) => {
        if (e.data && e.data.size > 0) {
          audioChunks.push(e.data);
        }
      };
      recorder.onstop = async () => {
        stream.getTracks().forEach((t) => t.stop());
        if (voiceInterval) {
          clearInterval(voiceInterval);
          voiceInterval = null;
        }
        if (audioChunks.length === 0) return;
        const blob = new Blob(audioChunks, { type: mimeType });
        audioChunks = [];
        uploading = true;
        try {
          const arrayBuf = await blob.arrayBuffer();
          const bytes = new Uint8Array(arrayBuf);
          const ext = mimeType.includes('webm') ? 'webm' : mimeType.includes('mp4') ? 'mp4' : 'ogg';
          const fileName = `ses-kaydi-${Date.now()}.${ext}`;
          const info = await fileApi.uploadBytes({ bytes, channelId });
          pendingFiles = [
            ...pendingFiles,
            {
              name: fileName,
              fileId: info.fileId,
              r2Key: info.r2Key ?? '',
              sizeBytes: info.sizeBytes,
              mimeTypeHint: mimeType,
              contentKeyCiphertext: info.contentKeyCiphertext ?? null,
            },
          ];
          syncDraft();
          toastStore.success('Ses kaydı eklendi — oynatıp sonra Gönder ile iletebilirsin.');
        } catch {
          toastStore.error('Ses kaydı yüklenemedi.');
          return;
        } finally {
          uploading = false;
        }
      };
      mediaRecorder = recorder;
      recorder.start(250);
      isRecordingVoice = true;
      voiceDuration = 0;
      voiceInterval = setInterval(() => {
        voiceDuration += 1;
      }, 1000);
    } catch {
      toastStore.error('Mikrofon erişimi sağlanamadı.');
    }
  }

  function stopVoiceRecording() {
    if (mediaRecorder && mediaRecorder.state !== 'inactive') {
      mediaRecorder.stop();
    }
    isRecordingVoice = false;
    if (voiceInterval) {
      clearInterval(voiceInterval);
      voiceInterval = null;
    }
  }

  function cancelVoiceRecording() {
    if (mediaRecorder && mediaRecorder.state !== 'inactive') {
      mediaRecorder.ondataavailable = null;
      mediaRecorder.onstop = () => {
        mediaRecorder?.stream?.getTracks().forEach((t) => t.stop());
      };
      mediaRecorder.stop();
    }
    audioChunks = [];
    isRecordingVoice = false;
    if (voiceInterval) {
      clearInterval(voiceInterval);
      voiceInterval = null;
    }
    toastStore.info('Ses kaydı iptal edildi.');
  }

  function formatDuration(sec: number): string {
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
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
  const showCharCounter = $derived((charCount > 0 || inputFocused) && !isRecordingVoice);
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

  {#if detectedUrl && dismissedUrl !== detectedUrl && (linkPreview || linkPreviewLoading)}
    <div class="veil-composer-link-preview" role="region" aria-label="Bağlantı önizleme çubuğu">
      <div class="veil-clp-info">
        {#if linkPreview?.image}
          <img src={linkPreview.image} alt="" class="veil-clp-thumb" />
        {:else}
          <div class="veil-clp-icon">
            <Icon name="link" size={14} />
          </div>
        {/if}
        <div class="veil-clp-text">
          <span class="veil-clp-title">{linkPreview?.title || detectedUrl}</span>
          <span class="veil-clp-domain">{linkPreview?.siteName || 'Web Bağlantısı'}</span>
        </div>
      </div>
      <div class="veil-clp-controls">
        <label class="veil-clp-toggle" title="Önizlemeyi aç / kapat">
          <input
            type="checkbox"
            checked={linkPreviewEnabled}
            onchange={(e) => (linkPreviewEnabled = (e.currentTarget as HTMLInputElement).checked)}
          />
          <span class="veil-clp-toggle-label">{linkPreviewEnabled ? 'Önizleme Açık' : 'Önizleme Kapalı'}</span>
        </label>
        <button
          type="button"
          class="btn-icon veil-clp-close"
          onclick={dismissLinkPreview}
          title="Önizlemeyi kaldır"
          aria-label="Önizlemeyi kaldır"
        >
          <Icon name="x" size={13} />
        </button>
      </div>
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

  {#if isRecordingVoice}
    <div class="veil-voice-recording-bar" role="region" aria-label="Ses kaydı yapılıyor">
      <div class="veil-recording-pulse" aria-hidden="true"></div>
      <span class="veil-recording-time">{formatDuration(voiceDuration)}</span>
      <span class="veil-recording-label">Ses kaydediliyor…</span>
      <button
        type="button"
        class="btn-icon veil-voice-cancel-btn"
        title="Kaydı iptal et"
        aria-label="Kaydı iptal et"
        onclick={cancelVoiceRecording}
      >
        <Icon name="trash" size={16} />
      </button>
      <button
        type="button"
        class="btn-send veil-voice-stop-btn"
        title="Kaydı tamamla ve ekle"
        aria-label="Kaydı tamamla"
        onclick={stopVoiceRecording}
      >
        <Icon name="check" size={16} />
      </button>
    </div>
  {:else}
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
      onfocus={() => (inputFocused = true)}
      onblur={() => (inputFocused = false)}
    ></textarea>
  {/if}

  <div class="veil-input-actions">
    <!-- Rich Character Counter with Radial Progress Ring -->
    {#if showCharCounter}
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
          <span class="veil-char-remaining">{charCount}/{MAX_CHAR_LIMIT}</span>
        {:else}
          <span class="veil-char-badge">{charCount}/{MAX_CHAR_LIMIT}</span>
        {/if}
      </div>
    {/if}

    {#if !isRecordingVoice}
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
        class="btn-icon"
        title="Sesli mesaj kaydet"
        aria-label="Sesli mesaj kaydet"
        onclick={startVoiceRecording}
        disabled={uploading || !canAttach}
      >
        <Icon name="mic" size={17} />
      </button>

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
    {/if}
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

  .veil-voice-recording-bar {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: color-mix(in srgb, hsl(0 84% 60%) 12%, var(--veil-bg-surface));
    border: 1px solid color-mix(in srgb, hsl(0 84% 60%) 35%, transparent);
    border-radius: var(--radius-xl);
    min-height: 40px;
  }
  .veil-recording-pulse {
    width: 10px;
    height: 10px;
    background: hsl(0 84% 60%);
    border-radius: 50%;
    animation: rec-pulse 1s infinite ease-in-out;
  }
  @keyframes rec-pulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.4); opacity: 0.4; }
  }
  .veil-recording-time {
    font-weight: 700;
    font-size: var(--text-sm);
    color: hsl(0 84% 60%);
    font-family: var(--font-mono, monospace);
  }
  .veil-recording-label {
    flex: 1;
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
  }
  .veil-voice-cancel-btn {
    color: var(--veil-text-muted);
  }
  .veil-voice-cancel-btn:hover {
    color: hsl(0 84% 60%);
    background: color-mix(in srgb, hsl(0 84% 60%) 15%, transparent);
  }
  .veil-voice-stop-btn {
    background: var(--veil-brand);
    color: #fff;
    border: none;
    border-radius: var(--radius-full);
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: transform var(--t-fast);
  }
  .veil-voice-stop-btn:hover {
    transform: scale(1.08);
  }

  /* ── Rich Link Preview Composer Bar ── */
  .veil-composer-link-preview {
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: calc(100% + 6px);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    padding: 6px 10px;
    background: color-mix(in srgb, var(--veil-bg-elevated) 95%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid var(--veil-border);
    border-left: 3px solid var(--veil-brand);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    animation: clp-slide-in 0.2s ease-out;
  }

  @keyframes clp-slide-in {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .veil-clp-info {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    flex: 1;
  }

  .veil-clp-thumb {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    object-fit: cover;
    flex-shrink: 0;
  }

  .veil-clp-icon {
    width: 30px;
    height: 30px;
    border-radius: var(--radius-md);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .veil-clp-text {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .veil-clp-title {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-clp-domain {
    font-size: 11px;
    color: var(--veil-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .veil-clp-controls {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .veil-clp-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--veil-text-secondary);
    cursor: pointer;
    user-select: none;
  }

  .veil-clp-toggle input[type="checkbox"] {
    accent-color: var(--veil-brand);
    cursor: pointer;
  }

  .veil-clp-close {
    padding: 2px;
    color: var(--veil-text-muted);
  }

  .veil-clp-close:hover {
    color: var(--veil-text-primary);
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte';
  import { fileApi } from '$lib/api/tauri';
  import { save } from '@tauri-apps/plugin-dialog';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '../ui/Icon.svelte';
  import LightboxModal from '../ui/LightboxModal.svelte';

  interface Attachment {
    fileId: string;
    r2Key: string;
    sizeBytes: number;
    contentKeyCiphertext?: string | null;
    mimeTypeHint?: string | null;
    fileName?: string | null;
  }

  let { attachment } = $props<{ attachment: Attachment }>();

  let dataUrl = $state<string | null>(null);
  let loading = $state(true);
  let error = $state(false);
  let lightboxOpen = $state(false);
  let downloading = $state(false);

  function formatBytes(bytes: number): string {
    if (!bytes) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  const mime = $derived.by(() => {
    if (dataUrl?.startsWith('data:image/')) return 'image';
    if (dataUrl?.startsWith('data:video/')) return 'video';
    if (dataUrl?.startsWith('data:audio/')) return 'audio';
    if (attachment.mimeTypeHint?.startsWith('image/')) return 'image';
    if (attachment.mimeTypeHint?.startsWith('video/')) return 'video';
    if (attachment.mimeTypeHint?.startsWith('audio/')) return 'audio';
    return 'file';
  });

  const displayFileName = $derived.by(() => {
    if (attachment.fileName) return attachment.fileName;
    if (attachment.r2Key) {
      const parts = attachment.r2Key.split('/');
      const last = parts[parts.length - 1];
      if (last && !last.startsWith('enc-') && !last.startsWith('blob-')) return last;
    }
    const ext = mime === 'image' ? 'png' : mime === 'video' ? 'mp4' : mime === 'audio' ? 'mp3' : 'bin';
    return `dosya-${attachment.fileId.slice(0, 8)}.${ext}`;
  });

  onMount(() => {
    let cancelled = false;
    fileApi.getDataUrl(attachment.fileId)
      .then((url) => {
        if (!cancelled) {
          dataUrl = url;
          loading = false;
        }
      })
      .catch(() => {
        if (!cancelled) {
          error = true;
          loading = false;
        }
      });

    return () => { cancelled = true; };
  });

  async function handleDownload() {
    if (downloading) return;
    const dest = await save({
      title: 'Dosyayı kaydet',
      defaultPath: displayFileName,
      filters: [{ name: 'Tüm dosyalar', extensions: ['*'] }],
    });
    if (!dest) return;
    downloading = true;
    try {
      await fileApi.download({ fileId: attachment.fileId, destinationPath: dest });
      toastStore.success('Dosya başarıyla indirildi.');
    } catch {
      toastStore.error('Dosya indirilemedi.');
    } finally {
      downloading = false;
    }
  }
</script>

<div class="veil-media-attachment">
  {#if loading}
    <div class="veil-media-skeleton">
      <div class="veil-spinner veil-spinner-sm"></div>
      <span>Dosya şifresi çözülüyor…</span>
    </div>
  {:else if error || !dataUrl}
    <div class="veil-file-card">
      <div class="veil-file-icon">
        <Icon name="lock" size={20} />
      </div>
      <div class="veil-file-info">
        <span class="veil-file-name">{displayFileName}</span>
        <span class="veil-file-size">{formatBytes(attachment.sizeBytes)}</span>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={handleDownload} disabled={downloading}>
        <Icon name="download" size={14} />
        {downloading ? 'İndiriliyor…' : 'İndir'}
      </button>
    </div>
  {:else if mime === 'image'}
    <div class="veil-image-wrap">
      <button
        type="button"
        class="veil-image-btn"
        onclick={() => (lightboxOpen = true)}
        title="Tam boyutta görüntüle"
      >
        <img src={dataUrl} alt={displayFileName} class="veil-inline-image" loading="lazy" />
      </button>
      <button
        type="button"
        class="veil-media-dl-btn"
        onclick={handleDownload}
        title="Görseli indir"
      >
        <Icon name="download" size={14} />
      </button>
    </div>
  {:else if mime === 'video'}
    <div class="veil-video-wrap">
      <!-- svelte-ignore a11y_media_has_caption -->
      <video src={dataUrl} controls class="veil-inline-video" preload="metadata"></video>
      <button
        type="button"
        class="veil-media-dl-btn"
        onclick={handleDownload}
        title="Videoyu indir"
      >
        <Icon name="download" size={14} />
      </button>
    </div>
  {:else if mime === 'audio'}
    <div class="veil-audio-wrap">
      <audio src={dataUrl} controls class="veil-inline-audio" preload="metadata"></audio>
      <button class="btn btn-ghost btn-sm" onclick={handleDownload} title="Sesi indir">
        <Icon name="download" size={14} />
      </button>
    </div>
  {:else}
    <div class="veil-file-card">
      <div class="veil-file-icon">
        <Icon name="file" size={20} />
      </div>
      <div class="veil-file-info">
        <span class="veil-file-name">{displayFileName}</span>
        <span class="veil-file-size">{formatBytes(attachment.sizeBytes)}</span>
      </div>
      <button class="btn btn-secondary btn-sm" onclick={handleDownload} disabled={downloading}>
        <Icon name="download" size={14} />
        {downloading ? 'İndiriliyor…' : 'İndir'}
      </button>
    </div>
  {/if}
</div>

{#if lightboxOpen && dataUrl}
  <LightboxModal
    open={lightboxOpen}
    src={dataUrl}
    alt={displayFileName}
    onClose={() => (lightboxOpen = false)}
  />
{/if}

<style>
  .veil-media-attachment {
    margin-top: var(--space-2);
    display: inline-block;
    max-width: 100%;
  }
  .veil-media-skeleton {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-image-wrap {
    position: relative;
    display: inline-block;
    max-width: 480px;
    max-height: 380px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
  }
  .veil-image-btn {
    display: block;
    padding: 0;
    margin: 0;
    border: none;
    background: none;
    cursor: zoom-in;
    max-width: 100%;
  }
  .veil-inline-image {
    max-width: 100%;
    max-height: 380px;
    object-fit: contain;
    display: block;
  }
  .veil-media-dl-btn {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    background: hsl(220 20% 10% / 0.75);
    border: 1px solid var(--veil-border);
    color: #fff;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--t-fast);
  }
  .veil-image-wrap:hover .veil-media-dl-btn,
  .veil-video-wrap:hover .veil-media-dl-btn {
    opacity: 1;
  }
  .veil-media-dl-btn:hover {
    background: hsl(220 20% 16% / 0.95);
  }
  .veil-video-wrap {
    position: relative;
    display: inline-block;
    max-width: 520px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: #000;
    border: 1px solid var(--veil-border-subtle);
  }
  .veil-inline-video {
    width: 100%;
    max-height: 360px;
    display: block;
  }
  .veil-audio-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    max-width: 360px;
  }
  .veil-inline-audio {
    flex: 1;
    height: 36px;
  }
  .veil-file-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    min-width: 260px;
    max-width: 420px;
  }
  .veil-file-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: var(--veil-brand-subtle);
    color: var(--veil-brand);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .veil-file-info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }
  .veil-file-name {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-file-size {
    font-size: 11px;
    color: var(--veil-text-muted);
  }
</style>

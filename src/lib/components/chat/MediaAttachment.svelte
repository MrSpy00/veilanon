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
  let blobUrl = $state<string | null>(null);
  let loading = $state(true);
  let error = $state(false);
  let lightboxOpen = $state(false);
  let downloading = $state(false);

  // Audio player state
  let audioEl = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let playbackRate = $state(1);

  function formatBytes(bytes: number): string {
    if (!bytes) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDuration(secs: number): string {
    if (!Number.isFinite(secs) || secs < 0) return '0:00';
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? '0' : ''}${s}`;
  }

  const mime = $derived.by(() => {
    const fn = (attachment.fileName || '').toLowerCase();
    const hint = (attachment.mimeTypeHint || '').toLowerCase();
    if (fn.includes('ses-kaydi') || fn.includes('ses') || fn.includes('voice') || fn.includes('audio')) return 'audio';
    if (dataUrl?.startsWith('data:audio/')) return 'audio';
    if (hint.startsWith('audio/') || hint.includes('opus') || hint.includes('weba') || hint.includes('voice')) return 'audio';
    if (fn.endsWith('.mp3') || fn.endsWith('.wav') || fn.endsWith('.ogg') || fn.endsWith('.m4a') || fn.endsWith('.flac') || fn.endsWith('.weba') || fn.endsWith('.aac') || (fn.endsWith('.webm') && !hint.startsWith('video/'))) return 'audio';
    if (dataUrl?.startsWith('data:image/')) return 'image';
    if (dataUrl?.startsWith('data:video/')) return 'video';
    if (hint.startsWith('image/')) return 'image';
    if (hint.startsWith('video/')) return 'video';
    if (fn.endsWith('.png') || fn.endsWith('.jpg') || fn.endsWith('.jpeg') || fn.endsWith('.gif') || fn.endsWith('.webp') || fn.endsWith('.bmp') || fn.endsWith('.svg')) return 'image';
    if (fn.endsWith('.mp4') || fn.endsWith('.mov') || fn.endsWith('.mkv') || fn.endsWith('.avi')) return 'video';
    return 'file';
  });

  const mediaSourceUrl = $derived(blobUrl || dataUrl);

  let calculatedWaveform = $state<number[] | null>(null);

  const waveformBars = $derived.by(() => {
    if (calculatedWaveform && calculatedWaveform.length > 0) {
      return calculatedWaveform;
    }
    const bars: number[] = [];
    const seed = attachment.fileId || 'veilanon';
    for (let i = 0; i < 32; i++) {
      const code = seed.charCodeAt(i % seed.length) + i * 17;
      const height = 20 + (code % 75);
      bars.push(height);
    }
    return bars;
  });

  const displayFileName = $derived.by(() => {
    if (attachment.fileName) return attachment.fileName;
    if (attachment.r2Key) {
      const parts = attachment.r2Key.split('/');
      const last = parts[parts.length - 1];
      if (last && !last.startsWith('enc-') && !last.startsWith('blob-')) return last;
    }
    const ext = mime === 'image' ? 'png' : mime === 'video' ? 'mp4' : mime === 'audio' ? 'webm' : 'bin';
    return `dosya-${attachment.fileId.slice(0, 8)}.${ext}`;
  });

  function getEffectiveMimeType(dataUrlMime: string): string {
    if (dataUrlMime && dataUrlMime !== 'application/octet-stream' && dataUrlMime !== 'binary/octet-stream') {
      return dataUrlMime;
    }
    const fn = (attachment.fileName || '').toLowerCase();
    const hint = (attachment.mimeTypeHint || '').toLowerCase();
    if (hint && hint !== 'application/octet-stream' && hint !== 'binary/octet-stream') return hint;
    if (fn.endsWith('.webm')) return mime === 'video' ? 'video/webm' : 'audio/webm;codecs=opus';
    if (fn.endsWith('.mp3')) return 'audio/mpeg';
    if (fn.endsWith('.wav')) return 'audio/wav';
    if (fn.endsWith('.ogg') || fn.endsWith('.opus')) return 'audio/ogg';
    if (fn.endsWith('.m4a') || fn.endsWith('.aac')) return 'audio/mp4';
    if (fn.endsWith('.flac')) return 'audio/flac';
    if (fn.endsWith('.mp4')) return 'video/mp4';
    if (fn.endsWith('.mov')) return 'video/quicktime';
    if (fn.endsWith('.png')) return 'image/png';
    if (fn.endsWith('.jpg') || fn.endsWith('.jpeg')) return 'image/jpeg';
    if (fn.endsWith('.gif')) return 'image/gif';
    if (fn.endsWith('.webp')) return 'image/webp';
    if (mime === 'audio') return 'audio/webm;codecs=opus';
    if (mime === 'video') return 'video/mp4';
    if (mime === 'image') return 'image/png';
    return dataUrlMime || 'application/octet-stream';
  }

  function createBlobFromDataUrl(url: string): Blob | null {
    try {
      const parts = url.split(',');
      if (parts.length < 2) return null;
      const match = parts[0].match(/:(.*?);/);
      const rawMime = match ? match[1] : 'application/octet-stream';
      const effectiveMime = getEffectiveMimeType(rawMime);
      const binary = atob(parts[1]);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
      }
      return new Blob([bytes], { type: effectiveMime });
    } catch {
      return null;
    }
  }

  async function decodeAudioDurationAndPeaks(url: string) {
    try {
      let arrayBuf: ArrayBuffer;
      if (url.startsWith('data:')) {
        const base64 = url.split(',')[1] ?? '';
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
        arrayBuf = bytes.buffer;
      } else {
        const resp = await fetch(url);
        arrayBuf = await resp.arrayBuffer();
      }
      const audioCtx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
      const audioBuffer = await audioCtx.decodeAudioData(arrayBuf.slice(0) as ArrayBuffer);
      if (audioBuffer && audioBuffer.duration > 0 && isFinite(audioBuffer.duration)) {
        duration = audioBuffer.duration;
        const channel = audioBuffer.getChannelData(0);
        const step = Math.max(1, Math.floor(channel.length / 32));
        const bars: number[] = [];
        for (let i = 0; i < 32; i++) {
          let max = 0;
          for (let j = 0; j < step; j++) {
            const val = Math.abs(channel[i * step + j] || 0);
            if (val > max) max = val;
          }
          bars.push(Math.max(15, Math.min(100, Math.round(max * 100))));
        }
        calculatedWaveform = bars;
      }
      await audioCtx.close().catch(() => {});
    } catch {
      // Fallback: keep pseudo waveform, duration from <audio> metadata will fill
    }
  }

  onMount(() => {
    let cancelled = false;
    fileApi.getDataUrl(attachment.fileId)
      .then((url) => {
        if (!cancelled && url) {
          dataUrl = url;
          loading = false;
          // Create streamable Blob URL for all media types in WebView2 to ensure codec support
          const blob = createBlobFromDataUrl(url);
          if (blob) {
            blobUrl = URL.createObjectURL(blob);
          }
          if (mime === 'audio' || url.startsWith('data:audio/') || attachment.mimeTypeHint?.startsWith('audio/')) {
            void decodeAudioDurationAndPeaks(url);
          }
        }
      })
      .catch(() => {
        if (!cancelled) {
          error = true;
          loading = false;
        }
      });

    return () => {
      cancelled = true;
      if (blobUrl) {
        URL.revokeObjectURL(blobUrl);
        blobUrl = null;
      }
    };
  });

  function toggleAudio() {
    if (!audioEl) return;
    if (isPlaying) {
      audioEl.pause();
    } else {
      if (audioEl.ended || (duration > 0 && currentTime >= duration - 0.1)) {
        audioEl.currentTime = 0;
      }
      audioEl.play().catch(async () => {
        try {
          await audioEl?.play();
        } catch {
          // Autoplay blocked - user gesture already given, retry without load()
          setTimeout(() => audioEl?.play().catch(() => {}), 80);
        }
      });
    }
  }

  function cycleRate() {
    if (!audioEl) return;
    if (playbackRate === 1) playbackRate = 1.5;
    else if (playbackRate === 1.5) playbackRate = 2;
    else playbackRate = 1;
    audioEl.playbackRate = playbackRate;
  }

  function seekWaveform(e: MouseEvent) {
    if (!audioEl || !duration) return;
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const pos = Math.max(0, Math.min((e.clientX - rect.left) / rect.width, 1));
    audioEl.currentTime = pos * duration;
  }

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
  {:else if error || !dataUrl || !attachment.fileId || attachment.sizeBytes === 0}
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
        <img src={mediaSourceUrl} alt={displayFileName} class="veil-inline-image" loading="lazy" />
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
      <video
        src={mediaSourceUrl}
        controls
        playsinline
        class="veil-inline-video"
        preload="metadata"
      ></video>
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
    <!-- Modern Voice Message Waveform Player -->
    <div class="veil-voice-note-card" class:playing={isPlaying}>
      <audio
        bind:this={audioEl}
        src={mediaSourceUrl}
        preload="auto"
        onplay={() => (isPlaying = true)}
        onpause={() => (isPlaying = false)}
        onended={() => { isPlaying = false; currentTime = 0; }}
        ontimeupdate={() => {
          if (audioEl) {
            currentTime = audioEl.currentTime;
            if (!isFinite(duration) || duration <= 0 || audioEl.currentTime > duration) {
              if (isFinite(audioEl.duration) && audioEl.duration > 0) {
                duration = audioEl.duration;
              } else {
                duration = Math.max(duration || 0, audioEl.currentTime);
              }
            }
          }
        }}
        onloadedmetadata={() => {
          if (audioEl && isFinite(audioEl.duration) && audioEl.duration > 0) {
            duration = audioEl.duration;
          }
        }}
      ></audio>

      <button
        type="button"
        class="veil-voice-play-btn"
        onclick={toggleAudio}
        aria-label={isPlaying ? 'Durdur' : 'Oynat'}
        title={isPlaying ? 'Durdur' : 'Oynat'}
      >
        {#if isPlaying}
          <Icon name="pause" size={18} />
        {:else}
          <Icon name="play" size={18} />
        {/if}
      </button>

      <div class="veil-voice-body">
        <!-- Interactive Waveform -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="veil-voice-waveform" onclick={seekWaveform} title="İlerle">
          {#each waveformBars as barH, i}
            {@const progressPercent = duration ? (currentTime / duration) * 100 : 0}
            {@const barPercent = (i / waveformBars.length) * 100}
            <div
              class="veil-waveform-bar"
              class:passed={barPercent <= progressPercent}
              style="height: {barH}%;"
            ></div>
          {/each}
        </div>

        <div class="veil-voice-meta">
          <span class="veil-voice-time">
            {formatDuration(currentTime)} / {formatDuration(duration || 0)}
          </span>
          <div class="veil-voice-controls">
            <button
              type="button"
              class="veil-voice-rate-btn"
              onclick={cycleRate}
              title="Oynatma hızı"
            >
              {playbackRate}x
            </button>
            <button
              type="button"
              class="veil-voice-dl-btn"
              onclick={handleDownload}
              title="Sesi indir"
            >
              <Icon name="download" size={13} />
            </button>
          </div>
        </div>
      </div>
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

{#if lightboxOpen && mediaSourceUrl}
  <LightboxModal
    open={lightboxOpen}
    src={mediaSourceUrl}
    alt={displayFileName}
    onClose={() => (lightboxOpen = false)}
  />
{/if}

<style>
  .veil-media-attachment {
    margin-top: 4px;
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
    background: transparent !important;
    background-image: none !important;
    border: none !important;
    box-shadow: none !important;
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
    border-radius: var(--radius-lg);
    background: transparent !important;
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
    min-width: 280px;
    min-height: 180px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: #000;
    border: 1px solid var(--veil-border-subtle);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }
  .veil-inline-video {
    width: 100%;
    max-height: 360px;
    display: block;
    background: #000;
  }

  /* ── Dedicated Voice Message Waveform Player ── */
  .veil-voice-note-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 8px 14px;
    background: color-mix(in srgb, var(--veil-bg-elevated, #171b26) 85%, transparent);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.1));
    border-radius: var(--radius-xl, 14px);
    min-width: 290px;
    max-width: 380px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .veil-voice-note-card.playing {
    border-color: var(--veil-brand, #7c3aed);
    box-shadow: 0 0 16px color-mix(in srgb, var(--veil-brand, #7c3aed) 25%, transparent);
  }
  .veil-voice-play-btn {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: none;
    background: var(--veil-brand, #7c3aed);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    transition: transform 0.15s, filter 0.15s;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }
  .veil-voice-play-btn:hover {
    transform: scale(1.06);
    filter: brightness(1.1);
  }
  .veil-voice-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .veil-voice-waveform {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 24px;
    cursor: pointer;
    padding: 2px 0;
  }
  .veil-waveform-bar {
    flex: 1;
    min-width: 3px;
    background: rgba(255, 255, 255, 0.25);
    border-radius: 2px;
    transition: background 0.15s, transform 0.15s;
  }
  .veil-waveform-bar.passed {
    background: var(--veil-brand, #7c3aed);
  }
  .veil-voice-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    color: var(--veil-text-muted, #94a3b8);
  }
  .veil-voice-time {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 500;
  }
  .veil-voice-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .veil-voice-rate-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: var(--veil-text-secondary, #cbd5e1);
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    cursor: pointer;
    transition: background 0.15s;
  }
  .veil-voice-rate-btn:hover {
    background: var(--veil-brand-subtle, rgba(124, 58, 237, 0.2));
    color: #fff;
  }
  .veil-voice-dl-btn {
    background: transparent;
    border: none;
    color: var(--veil-text-muted, #94a3b8);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: color 0.15s;
  }
  .veil-voice-dl-btn:hover {
    color: var(--veil-text-primary, #f1f5f9);
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

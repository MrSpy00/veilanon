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

  // ── Audio player state ───────────────────────────────────────────────────
  let audioEl = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let playbackRate = $state(1);
  let audioVolume = $state(1);

  // ── Video player state ───────────────────────────────────────────────────
  let videoEl = $state<HTMLVideoElement | null>(null);
  let videoPlaying = $state(false);
  let videoCurrentTime = $state(0);
  let videoDuration = $state(0);
  let videoVolume = $state(1);
  let videoMuted = $state(false);
  let videoFullscreen = $state(false);
  let videoShowControls = $state(false);
  let videoControlsTimer: ReturnType<typeof setTimeout> | null = null;

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
    if (fn.endsWith('.mp4') || fn.endsWith('.mov') || fn.endsWith('.mkv') || fn.endsWith('.avi') || fn.endsWith('.webm')) return 'video';
    return 'file';
  });

  // File type for icon
  const fileType = $derived.by(() => {
    const fn = (attachment.fileName || '').toLowerCase();
    const hint = (attachment.mimeTypeHint || '').toLowerCase();
    if (fn.endsWith('.pdf') || hint.includes('pdf')) return 'pdf';
    if (fn.endsWith('.zip') || fn.endsWith('.rar') || fn.endsWith('.7z') || fn.endsWith('.tar') || fn.endsWith('.gz') || hint.includes('zip') || hint.includes('compressed')) return 'archive';
    if (fn.endsWith('.doc') || fn.endsWith('.docx') || hint.includes('word')) return 'doc';
    if (fn.endsWith('.xls') || fn.endsWith('.xlsx') || hint.includes('excel') || hint.includes('spreadsheet')) return 'spreadsheet';
    if (fn.endsWith('.ppt') || fn.endsWith('.pptx') || hint.includes('powerpoint') || hint.includes('presentation')) return 'presentation';
    if (fn.endsWith('.txt') || fn.endsWith('.md') || fn.endsWith('.log') || hint.includes('text/plain')) return 'text';
    if (fn.endsWith('.json') || fn.endsWith('.xml') || fn.endsWith('.yaml') || fn.endsWith('.yml') || fn.endsWith('.toml') || fn.endsWith('.ini') || fn.endsWith('.env')) return 'code';
    if (fn.endsWith('.js') || fn.endsWith('.ts') || fn.endsWith('.jsx') || fn.endsWith('.tsx') || fn.endsWith('.py') || fn.endsWith('.rs') || fn.endsWith('.go') || fn.endsWith('.java') || fn.endsWith('.cpp') || fn.endsWith('.c') || fn.endsWith('.cs') || fn.endsWith('.html') || fn.endsWith('.css') || fn.endsWith('.svelte') || fn.endsWith('.vue')) return 'code';
    if (fn.endsWith('.exe') || fn.endsWith('.msi') || fn.endsWith('.dmg') || fn.endsWith('.deb') || fn.endsWith('.rpm') || fn.endsWith('.appimage') || fn.endsWith('.apk')) return 'executable';
    return 'generic';
  });

  // File type icon and color
  const fileTypeConfig = $derived.by(() => {
    switch (fileType) {
      case 'pdf': return { icon: 'file' as const, color: '#ef4444', label: 'PDF' };
      case 'archive': return { icon: 'file' as const, color: '#f59e0b', label: 'Arşiv' };
      case 'doc': return { icon: 'file' as const, color: '#3b82f6', label: 'Belge' };
      case 'spreadsheet': return { icon: 'file' as const, color: '#22c55e', label: 'Tablo' };
      case 'presentation': return { icon: 'file' as const, color: '#f97316', label: 'Sunu' };
      case 'text': return { icon: 'file' as const, color: '#94a3b8', label: 'Metin' };
      case 'code': return { icon: 'file' as const, color: '#a78bfa', label: 'Kod' };
      case 'executable': return { icon: 'file' as const, color: '#ef4444', label: 'Uygulama' };
      default: return { icon: 'file' as const, color: '#64748b', label: 'Dosya' };
    }
  });

  const mediaSourceUrl = $derived(blobUrl || dataUrl);

  let calculatedWaveform = $state<number[] | null>(null);

  const waveformBars = $derived.by(() => {
    if (calculatedWaveform && calculatedWaveform.length > 0) {
      return calculatedWaveform;
    }
    const bars: number[] = [];
    const seed = attachment.fileId || 'veilanon';
    for (let i = 0; i < 40; i++) {
      const code = seed.charCodeAt(i % seed.length) + i * 17;
      const height = 15 + (code % 80);
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
        const step = Math.max(1, Math.floor(channel.length / 40));
        const bars: number[] = [];
        for (let i = 0; i < 40; i++) {
          let max = 0;
          for (let j = 0; j < step; j++) {
            const val = Math.abs(channel[i * step + j] || 0);
            if (val > max) max = val;
          }
          bars.push(Math.max(8, Math.min(100, Math.round(max * 100))));
        }
        calculatedWaveform = bars;
      }
      await audioCtx.close().catch(() => {});
    } catch {
      // Fallback: keep pseudo waveform
    }
  }

  onMount(() => {
    let cancelled = false;
    fileApi.getDataUrl(attachment.fileId)
      .then((url) => {
        if (!cancelled && url) {
          dataUrl = url;
          loading = false;
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

  // ── Audio controls ───────────────────────────────────────────────────────
  function toggleAudio() {
    if (!audioEl) return;
    if (isPlaying) {
      audioEl.pause();
      isPlaying = false;
    } else {
      const effDur = isFinite(duration) && duration > 0 ? duration : (isFinite(audioEl.duration) ? audioEl.duration : 0);
      if (audioEl.ended || (effDur > 0 && currentTime >= effDur - 0.15)) {
        audioEl.currentTime = 0;
        currentTime = 0;
      }
      audioEl.play().then(() => {
        isPlaying = true;
      }).catch(async () => {
        try {
          await audioEl?.play();
          isPlaying = true;
        } catch {
          setTimeout(() => {
            audioEl?.play().then(() => { isPlaying = true; }).catch(() => {});
          }, 80);
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
    if (!audioEl) return;
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const pos = Math.max(0, Math.min((e.clientX - rect.left) / rect.width, 1));
    const targetDur = isFinite(duration) && duration > 0 ? duration : (isFinite(audioEl.duration) && audioEl.duration > 0 ? audioEl.duration : 0);
    if (targetDur > 0) {
      audioEl.currentTime = pos * targetDur;
      currentTime = pos * targetDur;
    }
  }

  // ── Video controls ───────────────────────────────────────────────────────
  function toggleVideo() {
    if (!videoEl) return;
    if (videoPlaying) {
      videoEl.pause();
    } else {
      videoEl.play().catch(() => {});
    }
  }

  function seekVideo(e: MouseEvent) {
    if (!videoEl || !videoDuration) return;
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const pos = Math.max(0, Math.min((e.clientX - rect.left) / rect.width, 1));
    videoEl.currentTime = pos * videoDuration;
  }

  function toggleVideoMute() {
    if (!videoEl) return;
    videoMuted = !videoMuted;
    videoEl.muted = videoMuted;
  }

  function toggleVideoVolume() {
    if (!videoEl) return;
    if (videoVolume > 0) {
      videoVolume = 0;
      videoEl.volume = 0;
      videoMuted = true;
      videoEl.muted = true;
    } else {
      videoVolume = 1;
      videoEl.volume = 1;
      videoMuted = false;
      videoEl.muted = false;
    }
  }

  let fullscreenWrapEl: HTMLElement | null = null;

  function toggleFullscreen() {
    if (!fullscreenWrapEl) {
      fullscreenWrapEl = document.querySelector('.veil-video-custom-wrap');
    }
    const wrap = fullscreenWrapEl;
    if (!wrap) return;
    if (!document.fullscreenElement) {
      wrap.requestFullscreen().then(() => {
        videoFullscreen = true;
      }).catch(() => {});
    } else {
      document.exitFullscreen().then(() => {
        videoFullscreen = false;
      }).catch(() => {});
    }
  }

  function onDoubleClickFullscreen(e: MouseEvent) {
    e.preventDefault();
    toggleFullscreen();
  }

  function showVideoControls() {
    videoShowControls = true;
    if (videoControlsTimer) clearTimeout(videoControlsTimer);
    videoControlsTimer = setTimeout(() => {
      if (videoPlaying) videoShowControls = false;
    }, 2500);
  }

  // ── Download ─────────────────────────────────────────────────────────────
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
    <!-- Error / download fallback card -->
    <div class="veil-file-card">
      <div class="veil-file-icon" style="color: {fileTypeConfig.color};">
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
    <!-- ── Image ────────────────────────────────────────────────────────── -->
    <div class="veil-image-wrap">
      <button
        type="button"
        class="veil-image-btn"
        onclick={() => (lightboxOpen = true)}
        title="Tam boyutta görüntüle"
      >
        <img src={mediaSourceUrl} alt={displayFileName} class="veil-inline-image" loading="lazy" />
        <div class="veil-image-overlay">
          <span class="veil-image-overlay-icon"><Icon name="screen" size={20} /></span>
        </div>
      </button>
      <button
        type="button"
        class="veil-media-dl-btn"
        onclick={handleDownload}
        title="Görseli indir"
        disabled={downloading}
      >
        {#if downloading}
          <div class="veil-spinner" style="width:12px;height:12px;border-width:1.5px;"></div>
        {:else}
          <Icon name="download" size={14} />
        {/if}
      </button>
    </div>

  {:else if mime === 'video'}
    <div
      class="veil-video-custom-wrap"
      class:controls-visible={videoShowControls || !videoPlaying}
      class:fullscreen={videoFullscreen}
      onmousemove={showVideoControls}
      onmouseenter={showVideoControls}
      ondblclick={onDoubleClickFullscreen}
      onfullscreenchange={() => { videoFullscreen = !!document.fullscreenElement; }}
    >
      <video
        bind:this={videoEl}
        src={mediaSourceUrl}
        class="veil-inline-video"
        preload="metadata"
        muted={videoMuted}
        onplay={() => { videoPlaying = true; }}
        onpause={() => { videoPlaying = false; }}
        onended={() => { videoPlaying = false; videoCurrentTime = 0; }}
        ontimeupdate={() => { if (videoEl) videoCurrentTime = videoEl.currentTime; }}
        onloadedmetadata={() => { if (videoEl) videoDuration = videoEl.duration; }}
        onclick={toggleVideo}
      ></video>

      {#if !videoPlaying}
        <div class="veil-video-center-play" onclick={toggleVideo} role="button" tabindex="0">
          <Icon name="play" size={32} />
        </div>
      {/if}

      <div class="veil-video-controls">
        <div
          class="veil-video-progress"
          onclick={seekVideo}
          role="slider"
          aria-label="Video konumu"
          aria-valuemin={0}
          aria-valuemax={videoDuration || 100}
          aria-valuenow={videoCurrentTime}
          tabindex="0"
          onkeydown={(e) => {
            if (!videoEl || !videoDuration) return;
            if (e.key === 'ArrowRight') videoEl.currentTime = Math.min(videoDuration, videoCurrentTime + 5);
            if (e.key === 'ArrowLeft') videoEl.currentTime = Math.max(0, videoCurrentTime - 5);
          }}
        >
          <div class="veil-video-progress-track">
            <div
              class="veil-video-progress-fill"
              style="width: {videoDuration ? (videoCurrentTime / videoDuration) * 100 : 0}%"
            ></div>
            <div
              class="veil-video-progress-thumb"
              style="left: {videoDuration ? (videoCurrentTime / videoDuration) * 100 : 0}%"
            ></div>
          </div>
        </div>

        <div class="veil-video-controls-row">
          <button type="button" class="veil-vc-btn" onclick={toggleVideo} title={videoPlaying ? 'Durdur' : 'Oynat'}>
            <Icon name={videoPlaying ? 'pause' : 'play'} size={16} />
          </button>
          <span class="veil-video-time">
            {formatDuration(videoCurrentTime)} / {formatDuration(videoDuration)}
          </span>
          <div class="veil-vc-spacer"></div>
          <button type="button" class="veil-vc-btn" onclick={toggleVideoVolume} title={videoMuted ? 'Sesi aç' : 'Sessizleştir'}>
            <Icon name={videoMuted ? 'volume-x' : 'volume'} size={15} />
          </button>
          <button type="button" class="veil-vc-btn" onclick={handleDownload} title="İndir" disabled={downloading}>
            <Icon name="download" size={15} />
          </button>
          <button type="button" class="veil-vc-btn" onclick={toggleFullscreen} title={videoFullscreen ? 'Küçült' : 'Tam ekran'}>
            <Icon name={videoFullscreen ? 'minimize-2' : 'maximize-2'} size={15} />
          </button>
        </div>
      </div>
    </div>

  {:else if mime === 'audio'}
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
        ondurationchange={() => {
          if (audioEl && isFinite(audioEl.duration) && audioEl.duration > 0) {
            duration = audioEl.duration;
          }
        }}
      ></audio>

      <button
        type="button"
        class="veil-voice-play-btn"
        class:playing={isPlaying}
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
              class:current={Math.abs(barPercent - progressPercent) < (100 / waveformBars.length)}
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
              {playbackRate}×
            </button>
            <button
              type="button"
              class="veil-voice-dl-btn"
              onclick={handleDownload}
              title="Sesi indir"
              disabled={downloading}
            >
              <Icon name="download" size={13} />
            </button>
          </div>
        </div>
      </div>
    </div>

  {:else}
    <!-- ── Generic File Card ─────────────────────────────────────────────── -->
    <div class="veil-file-card">
      <div class="veil-file-icon" style="background: color-mix(in srgb, {fileTypeConfig.color} 15%, transparent); color: {fileTypeConfig.color}; border: 1px solid color-mix(in srgb, {fileTypeConfig.color} 25%, transparent);">
        <Icon name={fileTypeConfig.icon} size={22} />
      </div>
      <div class="veil-file-info">
        <span class="veil-file-name" title={displayFileName}>{displayFileName}</span>
        <div class="veil-file-meta">
          <span class="veil-file-type-badge" style="color: {fileTypeConfig.color};">{fileTypeConfig.label}</span>
          <span class="veil-file-size">{formatBytes(attachment.sizeBytes)}</span>
        </div>
      </div>
      <button class="veil-file-dl-btn" onclick={handleDownload} disabled={downloading} title="İndir">
        {#if downloading}
          <div class="veil-spinner" style="width:14px;height:14px;border-width:2px;"></div>
        {:else}
          <Icon name="download" size={16} />
        {/if}
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

  /* ── Loading skeleton ─────────────────────────────────────────────────── */
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

  /* ── Image ─────────────────────────────────────────────────────────────── */
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
    position: relative;
  }
  .veil-inline-image {
    max-width: 100%;
    max-height: 380px;
    object-fit: contain;
    display: block;
    border-radius: var(--radius-lg);
    background: transparent !important;
    transition: filter 0.2s;
  }
  .veil-image-btn:hover .veil-inline-image {
    filter: brightness(0.88);
  }
  .veil-image-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.18s;
    background: rgba(0, 0, 0, 0.18);
    border-radius: var(--radius-lg);
    pointer-events: none;
  }
  .veil-image-btn:hover .veil-image-overlay { opacity: 1; }
  .veil-image-overlay-icon {
    background: hsl(220 20% 8% / 0.7);
    border-radius: var(--radius-full);
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    backdrop-filter: blur(4px);
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
  .veil-image-wrap:hover .veil-media-dl-btn { opacity: 1; }
  .veil-media-dl-btn:hover {
    background: hsl(220 20% 16% / 0.95);
  }

  /* ── Custom Video Player ──────────────────────────────────────────────── */
  .veil-video-custom-wrap {
    position: relative;
    display: inline-flex;
    flex-direction: column;
    max-width: 520px;
    min-width: 280px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: #000;
    border: 1px solid var(--veil-border-subtle);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
    cursor: pointer;
    user-select: none;
  }
  .veil-inline-video {
    width: 100%;
    max-height: 360px;
    display: block;
    background: #000;
    object-fit: contain;
  }
  .veil-video-center-play {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(6px);
    border: 2px solid rgba(255, 255, 255, 0.4);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: transform 0.15s, background 0.15s;
    z-index: 2;
  }
  .veil-video-center-play:hover {
    transform: translate(-50%, -50%) scale(1.08);
    background: rgba(0, 0, 0, 0.8);
  }
  .veil-video-controls {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: linear-gradient(to top, rgba(0,0,0,0.85) 0%, transparent 100%);
    padding: 20px 10px 8px;
    opacity: 0;
    transition: opacity 0.22s;
    z-index: 3;
  }
  .veil-video-custom-wrap.controls-visible .veil-video-controls,
  .veil-video-custom-wrap:hover .veil-video-controls {
    opacity: 1;
  }
  .veil-video-progress {
    position: relative;
    cursor: pointer;
    padding: 6px 0;
    margin-bottom: 4px;
  }
  .veil-video-progress-track {
    height: 3px;
    background: rgba(255, 255, 255, 0.25);
    border-radius: 2px;
    position: relative;
  }
  .veil-video-progress-fill {
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    background: var(--veil-brand, #7c3aed);
    border-radius: 2px;
    transition: width 0.1s linear;
  }
  .veil-video-progress-thumb {
    position: absolute;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
    transform: translate(-50%, -50%);
    box-shadow: 0 0 4px rgba(0,0,0,0.4);
    opacity: 0;
    transition: opacity 0.15s;
  }
  .veil-video-progress:hover .veil-video-progress-thumb { opacity: 1; }
  .veil-video-progress:hover .veil-video-progress-track { height: 5px; }
  .veil-video-controls-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .veil-vc-btn {
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.85);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background 0.15s;
    flex-shrink: 0;
  }
  .veil-vc-btn:hover { color: #fff; background: rgba(255, 255, 255, 0.12); }
  .veil-vc-btn:disabled { opacity: 0.5; cursor: default; }
  .veil-video-time {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: rgba(255, 255, 255, 0.75);
    flex-shrink: 0;
  }
  .veil-vc-spacer { flex: 1; }

  /* ── Voice Note / Audio Waveform Player ──────────────────────────────── */
  .veil-voice-note-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 10px 14px;
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
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: var(--veil-brand, #7c3aed);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    transition: transform 0.15s, filter 0.15s, box-shadow 0.2s;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }
  .veil-voice-play-btn:hover {
    transform: scale(1.07);
    filter: brightness(1.12);
  }
  .veil-voice-play-btn.playing {
    box-shadow: 0 0 14px color-mix(in srgb, var(--veil-brand, #7c3aed) 50%, transparent);
  }
  .veil-voice-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .veil-voice-waveform {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 28px;
    cursor: pointer;
    padding: 2px 0;
  }
  .veil-waveform-bar {
    flex: 1;
    min-width: 2px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 2px;
    transition: background 0.12s, transform 0.12s;
  }
  .veil-waveform-bar.passed {
    background: var(--veil-brand, #7c3aed);
  }
  .veil-waveform-bar.current {
    background: color-mix(in srgb, var(--veil-brand, #7c3aed) 70%, white);
    transform: scaleY(1.1);
  }
  .veil-voice-waveform:hover .veil-waveform-bar { opacity: 0.9; }
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
  .veil-voice-dl-btn:hover { color: var(--veil-text-primary, #f1f5f9); }

  /* ── Generic File Card ──────────────────────────────────────────────────── */
  .veil-file-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    min-width: 260px;
    max-width: 420px;
    transition: border-color var(--t-fast), box-shadow var(--t-fast);
  }
  .veil-file-card:hover {
    border-color: var(--veil-border);
    box-shadow: var(--shadow-sm);
  }
  .veil-file-icon {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-lg);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: transform 0.15s;
  }
  .veil-file-card:hover .veil-file-icon { transform: scale(1.05); }
  .veil-file-info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
    gap: 2px;
  }
  .veil-file-name {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-file-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .veil-file-type-badge {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .veil-file-size {
    font-size: 11px;
    color: var(--veil-text-muted);
  }
  .veil-file-dl-btn {
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border);
    color: var(--veil-text-muted);
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    transition: background var(--t-fast), color var(--t-fast), border-color var(--t-fast);
  }
  .veil-file-dl-btn:hover {
    background: var(--veil-brand-subtle);
    border-color: var(--veil-brand);
    color: var(--veil-brand);
  }
  .veil-file-dl-btn:disabled { opacity: 0.5; cursor: default; }
</style>

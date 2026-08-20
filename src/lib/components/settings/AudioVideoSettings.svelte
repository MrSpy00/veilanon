<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Toggle from '../ui/Toggle.svelte';
  import VeilSelect from '../ui/VeilSelect.svelte';
  import Icon from '../ui/Icon.svelte';
  import { settingsApi, voiceApi, type AppSettings } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import { mediaStore } from '$lib/stores/media';

  let settings = $state<AppSettings | null>(null);
  let loading = $state(true);
  let loadingDevices = $state(true);

  let audioDevices = $state<MediaDeviceInfo[]>([]);
  let outputDevices = $state<MediaDeviceInfo[]>([]);
  let videoDevices = $state<MediaDeviceInfo[]>([]);

  let testing = $state(false);
  let testResult = $state<'idle' | 'ok' | 'fail'>('idle');
  let micLevel = $state(0);
  let testStream: MediaStream | null = null;
  let testRaf: number | null = null;

  function deviceOptions(devices: MediaDeviceInfo[], kind: string) {
    return [
      { value: '', label: 'Varsayılan' },
      ...devices.map((d, i) => ({
        value: d.deviceId,
        label: (d.label && d.label.trim()) ? d.label : `${kind} ${i + 1}`,
      })),
    ];
  }

  async function loadSettings() {
    try {
      settings = await settingsApi.get();
    } catch {
      toastStore.error('Ses & görüntü ayarları yüklenemedi.');
    } finally {
      loading = false;
    }
  }

  async function loadDevices() {
    try {
      const devices = await navigator.mediaDevices.enumerateDevices();
      audioDevices = devices.filter((d) => d.kind === 'audioinput');
      outputDevices = devices.filter((d) => d.kind === 'audiooutput');
      videoDevices = devices.filter((d) => d.kind === 'videoinput');
    } catch {
      toastStore.info('Cihaz listesi alınamadı.');
    } finally {
      loadingDevices = false;
    }
  }

  onMount(() => {
    void loadSettings();
    void loadDevices();
  });

  async function save(patch: Partial<AppSettings>) {
    if (!settings) return;
    const next = { ...settings, ...patch };
    try {
      settings = await settingsApi.update(next);
      return true;
    } catch {
      settings = next;
      toastStore.error('Ayarlar kaydedilemedi.');
      return false;
    }
  }

  async function setAudio(deviceId: string) {
    const nextId = deviceId || null;
    const ok = await save({ inputDeviceId: nextId });
    if (ok) {
      try {
        await voiceApi.setAudioDevice({ deviceId: nextId, deviceType: 'input' });
        if (nextId) void mediaStore.switchActiveDevice('audioinput', nextId);
      } catch {
        // Best-effort.
      }
      toastStore.success('Mikrofon seçildi.');
    }
  }

  async function setOutput(deviceId: string) {
    const nextId = deviceId || null;
    const ok = await save({ outputDeviceId: nextId });
    if (ok) {
      try {
        await voiceApi.setAudioDevice({ deviceId: nextId, deviceType: 'output' });
        if (nextId) void mediaStore.switchActiveDevice('audiooutput', nextId);
      } catch {
        // Best-effort.
      }
      toastStore.success('Hoparlör seçildi.');
    }
  }

  async function setVideo(deviceId: string) {
    const nextId = deviceId || null;
    const ok = await save({ videoDeviceId: nextId });
    if (ok) {
      try {
        await voiceApi.setVideoDevice({ deviceId: nextId });
        if (nextId) void mediaStore.switchActiveDevice('videoinput', nextId);
      } catch {
        // Best-effort.
      }
      toastStore.success('Kamera seçildi.');
    }
  }

  /** Testi başlat: canlı seviye ölçümü — çubuk anlık ses şiddetini gösterir. */
  async function startMicTest() {
    if (testing) return;
    testing = true;
    testResult = 'idle';
    micLevel = 0;
    try {
      testStream = await navigator.mediaDevices.getUserMedia(
        settings?.inputDeviceId
          ? { audio: { deviceId: { exact: settings.inputDeviceId } } }
          : { audio: true }
      );
      const AC =
        window.AudioContext ??
        (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AC) {
        testResult = 'fail';
        toastStore.error('Ses motoru başlatılamadı.');
        stopMicTest();
        return;
      }
      const ctx = new AC();
      await ctx.resume();
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 512;
      ctx.createMediaStreamSource(testStream).connect(analyser);
      const data = new Uint8Array(analyser.frequencyBinCount);

      const loop = () => {
        if (!testing) return;
        analyser.getByteTimeDomainData(data);
        let max = 0;
        for (let i = 0; i < data.length; i++) {
          const v = Math.abs(data[i] - 128);
          if (v > max) max = v;
        }
        // Eşik üstü ses geldiğinde "ok" işaretini göster.
        if (max / 128 > 0.04) testResult = 'ok';
        micLevel = Math.min(1, max / 128);
        testRaf = requestAnimationFrame(loop);
      };
      testRaf = requestAnimationFrame(loop);
    } catch {
      testResult = 'fail';
      testing = false;
      toastStore.error('Mikrofon açılamadı.');
    }
  }

  let videoTestStream: MediaStream | null = null;
  let videoPreviewEl = $state<HTMLVideoElement | null>(null);
  let testingVideo = $state(false);

  async function startVideoTest() {
    if (testingVideo) return;
    try {
      videoTestStream = await navigator.mediaDevices.getUserMedia(
        settings?.videoDeviceId
          ? { video: { deviceId: { exact: settings.videoDeviceId } } }
          : { video: true }
      );
      testingVideo = true;
      if (videoPreviewEl && videoTestStream) {
        videoPreviewEl.srcObject = videoTestStream;
        await videoPreviewEl.play().catch(() => {});
      }
    } catch {
      testingVideo = false;
      toastStore.error('Kamera açılamadı.');
    }
  }

  $effect(() => {
    if (testingVideo && videoPreviewEl && videoTestStream) {
      videoPreviewEl.srcObject = videoTestStream;
      videoPreviewEl.play().catch(() => {});
    }
  });

  function stopVideoTest() {
    testingVideo = false;
    videoTestStream?.getTracks().forEach((t) => t.stop());
    videoTestStream = null;
    if (videoPreviewEl) {
      videoPreviewEl.srcObject = null;
    }
  }

  function stopMicTest() {
    testing = false;
    if (testRaf !== null) cancelAnimationFrame(testRaf);
    testRaf = null;
    testStream?.getTracks().forEach((t) => t.stop());
    testStream = null;
    if (micLevel > 0.05) toastStore.success('Ses testi başarılı.');
  }

  onDestroy(() => {
    stopMicTest();
    stopVideoTest();
  });
</script>

<section aria-labelledby="ses-title">
  <h2 class="veil-settings-title" id="ses-title">Ses & Görüntü</h2>

  {#if loading}
    <div class="veil-spinner" style="margin:2rem auto;"></div>
  {:else if settings}
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Cihazlar</div>
      <p class="veil-settings-row-desc veil-note">
        Cihaz adları, mikrofonu bir aramada veya ses testinde ilk kez kullandığında
        sistem tarafından açıklanır — uygulama açılışında izin istemez.
      </p>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Mikrofon</div>
          <div class="veil-settings-row-desc">Sesli aramalarda kullanılacak giriş cihazı.</div>
        </div>
        <VeilSelect
          options={deviceOptions(audioDevices, 'Mikrofon')}
          value={settings.inputDeviceId ?? ''}
          label="Mikrofon"
          disabled={loadingDevices || audioDevices.length === 0}
          onChange={setAudio}
        />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Hoparlör</div>
          <div class="veil-settings-row-desc">Aramalarda sesin geldiği çıkış cihazı.</div>
        </div>
        <VeilSelect
          options={deviceOptions(outputDevices, 'Hoparlör')}
          value={settings.outputDeviceId ?? ''}
          label="Hoparlör"
          disabled={loadingDevices || outputDevices.length === 0}
          onChange={setOutput}
        />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Kamera</div>
          <div class="veil-settings-row-desc">Görüntülü aramalarda kullanılacak giriş cihazı.</div>
        </div>
        <VeilSelect
          options={deviceOptions(videoDevices, 'Kamera')}
          value={settings.videoDeviceId ?? ''}
          label="Kamera"
          disabled={loadingDevices || videoDevices.length === 0}
          onChange={setVideo}
        />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Kamerayı Aynala (Ters Çevir)</div>
          <div class="veil-settings-row-desc">Kamera görüntüsünü ayna gibi yatay çevirir. Kapalıyken düz görünür.</div>
        </div>
        <Toggle
          checked={settings.mirrorCamera ?? false}
          onChange={(v) => void save({ mirrorCamera: v })}
          label="Kamerayı Aynala"
        />
      </div>
      <div class="veil-settings-row veil-settings-row-stack">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Kamera Önizlemesi & Testi</div>
          <div class="veil-settings-row-desc">Kameranı test et ve ayna modunun görüntüsünü kontrol et.</div>
        </div>
        <div class="veil-camera-test-box">
          {#if testingVideo}
            <video
              class="veil-camera-preview-video"
              class:mirrored={settings.mirrorCamera ?? false}
              autoplay
              playsinline
              muted
              bind:this={videoPreviewEl}
            ></video>
            <button class="btn btn-secondary btn-sm veil-cam-btn" onclick={stopVideoTest}>
              <Icon name="video-off" size={14} />
              Önizlemeyi Kapat
            </button>
          {:else}
            <button class="btn btn-secondary btn-sm" onclick={startVideoTest}>
              <Icon name="video" size={14} />
              Kamerayı Test Et
            </button>
          {/if}
        </div>
      </div>
    </div>

    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Ses kalitesi</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Gürültü bastırma</div>
          <div class="veil-settings-row-desc">Arka plan gürültüsünü yumuşatır.</div>
        </div>
        <Toggle
          checked={settings.noiseSuppression ?? true}
          onChange={(v) => void save({ noiseSuppression: v })}
          label="Gürültü bastırma"
        />
      </div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Yankı iptali</div>
          <div class="veil-settings-row-desc">Hoparlör yankısını otomatik engeller.</div>
        </div>
        <Toggle
          checked={settings.echoCancellation ?? true}
          onChange={(v) => void save({ echoCancellation: v })}
          label="Yankı iptali"
        />
      </div>
    </div>

    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Ses Efektleri & Bildirim Sesleri</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Ses Kanalı Bağlantı / Ayrılış Efektleri</div>
          <div class="veil-settings-row-desc">Bir ses odasına katıldığınızda veya ayrıldığınızda ses efekti çal.</div>
        </div>
        <Toggle
          checked={settings.soundCalls ?? true}
          onChange={(v) => void save({ soundCalls: v })}
          label="Ses kanalı bağlantı sesleri"
        />
      </div>
    </div>

    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Konuşma modu</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Bas-konuş (Push-to-talk)</div>
          <div class="veil-settings-row-desc">Mikrofon yalnızca tuşa basılıyken açık olsun.</div>
        </div>
        <Toggle
          checked={settings.pushToTalk ?? false}
          onChange={(v) => void save({ pushToTalk: v })}
          label="Bas-konuş"
        />
      </div>
      {#if settings.pushToTalk}
        <div class="veil-settings-row">
          <div class="veil-settings-row-info">
            <div class="veil-settings-row-label">Kısayol tuşu</div>
            <div class="veil-settings-row-desc">
              Mikrofonu açacak tuşu seçmek için kutuya tıkla ve tuşa bas.
            </div>
          </div>
          <input
            class="veil-input veil-ptt-key"
            value={settings.pushToTalkKey ?? ''}
            maxlength={1}
            aria-label="Kısayol tuşu"
            placeholder="Tuşa bas…"
            onkeydown={(e) => {
              e.preventDefault();
              const k = e.key.length === 1 ? e.key.toUpperCase() : '';
              if (k && settings && settings.pushToTalkKey !== k) {
                settings = { ...settings, pushToTalkKey: k };
                void save({ pushToTalkKey: k });
                toastStore.success(`Bas-konuş tuşu: ${k}`);
              }
            }}
          />
        </div>
      {/if}
    </div>

    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Ses testi</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Mikrofon testi</div>
          <div class="veil-settings-row-desc">
            Konuş — çubuk anlık ses seviyeni gösterir.
          </div>
        </div>
        <button class="btn btn-secondary" onclick={testing ? stopMicTest : startMicTest}>
          {#if testing}
            <Icon name="x" size={16} />
            Testi Durdur
          {:else}
            <Icon name="mic" size={16} />
            Test et
          {/if}
        </button>
      </div>

      <div class="veil-mic-meter" role="meter" aria-label="Ses seviyesi" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(micLevel * 100)}>
        <div class="veil-mic-meter-track">
          <div
            class="veil-mic-meter-fill"
            class:live={testing}
            style={`width: ${Math.max(3, micLevel * 100)}%; --level: ${micLevel};`}
          ></div>
        </div>
        <span class="veil-mic-meter-pct">{Math.round(micLevel * 100)}%</span>
      </div>

      {#if testResult === 'fail'}
        <p class="veil-test-result fail">
          <Icon name="x" size={14} />
          Mikrofon açılamadı — izinleri ve cihaz seçimini kontrol et.
        </p>
      {:else if testing}
        <p class="veil-test-result ok">
          <Icon name="mic" size={14} />
          Konuşmaya başla…
        </p>
      {/if}
    </div>

    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Ekran Paylaşımı & Yayın</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Varsayılan Yayın Kalitesi</div>
          <div class="veil-settings-row-desc">Ekran paylaşımı başlatıldığında varsayılan profil (1080p 60fps akıcı önerilir).</div>
        </div>
        <VeilSelect
          options={[
            { value: '1080p60', label: '1080p · 60 FPS (Yüksek Kalite / Akıcı)' },
            { value: '1080p30', label: '1080p · 30 FPS (Full HD Standart)' },
            { value: '720p60', label: '720p · 60 FPS (Akıcı HD)' },
            { value: '720p30', label: '720p · 30 FPS (Dengeli)' },
            { value: '480p30', label: '480p · 30 FPS (Düşük Bant Genişliği)' },
          ]}
          value={'1080p60'}
          label="Yayın Kalitesi"
          onChange={() => toastStore.success('Varsayılan yayın kalitesi 1080p 60 FPS olarak ayarlandı.')}
        />
      </div>
    </div>

    <p class="veil-settings-row-desc veil-note">
      Cihaz ve ses tercihleri uygulama ayarlarına kaydedilir.
    </p>
  {/if}
</section>

<style>
  .veil-ptt-key {
    width: 96px;
    text-align: center;
    font-family: var(--font-mono);
    font-weight: 700;
    text-transform: uppercase;
  }
  .veil-note { margin-top: var(--space-2); }
  .veil-test-result {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    font-size: var(--text-sm);
    font-weight: 600;
  }
  .veil-test-result.ok { color: var(--veil-success); }
  .veil-test-result.fail { color: var(--veil-danger); }

  .veil-mic-meter {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    margin-top: var(--space-3);
  }
  .veil-mic-meter-track {
    flex: 1;
    height: 12px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    overflow: hidden;
    position: relative;
  }
  .veil-mic-meter-fill {
    height: 100%;
    border-radius: var(--radius-full);
    background: linear-gradient(90deg, var(--veil-success), var(--veil-warning), var(--veil-danger));
    background-size: 200% 100%;
    background-position: calc(100% - var(--level) * 100%) 0;
    opacity: 0.35;
    transition: width 90ms linear, opacity var(--t-fast);
  }
  .veil-mic-meter-fill.live { opacity: 1; }
  .veil-mic-meter-pct {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--veil-text-secondary);
    min-width: 3ch;
    text-align: right;
  }
  .veil-camera-test-box {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
  }
  .veil-camera-preview-video {
    width: 100%;
    max-width: 360px;
    aspect-ratio: 16 / 9;
    background: #000;
    border-radius: var(--radius-lg);
    border: 1px solid var(--veil-border);
    object-fit: cover;
  }
  .veil-camera-preview-video.mirrored {
    transform: scaleX(-1);
  }
</style>

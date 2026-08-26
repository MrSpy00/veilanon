<script lang="ts">
  import { onMount } from 'svelte';
  import Toggle from '../ui/Toggle.svelte';
  import VeilSelect from '../ui/VeilSelect.svelte';
  import Icon from '../ui/Icon.svelte';
  import { settingsApi, type AppSettings, type NotificationPreview } from '$lib/api/tauri';
  import { toastStore } from '$lib/stores/notifications';
  import {
    playMessageSound,
    playMentionSound,
    playFriendRequestSound,
    playCallJoinSound,
  } from '$lib/utils/sound';

  let settings = $state<AppSettings | null>(null);
  let loading = $state(true);

  const previewOptions: Array<{ value: NotificationPreview; label: string }> = [
    { value: 'full', label: 'Tam içerik (Önerilen)' },
    { value: 'sender', label: 'Yalnızca gönderen' },
    { value: 'none', label: 'Gizli (içerik yok)' },
  ];

  onMount(async () => {
    try {
      settings = await settingsApi.get();
    } catch {
      toastStore.error('Bildirim ayarları yüklenemedi.');
    } finally {
      loading = false;
    }
  });

  async function save(patch: Partial<AppSettings>) {
    if (!settings) return;
    const previous = settings;
    const next = { ...settings, ...patch };
    settings = next;
    try {
      settings = await settingsApi.update(next);
      return true;
    } catch {
      settings = previous;
      toastStore.error('Ayarlar kaydedilemedi.');
      return false;
    }
  }

  function getVolumeMultiplier(): number {
    const vol = settings?.notificationVolume ?? 80;
    return Math.max(0, Math.min(100, vol)) / 100;
  }
</script>

<section aria-labelledby="bildirim-title">
  <h2 class="veil-settings-title" id="bildirim-title">Bildirimler</h2>

  {#if loading}
    <div class="veil-spinner" style="margin:2rem auto;"></div>
  {:else if settings}
    <!-- Önizleme ve Gizlilik -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Gizlilik ve Önizleme</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Bildirim Önizlemesi</div>
          <div class="veil-settings-row-desc">
            Sistem bildirimlerinde ve kilit ekranında mesaj içeriği nasıl görünsün?
          </div>
        </div>
        <div class="veil-select-wrap">
          <VeilSelect
            options={previewOptions}
            value={settings.notificationPreview}
            label="Bildirim önizlemesi"
            onChange={(val) => void save({ notificationPreview: val as NotificationPreview })}
          />
        </div>
      </div>
    </div>

    <!-- Masaüstü & Sistem Bildirimleri -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Masaüstü Bildirimleri</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Masaüstü Bildirimlerini Göster</div>
          <div class="veil-settings-row-desc">
            Yeni mesajlar ve aktiviteler geldiğinde işletim sistemi bildirimi göster.
          </div>
        </div>
        <Toggle
          checked={settings.desktopNotifications ?? true}
          label="Masaüstü bildirimleri"
          onChange={(v) => void save({ desktopNotifications: v })}
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Yalnızca Bahsedilme (@mention)</div>
          <div class="veil-settings-row-desc">
            Yalnızca adın doğrudan geçtiğinde veya direkt mesaj geldiğinde bildirim göster.
          </div>
        </div>
        <Toggle
          checked={settings.mentionOnly ?? false}
          label="Yalnızca bahsedilme"
          onChange={(v) => void save({ mentionOnly: v })}
        />
      </div>

      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Rahatsız Etmeyin (DND) Susturması</div>
          <div class="veil-settings-row-desc">
            Durumun "Rahatsız Etme" veya "Görünmez" iken sesleri ve masaüstü bildirimlerini otomatik bastır.
          </div>
        </div>
        <Toggle
          checked={settings.dndSuppressNotifications ?? true}
          label="DND susturma"
          onChange={(v) => void save({ dndSuppressNotifications: v })}
        />
      </div>
    </div>

    <!-- Sesler ve Efektler -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Sesler ve Bildirim Tonları</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Ana Bildirim Sesi</div>
          <div class="veil-settings-row-desc">Tüm uygulama bildirim seslerini etkinleştir/kapat.</div>
        </div>
        <Toggle
          checked={settings.notificationSound ?? true}
          label="Ana bildirim sesi"
          onChange={(v) => void save({ notificationSound: v })}
        />
      </div>

      {#if settings.notificationSound}
        <div class="veil-settings-row">
          <div class="veil-settings-row-info">
            <div class="veil-settings-row-label">Bildirim Ses Seviyesi</div>
            <div class="veil-settings-row-desc">Tüm ses efektlerinin genel şiddet düzeyi (%{settings.notificationVolume ?? 80}).</div>
          </div>
          <div class="veil-volume-slider-wrap">
            <Icon name={(settings.notificationVolume ?? 80) === 0 ? 'volume-x' : 'volume'} size={16} />
            <input
              type="range"
              min="0"
              max="100"
              value={settings.notificationVolume ?? 80}
              class="veil-slider"
              aria-label="Bildirim ses seviyesi"
              oninput={(e) => {
                const val = Number((e.target as HTMLInputElement).value);
                if (settings) settings.notificationVolume = val;
              }}
              onchange={(e) => {
                const val = Number((e.target as HTMLInputElement).value);
                void save({ notificationVolume: val });
              }}
            />
            <span class="veil-volume-val">%{settings.notificationVolume ?? 80}</span>
          </div>
        </div>

        <div class="veil-settings-subgroup">
          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Yeni Mesaj Sesi</div>
              <div class="veil-settings-row-desc">Normal sohbet ve kanal mesajlarında çalar.</div>
            </div>
            <div class="veil-sound-actions">
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                onclick={() => playMessageSound(0.4 * getVolumeMultiplier())}
                title="Sesi Dinle"
              >
                <Icon name="volume" size={13} />
                Dinle
              </button>
              <Toggle
                checked={settings.soundMessages ?? true}
                label="Yeni mesaj sesi"
                onChange={(v) => void save({ soundMessages: v })}
              />
            </div>
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Bahsedilme Sesi (@mention)</div>
              <div class="veil-settings-row-desc">Direkt mesajlar ve @etiketlendiğinde çalar.</div>
            </div>
            <div class="veil-sound-actions">
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                onclick={() => playMentionSound(0.5 * getVolumeMultiplier())}
                title="Sesi Dinle"
              >
                <Icon name="volume" size={13} />
                Dinle
              </button>
              <Toggle
                checked={settings.soundMentions ?? true}
                label="Bahsedilme sesi"
                onChange={(v) => void save({ soundMentions: v })}
              />
            </div>
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Arkadaşlık İsteği Sesi</div>
              <div class="veil-settings-row-desc">Gelen arkadaşlık isteklerinde çalar.</div>
            </div>
            <div class="veil-sound-actions">
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                onclick={() => playFriendRequestSound(0.45 * getVolumeMultiplier())}
                title="Sesi Dinle"
              >
                <Icon name="volume" size={13} />
                Dinle
              </button>
              <Toggle
                checked={settings.soundFriends ?? true}
                label="Arkadaşlık isteği sesi"
                onChange={(v) => void save({ soundFriends: v })}
              />
            </div>
          </div>

          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label">Ses Kanalı Bağlantı Sesleri</div>
              <div class="veil-settings-row-desc">Odaya giriş ve çıkışlarda çalar.</div>
            </div>
            <div class="veil-sound-actions">
              <button
                type="button"
                class="btn btn-ghost btn-xs"
                onclick={() => playCallJoinSound(0.35 * getVolumeMultiplier())}
                title="Sesi Dinle"
              >
                <Icon name="volume" size={13} />
                Dinle
              </button>
              <Toggle
                checked={settings.soundCalls ?? true}
                label="Ses kanalı sesleri"
                onChange={(v) => void save({ soundCalls: v })}
              />
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Test & Doğrulama -->
    <div class="veil-settings-group">
      <div class="veil-settings-group-label">Test & Doğrulama</div>
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Örnek Bildirim Gönder</div>
          <div class="veil-settings-row-desc">Seçili ses ve masaüstü tercihlerinize göre örnek bir bildirim gönderir.</div>
        </div>
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          onclick={() => {
            const now = Date.now();
            void toastStore.notifyMessage({
              senderName: 'veilanon Bot',
              content: `Bildirimleriniz ve ses efektleriniz başarıyla yapılandırıldı! 🔔 (${new Date().toLocaleTimeString('tr-TR')})`,
              channelName: 'genel',
              isMention: false,
              isDm: false,
            });
            toastStore.success('Test bildirimi gönderildi ✓');
            // Ensure desktop permission prompt is triggered even if DND
            void toastStore.notifySystem('Test Bildirimi', 'Bildirim sistemi aktif.');
          }}
        >
          <Icon name="sparkle" size={14} />
          Örnek Bildirim Gönder
        </button>
      </div>
    </div>

    <p class="veil-settings-row-desc veil-note">
      veilanon tüm bildirimleri Web Audio API ile sıfır gecikmeli ve yerel olarak sentezler; gizlilik ayarlarınıza tam uyum sağlar.
    </p>
  {/if}
</section>

<style>
  .veil-select-wrap {
    min-width: 180px;
  }
  .veil-note { margin-top: var(--space-2); }
  .veil-volume-slider-wrap {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 200px;
    color: var(--veil-text-muted);
  }
  .veil-volume-slider-wrap input[type="range"] {
    flex: 1;
    accent-color: var(--veil-brand);
    cursor: pointer;
  }
  .veil-volume-val {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    min-width: 3.5ch;
    text-align: right;
  }
  .veil-settings-subgroup {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-left: var(--space-3);
    border-left: 2px solid var(--veil-border-subtle);
    margin-top: var(--space-2);
  }
  .veil-sound-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>

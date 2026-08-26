<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth';
  import { cryptoApi, identityApi } from '$lib/api/tauri';
  import { formatFingerprint } from '$lib/crypto/verify';
  import { toastStore } from '$lib/stores/notifications';
  import { uiStore } from '$lib/stores/ui';
  import { open } from '@tauri-apps/plugin-dialog';
  import Avatar, { cacheAvatar } from '$lib/components/ui/Avatar.svelte';
  import BannerImage, { cacheBanner, removeBannerCache } from '$lib/components/ui/BannerImage.svelte';
  import BannerCropModal from '$lib/components/ui/BannerCropModal.svelte';
  import ImageCropModal from '$lib/components/ui/ImageCropModal.svelte';
  import CameraCaptureModal from '$lib/components/ui/CameraCaptureModal.svelte';
  import MediaScraperModal from '$lib/components/ui/MediaScraperModal.svelte';
  import ProfilePlaylistModal from '$lib/components/ui/ProfilePlaylistModal.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { readLocalImageAsDataUrl } from '$lib/utils/image-loader';
  import { privacyShield, revealedMap } from '$lib/stores/privacyShield';
  import { copyText as copyToClipboard } from '$lib/utils/clipboard';
  const isDeviceRevealed = $derived(($revealedMap['device-id'] ?? 0) > Date.now());
  const isFpRevealed = $derived(($revealedMap['fingerprint'] ?? 0) > Date.now());

  let cropSrc = $state<string | null>(null);
  let avatarCropSrc = $state<string | null>(null);
  let showAvatarCameraModal = $state(false);
  let showBannerCameraModal = $state(false);
  let showAvatarScraperModal = $state(false);
  let showBannerScraperModal = $state(false);
  let showPlaylistModal = $state(false);
  let playlistInitialTab = $state<'avatar' | 'banner'>('avatar');

  let deviceName = $state('…');
  let deviceOs = $state('');
  let fingerprint = $state('');
  let fingerprintError = $state<string | null>(null);
  let signingOut = $state(false);
  let autoUnlock = $state(false);
  let autoUnlockBusy = $state(false);

  let nameDraft = $state('');
  let committedName = $state('');
  let nameError = $state<string | null>(null);
  let savingName = $state(false);

  let usernameDraft = $state('');
  let committedUsername = $state('');
  let usernameError = $state<string | null>(null);
  let savingUsername = $state(false);

  let bioDraft = $state('');
  let committedBio = $state('');
  let savingBio = $state(false);
  let avatarBusy = $state(false);
  let bannerBusy = $state(false);

  const auth = $derived($authStore);
  const identity = $derived(auth.identity);
  const trimmedName = $derived(nameDraft.trim());
  const nameUnchanged = $derived(trimmedName === committedName);
  const trimmedUsername = $derived(usernameDraft.trim().toLowerCase());
  const usernameUnchanged = $derived(trimmedUsername === committedUsername.toLowerCase());
  const bioUnchanged = $derived(bioDraft.trim() === committedBio);
  const bioTooLong = $derived(bioDraft.length > 200);

  onMount(async () => {
    try {
      const device = await identityApi.getDeviceInfo();
      deviceName = device.name;
      deviceOs = device.os;
    } catch {
      deviceName = 'Bilinmeyen cihaz';
    }
    try {
      fingerprint = await cryptoApi.fingerprint();
    } catch {
      fingerprintError = 'Parmak izi alınamadı.';
    }
    try {
      autoUnlock = await identityApi.hasAutoUnlock();
    } catch { /* ignored */ }
  });

  async function toggleAutoUnlock(enabled: boolean) {
    if (autoUnlockBusy) return;
    autoUnlockBusy = true;
    if (enabled) {
      const pass = await uiStore.promptInput('Açılışta oturumu hatırlamak için parolanı doğrula:', {
        title: 'Beni Hatırla',
        secret: true,
        confirmLabel: 'Kaydet',
      });
      if (!pass) {
        autoUnlockBusy = false;
        return;
      }
      try {
        await identityApi.setAutoUnlock(true, pass);
        autoUnlock = true;
        toastStore.success('Açılışta otomatik kilit açma etkinleştirildi.');
      } catch {
        autoUnlock = false;
        toastStore.error('Parola doğrulanamadı.');
      } finally {
        autoUnlockBusy = false;
      }
    } else {
      try {
        await identityApi.setAutoUnlock(false);
        autoUnlock = false;
        toastStore.success('Açılışta otomatik kilit açma kapatıldı.');
      } catch {
        toastStore.error('İşlem başarısız.');
      } finally {
        autoUnlockBusy = false;
      }
    }
  }

  $effect(() => {
    const display = identity?.displayName ?? '';
    if (display !== committedName) {
      committedName = display;
      nameDraft = display;
    }
    const un = identity?.username ?? '';
    if (un !== committedUsername) {
      committedUsername = un;
      usernameDraft = un;
    }
  });

  async function saveDisplayName() {
    nameError = null;
    if (!trimmedName) {
      nameError = 'Görünen ad boş olamaz.';
      return;
    }
    if (Array.from(trimmedName).length > 32) {
      nameError = 'Görünen ad en fazla 32 karakter olabilir.';
      return;
    }
    savingName = true;
    try {
      await identityApi.updateProfile({ displayName: trimmedName });
      authStore.updateIdentity({ displayName: trimmedName });
      committedName = trimmedName;
      nameDraft = trimmedName;
      toastStore.success('Görünen ad güncellendi.');
    } catch (err) {
      nameError = `Güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`;
    } finally {
      savingName = false;
    }
  }

  let checkingUsername = $state(false);
  let usernameAvailable = $state<boolean | null>(null);
  let usernameDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const un = trimmedUsername;
    usernameAvailable = null;
    if (usernameDebounceTimer) clearTimeout(usernameDebounceTimer);
    if (!un || un === committedUsername || un.length < 3 || !/^[a-z0-9_-]+$/.test(un)) {
      checkingUsername = false;
      return;
    }
    checkingUsername = true;
    usernameDebounceTimer = setTimeout(async () => {
      try {
        const ok = await identityApi.checkUsernameAvailable(un);
        if (trimmedUsername === un) {
          usernameAvailable = ok;
        }
      } catch {
        usernameAvailable = null;
      } finally {
        checkingUsername = false;
      }
    }, 300);
  });

  async function saveUsername() {
    usernameError = null;
    if (!trimmedUsername) {
      usernameError = 'Kullanıcı adı boş olamaz.';
      return;
    }
    if (trimmedUsername.length < 3 || trimmedUsername.length > 32) {
      usernameError = 'Kullanıcı adı 3-32 karakter arasında olmalıdır.';
      return;
    }
    if (!/^[a-z0-9_-]+$/.test(trimmedUsername)) {
      usernameError = 'Kullanıcı adı yalnızca küçük harf, rakam, alt çizgi (_) ve tire (-) içerebilir.';
      return;
    }
    if (usernameAvailable === false) {
      usernameError = 'Bu kullanıcı adı başka birisi tarafından kullanılıyor.';
      return;
    }
    savingUsername = true;
    try {
      await identityApi.updateProfile({
        displayName: identity?.displayName ?? trimmedUsername,
        username: trimmedUsername,
      });
      authStore.updateIdentity({ username: trimmedUsername });
      committedUsername = trimmedUsername;
      usernameDraft = trimmedUsername;
      usernameAvailable = null;
      toastStore.success('Kullanıcı adı güncellendi.');
    } catch (err) {
      usernameError = `Güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`;
    } finally {
      savingUsername = false;
    }
  }

  async function saveBio() {
    if (bioTooLong || !identity) return;
    savingBio = true;
    try {
      const bio = bioDraft.trim();
      await identityApi.updateProfile({ displayName: identity.displayName, bio: bio || null });
      committedBio = bio;
      toastStore.success('Hakkında güncellendi.');
    } catch {
      toastStore.error('Hakkında kaydedilemedi.');
    } finally {
      savingBio = false;
    }
  }

  async function changeAvatar() {
    if (avatarBusy) return;
    try {
      const selected = await open({
        title: 'Profil fotoğrafı seç',
        multiple: false,
        filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
      });
      if (!selected || typeof selected !== 'string') return;
      avatarCropSrc = await readLocalImageAsDataUrl(selected);
    } catch {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'image/png,image/jpeg,image/webp,image/gif';
      input.onchange = async () => {
        const file = input.files?.[0];
        if (file) {
          const reader = new FileReader();
          reader.onload = () => {
            if (typeof reader.result === 'string') {
              avatarCropSrc = reader.result;
            }
          };
          reader.readAsDataURL(file);
        }
      };
      input.click();
    }
  }

  function captureAvatar() {
    if (avatarBusy) return;
    showAvatarCameraModal = true;
  }

  async function handleAvatarCropSave(croppedDataUrl: string) {
    avatarCropSrc = null;
    avatarBusy = true;
    try {
      const hash = await identityApi.setAvatar(croppedDataUrl);
      cacheAvatar(hash, croppedDataUrl);
      authStore.updateIdentity({ avatarHash: hash });
      toastStore.success('Profil fotoğrafı güncellendi.');
    } catch {
      toastStore.error('Profil fotoğrafı yüklenemedi.');
    } finally {
      avatarBusy = false;
    }
  }

  async function removeAvatar() {
    if (!identity?.avatarHash) return;
    const ok = await uiStore.confirm('Profil fotoğrafını kaldırmak istiyor musun?', {
      title: 'Fotoğrafı Kaldır',
      confirmLabel: 'Kaldır',
      danger: true,
    });
    if (!ok) return;
    try {
      await identityApi.updateProfile({ displayName: identity.displayName, avatarHash: null });
      authStore.updateIdentity({ avatarHash: null });
      toastStore.success('Profil fotoğrafı kaldırıldı.');
    } catch {
      toastStore.error('Fotoğraf kaldırılamadı.');
    }
  }

  async function changeBanner() {
    if (bannerBusy) return;
    try {
      const selected = await open({
        title: 'Profil bannerı seç',
        multiple: false,
        filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
      });
      if (!selected || typeof selected !== 'string') return;
      cropSrc = await readLocalImageAsDataUrl(selected);
    } catch {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = 'image/png,image/jpeg,image/webp,image/gif';
      input.onchange = async () => {
        const file = input.files?.[0];
        if (file) {
          const reader = new FileReader();
          reader.onload = () => {
            if (typeof reader.result === 'string') {
              cropSrc = reader.result;
            }
          };
          reader.readAsDataURL(file);
        }
      };
      input.click();
    }
  }

  function captureBanner() {
    if (bannerBusy) return;
    showBannerCameraModal = true;
  }

  async function handleBannerCropSave(croppedDataUrl: string) {
    cropSrc = null;
    bannerBusy = true;
    try {
      const hash = await identityApi.setBanner(croppedDataUrl);
      cacheBanner(hash, croppedDataUrl);
      authStore.updateIdentity({ bannerHash: hash });
      toastStore.success('Profil bannerı güncellendi.');
    } catch {
      toastStore.error('Banner yüklenemedi.');
    } finally {
      bannerBusy = false;
    }
  }

  async function removeBanner() {
    if (!identity?.bannerHash) return;
    const ok = await uiStore.confirm('Profil bannerını kaldırmak istiyor musun?', {
      title: 'Bannerı Kaldır',
      confirmLabel: 'Kaldır',
      danger: true,
    });
    if (!ok) return;
    try {
      await identityApi.updateProfile({ displayName: identity.displayName, bannerHash: null });
      removeBannerCache(identity.bannerHash);
      authStore.updateIdentity({ bannerHash: null });
      toastStore.success('Profil bannerı kaldırıldı.');
    } catch {
      toastStore.error('Banner kaldırılamadı.');
    }
  }

  function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    saveDisplayName();
  }

  async function copyText(text: string, label: string) {
    const success = await copyToClipboard(text);
    if (success) {
      toastStore.success(`${label} kopyalandı.`);
    } else {
      toastStore.error(`${label} kopyalanamadı.`);
    }
  }

  async function signOut() {
    const ok = await uiStore.confirm(
      'Bu cihazda oturumu kapatmak istediğine emin misin? Mesajların cihazında şifreli kalır.',
      { title: 'Oturumu Kapat', confirmLabel: 'Kapat', danger: true }
    );
    if (!ok) return;
    signingOut = true;
    try {
      await authStore.signOut();
      toastStore.success('Oturum kapatıldı.');
    } catch {
      signingOut = false;
      toastStore.error('Oturum kapatılamadı.');
    }
  }
</script>

<section aria-labelledby="hesap-title">
  <h2 class="veil-settings-title" id="hesap-title">Hesap</h2>

  <div class="veil-profile-hero">
    <div class="veil-profile-hero-banner">
      <BannerImage hash={identity?.bannerHash} alt="" class="veil-profile-hero-banner-img" />
      <div class="veil-banner-actions-overlay">
        <button class="btn btn-secondary btn-sm veil-banner-action-btn" type="button" onclick={changeBanner} disabled={bannerBusy}>
          <Icon name="image" size={13} />
          <span>{identity?.bannerHash ? 'Bannerı Değiştir' : 'Banner Ekle'}</span>
        </button>
        <button class="btn btn-secondary btn-sm veil-banner-action-btn" type="button" onclick={captureBanner} disabled={bannerBusy} title="Kameradan çek">
          <Icon name="camera" size={13} />
          <span>Kamera</span>
        </button>
        <button class="btn btn-secondary btn-sm veil-banner-action-btn" type="button" onclick={() => { showBannerScraperModal = true; }} disabled={bannerBusy} title="Webden veya URL'den tara">
          <Icon name="globe" size={13} />
          <span>Webden Tara</span>
        </button>
        <button class="btn btn-secondary btn-sm veil-banner-action-btn" type="button" onclick={() => { playlistInitialTab = 'banner'; showPlaylistModal = true; }} title="Banner oynatma listesi">
          <Icon name="refresh-cw" size={13} />
          <span>Slayt / Liste</span>
        </button>
        {#if identity?.bannerHash}
          <button class="btn btn-ghost btn-sm veil-banner-action-btn" type="button" onclick={removeBanner} disabled={bannerBusy}>
            Kaldır
          </button>
        {/if}
      </div>
    </div>

    <div class="veil-profile-hero-body">
      <div class="veil-profile-hero-top">
        <div class="veil-avatar-edit" title="Profil fotoğrafını değiştir">
          <Avatar name={identity?.displayName ?? '…'} size="xl" hash={identity?.avatarHash} />
          <button
            class="veil-avatar-edit-btn"
            type="button"
            aria-label="Profil fotoğrafını değiştir"
            onclick={changeAvatar}
            disabled={avatarBusy}
          >
            {#if avatarBusy}
              <div class="veil-spinner veil-spinner-sm"></div>
            {:else}
              <Icon name="user" size={16} />
            {/if}
          </button>
        </div>

        <div class="veil-avatar-actions">
          <button class="btn btn-secondary btn-sm" type="button" onclick={changeAvatar} disabled={avatarBusy}>
            <Icon name="user" size={13} />
            Fotoğraf Değiştir
          </button>
          <button class="btn btn-secondary btn-sm" type="button" onclick={captureAvatar} disabled={avatarBusy} title="Kameradan çek">
            <Icon name="camera" size={13} />
            Kamera
          </button>
          <button class="btn btn-secondary btn-sm" type="button" onclick={() => { showAvatarScraperModal = true; }} disabled={avatarBusy} title="Webden veya URL'den tara">
            <Icon name="globe" size={13} />
            Webden Tara
          </button>
          <button class="btn btn-secondary btn-sm" type="button" onclick={() => { playlistInitialTab = 'avatar'; showPlaylistModal = true; }} title="Avatar oynatma listesi">
            <Icon name="refresh-cw" size={13} />
            Slayt
          </button>
          {#if identity?.avatarHash}
            <button class="btn btn-ghost btn-sm" type="button" onclick={removeAvatar} disabled={avatarBusy}>
              Kaldır
            </button>
          {/if}
        </div>
      </div>

      <div class="veil-profile-info">
        <div class="veil-profile-name">{identity?.displayName ?? '—'}</div>
        <div class="veil-profile-username">@{identity?.username ?? '…'}</div>
      </div>
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Profil</div>
    <div class="veil-settings-row veil-settings-row-stack">
      <div class="veil-settings-row-info">
        <label class="veil-settings-row-label" for="display-name-input">Görünen Ad</label>
        <p class="veil-settings-row-desc">Başkalarının senin için gördüğü ad.</p>
      </div>
      <form class="veil-displayname-form" onsubmit={handleSubmit}>
        <input
          id="display-name-input"
          class="veil-input"
          class:error={!!nameError}
          type="text"
          bind:value={nameDraft}
          placeholder="Görünen ad"
          maxlength={32}
          autocomplete="off"
          required
          disabled={!identity}
        />
        <button
          class="btn btn-primary"
          type="submit"
          disabled={!identity || savingName || nameUnchanged}
        >
          {savingName ? 'Kaydediliyor…' : 'Kaydet'}
        </button>
      </form>
      {#if nameError}<p class="veil-form-error" role="alert">{nameError}</p>{/if}
    </div>
    <div class="veil-settings-row veil-settings-row-stack">
      <div class="veil-settings-row-info">
        <label class="veil-settings-row-label" for="username-input">Benzersiz Kullanıcı Adı (@kullaniciadi)</label>
        <p class="veil-settings-row-desc">Benzersiz kimlik adın (arkadaş ekleme ve etiketlemeler için kullanılır).</p>
      </div>
      <form class="veil-displayname-form" onsubmit={(e) => { e.preventDefault(); saveUsername(); }}>
        <input
          id="username-input"
          class="veil-input"
          class:error={!!usernameError || usernameAvailable === false}
          class:success={usernameAvailable === true && !usernameUnchanged}
          type="text"
          bind:value={usernameDraft}
          placeholder="Kullanıcı adı (ör. aegis)"
          minlength={3}
          maxlength={32}
          autocomplete="off"
          required
          disabled={!identity}
        />
        <button
          class="btn btn-primary"
          type="submit"
          disabled={!identity || savingUsername || usernameUnchanged || usernameAvailable === false || checkingUsername}
        >
          {savingUsername ? 'Kaydediliyor…' : 'Kaydet'}
        </button>
      </form>
      {#if checkingUsername}
        <p class="veil-form-hint veil-checking"><span class="veil-spinner veil-spinner-xs"></span> Kullanıcı adı kontrol ediliyor…</p>
      {:else if usernameAvailable === true && !usernameUnchanged}
        <p class="veil-form-hint veil-available">✓ Bu kullanıcı adı kullanılabilir</p>
      {:else if usernameAvailable === false && !usernameUnchanged}
        <p class="veil-form-error" role="alert">✕ Bu kullanıcı adı başka birisi tarafından kullanılıyor</p>
      {/if}
      {#if usernameError}<p class="veil-form-error" role="alert">{usernameError}</p>{/if}
    </div>
    {#if identity?.username}
      <div class="veil-settings-row">
        <div class="veil-settings-row-info">
          <div class="veil-settings-row-label">Profil Bağlantısı</div>
          <div class="veil-settings-row-desc">Arkadaşlarınla paylaşarak seni kolayca eklemelerini sağla.</div>
        </div>
        <button
          class="btn btn-secondary btn-sm"
          type="button"
          onclick={() => copyText(`https://veilanon.com/u/${identity.username}`, 'Profil bağlantısı')}
        >
          <Icon name="copy" size={14} />
          veilanon.com/u/{identity.username}
        </button>
      </div>
    {/if}
    <div class="veil-settings-row veil-settings-row-stack">
      <div class="veil-settings-row-info">
        <label class="veil-settings-row-label" for="bio-input">Hakkımda</label>
        <p class="veil-settings-row-desc">Profilinde görünecek kısa bir tanıtım (en fazla 200 karakter).</p>
      </div>
      <textarea
        id="bio-input"
        class="veil-input veil-bio-input"
        bind:value={bioDraft}
        rows={3}
        maxlength={200}
        placeholder="Örn: Gizlilik meraklısı, kahve bağımlısı ☕"
        disabled={!identity}
      ></textarea>
      <div class="veil-bio-actions">
        <span class="veil-bio-count" class:over={bioTooLong}>{bioDraft.length}/200</span>
        <button
          class="btn btn-primary btn-sm"
          type="button"
          onclick={saveBio}
          disabled={!identity || savingBio || bioUnchanged || bioTooLong}
        >
          {savingBio ? 'Kaydediliyor…' : 'Kaydet'}
        </button>
      </div>
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Cihaz</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Cihaz adı</div>
        <div class="veil-settings-row-desc veil-mono">{deviceName}</div>
      </div>
    </div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Platform</div>
        <div class="veil-settings-row-desc veil-mono">{deviceOs || '—'}</div>
      </div>
    </div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Cihaz Kimliği</div>
        <div
          class="veil-settings-row-desc veil-mono"
          data-streamer-mask="id"
          data-auto-protect="secret"
          data-revealed={isDeviceRevealed}
        >
          {isDeviceRevealed ? (identity?.deviceId ?? '') : privacyShield.formatSecretManual(identity?.deviceId, 'device-id')}
        </div>
      </div>
      {#if identity?.deviceId}
        <div style="display:flex;gap:4px;">
          <button
            class="btn-icon"
            type="button"
            title={isDeviceRevealed ? 'Gizle' : '5 saniyeliğine göster'}
            onclick={() => privacyShield.toggleSecret('device-id', 5)}
          >
            <Icon name={isDeviceRevealed ? 'eye-off' : 'eye'} size={14} />
          </button>
          <button
            class="btn-icon"
            type="button"
            title="Cihaz kimliğini kopyala"
            aria-label="Cihaz kimliğini kopyala"
            onclick={() => copyText(identity.deviceId, 'Cihaz kimliği')}
          >
            <Icon name="copy" size={14} />
          </button>
        </div>
      {/if}
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Parmak İzi</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <div class="veil-settings-row-label">Kimlik parmak izi</div>
        <div class="veil-settings-row-desc">
          Karşı tarafın parmak iziyle eşleştiğinden emin olarak kimliğini doğrula.
        </div>
      </div>
    </div>
    {#if fingerprint}
      <pre
        class="veil-fingerprint"
        aria-label="Kimlik parmak izi"
        data-streamer-mask="id"
        data-auto-protect="secret"
        data-revealed={isFpRevealed}
      >{isFpRevealed ? formatFingerprint(fingerprint) : formatFingerprint(fingerprint).split(' ').map(()=>'••••').join(' ')}</pre>
      <div style="display:flex;gap:8px;margin-top:4px;">
        <button
          class="btn btn-secondary btn-sm"
          type="button"
          onclick={() => privacyShield.toggleSecret('fingerprint', 5)}
        >
          <Icon name={isFpRevealed ? 'eye-off' : 'eye'} size={14} />
          {isFpRevealed ? 'Gizle' : '5 sn Göster'}
        </button>
        <button
          class="btn btn-secondary btn-sm"
          type="button"
          onclick={() => copyText(fingerprint, 'Parmak izi')}
        >
          <Icon name="copy" size={14} />
          Kopyala
        </button>
      </div>
    {:else if fingerprintError}
      <p class="veil-form-error">{fingerprintError}</p>
    {/if}
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Güvenlik & Giriş</div>
    <div class="veil-settings-row">
      <div class="veil-settings-row-info">
        <label class="veil-settings-row-label" for="auto-unlock-toggle">Bu Cihazda Beni Hatırla</label>
        <p class="veil-settings-row-desc">
          Uygulama açılışında her seferinde parola sormadan cihazındaki şifreli anahtarlarının kilidini otomatik olarak açar.
        </p>
      </div>
      <Toggle
        id="auto-unlock-toggle"
        checked={autoUnlock}
        disabled={autoUnlockBusy}
        onChange={toggleAutoUnlock}
      />
    </div>
  </div>

  <div class="veil-settings-group">
    <div class="veil-settings-group-label">Oturum</div>
    <button
      class="btn btn-danger"
      onclick={signOut}
      disabled={signingOut}
    >
      {signingOut ? 'Kapatılıyor…' : 'Oturumu Kapat'}
      {#if !signingOut}<Icon name="logout" size={16} />{/if}
    </button>
    <p class="veil-settings-row-desc veil-settings-footnote">
      Oturum kapatmak kimliğini silmez — veriler cihazında şifreli olarak kalır.
    </p>
  </div>

  {#if showAvatarCameraModal}
    <CameraCaptureModal
      title="Profil Fotoğrafı Çek"
      aspectRatio={1}
      onCapture={(dataUrl) => {
        showAvatarCameraModal = false;
        avatarCropSrc = dataUrl;
      }}
      onClose={() => {
        showAvatarCameraModal = false;
      }}
    />
  {/if}

  {#if showBannerCameraModal}
    <CameraCaptureModal
      title="Profil Bannerı Çek"
      aspectRatio={3}
      onCapture={(dataUrl) => {
        showBannerCameraModal = false;
        cropSrc = dataUrl;
      }}
      onClose={() => {
        showBannerCameraModal = false;
      }}
    />
  {/if}

  {#if showAvatarScraperModal}
    <MediaScraperModal
      title="Webden Profil Fotoğrafı Tara"
      aspectRatio={1}
      onSelect={(url) => {
        showAvatarScraperModal = false;
        avatarCropSrc = url;
      }}
      onClose={() => {
        showAvatarScraperModal = false;
      }}
    />
  {/if}

  {#if showBannerScraperModal}
    <MediaScraperModal
      title="Webden Profil Bannerı Tara"
      aspectRatio={3}
      onSelect={(url) => {
        showBannerScraperModal = false;
        cropSrc = url;
      }}
      onClose={() => {
        showBannerScraperModal = false;
      }}
    />
  {/if}

  {#if showPlaylistModal}
    <ProfilePlaylistModal
      initialTab={playlistInitialTab}
      onClose={() => {
        showPlaylistModal = false;
      }}
      onAddViaScraper={(type) => {
        showPlaylistModal = false;
        if (type === 'avatar') {
          showAvatarScraperModal = true;
        } else {
          showBannerScraperModal = true;
        }
      }}
      onAddViaCamera={(type) => {
        showPlaylistModal = false;
        if (type === 'avatar') {
          showAvatarCameraModal = true;
        } else {
          showBannerCameraModal = true;
        }
      }}
    />
  {/if}

  {#if avatarCropSrc}
    <ImageCropModal
      src={avatarCropSrc}
      shape="circle"
      aspectRatio={1}
      title="Profil Fotoğrafını Ayarla"
      onSave={handleAvatarCropSave}
      onClose={() => { avatarCropSrc = null; }}
    />
  {/if}

  {#if cropSrc}
    <BannerCropModal
      src={cropSrc}
      aspectRatio={3}
      title="Profil Bannerını Ayarla"
      hasAvatarPreview={true}
      avatarName={identity?.displayName ?? identity?.username}
      avatarHash={identity?.avatarHash}
      onSave={handleBannerCropSave}
      onClose={() => { cropSrc = null; }}
    />
  {/if}
</section>

<style>
  .veil-mono { font-family: var(--font-mono); font-size: var(--text-xs); word-break: break-all; }
  .veil-fingerprint {
    margin: var(--space-2) 0;
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    letter-spacing: 0.08em;
    line-height: var(--leading-relaxed);
    user-select: text;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .veil-settings-footnote { margin-top: var(--space-2); }

  .veil-profile-hero {
    position: relative;
    border-radius: var(--radius-2xl);
    overflow: hidden;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    margin-bottom: var(--space-8);
    box-shadow: var(--shadow-md);
  }
  .veil-profile-hero-banner {
    width: 100%;
    height: 160px;
    aspect-ratio: 3 / 1;
    max-height: 160px;
    position: relative;
    background:
      radial-gradient(120% 160% at 15% 0%, var(--veil-brand-subtle), transparent 55%),
      linear-gradient(160deg, var(--veil-bg-surface), var(--veil-bg-void));
    overflow: hidden;
    display: block;
  }
  :global(.veil-profile-hero-banner-img) {
    width: 100%;
    height: 100%;
    min-height: 160px;
    object-fit: cover;
    object-position: center;
    display: block;
  }
  :global(.veil-profile-hero-banner-img .veil-banner-wrapper) {
    height: 100%;
    min-height: 160px;
  }
  :global(.veil-profile-hero-banner-img img) {
    object-fit: cover;
    object-position: center;
  }
  .veil-banner-actions-overlay {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    display: flex;
    gap: var(--space-2);
    z-index: 5;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .veil-banner-action-btn {
    background: rgba(15, 17, 23, 0.88);
    backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.22);
    color: #fff;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
    font-weight: 500;
  }
  .veil-banner-action-btn:hover {
    background: rgba(15, 17, 23, 0.98);
    border-color: rgba(255, 255, 255, 0.4);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
  }
  .veil-profile-hero-body {
    padding: 0 var(--space-6) var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-profile-hero-top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    margin-top: -36px;
    position: relative;
    z-index: 2;
  }
  .veil-profile-hero-top :global(.veil-avatar) {
    border: 4px solid var(--veil-bg-elevated);
    border-radius: var(--radius-full);
    box-shadow: var(--shadow-lg);
  }
  .veil-avatar-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
    flex-wrap: wrap;
  }
  .veil-profile-info { min-width: 0; }
  .veil-profile-name {
    font-size: var(--text-xl);
    font-weight: 700;
    letter-spacing: var(--tracking-tight);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-profile-username {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--veil-text-muted);
    margin-top: 2px;
  }

  .veil-settings-row-stack { flex-direction: column; align-items: stretch; gap: var(--space-3); }
  .veil-displayname-form { display: flex; gap: var(--space-3); }
  .veil-displayname-form .veil-input { flex: 1; min-width: 0; }
  .veil-input.success { border-color: var(--veil-success); }
  .veil-form-hint.veil-available { color: var(--veil-success); font-size: var(--text-xs); font-weight: 600; margin-top: 2px; }
  .veil-form-hint.veil-checking { color: var(--veil-text-muted); font-size: var(--text-xs); margin-top: 2px; display: inline-flex; align-items: center; gap: 6px; }
  .veil-spinner-xs { width: 12px; height: 12px; border-width: 1.5px; }

  .veil-avatar-edit { position: relative; flex-shrink: 0; }
  .veil-avatar-edit-btn {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--radius-full);
    background: hsl(220 20% 4% / 0.55);
    color: #fff;
    opacity: 0;
    cursor: pointer;
    transition: opacity var(--t-base);
  }
  .veil-avatar-edit:hover .veil-avatar-edit-btn { opacity: 1; }
  .veil-avatar-edit-btn:disabled { cursor: wait; }

  .veil-bio-input { resize: vertical; min-height: 72px; }
  .veil-bio-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .veil-bio-count { font-size: var(--text-xs); color: var(--veil-text-muted); font-variant-numeric: tabular-nums; }
  .veil-bio-count.over { color: var(--veil-danger); }
</style>

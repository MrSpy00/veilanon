<script lang="ts">
  import { authStore, type IdentityInfo } from '$lib/stores/auth';
  import { privacyToolsApi } from '$lib/api/tauri';
  import Icon from '$lib/components/ui/Icon.svelte';

  let {
    onBack,
    onCreated,
    identityExists = false,
  }: {
    onBack: () => void;
    /** Called with the recovery code after a successful create. */
    onCreated: (recoveryCode: string | null) => void;
    /** True when the backend already holds an identity on this device. */
    identityExists?: boolean;
  } = $props();

  let username    = $state('');
  let displayName = $state('');
  let passphrase  = $state('');
  let passConfirm = $state('');
  let showPass    = $state(false);
  let rememberMe  = $state(true);
  let loading     = $state(false);
  let error       = $state<string | null>(null);

  let pwnedWarning = $state<string | null>(null);
  let pwnedDebounce: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const p = passphrase;
    if (pwnedDebounce) clearTimeout(pwnedDebounce);
    if (p.length >= 8) {
      pwnedDebounce = setTimeout(async () => {
        try {
          const res = await privacyToolsApi.checkPasswordPwned(p);
          if (res.isPwned) {
            pwnedWarning = `⚠️ Bu parola bilinen ${res.breachCount.toLocaleString('tr-TR')} veri sızıntısında bulundu! Farklı bir parola seçmeniz önerilir.`;
          } else {
            pwnedWarning = null;
          }
        } catch {
          pwnedWarning = null;
        }
      }, 600);
    } else {
      pwnedWarning = null;
    }
  });

  function scorePassphrase(p: string): { label: string; color: string; score: number } {
    if (!p) return { label: '', color: 'transparent', score: 0 };
    let score = 0;
    if (p.length >= 12) score++;
    if (/[A-Z]/.test(p)) score++;
    if (/[0-9]/.test(p)) score++;
    if (/[^A-Za-z0-9]/.test(p)) score++;
    if (p.length >= 20) score++;
    if (score <= 1) return { label: 'Çok zayıf', color: 'var(--veil-danger)', score };
    if (score === 2) return { label: 'Zayıf', color: 'var(--veil-warning)', score };
    if (score === 3) return { label: 'Orta', color: 'var(--veil-info)', score };
    if (score === 4) return { label: 'Güçlü', color: 'var(--veil-success)', score };
    return { label: 'Çok güçlü', color: 'var(--veil-success)', score };
  }

  const passStrength = $derived(scorePassphrase(passphrase));

  const canCreate = $derived(
    username.trim().length >= 2 &&
    passphrase.length >= 8 &&
    passphrase === passConfirm &&
    !loading
  );

  async function createIdentity() {
    if (!canCreate) return;
    error = null;
    loading = true;
    try {
      const resp = await authStore.createIdentity(
        username.trim(),
        displayName.trim() || username.trim(),
        passphrase,
        rememberMe
      );
      const code = (resp as IdentityInfo & { recoveryCode?: string }).recoveryCode ?? null;
      onCreated(code);
    } catch (err) {
      const raw = String(err).replace(/^Error:\s*/, '');
      const msg = raw.toLowerCase();
      if (raw.includes('Invalid input:')) {
        error = raw.replace('Invalid input:', '').trim();
      } else if (msg.includes('identity exists') || msg.includes('identityexists') || msg.includes('kimlik var')) {
        error = 'Bu cihazda zaten kayıtlı bir kimlik var. Yeni kimlik oluşturmak için önce mevcut kimliği sıfırlamalısınız.';
      } else if (msg.includes('rate limit')) {
        error = 'Çok fazla deneme yapıldı. Lütfen biraz bekleyip tekrar deneyin.';
      } else {
        error = raw || 'Kimlik oluşturulamadı. Lütfen bilgilerinizi kontrol edip tekrar deneyin.';
      }
    } finally {
      loading = false;
    }
  }
</script>

<h1 class="veil-onboarding-title" id="create-title">Kimlik Oluştur</h1>
<p class="veil-onboarding-subtitle">
  E-posta gerekmez. Parola tamamen cihazında kalır.
</p>

{#if error}
  <div class="veil-alert-error" role="alert">
    <Icon name="warning" size={16} />
    <span>{error}</span>
  </div>
{/if}

<form onsubmit={(e) => { e.preventDefault(); createIdentity(); }}>
  <div class="veil-form-group">
    <label class="veil-form-label" for="username">Kullanıcı Adı</label>
    <!-- svelte-ignore a11y_autofocus — first field of the creation flow -->
    <input
      id="username"
      class="veil-input"
      class:error={username.length > 0 && username.trim().length < 2}
      type="text"
      bind:value={username}
      placeholder="örn: mrspy"
      maxlength={32}
      autocomplete="off"
      autofocus
      required
    />
    <span class="veil-form-desc">2-32 karakter, harf ve rakam</span>
  </div>

  <div class="veil-form-group">
    <label class="veil-form-label" for="displayName">Görünen Ad</label>
    <input
      id="displayName"
      class="veil-input"
      type="text"
      bind:value={displayName}
      placeholder={username || 'Görünen adın'}
      maxlength={64}
      autocomplete="off"
    />
    <span class="veil-form-desc">Boş bırakırsan kullanıcı adın kullanılır</span>
  </div>

  <div class="veil-form-group">
    <label class="veil-form-label" for="passphrase">Parola</label>
    <div class="veil-pass-field">
      <input
        id="passphrase"
        class="veil-input"
        type={showPass ? 'text' : 'password'}
        bind:value={passphrase}
        placeholder="En az 8 karakter"
        minlength={8}
        autocomplete="new-password"
        required
      />
      <button
        type="button"
        class="btn-icon veil-pass-toggle"
        onclick={() => showPass = !showPass}
        aria-label={showPass ? 'Parolayı gizle' : 'Parolayı göster'}
      >
        <Icon name={showPass ? 'eye-off' : 'eye'} size={18} />
      </button>
    </div>

    {#if passphrase}
      <div
        class="veil-strength-bar"
        role="meter"
        aria-label="Parola gücü: {passStrength.label}"
        aria-valuemin={0}
        aria-valuemax={5}
        aria-valuenow={passStrength.score}
      >
        {#each [1, 2, 3, 4, 5] as n}
          <div class="veil-strength-seg" class:active={n <= passStrength.score}></div>
        {/each}
        <span
          class="veil-strength-label"
          style:--seg-color={passStrength.color}
        >{passStrength.label}</span>
      </div>
    {/if}

    {#if pwnedWarning}
      <div class="veil-form-error" style="margin-top: 6px; font-size: 11px; line-height: 1.4;">
        {pwnedWarning}
      </div>
    {/if}
  </div>

  <div class="veil-form-group">
    <label class="veil-form-label" for="passConfirm">Parolayı Onayla</label>
    <input
      id="passConfirm"
      class="veil-input"
      class:error={passConfirm.length > 0 && passConfirm !== passphrase}
      type="password"
      bind:value={passConfirm}
      placeholder="Parolanı tekrar gir"
      autocomplete="new-password"
      required
    />
    {#if passConfirm.length > 0 && passConfirm !== passphrase}
      <span class="veil-form-error" role="alert">Parolalar eşleşmiyor</span>
    {/if}
  </div>

  <label class="veil-remember-box" style="margin-bottom: var(--space-4);">
    <input type="checkbox" bind:checked={rememberMe} />
    <span>Bu cihazda beni hatırla (Açılışta parola sorma)</span>
  </label>

  <div class="veil-form-row">
    <button type="button" class="btn btn-secondary" onclick={onBack}>
      <Icon name="arrow-left" size={16} />
      Geri
    </button>
    <button type="submit" class="btn btn-primary" disabled={!canCreate}>
      {#if loading}
        <div class="veil-spinner veil-spinner-sm" aria-hidden="true"></div>
        Oluşturuluyor…
      {:else}
        Kimlik Oluştur
      {/if}
    </button>
  </div>
</form>

<div class="veil-onboarding-footer-card">
  <Icon name="key" size={15} />
  <span>Parolanı unutursan verilerine erişemezsin. Güçlü bir parola seç.</span>
</div>

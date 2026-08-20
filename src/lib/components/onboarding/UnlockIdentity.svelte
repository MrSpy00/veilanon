<script lang="ts">
  import { authStore } from '$lib/stores/auth';
  import type { IdentityHint } from '$lib/api/tauri';
  import Icon from '$lib/components/ui/Icon.svelte';
  import Avatar from '$lib/components/ui/Avatar.svelte';
  import { recoveryAcknowledged } from './Welcome.svelte';

  let {
    onBack,
    onReset = null,
    identityHint: propHint = null,
    initialMode = 'pass',
  }: {
    onBack: () => void;
    /** Called when the user requests a device factory reset from this screen. */
    onReset?: (() => void) | null;
    /** Optional hint showing which identity is being unlocked. */
    identityHint?: IdentityHint | null;
    /** Which unlock form to show first: passphrase or recovery code. */
    initialMode?: 'pass' | 'recover';
  } = $props();

  let username       = $state('');
  let pass           = $state('');
  let showPass       = $state(false);
  let rememberMe     = $state(true);
  let mode           = $state<'pass' | 'credentials' | 'recover'>('pass');
  let recoveryCode   = $state('');
  let newPass        = $state('');
  let confirmNewPass = $state('');
  let showNewPass    = $state(false);
  let loading        = $state(false);
  let error          = $state<string | null>(null);

  // Identity badge — prefer the prop, fall back to the store hint
  let hint        = $state<IdentityHint | null>(null);
  let hintFetched = $state(false);

  $effect(() => {
    if (hint) return;
    const candidate = propHint ?? $authStore.identityHint;
    if (candidate) {
      hint = candidate;
      if (!hint.hasIdentity && initialMode === 'pass') {
        mode = 'credentials';
      }
    } else if (!hintFetched) {
      hintFetched = true;
      authStore.getIdentityHint().then(h => {
        if (h) {
          hint = h;
          if (!h.hasIdentity && initialMode === 'pass') {
            mode = 'credentials';
          }
        } else if (initialMode === 'pass') {
          mode = 'credentials';
        }
      });
    }
  });

  $effect(() => {
    if (initialMode === 'recover') {
      mode = 'recover';
    } else if (hint && !hint.hasIdentity) {
      mode = 'credentials';
    }
  });

  // Rate-limit cooldown
  const COOLDOWN_SECONDS = 30;
  let cooldown      = $state(0);
  let cooldownTimer: ReturnType<typeof setInterval> | null = null;

  $effect(() => {
    return () => {
      if (cooldownTimer) { clearInterval(cooldownTimer); cooldownTimer = null; }
    };
  });

  function startCooldown() {
    if (cooldownTimer) clearInterval(cooldownTimer);
    cooldown = COOLDOWN_SECONDS;
    cooldownTimer = setInterval(() => {
      cooldown -= 1;
      if (cooldown <= 0) {
        if (cooldownTimer) clearInterval(cooldownTimer);
        cooldownTimer = null;
      }
    }, 1000);
  }

  const passMismatch = $derived(
    mode === 'recover' && confirmNewPass.length > 0 && newPass !== confirmNewPass
  );

  const canSubmit = $derived(
    mode === 'pass'
      ? pass.length > 0 && !loading && cooldown <= 0
      : mode === 'credentials'
        ? username.trim().length >= 2 && pass.length >= 8 && !loading && cooldown <= 0
        : (username.trim().length >= 2 || (hint?.username ? hint.username.length >= 2 : false)) &&
          recoveryCode.trim().length >= 24 &&
          newPass.length >= 8 &&
          newPass === confirmNewPass &&
          !loading &&
          cooldown <= 0
  );

  function handleError(err: unknown) {
    const raw = String(err).replace(/^Error:\s*/, '');
    const msg = raw.toLowerCase();
    if (msg.includes('rate limit') || msg.includes('ratelimit') || msg.includes('çok fazla deneme')) {
      error = 'Çok fazla hatalı deneme yapıldı. Lütfen biraz bekleyip tekrar deneyin.';
      startCooldown();
    } else if (raw.includes('Invalid input:')) {
      error = raw.replace('Invalid input:', '').trim();
    } else if (
      msg.includes('invalid passphrase') ||
      msg.includes('parola hatalı') ||
      msg.includes('hatalı veya geçersiz') ||
      msg.includes('invalid login credentials') ||
      msg.includes('invalid credentials')
    ) {
      error = 'Kullanıcı adı veya parola hatalı. Lütfen bilgilerinizi kontrol edin.';
    } else if (
      msg.includes('recovery code invalid') ||
      msg.includes('invalid recovery code') ||
      msg.includes('kurtarma kodu') ||
      msg.includes('expired')
    ) {
      error = 'Kurtarma kodu geçersiz veya hatalı. Lütfen kurtarma kodunuzu kontrol edip tekrar deneyin.';
    } else if (msg.includes('identity not found') || msg.includes('kayıtlı bir kimlik bulunamadı')) {
      error = 'Kayıtlı bir kimlik bulunamadı. Kullanıcı adı ve parolanızla giriş yapabilir veya yeni bir kimlik oluşturabilirsiniz.';
    } else {
      error = raw || 'Giriş işlemi tamamlanamadı. Lütfen bilgilerinizi kontrol edip tekrar deneyin.';
    }
  }

  async function unlock() {
    if (!canSubmit) return;
    if (mode === 'recover' && newPass !== confirmNewPass) {
      error = 'Yeni parolalar birbiriyle eşleşmiyor.';
      return;
    }
    error = null;
    loading = true;
    try {
      if (mode === 'pass') {
        await authStore.loadIdentity(pass, rememberMe);
      } else if (mode === 'credentials') {
        await authStore.loginWithCredentials(username.trim().toLowerCase(), pass, rememberMe);
      } else {
        const targetUser = username.trim() || hint?.username || undefined;
        await authStore.recoverIdentity(recoveryCode.trim(), newPass, targetUser);
      }
      recoveryAcknowledged.set(true);
    } catch (err) {
      handleError(err);
    } finally {
      loading = false;
    }
  }

  function switchToRecover() {
    mode = 'recover';
    error = null;
  }

  function switchToCredentials() {
    mode = 'credentials';
    error = null;
  }

  function switchToPass() {
    mode = 'pass';
    error = null;
  }

  function backFromRecover() {
    if (initialMode === 'recover') {
      onBack();
    } else if (hint?.hasIdentity) {
      switchToPass();
    } else {
      switchToCredentials();
    }
  }
</script>

<div class="veil-unlock-card">
  <div class="veil-unlock-header">
    {#if mode === 'pass' && hint?.hasIdentity}
      <div class="veil-unlock-avatar-wrap">
        <Avatar name={(hint.displayName || hint.username) ?? 'veilanon'} size="xl" hash={hint.avatarHash ?? null} />
      </div>
      <h2 class="veil-unlock-title">{hint.displayName || hint.username}</h2>
      {#if hint.username}
        <div class="veil-unlock-tag-badge">
          <span class="veil-unlock-tag">@{hint.username}</span>
        </div>
      {/if}
    {:else if mode === 'credentials'}
      <div class="veil-lock-icon" aria-hidden="true">
        <Icon name="user" size={32} />
      </div>
      <h2 class="veil-unlock-title">Mevcut Kimliğinle Giriş Yap</h2>
      <p class="veil-unlock-desc">Kullanıcı adını ve parolanı girerek hesabını bu cihaza güvenle yükle.</p>
    {:else}
      <div class="veil-lock-icon" aria-hidden="true">
        <Icon name="shield" size={32} />
      </div>
      <h2 class="veil-unlock-title">Parolayı Sıfırla & Hesabı Kurtar</h2>
      <p class="veil-unlock-desc">Kurtarma kodunuz ile hesabınıza yeni bir parola belirleyerek hesabınızı kurtarın.</p>
    {/if}
  </div>

  {#if error}
    <div class="veil-alert-error" role="alert">
      <Icon name="warning" size={16} />
      <span>{error}</span>
    </div>
  {/if}

  {#if mode === 'pass'}
    <form onsubmit={(e) => { e.preventDefault(); unlock(); }}>
      <div class="veil-form-group">
        <label class="veil-form-label" for="unlockPass">Parola</label>
        <div class="veil-pass-field">
          <!-- svelte-ignore a11y_autofocus -->
          <input
            id="unlockPass"
            class="veil-input"
            type={showPass ? 'text' : 'password'}
            bind:value={pass}
            placeholder="Parolanı gir"
            autocomplete="current-password"
            spellcheck={false}
            autofocus
            required
          />
          <button
            type="button"
            class="btn-icon veil-pass-toggle"
            title={showPass ? 'Parolayı gizle' : 'Parolayı göster'}
            aria-label={showPass ? 'Parolayı gizle' : 'Parolayı göster'}
            onclick={() => (showPass = !showPass)}
          >
            <Icon name={showPass ? 'eye-off' : 'eye'} size={16} />
          </button>
        </div>
      </div>

      <label class="veil-remember-box">
        <input type="checkbox" bind:checked={rememberMe} />
        <span>Bu cihazda beni hatırla (Açılışta parola sorma)</span>
      </label>

      <div class="veil-form-row">
        <button type="button" class="btn btn-secondary" onclick={onBack}>
          <Icon name="arrow-left" size={16} />
          Geri
        </button>
        <button type="submit" class="btn btn-primary" disabled={!canSubmit}>
          {#if loading}
            <div class="veil-spinner veil-spinner-sm" aria-hidden="true"></div>
            Açılıyor…
          {:else if cooldown > 0}
            <Icon name="lock" size={16} />
            Bekle ({cooldown} sn)
          {:else}
            Kilidi Aç
          {/if}
        </button>
      </div>
    </form>

    <div class="veil-unlock-links">
      <button type="button" class="veil-recovery-link" onclick={switchToCredentials}>
        <Icon name="user" size={14} />
        Farklı bir hesapla giriş yap
      </button>
      <button type="button" class="veil-recovery-link" onclick={switchToRecover}>
        <Icon name="key" size={14} />
        Parolanı mı unuttun? Kurtarma kodu ile sıfırla
      </button>
      {#if onReset}
        <button type="button" class="veil-reset-link" onclick={onReset}>
          <Icon name="trash" size={14} />
          Kod da mı yok? Cihazı sıfırla
        </button>
      {/if}
    </div>

  {:else if mode === 'credentials'}
    <form onsubmit={(e) => { e.preventDefault(); unlock(); }}>
      <div class="veil-form-group">
        <label class="veil-form-label" for="loginUsername">Kullanıcı Adı</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="loginUsername"
          class="veil-input"
          type="text"
          bind:value={username}
          placeholder="kullaniciadi"
          autocomplete="username"
          spellcheck={false}
          autofocus
          required
        />
      </div>

      <div class="veil-form-group">
        <label class="veil-form-label" for="loginPass">Parola</label>
        <div class="veil-pass-field">
          <input
            id="loginPass"
            class="veil-input"
            type={showPass ? 'text' : 'password'}
            bind:value={pass}
            placeholder="Parolanı gir (en az 8 karakter)"
            autocomplete="current-password"
            spellcheck={false}
            required
          />
          <button
            type="button"
            class="btn-icon veil-pass-toggle"
            title={showPass ? 'Parolayı gizle' : 'Parolayı göster'}
            aria-label={showPass ? 'Parolayı gizle' : 'Parolayı göster'}
            onclick={() => (showPass = !showPass)}
          >
            <Icon name={showPass ? 'eye-off' : 'eye'} size={16} />
          </button>
        </div>
      </div>

      <label class="veil-remember-box">
        <input type="checkbox" bind:checked={rememberMe} />
        <span>Bu cihazda beni hatırla (Açılışta parola sorma)</span>
      </label>

      <div class="veil-form-row">
        <button type="button" class="btn btn-secondary" onclick={onBack}>
          <Icon name="arrow-left" size={16} />
          Geri
        </button>
        <button type="submit" class="btn btn-primary" disabled={!canSubmit}>
          {#if loading}
            <div class="veil-spinner veil-spinner-sm" aria-hidden="true"></div>
            Giriş Yapılıyor…
          {:else if cooldown > 0}
            <Icon name="lock" size={16} />
            Bekle ({cooldown} sn)
          {:else}
            Giriş Yap
          {/if}
        </button>
      </div>
    </form>

    <div class="veil-unlock-links">
      {#if hint?.hasIdentity}
        <button type="button" class="veil-recovery-link" onclick={switchToPass}>
          <Icon name="arrow-left" size={14} />
          Cihazdaki kayıtlı hesaba dön
        </button>
      {/if}
      <button type="button" class="veil-recovery-link" onclick={switchToRecover}>
        <Icon name="shield" size={14} />
        Parolanı mı unuttun? Kurtarma kodu ile sıfırla
      </button>
    </div>

  {:else}
    <form onsubmit={(e) => { e.preventDefault(); unlock(); }}>
      <div class="veil-form-group">
        <label class="veil-form-label" for="recoveryUser">Kullanıcı Adı</label>
        <input
          id="recoveryUser"
          class="veil-input"
          type="text"
          bind:value={username}
          placeholder={hint?.username ?? 'kullaniciadi'}
          autocomplete="username"
          spellcheck={false}
          required={!hint?.username}
        />
        <span class="veil-form-desc">Kurtarmak istediğiniz hesabın kullanıcı adı</span>
      </div>

      <div class="veil-form-group">
        <label class="veil-form-label" for="recoveryCode">Kurtarma Kodu (Acil Durum Kiti)</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="recoveryCode"
          class="veil-input veil-mono-input"
          type="text"
          bind:value={recoveryCode}
          placeholder="Kurtarma kodunuzu girin veya yapıştırın"
          autocomplete="off"
          spellcheck={false}
          autofocus
          required
        />
      </div>

      <div class="veil-form-group">
        <label class="veil-form-label" for="newPass">Yeni Parola Belirleyin</label>
        <div class="veil-pass-field">
          <input
            id="newPass"
            class="veil-input"
            type={showNewPass ? 'text' : 'password'}
            bind:value={newPass}
            placeholder="En az 8 karakter"
            autocomplete="new-password"
            spellcheck={false}
            required
          />
          <button
            type="button"
            class="btn-icon veil-pass-toggle"
            title={showNewPass ? 'Parolayı gizle' : 'Parolayı göster'}
            aria-label={showNewPass ? 'Parolayı gizle' : 'Parolayı göster'}
            onclick={() => (showNewPass = !showNewPass)}
          >
            <Icon name={showNewPass ? 'eye-off' : 'eye'} size={16} />
          </button>
        </div>
      </div>

      <div class="veil-form-group">
        <label class="veil-form-label" for="confirmNewPass">Yeni Parolayı Doğrulayın</label>
        <div class="veil-pass-field">
          <input
            id="confirmNewPass"
            class="veil-input"
            type={showNewPass ? 'text' : 'password'}
            bind:value={confirmNewPass}
            placeholder="Yeni parolayı tekrar girin"
            autocomplete="new-password"
            spellcheck={false}
            required
          />
        </div>
        {#if passMismatch}
          <span class="veil-form-error">Parolalar eşleşmiyor!</span>
        {/if}
        <p class="veil-form-desc">Kurtarma kodu doğrulandığında yeni parolanız kaydedilir ve hesabınıza giriş yapılır.</p>
      </div>

      <div class="veil-form-row">
        <button type="button" class="btn btn-secondary" onclick={backFromRecover}>
          <Icon name="arrow-left" size={16} />
          Geri
        </button>
        <button type="submit" class="btn btn-primary" disabled={!canSubmit}>
          {#if loading}
            <div class="veil-spinner veil-spinner-sm" aria-hidden="true"></div>
            Sıfırlanıyor…
          {:else}
            <Icon name="shield" size={16} />
            Parolayı Sıfırla ve Giriş Yap
          {/if}
        </button>
      </div>
    </form>
  {/if}
</div>

<style>
  .veil-remember-box {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    cursor: pointer;
    user-select: none;
  }
  .veil-remember-box input[type="checkbox"] {
    accent-color: var(--veil-brand);
    width: 16px;
    height: 16px;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .veil-remember-box:hover {
    color: var(--veil-text-primary);
  }
  .veil-unlock-links {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-3);
    align-items: center;
  }
</style>

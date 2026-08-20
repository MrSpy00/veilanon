<script module lang="ts">
  import { writable } from 'svelte/store';

  /**
   * Set to true once the recovery code has been acknowledged.
   * authStore flips isAuthenticated at the end of createIdentity, so the
   * AppLayout gate in +page.svelte must also wait for this flag —
   * otherwise the recovery step would unmount before it is ever shown.
   */
  export const recoveryAcknowledged = writable(false);
</script>

<script lang="ts">
  import { prefersReducedMotion } from 'svelte/motion';
  import { fly } from 'svelte/transition';
  import { authStore } from '$lib/stores/auth';
  import Icon from '$lib/components/ui/Icon.svelte';
  import AppLogo from '$lib/components/ui/AppLogo.svelte';
  import { toastStore } from '$lib/stores/notifications';
  import CreateIdentity from './CreateIdentity.svelte';
  import UnlockIdentity from './UnlockIdentity.svelte';
  import RecoveryCode from './RecoveryCode.svelte';

  type Step = 'welcome' | 'create' | 'unlock' | 'recovery' | 'reset';
  let step = $state<Step>('welcome');
  let recoveryCode = $state<string | null>(null);
  let resetConfirm = $state('');
  let resetLoading = $state(false);
  let resetError = $state<string | null>(null);
  /** Unlock ekranı hangi modda açılacak: parola veya kurtarma kodu. */
  let unlockMode = $state<'pass' | 'recover'>('pass');

  // Identity exists → only the unlock path is offered; creating a second
  // identity on this device is intentionally blocked by the backend too.
  const hasIdentity = $derived(!!$authStore.identityHint?.hasIdentity);

  const stepTransition = $derived(
    prefersReducedMotion.current ? { duration: 0, y: 0 } : { duration: 160, y: 10 }
  );

  function go(where: Step) {
    step = where;
    resetConfirm = '';
    resetError = null;
  }

  function goUnlock(mode: 'pass' | 'recover') {
    unlockMode = mode;
    step = 'unlock';
  }

  function handleCreated(code: string | null) {
    if (!code) {
      // No code surfaced from the backend — do not block the user.
      recoveryAcknowledged.set(true);
      return;
    }
    recoveryCode = code;
    step = 'recovery';
  }

  async function performReset() {
    if (resetConfirm.trim().toLocaleLowerCase('tr') !== 'sıfırla') return;
    resetLoading = true;
    resetError = null;
    try {
      await authStore.resetIdentity();
      toastStore.success('Bu cihaz sıfırlandı — artık yeni bir kimlik oluşturabilirsin.');
      go('welcome');
    } catch (err) {
      resetError = String(err).replace('Error: ', '');
    } finally {
      resetLoading = false;
    }
  }
</script>

<div class="veil-onboarding" role="main">
  {#key step}
    <div class="veil-onboarding-card" transition:fly={stepTransition}>
      {#if step === 'welcome'}
        <div class="veil-onboarding-logo" aria-hidden="true">
          <AppLogo size={88} radius={24} />
        </div>
        <h1 class="veil-onboarding-title" id="welcome-title">veilanon</h1>
        <p class="veil-onboarding-subtitle">
          Gizliliğe öncelik veren, uçtan uca şifreli açık kaynaklı iletişim platformu.<br />
          Hiçbir şey sunucu tarafından okunamaz.
        </p>

        <div class="veil-onboarding-actions">
          {#if hasIdentity}
            <div class="veil-alert-info veil-identity-exists" role="status">
              <Icon name="info" size={16} />
              <span>
                Bu cihazda <strong>{$authStore.identityHint?.displayName ?? $authStore.identityHint?.username ?? 'bir kimlik'}</strong> kayıtlı.
                Kayıtlı hesabına giriş yapabilir veya sıfırdan yeni bir kimlik oluşturabilirsin.
              </span>
            </div>
          {/if}

          <button class="btn btn-primary btn-lg" onclick={() => go('create')}>
            <Icon name="plus" size={18} />
            Yeni Kimlik Oluştur
          </button>

          <button class="btn btn-secondary btn-lg" onclick={() => goUnlock('pass')}>
            <Icon name="key" size={18} />
            Mevcut Kimliğimle Giriş Yap
          </button>

          <button class="btn btn-secondary btn-lg" onclick={() => goUnlock('recover')}>
            <Icon name="shield" size={18} />
            Kurtarma Kodu ile Giriş
          </button>

          {#if hasIdentity}
            <button type="button" class="veil-reset-link" onclick={() => go('reset')}>
              <Icon name="trash" size={14} />
              <span>Parolayı mı unuttun? Cihazı sıfırla</span>
            </button>
          {/if}
        </div>

        <div class="veil-onboarding-footer-card">
          <Icon name="shield" size={15} />
          <span>Tüm veriler cihazında şifreli saklanır. Sunucu mesaj içeriğini asla göremez.</span>
        </div>

      {:else if step === 'create'}
        <CreateIdentity
          onBack={() => go('welcome')}
          onCreated={handleCreated}
          identityExists={hasIdentity}
        />

      {:else if step === 'unlock'}
        <UnlockIdentity
          onBack={() => go('welcome')}
          onReset={() => go('reset')}
          identityHint={$authStore.identityHint}
          initialMode={unlockMode}
        />

      {:else if step === 'reset'}
        <div class="veil-onboarding-logo" aria-hidden="true">
          <Icon name="warning" size={28} />
        </div>
        <h1 class="veil-onboarding-title" id="reset-title">Bu Cihazı Sıfırla</h1>
        <p class="veil-onboarding-subtitle">
          Parolanı ve kurtarma kodunu da kaybettiysen, şifreleme tasarımı gereği kimliğine
          erişmek mümkün değildir. Cihazı sıfırlamak <strong>tüm yerel verileri kalıcı olarak
          siler</strong> (mesajlar, alanlar, anahtarlar) ve yeni bir kimlikle baştan başlarsın.
        </p>

        {#if resetError}
          <div class="veil-alert-error" role="alert">
            <Icon name="warning" size={16} />
            <span>{resetError}</span>
          </div>
        {/if}

        <div class="veil-recovery-warning" role="alert">
          <Icon name="warning" size={16} />
          <span>Bu işlem geri alınamaz. Emin değilsen önce kurtarma kodunu dene.</span>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); performReset(); }}>
          <div class="veil-form-group">
            <label class="veil-form-label" for="resetConfirm">Onaylamak için aşağıya "sıfırla" yaz</label>
            <!-- svelte-ignore a11y_autofocus — confirmation field is the only interactive step -->
            <input
              id="resetConfirm"
              class="veil-input"
              type="text"
              bind:value={resetConfirm}
              placeholder="sıfırla"
              autocomplete="off"
              spellcheck={false}
              autofocus
              required
            />
          </div>
          <div class="veil-form-row">
            <button type="button" class="btn btn-secondary" onclick={() => go('welcome')}>
              <Icon name="arrow-left" size={16} />
              Geri
            </button>
            <button
              type="submit"
              class="btn btn-danger"
              disabled={resetConfirm.trim().toLocaleLowerCase('tr') !== 'sıfırla' || resetLoading}
            >
              {#if resetLoading}
                <div class="veil-spinner veil-spinner-sm" aria-hidden="true"></div>
                Sıfırlanıyor…
              {:else}
                <Icon name="trash" size={16} />
                Cihazı Sıfırla
              {/if}
            </button>
          </div>
        </form>

      {:else if step === 'recovery' && recoveryCode}
        <RecoveryCode code={recoveryCode} onContinue={() => recoveryAcknowledged.set(true)} />

      {:else if step === 'recovery'}
        <div class="veil-onboarding-logo" aria-hidden="true">
          <Icon name="shield" size={24} />
        </div>
        <h1 class="veil-onboarding-title" id="recovery-help-title">Kurtarma</h1>
        <p class="veil-onboarding-subtitle">
          Kurtarma kodu yalnızca kimlik oluşturulurken bir kez gösterilir.
        </p>
        <div class="veil-recovery-warning" role="note">
          <Icon name="warning" size={16} />
          <span>
            Parolanı ve kurtarma kodunu kaybettiysen, şifreleme tasarımı gereği kimliğine
            erişmek mümkün değildir — bu cihazda yeni bir kimlik oluşturman gerekir.
          </span>
        </div>
        <div class="veil-form-row">
          <button type="button" class="btn btn-secondary" onclick={() => go('welcome')}>
            <Icon name="arrow-left" size={16} />
            Geri
          </button>
        </div>
      {/if}
    </div>
  {/key}
</div>

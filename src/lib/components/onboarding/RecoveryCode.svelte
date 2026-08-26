<script lang="ts">
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toastStore } from '$lib/stores/notifications';
  import { copyText } from '$lib/utils/clipboard';

  let {
    code,
    onContinue,
  }: {
    code: string;
    onContinue: () => void;
  } = $props();

  async function copyCode() {
    const success = await copyText(code);
    if (success) {
      toastStore.success('Kurtarma kodu kopyalandı — güvenli bir yere kaydet!');
    } else {
      toastStore.error('Kopyalanamadı. Kodu elle not al.');
    }
  }
</script>

<div class="veil-onboarding-logo success" aria-hidden="true">
  <Icon name="check" size={28} strokeWidth={2.2} />
</div>
<h1 class="veil-onboarding-title" id="recovery-title">Kimliğin Hazır!</h1>
<p class="veil-onboarding-subtitle">
  Kurtarma kodunu güvenli bir yere kaydet. Bu kodu kaybedersen ve parolanı unutursan verilerine erişemezsin.
</p>

<div class="veil-recovery-box" aria-label="Kurtarma kodu">
  <code class="veil-selectable">{code}</code>
</div>
<button class="btn btn-secondary veil-recovery-copy" onclick={copyCode}>
  <Icon name="copy" size={16} />
  Kopyala
</button>

<div class="veil-recovery-warning" role="alert">
  <Icon name="warning" size={16} />
  <span>Bu kodu şimdi kaydet. Bir daha gösterilmeyecek.</span>
</div>

<button class="btn btn-primary btn-lg veil-recovery-continue" onclick={onContinue}>
  Devam Et
  <Icon name="arrow-right" size={18} />
</button>

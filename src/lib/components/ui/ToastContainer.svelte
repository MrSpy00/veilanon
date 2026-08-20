<script lang="ts">
  import { toastStore } from '$lib/stores/notifications';
  import Toast from './Toast.svelte';

  const toasts = $derived($toastStore);
</script>

{#if toasts.length > 0}
  <div class="veil-toast-container" aria-label="Bildirimler">
    {#each toasts as toast (toast.id)}
      <Toast {toast} />
    {/each}
  </div>
{/if}

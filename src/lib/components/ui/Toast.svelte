<script lang="ts">
  import type { Toast } from '$lib/stores/notifications';
  import { toastStore } from '$lib/stores/notifications';

  let { toast }: { toast: Toast } = $props();

  const icons: Record<Toast['type'], string> = {
    success: '✓',
    error: '✕',
    warning: '⚠',
    info: 'ℹ',
  };
</script>

<div class="veil-toast {toast.type}" role="status" aria-live="polite">
  <span class="veil-toast-icon" aria-hidden="true">{icons[toast.type]}</span>
  <span class="veil-toast-message">{toast.message}</span>
  <button class="btn-icon veil-toast-close" onclick={() => toastStore.remove(toast.id)} aria-label="Bildirimi kapat">✕</button>
</div>

<style>
  .veil-toast-icon { flex-shrink: 0; font-weight: 700; }
  .veil-toast-message { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .veil-toast-close { padding: var(--space-1); }
</style>

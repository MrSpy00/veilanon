<script lang="ts">
  let {
    variant = 'danger',
    value = null,
    label = '',
    title = '',
  }: {
    variant?: 'danger' | 'brand' | 'success' | 'e2ee';
    value?: number | string | null;
    label?: string;
    title?: string;
  } = $props();

  const display = $derived(value !== null && value !== '' ? (typeof value === 'number' && value > 99 ? '99+' : String(value)) : label);
</script>

{#if display}
  <span
    class="veil-badge {variant === 'brand' ? 'brand' : variant === 'success' ? 'success' : ''} {variant === 'e2ee' ? 'e2ee' : ''}"
    {title}
  >
    {display}
  </span>
{/if}

<style>
  .veil-badge.e2ee {
    background: hsl(142 71% 45% / 0.15);
    color: var(--veil-e2ee);
    border: 1px solid hsl(142 71% 45% / 0.25);
  }
</style>

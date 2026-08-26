<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    variant = 'primary',
    size = 'md',
    type = 'button',
    disabled = false,
    title = '',
    class: className = '',
    ariaLabel = '',
    onclick,
    children,
  }: {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    type?: 'button' | 'submit' | 'reset';
    disabled?: boolean;
    title?: string;
    class?: string;
    ariaLabel?: string;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
  } = $props();
</script>

<button
  class="btn btn-{variant} {size !== 'md' ? `btn-${size}` : ''} {className}"
  {type}
  {disabled}
  {title}
  aria-label={ariaLabel || undefined}
  onclick={onclick}
>
  {@render children?.()}
</button>

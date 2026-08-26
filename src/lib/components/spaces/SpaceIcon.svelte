<script lang="ts">
  import { identityApi } from '$lib/api/tauri';

  let {
    name = '',
    iconHash = null,
    size = 'md',
    active = false,
    onClick,
    ariaLabel = '',
  }: {
    name?: string;
    iconHash?: string | null;
    size?: 'md' | 'lg';
    active?: boolean;
    onClick?: () => void;
    ariaLabel?: string;
  } = $props();

  let resolvedSrc = $state<string | null>(null);

  $effect(() => {
    resolvedSrc = null;
    if (!iconHash || typeof iconHash !== 'string' || !iconHash.trim()) return;
    const cleanHash = iconHash.trim();
    if (cleanHash.startsWith('data:') || cleanHash.startsWith('http://') || cleanHash.startsWith('https://')) {
      resolvedSrc = cleanHash;
      return;
    }
    let cancelled = false;
    identityApi.getAvatar(cleanHash).then((dataUrl) => {
      if (!cancelled && dataUrl) resolvedSrc = dataUrl;
    }).catch(() => {});
    return () => { cancelled = true; };
  });

  const initials = $derived(name ? name.trim().substring(0, 2).toUpperCase() : '?');
</script>

<button
  class="veil-space-icon {size === 'lg' ? 'veil-space-icon-lg' : ''}"
  class:active
  title={name}
  aria-label={ariaLabel || name}
  aria-pressed={active}
  onclick={onClick}
>
  {#if resolvedSrc}
    <img class="veil-space-icon-img" src={resolvedSrc} alt={name} loading="lazy" />
  {:else}
    <span class="veil-space-icon-initials">{initials}</span>
  {/if}
</button>

<style>
  .veil-space-icon-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: inherit;
    display: block;
  }
  .veil-space-icon-lg {
    width: 56px;
    height: 56px;
    font-size: var(--text-lg);
  }
  .veil-space-icon-initials {
    font-weight: 700;
    user-select: none;
    letter-spacing: -0.02em;
  }
</style>

<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { renderMarkdown, setupSpoilerReveal } from '$lib/utils/markdown';
  import { trustedDomainsStore } from '$lib/stores/trustedDomains';
  import ExternalLinkModal from './ExternalLinkModal.svelte';

  let { content = '' }: { content?: string } = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let linkModalOpen = $state(false);
  let selectedUrl = $state('');

  $effect(() => {
    const el = containerEl;
    if (!el) return;
    const cleanupSpoiler = setupSpoilerReveal(el);

    const onLinkClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      const anchor = target?.closest('a') as HTMLAnchorElement | null;
      if (!anchor || !el.contains(anchor)) return;

      const url = anchor.getAttribute('data-external-url') || anchor.href;
      if (!url) return;

      e.preventDefault();
      e.stopPropagation();

      if (trustedDomainsStore.shouldDirectRedirect(url)) {
        openUrl(url).catch(() => {
          window.open(url, '_blank', 'noopener,noreferrer');
        });
      } else {
        selectedUrl = url;
        linkModalOpen = true;
      }
    };

    el.addEventListener('click', onLinkClick);

    return () => {
      cleanupSpoiler?.();
      el.removeEventListener('click', onLinkClick);
    };
  });
</script>

<div class="veil-markdown" bind:this={containerEl}>
  <!-- renderMarkdown escapes ALL user content — never raw HTML. -->
  {@html renderMarkdown(content)}
</div>

<ExternalLinkModal
  open={linkModalOpen}
  url={selectedUrl}
  onClose={() => { linkModalOpen = false; selectedUrl = ''; }}
/>

<style>
  .veil-markdown {
    white-space: pre-wrap;
    word-break: break-word;
    overflow-wrap: anywhere;
    line-height: 1.32;
  }
  .veil-markdown :global(a) {
    color: var(--veil-brand);
    text-decoration: none;
    cursor: pointer;
    transition: color var(--t-fast), text-decoration var(--t-fast);
  }
  .veil-markdown :global(a:hover) {
    text-decoration: underline;
    color: var(--veil-brand-hover);
  }
  .veil-markdown :global(a.trusted) {
    color: var(--veil-brand);
  }
  .veil-markdown :global(pre) { white-space: pre; }
</style>

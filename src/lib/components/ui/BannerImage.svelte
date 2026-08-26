<script lang="ts" module>
  const bannerMemoryCache = new Map<string, string>();

  export function cacheBanner(hash: string, dataUrl: string) {
    if (hash && dataUrl) {
      bannerMemoryCache.set(hash.trim(), dataUrl);
    }
  }

  export function removeBannerCache(hash: string | null | undefined) {
    if (hash) {
      bannerMemoryCache.delete(hash.trim());
    }
  }
</script>

<script lang="ts">
  import { identityApi } from '$lib/api/tauri';

  let {
    hash = null,
    alt = '',
    class: className = '',
  }: {
    hash?: string | null;
    alt?: string;
    class?: string;
  } = $props();

  let resolvedSrc = $state<string | null>(null);
  let isLoaded = $state<boolean>(false);
  let hasError = $state(false);
  let lastHash = $state<string | null>(null);

  $effect(() => {
    const cleanHash = (hash && typeof hash === 'string') ? hash.trim() : null;
    if (cleanHash === lastHash) {
      return;
    }
    lastHash = cleanHash;

    if (!cleanHash) {
      resolvedSrc = null;
      isLoaded = false;
      hasError = false;
      return;
    }

    if (cleanHash.startsWith('data:') || cleanHash.startsWith('http://') || cleanHash.startsWith('https://')) {
      resolvedSrc = cleanHash;
      isLoaded = true;
      hasError = false;
      return;
    }

    const cached = bannerMemoryCache.get(cleanHash);
    if (cached) {
      resolvedSrc = cached;
      isLoaded = true;
      hasError = false;
      return;
    }

    // Keep current banner visible while fetching new one — no flicker gap
    let cancelled = false;
    hasError = false;
    identityApi.getAvatar(cleanHash).then((dataUrl) => {
      if (cancelled) return;
      if (dataUrl) {
        bannerMemoryCache.set(cleanHash, dataUrl);
        resolvedSrc = dataUrl;
        isLoaded = true;
        hasError = false;
      } else {
        // Keep previous image on empty response; mark transient error
        hasError = true;
      }
    }).catch(() => {
      if (!cancelled) {
        hasError = true;
      }
    });
    return () => { cancelled = true; };
  });

  const isVideo = $derived(
    !!resolvedSrc && (
      resolvedSrc.startsWith('data:video/') ||
      resolvedSrc.includes('.mp4') ||
      resolvedSrc.includes('.webm') ||
      resolvedSrc.includes('.mov')
    )
  );
</script>

<div class="veil-banner-wrapper {className}">
  <div class="veil-banner-fallback" aria-hidden="true"></div>
  {#if resolvedSrc && !hasError}
    {#if isVideo}
      <video
        src={resolvedSrc}
        autoplay
        loop
        muted
        playsinline
        disablepictureinpicture
        class="veil-banner-img loaded"
      ></video>
    {:else}
      <img
        src={resolvedSrc}
        {alt}
        loading="eager"
        class="veil-banner-img loaded"
        onload={() => { isLoaded = true; hasError = false; }}
        onerror={() => (hasError = true)}
      />
    {/if}
  {/if}
</div>

<style>
  .veil-banner-wrapper {
    width: 100%;
    height: 100%;
    min-height: 100px;
    position: relative;
    overflow: hidden;
    display: block;
    background-color: var(--veil-bg-surface, #1e1f22);
  }
  .veil-banner-fallback {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background:
      radial-gradient(120% 160% at 15% 0%, var(--veil-brand-subtle, rgba(88, 101, 242, 0.15)), transparent 55%),
      linear-gradient(160deg, var(--veil-bg-surface, #2b2d31), var(--veil-bg-void, #111214));
    z-index: 1;
  }
  .veil-banner-wrapper img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    display: block;
    opacity: 1;
    z-index: 2;
    transition: opacity 0.2s ease;
  }
</style>

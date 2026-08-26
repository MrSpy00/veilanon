<script lang="ts" module>
  const avatarMemoryCache = new Map<string, { url: string; at: number }>();
  const AVATAR_TTL_MS = 5 * 60 * 1000;

  export function cacheAvatar(hash: string, dataUrl: string) {
    if (hash && dataUrl) {
      avatarMemoryCache.set(hash.trim(), { url: dataUrl, at: Date.now() });
    }
  }

  export function removeAvatarCache(hash: string | null | undefined) {
    if (hash) {
      avatarMemoryCache.delete(hash.trim());
    }
  }

  export function clearAllAvatarCache() {
    avatarMemoryCache.clear();
  }

  function getCachedAvatar(hash: string): string | null {
    const e = avatarMemoryCache.get(hash);
    if (!e) return null;
    if (Date.now() - e.at > AVATAR_TTL_MS) {
      avatarMemoryCache.delete(hash);
      return null;
    }
    return e.url;
  }
</script>

<script lang="ts">
  import { identityApi } from '$lib/api/tauri';

  let {
    name = 'veilanon',
    size = 'md',
    src = null,
    hash = null,
    presence = null,
    speaking = false,
    themeColor = null,
  }: {
    name?: string;
    size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl';
    src?: string | null | undefined;
    hash?: string | null | undefined;
    presence?: 'online' | 'away' | 'dnd' | 'offline' | 'invisible' | null | undefined;
    /** Konuşma halkası — tema vurgu renginde dış çerçeve (ses kanalları). */
    speaking?: boolean;
    /** Konuşma halkası rengi (rol rengi veya tema rengi). */
    themeColor?: string | null;
  } = $props();

  let resolvedSrc = $state<string | null>(null);
  let resolveFailed = $state(false);
  let isLoading = $state(false);

  const initials = $derived((name ?? 'veilanon').trim().slice(0, 2).toUpperCase() || 'V');
  const directSrc = $derived(
    src ||
    (hash && (hash.startsWith('http://') || hash.startsWith('https://') || hash.startsWith('data:'))
      ? hash
      : null)
  );
  const imgSrc = $derived(resolvedSrc ?? directSrc);

  // Avatar dosyaları IPC üzerinden data URL olarak yüklenir (yerel önbellek + Supabase).
  $effect(() => {
    if (!hash || typeof hash !== 'string' || !hash.trim()) {
      resolvedSrc = null;
      resolveFailed = false;
      return;
    }
    const cleanHash = hash.trim();
    if (cleanHash.startsWith('http://') || cleanHash.startsWith('https://') || cleanHash.startsWith('data:')) {
      resolvedSrc = cleanHash;
      resolveFailed = false;
      return;
    }

    const cached = getCachedAvatar(cleanHash);
    if (cached) {
      resolvedSrc = cached;
      resolveFailed = false;
      return;
    }

    let cancelled = false;
    isLoading = true;
    // Keep previous src until new one resolves to avoid flicker
    identityApi.getAvatar(cleanHash).then((dataUrl) => {
      if (!cancelled && dataUrl) {
        cacheAvatar(cleanHash, dataUrl);
        resolvedSrc = dataUrl;
        resolveFailed = false;
      }
      if (!cancelled) isLoading = false;
    }).catch(() => {
      if (!cancelled) {
        resolveFailed = true;
        isLoading = false;
      }
    });
    return () => { cancelled = true; };
  });

  const presenceLabel = $derived(
    presence === 'online'
      ? 'Çevrimiçi'
      : presence === 'away'
        ? 'Uzakta'
        : presence === 'dnd'
          ? 'Rahatsız etmeyin'
          : presence === 'offline'
            ? 'Çevrimdışı'
            : presence === 'invisible'
              ? 'Görünmez'
              : null
  );
  const isVideo = $derived(
    !!imgSrc && (
      imgSrc.startsWith('data:video/') ||
      imgSrc.includes('.mp4') ||
      imgSrc.includes('.webm') ||
      imgSrc.includes('.mov')
    )
  );
</script>

<div
  class="veil-avatar veil-avatar-{size} {(imgSrc && !resolveFailed) ? '' : 'veil-avatar-fallback'}"
  class:has-presence={presence}
  class:speaking
  style={speaking && themeColor ? `--speaking-color: ${themeColor};` : ''}
  role="img"
  aria-label={name}
>
  <div class="veil-avatar-inner">
    {#if imgSrc && !resolveFailed}
      {#if isVideo}
        <video class="veil-avatar-img" src={imgSrc} autoplay loop muted playsinline disablepictureinpicture></video>
      {:else}
        <img class="veil-avatar-img" src={imgSrc} alt={name} />
      {/if}
    {:else if isLoading}
      <div class="veil-avatar-skeleton" aria-hidden="true"></div>
    {:else}
      {initials}
    {/if}
  </div>
  {#if presence}
    <span class="veil-presence {presence}" aria-hidden="true"></span>
    <span class="veil-sr-only">{presenceLabel}</span>
  {/if}
</div>

<style>
  .veil-avatar-inner {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .veil-avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: inherit;
    animation: veil-avatar-fade 0.22s ease;
  }
  @keyframes veil-avatar-fade {
    from { opacity: 0; transform: scale(0.98); }
    to { opacity: 1; transform: scale(1); }
  }
  .veil-avatar.speaking {
    box-shadow: 0 0 0 2px var(--veil-bg-channel, var(--veil-bg-base)), 0 0 0 4px var(--speaking-color, var(--veil-brand));
  }
  .veil-avatar-skeleton {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--veil-bg-surface) 25%, var(--veil-bg-elevated) 50%, var(--veil-bg-surface) 75%);
    background-size: 200% 100%;
    animation: skeleton-shimmer 1.4s ease-in-out infinite;
  }
  @keyframes skeleton-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  .veil-sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
</style>

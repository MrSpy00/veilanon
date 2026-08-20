<script lang="ts" module>
  const avatarMemoryCache = new Map<string, string>();
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

    const cached = avatarMemoryCache.get(cleanHash);
    if (cached) {
      resolvedSrc = cached;
      resolveFailed = false;
      return;
    }

    let cancelled = false;
    identityApi.getAvatar(cleanHash).then((dataUrl) => {
      if (!cancelled && dataUrl) {
        avatarMemoryCache.set(cleanHash, dataUrl);
        resolvedSrc = dataUrl;
        resolveFailed = false;
      }
    }).catch(() => {
      if (!cancelled) resolveFailed = true;
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
      <img class="veil-avatar-img" src={imgSrc} alt={name} />
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
  }
  .veil-avatar.speaking {
    box-shadow: 0 0 0 2px var(--veil-bg-channel, var(--veil-bg-base)), 0 0 0 4px var(--speaking-color, var(--veil-brand));
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

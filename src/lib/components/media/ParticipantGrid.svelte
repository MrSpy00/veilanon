<script lang="ts">
  import { onMount } from 'svelte';
  import type { Participant, Room } from 'livekit-client';
  import type { EffectBroadcastPayload } from '$lib/effects/types';
  import { authStore } from '$lib/stores/auth';
  import { mediaStore } from '$lib/stores/media';
  import ParticipantTile from './ParticipantTile.svelte';

  let {
    participants = [] as Participant[],
    localParticipant = null as Participant | null,
    room = null as Room | null,
    compact = false,
    viewMode = 'grid' as 'grid' | 'speaker',
    remoteEffects = new Map<string, EffectBroadcastPayload>(),
  }: {
    participants?: Participant[];
    localParticipant?: Participant | null;
    room?: Room | null;
    compact?: boolean;
    viewMode?: 'grid' | 'speaker';
    remoteEffects?: Map<string, EffectBroadcastPayload>;
  } = $props();

  const auth = $derived($authStore);
  const media = $derived($mediaStore);

  let mounted = $state(false);
  let containerWidth = $state(0);
  let containerHeight = $state(0);

  onMount(() => {
    // Enable layout transition only after the true container size is measured
    const t = setTimeout(() => {
      mounted = true;
    }, 150);
    return () => clearTimeout(t);
  });

  // Synthesize or collect all participants
  // STABLE ORDER: Kartlar konuşma/kamera durumuna göre yer değiştirmez
  // Local participant her zaman ilk sıradadır
  // Remote participantlar join sırasına (stable) göre tutulur
  const allParticipants = $derived.by(() => {
    const list: Array<{
      participant: Participant | any;
      isLocal: boolean;
      id: string;
      isSpeaking: boolean;
      hasVideo: boolean;
    }> = [];

    if (localParticipant) {
      list.push({
        participant: localParticipant,
        isLocal: true,
        id: localParticipant.identity || 'local',
        isSpeaking: localParticipant.isSpeaking || (!media.isMuted && media.isSpeaking),
        hasVideo: localParticipant.isCameraEnabled || localParticipant.isScreenShareEnabled || media.isCameraOn,
      });
    } else if (media.isInCall && auth.identity) {
      // Fallback local participant object if room is not yet fully published
      const syntheticLocal = {
        identity: auth.identity.id || 'local',
        name: auth.identity.displayName || auth.identity.username || 'Sen',
        isMicrophoneEnabled: !media.isMuted,
        isCameraEnabled: media.isCameraOn,
        isScreenShareEnabled: media.isScreenSharing,
        isSpeaking: !media.isMuted && media.isSpeaking,
        metadata: JSON.stringify({
          avatarHash: auth.identity.avatarHash,
          accentColor: 'var(--veil-brand)',
        }),
        getTrackPublication: () => null,
      };
      list.push({
        participant: syntheticLocal as any,
        isLocal: true,
        id: auth.identity.id || 'local',
        isSpeaking: !media.isMuted && media.isSpeaking,
        hasVideo: media.isCameraOn || media.isScreenSharing,
      });
    }

    for (const p of participants) {
      list.push({
        participant: p,
        isLocal: false,
        id: p.identity || p.sid,
        isSpeaking: p.isSpeaking,
        hasVideo: p.isCameraEnabled || p.isScreenShareEnabled,
      });
    }

    // STABLE ORDER: Sıralama yapılmaz — kararlı join sırası korunur
    // (konuşma/kamera açılınca kart pozisyonu değişmez)
    // Speaking/video bilgisi sadece visual indicator olarak tile içinde kullanılır

    return list;
  });


  const totalCount = $derived(allParticipants.length);

  /** Look up broadcast effect for a participant identity. */
  function lookupRemoteEffect(identity: string): EffectBroadcastPayload | null {
    return remoteEffects.get(identity) ?? null;
  }

  // Active speaker for speaker/focus mode
  const spotlightParticipant = $derived(
    allParticipants.find(p => p.isSpeaking) ?? allParticipants[0] ?? null
  );
  const filmstripParticipants = $derived(
    allParticipants.filter(p => p.id !== spotlightParticipant?.id)
  );

  /**
   * Smart 2D Fluid Adaptive Layout Engine:
   * - Dynamically evaluates horizontal AND vertical distribution
   * - Maximizes viewport coverage and area for any participant count (1 to 100+)
   * - Seamlessly adapts whether container is portrait, landscape, wide or square
   */
  const optimalLayout = $derived.by(() => {
    const n = totalCount;
    const w = Math.max(120, containerWidth || 800);
    const h = Math.max(100, containerHeight || 600);

    if (n <= 0) {
      return { cols: 1, tileW: w, tileH: h, gap: 8, scrollable: false };
    }

    const gap = compact
      ? 4
      : n > 25 ? 4 : n > 16 ? 6 : n > 9 ? 8 : n > 4 ? 10 : 12;

    // Single participant: maximize area
    if (n === 1) {
      const pad = gap * 2;
      const aspect = compact ? 1.4 : 16 / 9;
      let tileW = w - pad;
      let tileH = tileW / aspect;
      if (tileH > h - pad) {
        tileH = h - pad;
        tileW = tileH * aspect;
      }
      tileW = Math.min(tileW, w - pad);
      tileH = Math.min(tileH, h - pad);
      return {
        cols: 1,
        tileW: Math.max(100, Math.floor(tileW)),
        tileH: Math.max(80, Math.floor(tileH)),
        gap,
        scrollable: false,
      };
    }

    const isPortrait = h > w;
    const targetAspect = compact ? 1.33 : (isPortrait ? 1.25 : 16 / 9);

    if (n > 16) {
      const minTileW = compact ? 120 : 180;
      const minTileH = compact ? 90 : 135;
      const cols = Math.max(2, Math.floor((w - gap) / (minTileW + gap)));
      const actualTileW = Math.floor((w - (cols + 1) * gap) / cols);
      const actualTileH = Math.floor(actualTileW / targetAspect);
      return {
        cols,
        tileW: Math.max(minTileW, actualTileW),
        tileH: Math.max(minTileH, actualTileH),
        gap,
        scrollable: true,
      };
    }

    let bestCols = 1;
    let bestTileW = 80;
    let bestTileH = 60;
    let maxArea = 0;

    for (let c = 1; c <= n; c++) {
      const rows = Math.ceil(n / c);
      const availW = (w - (c + 1) * gap) / c;
      const availH = (h - (rows + 1) * gap) / rows;

      if (availW < 40 || availH < 35) continue;

      let tw = availW;
      let th = tw / targetAspect;

      if (th > availH) {
        th = availH;
        tw = th * targetAspect;
      }

      if (isPortrait && n <= 4) {
        let thAlt = availH;
        let twAlt = thAlt * targetAspect;
        if (twAlt > availW) {
          twAlt = availW;
          thAlt = twAlt / targetAspect;
        }
        if (twAlt * thAlt > tw * th) {
          tw = twAlt;
          th = thAlt;
        }
      }

      const area = tw * th;
      if (area > maxArea) {
        maxArea = area;
        bestCols = c;
        bestTileW = tw;
        bestTileH = th;
      }
    }

    if (maxArea === 0) {
      bestCols = Math.max(1, Math.min(n, Math.ceil(Math.sqrt(n))));
      const rows = Math.ceil(n / bestCols);
      bestTileW = Math.max(80, (w - (bestCols + 1) * gap) / bestCols);
      bestTileH = Math.max(60, (h - (rows + 1) * gap) / rows);
    }

    return {
      cols: bestCols,
      tileW: Math.max(70, Math.floor(bestTileW)),
      tileH: Math.max(55, Math.floor(bestTileH)),
      gap,
      scrollable: false,
    };
  });
</script>

<div
  class="veil-participant-grid"
  class:compact
  class:scrollable={optimalLayout.scrollable}
  bind:clientWidth={containerWidth}
  bind:clientHeight={containerHeight}
  aria-label="Katılımcılar"
>
  {#if viewMode === 'speaker' && spotlightParticipant && totalCount > 1}
      <div class="veil-speaker-mode-layout">
        <!-- Big spotlight main card -->
        <div class="veil-spotlight-tile">
          <ParticipantTile
            participant={spotlightParticipant.participant}
            isLocal={spotlightParticipant.isLocal}
            {room}
            remoteEffect={lookupRemoteEffect(spotlightParticipant.id)}
          />
      </div>

      <!-- Small filmstrip bottom/side cards -->
      {#if filmstripParticipants.length > 0}
        <div class="veil-filmstrip-strip">
          {#each filmstripParticipants as item (item.id)}
            <div class="veil-filmstrip-item">
              <ParticipantTile
                participant={item.participant}
                isLocal={item.isLocal}
                {room}
                compact
                remoteEffect={lookupRemoteEffect(item.id)}
              />
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <div
      class="veil-grid-inner"
      class:scrollable={optimalLayout.scrollable}
      style="
        --grid-cols: {optimalLayout.cols};
        --tile-w: {optimalLayout.tileW}px;
        --tile-h: {optimalLayout.tileH}px;
        --grid-gap: {optimalLayout.gap}px;
      "
    >
      {#each allParticipants as item (item.id)}
        <div class="veil-tile-container" class:animated={mounted}>
          <ParticipantTile
            participant={item.participant}
            isLocal={item.isLocal}
            {room}
            compact={compact || totalCount > 6}
            remoteEffect={lookupRemoteEffect(item.id)}
          />
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .veil-participant-grid {
    width: 100%;
    height: 100%;
    max-width: 100%;
    max-height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    place-items: center;
    place-content: center;
    overflow: hidden;
    position: relative;
    padding: var(--grid-gap, 8px);
    box-sizing: border-box;
  }

  .veil-participant-grid.scrollable {
    overflow-y: auto;
    align-items: flex-start;
  }

  .veil-grid-inner {
    display: grid;
    grid-template-columns: repeat(var(--grid-cols, 1), minmax(0, var(--tile-w, 100%)));
    grid-auto-rows: var(--tile-h, auto);
    gap: var(--grid-gap, 8px);
    align-content: center;
    justify-content: center;
    align-items: center;
    justify-items: center;
    width: 100%;
    height: 100%;
    max-width: 100%;
    max-height: 100%;
    margin: auto;
    overflow: hidden;
    box-sizing: border-box;
  }

  .veil-grid-inner.scrollable {
    height: auto;
    max-height: none;
    overflow: visible;
    padding-bottom: 72px;
  }

  .veil-tile-container {
    width: var(--tile-w, 100%);
    height: var(--tile-h, 100%);
    max-width: 100%;
    max-height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    align-items: stretch;
    justify-content: stretch;
    margin: auto;
  }

  .veil-tile-container.animated {
    transition: width 0.2s cubic-bezier(0.2, 0, 0, 1), height 0.2s cubic-bezier(0.2, 0, 0, 1);
  }

  /* Speaker / Focus Mode Layout */
  .veil-speaker-mode-layout {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    gap: 10px;
    padding-bottom: 60px;
    box-sizing: border-box;
  }

  .veil-spotlight-tile {
    flex: 1;
    min-height: 0;
    width: 100%;
    display: flex;
  }

  .veil-filmstrip-strip {
    height: 120px;
    display: flex;
    gap: 8px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 4px 0;
    flex-shrink: 0;
  }

  .veil-filmstrip-item {
    width: 170px;
    height: 100%;
    flex-shrink: 0;
  }

  .veil-participant-grid.compact {
    padding: 4px;
  }

  .veil-participant-grid.compact .veil-grid-inner {
    gap: 4px;
  }
</style>

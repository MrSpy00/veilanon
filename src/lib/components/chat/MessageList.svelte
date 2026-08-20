<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { messageStore } from '$lib/stores/messages';
  import { authStore } from '$lib/stores/auth';
  import MessageItem from './MessageItem.svelte';
  import type { Message } from '$lib/stores/messages';

  let { channelId, channelName = 'kanal' } = $props<{ channelId: string; channelName?: string }>();

  // Kanal adı UUID'ye benziyorsa (kanal bilgisi henüz yüklenmediyse) ham kimlik
  // yerine okunabilir etiket göster — ad yüklenince otomatik güncellenir.
  const safeName = $derived(
    channelName && channelName.length === 36 && channelName.includes('-')
      ? 'kanal'
      : (channelName || 'kanal')
  );

  const auth = $derived($authStore);
  const store = $derived($messageStore);
  const messages = $derived(store.byChannel[channelId] ?? []);
  const hasMore = $derived(store.hasMore[channelId] ?? false);

  let scrollEl = $state<HTMLDivElement | null>(null);
  let isAtBottom = $state(true);

  onMount(() => {
    tick().then(() => {
      if (scrollEl) {
        scrollEl.scrollTop = scrollEl.scrollHeight;
      }
    });
  });

  // Auto-scroll when new messages arrive
  $effect(() => {
    if (messages && isAtBottom && scrollEl) {
      tick().then(() => {
        scrollEl?.scrollTo({ top: scrollEl.scrollHeight, behavior: 'smooth' });
      });
    }
  });

  function onScroll() {
    if (!scrollEl) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollEl;
    isAtBottom = scrollHeight - scrollTop - clientHeight < 50;

    // Load more when scrolled to top
    if (scrollTop < 100 && hasMore) {
      const oldest = messages[0]?.id;
      if (oldest) {
        messageStore.loadMessages(channelId, oldest);
      }
    }
  }

  function getEffectiveSenderId(m: Message): string {
    if (m.isOwn || m.senderId === 'self' || m.senderId === auth.identity?.id) {
      return auth.identity?.id || 'self';
    }
    return m.senderId;
  }

  // Group consecutive messages from same sender within 5 minutes
  function shouldGroup(msg: Message, prev: Message | undefined): boolean {
    if (!prev) return false;
    if (getEffectiveSenderId(prev) !== getEffectiveSenderId(msg)) return false;
    if (showDayDivider(msg, prev)) return false;
    if (msg.replyToId) return false;
    // Group if within 5 minutes (300 seconds)
    return Math.abs(msg.createdAt - prev.createdAt) < 300;
  }

  // Format day divider
  function dayLabel(timestamp: number): string {
    const d = new Date(timestamp * 1000);
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(today.getDate() - 1);

    if (d.toDateString() === today.toDateString()) return 'Bugün';
    if (d.toDateString() === yesterday.toDateString()) return 'Dün';
    return d.toLocaleDateString('tr-TR', { year: 'numeric', month: 'long', day: 'numeric' });
  }

  function showDayDivider(msg: Message, prev: Message | undefined): boolean {
    if (!prev) return true;
    const d1 = new Date(msg.createdAt * 1000);
    const d2 = new Date(prev.createdAt * 1000);
    return d1.toDateString() !== d2.toDateString();
  }
</script>

<div
  class="veil-messages veil-selectable"
  bind:this={scrollEl}
  onscroll={onScroll}
  role="log"
  aria-label="Mesajlar"
  aria-live="polite"
>
  {#if hasMore}
    <div class="veil-load-more">
      <button
        class="btn btn-secondary btn-sm"
        onclick={() => messageStore.loadMessages(channelId, messages[0]?.id)}
      >
        Daha fazla yükle
      </button>
    </div>
  {/if}

  <!-- Welcome message for empty channel -->
  {#if messages.length === 0}
    <div class="veil-channel-welcome">
      <div class="veil-welcome-icon" aria-hidden="true">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 11.5a8.5 8.5 0 0 1-8.5 8.5c-1.5 0-3-.4-4.2-1.1L3 20l1.1-5.3a8.5 8.5 0 1 1 16.9-3.2Z"/>
          <path d="M8 11.5h.01M12 11.5h.01M16 11.5h.01"/>
        </svg>
      </div>
      <h3>Burası #{safeName} kanalının başlangıcı</h3>
      <p>İlk mesajı sen gönder — sohbeti başlat! 💬</p>
    </div>
  {/if}

  {#each messages as msg, i (msg.id)}
    {#if showDayDivider(msg, messages[i - 1])}
      <div class="veil-day-divider" role="separator" aria-label={dayLabel(msg.createdAt)}>
        {dayLabel(msg.createdAt)}
      </div>
    {/if}

    <MessageItem
      message={msg}
      grouped={shouldGroup(msg, messages[i - 1])}
      groupStart={i > 0 && !shouldGroup(msg, messages[i - 1]) && !showDayDivider(msg, messages[i - 1])}
      isOwn={msg.senderId === auth.identity?.id || msg.isOwn}
    />
  {/each}
</div>

<!-- Scroll-to-bottom button -->
{#if !isAtBottom && messages.length > 0}
  <button
    class="veil-scroll-bottom"
    onclick={() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight, behavior: 'smooth' })}
    aria-label="En alta git"
  >
    ↓ Yeni mesajlar
  </button>
{/if}

<style>
  .veil-load-more {
    display: flex;
    justify-content: center;
    padding: var(--space-4);
  }
  .veil-channel-welcome {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: var(--space-8) var(--space-6);
    color: var(--veil-text-muted);
  }
  .veil-welcome-icon {
    color: var(--veil-brand);
    background: var(--veil-brand-subtle);
    width: 72px;
    height: 72px;
    border-radius: var(--radius-2xl);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: var(--space-1);
  }
  .veil-channel-welcome h3 {
    font-size: var(--text-xl);
    font-weight: 700;
    color: var(--veil-text-primary);
  }
  .veil-scroll-bottom {
    position: absolute;
    bottom: 5rem;
    left: 50%;
    transform: translateX(-50%);
    background: var(--veil-brand);
    color: #fff;
    border: none;
    border-radius: var(--radius-full);
    padding: var(--space-2) var(--space-4);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    box-shadow: var(--shadow-md);
    transition: transform var(--t-spring), opacity var(--t-fast);
  }
  .veil-scroll-bottom:hover { transform: translateX(-50%) scale(1.05); }
</style>

<script lang="ts">
  import { messageStore } from '$lib/stores/messages';

  let { channelId } = $props<{ channelId: string }>();
  const store = $derived($messageStore);
  const typingUsers = $derived(store.typingUsers[channelId] ?? []);
</script>

<div class="veil-typing" aria-live="polite" aria-atomic="true">
  {#if typingUsers.length > 0}
    <div class="veil-typing-dots" aria-hidden="true">
      <span></span><span></span><span></span>
    </div>
    {#if typingUsers.length === 1}
      <span><strong>{typingUsers[0].name}</strong> yazıyor...</span>
    {:else if typingUsers.length === 2}
      <span><strong>{typingUsers[0].name}</strong> ve <strong>{typingUsers[1].name}</strong> yazıyor...</span>
    {:else}
      <span>Birkaç kişi yazıyor...</span>
    {/if}
  {/if}
</div>

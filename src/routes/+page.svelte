<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth';
  import AppLayout from '$lib/components/layout/AppLayout.svelte';
  import Onboarding, { recoveryAcknowledged } from '$lib/components/onboarding/Welcome.svelte';
</script>

{#if $authStore.isAuthenticated && ($recoveryAcknowledged || $authStore.identity)}
  <AppLayout />
{:else}
  <Onboarding />
{/if}

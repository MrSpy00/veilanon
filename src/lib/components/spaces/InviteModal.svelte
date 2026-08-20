<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '../ui/Icon.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import type { InviteInfo } from '$lib/api/tauri';
  import { streamerMode, maskInviteLink } from '$lib/stores/streamerMode';

  const ui = $derived($uiStore);
  const spaces = $derived($spaceStore);
  const spaceId = $derived((ui.modalData as { spaceId?: string } | null)?.spaceId ?? ui.activeSpaceId);

  const INVITE_BASE = 'https://veilanon.com/invite/';

  let invite = $state<InviteInfo | null>(null);
  let maxUses = $state(0);
  let expiresIn = $state(0);
  let creating = $state(false);

  onMount(async () => {
    if (spaceId) await create();
  });

  async function create() {
    if (!spaceId || creating) return;
    creating = true;
    try {
      const expiresAt = expiresIn > 0
        ? Math.floor(Date.now() / 1000) + expiresIn * 86400
        : null;
      invite = await spaceStore.invite(spaceId, maxUses > 0 ? maxUses : null, expiresAt);
      toastStore.success('Davet kodu oluşturuldu.');
    } catch {
      toastStore.error('Davet oluşturulamadı.');
    } finally {
      creating = false;
    }
  }

  async function copy() {
    if (!invite) return;
    await copyText(`${INVITE_BASE}${invite.code}`);
    toastStore.success('Davet linki kopyalandı.');
  }
  const currentSpace = $derived(spaces.spaces.find(s => s.id === spaceId));

  async function copyCustomLink() {
    if (!currentSpace?.customLink) return;
    await copyText(`https://veilanon.com/join/${currentSpace.customLink}`);
    toastStore.success('Özel topluluk bağlantısı kopyalandı.');
  }
</script>

<div class="veil-invite-modal">
  <h2 class="veil-settings-title">Topluluk Daveti</h2>

  {#if currentSpace?.customLink}
    <div class="veil-custom-vanity-box">
      <div class="veil-form-label">Özel Topluluk Bağlantısı</div>
      <div class="veil-custom-vanity-row">
        <code data-streamer-mask="invite" data-auto-protect="secret">{$streamerMode.enabled && $streamerMode.hideInviteLinks ? maskInviteLink(currentSpace.customLink) : `veilanon.com/join/${currentSpace.customLink}`}</code>
        <button class="btn btn-secondary btn-sm" onclick={copyCustomLink} title="Kopyala">
          <Icon name="copy" size={14} />
          Kopyala
        </button>
      </div>
    </div>
  {/if}

  {#if invite}
    <div class="veil-invite-box" aria-label="Davet linki">
      <code data-streamer-mask="invite" data-auto-protect="secret">{$streamerMode.enabled && $streamerMode.hideInviteLinks ? maskInviteLink(invite.code) : `${INVITE_BASE}${invite.code}`}</code>
    </div>
    <button class="btn btn-primary" style="width:100%;" onclick={copy}>
      <Icon name="copy" size={15} />
      Davet Linkini Kopyala
    </button>
    <p class="veil-invite-desc">
      Linki alan kişi, uygulamada Ana Menü → Topluluklar → "Davet Linkiyle Katıl" bölümüne yapıştırarak topluluğa katılabilir.
      {invite.maxUses ? `${invite.maxUses} kez kullanılabilir.` : 'Sınırsız kullanım.'}
      {invite.expiresAt ? ` Son kullanma: ${new Date(invite.expiresAt * 1000).toLocaleDateString('tr-TR')}.` : 'Süre sınırı yok.'}
    </p>
  {:else}
    <div class="veil-form-group">
      <label class="veil-form-label" for="invite-uses">Maksimum kullanım (0 = sınırsız)</label>
      <input id="invite-uses" class="veil-input" type="number" min="0" max="999" bind:value={maxUses} />
    </div>
    <div class="veil-form-group">
      <label class="veil-form-label" for="invite-expires">Geçerlilik (gün, 0 = süresiz)</label>
      <input id="invite-expires" class="veil-input" type="number" min="0" max="365" bind:value={expiresIn} />
    </div>
    <button class="btn btn-primary" style="width:100%;" onclick={create} disabled={creating}>
      {creating ? 'Oluşturuluyor…' : 'Davet Kodu Üret'}
    </button>
  {/if}

  <button class="btn btn-secondary veil-invite-close" onclick={() => uiStore.closeModal()}>Kapat</button>
</div>

<style>
  .veil-invite-modal { max-width: 380px; }
  .veil-invite-box {
    padding: var(--space-6);
    background: var(--veil-bg-void);
    border: 1px dashed var(--veil-brand-border);
    border-radius: var(--radius-xl);
    text-align: center;
    margin-bottom: var(--space-3);
  }
  .veil-invite-box code {
    font-family: var(--font-mono);
    font-size: var(--text-xl);
    font-weight: 700;
    letter-spacing: 0.1em;
    color: var(--veil-brand);
    user-select: text;
    word-break: break-all;
  }
  .veil-custom-vanity-box {
    padding: var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    margin-bottom: var(--space-4);
  }
  .veil-custom-vanity-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }
  .veil-custom-vanity-row code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--veil-brand);
    font-weight: 600;
  }
  .veil-invite-desc { font-size: var(--text-sm); color: var(--veil-text-muted); margin-top: var(--space-3); line-height: var(--leading-relaxed); }
  .veil-invite-close { width: 100%; margin-top: var(--space-3); }
</style>

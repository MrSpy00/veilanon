<script lang="ts">
  import { spaceStore } from '$lib/stores/spaces';
  import type { ChannelType } from '$lib/api/tauri';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '../ui/Icon.svelte';

  const ui = $derived($uiStore);
  const modalData = $derived(ui.modalData as { spaceId?: string; defaultType?: ChannelType } | null);
  const spaceId = $derived(modalData?.spaceId ?? ui.activeSpaceId ?? '');

  let channelType = $state<ChannelType>('text');
  let channelName = $state('');
  let isE2ee = $state(false);
  let isSubmitting = $state(false);
  let validationError = $state<string | null>(null);

  $effect(() => {
    if (modalData?.defaultType) {
      channelType = modalData.defaultType;
    }
  });

  const channelTypes: { type: ChannelType; title: string; desc: string; icon: 'hash' | 'volume' | 'megaphone' | 'chat' }[] = [
    {
      type: 'text',
      title: 'Metin Kanalı',
      desc: 'Mesajlar, resimler, çıkartmalar ve kod blokları gönder',
      icon: 'hash',
    },
    {
      type: 'voice',
      title: 'Ses Kanalı',
      desc: 'Ses, video ve ekran paylaşımıyla anında bağlan',
      icon: 'volume',
    },
    {
      type: 'announcement',
      title: 'Duyuru Kanalı',
      desc: 'Yalnızca yetkililerin duyuru ve haber yayınlayabildiği kanal',
      icon: 'megaphone',
    },
    {
      type: 'forum',
      title: 'Forum Kanalı',
      desc: 'Başlıklar ve organize konular halinde tartışmalar',
      icon: 'chat',
    },
  ];

  function formatChannelName(val: string) {
    if (channelType === 'voice') return val.trim();
    // Allow dots, hyphens, underscores, letters, numbers, turkish chars
    return val
      .toLowerCase()
      .replace(/\s+/g, '-')
      .replace(/[^a-z0-9-_çğıöşü.]/gi, '');
  }

  async function handleCreate() {
    validationError = null;
    const raw = channelName.trim();
    if (!raw) {
      validationError = 'Kanal ismi boş bırakılamaz.';
      return;
    }

    const formatted = formatChannelName(raw);
    if (!formatted || formatted.length < 1 || formatted.length > 64) {
      validationError = 'Kanal adı 1 ile 64 karakter arasında olmalıdır.';
      return;
    }

    if (!spaceId) {
      toastStore.error('Topluluk bulunamadı.');
      return;
    }

    isSubmitting = true;
    try {
      await spaceStore.createChannel(spaceId, formatted, channelType, undefined, isE2ee);
      toastStore.success(`'#${formatted}' kanalı oluşturuldu.`);
      uiStore.closeModal();
    } catch (err) {
      toastStore.error(`Kanal oluşturulamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="veil-create-channel">
  <div class="veil-channel-type-grid">
    <div class="veil-form-label">Kanal Türü</div>
    {#each channelTypes as item}
      <button
        type="button"
        class="veil-type-card"
        class:selected={channelType === item.type}
        onclick={() => (channelType = item.type)}
      >
        <div class="veil-type-icon">
          <Icon name={item.icon} size={20} />
        </div>
        <div class="veil-type-info">
          <div class="veil-type-title">{item.title}</div>
          <div class="veil-type-desc">{item.desc}</div>
        </div>
        <div class="veil-type-radio">
          <div class="veil-radio-dot"></div>
        </div>
      </button>
    {/each}
  </div>

  <div class="veil-form-group">
    <div class="veil-label-row">
      <label for="create-ch-name" class="veil-form-label">Kanal İsmi</label>
      <span class="veil-char-count">{channelName.length}/64</span>
    </div>
    <div class="veil-input-wrap">
      <span class="veil-input-prefix">
        {#if channelType === 'voice'}
          <Icon name="volume" size={16} />
        {:else if channelType === 'announcement'}
          <Icon name="megaphone" size={16} />
        {:else if channelType === 'forum'}
          <Icon name="chat" size={16} />
        {:else}
          #
        {/if}
      </span>
      <input
        id="create-ch-name"
        type="text"
        class="input veil-channel-input"
        placeholder="örn: genel-sohbet veya sürüm.1"
        bind:value={channelName}
        maxlength={64}
        autocomplete="off"
        onkeydown={(e) => { if (e.key === 'Enter') handleCreate(); }}
      />
    </div>
    {#if validationError}
      <div class="veil-form-error">{validationError}</div>
    {/if}
  </div>

  <div class="veil-e2ee-toggle-card">
    <div class="veil-e2ee-toggle-info">
      <div class="veil-e2ee-toggle-title">
        <Icon name="lock" size={15} />
        <span>Uçtan Uca Şifreleme (MLS E2EE)</span>
      </div>
      <div class="veil-e2ee-toggle-desc">
        Mesajlar ve medya cihazında şifrelenir; sunucu veya aracılar içeriği göremez.
      </div>
    </div>
    <label class="veil-toggle" aria-label="MLS Uçtan Uca Şifreleme">
      <input type="checkbox" bind:checked={isE2ee} />
      <span class="veil-toggle-track">
        <span class="veil-toggle-thumb"></span>
      </span>
    </label>
  </div>

  <div class="veil-create-channel-actions">
    <button type="button" class="btn btn-ghost" onclick={() => uiStore.closeModal()}>İptal</button>
    <button
      type="button"
      class="btn btn-primary"
      disabled={!channelName.trim() || isSubmitting}
      onclick={handleCreate}
    >
      {#if isSubmitting}
        <span class="veil-spinner veil-spinner-sm"></span>
      {/if}
      Kanal Oluştur
    </button>
  </div>
</div>

<style>
  .veil-create-channel {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: 100%;
    max-width: 440px;
  }
  .veil-form-label {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: var(--tracking-wider);
    color: var(--veil-text-muted);
  }
  .veil-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }
  .veil-char-count {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-family: var(--font-mono);
  }
  .veil-channel-type-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .veil-type-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    transition: background var(--t-fast), border-color var(--t-fast);
    width: 100%;
  }
  .veil-type-card:hover {
    background: var(--veil-bg-raised);
    border-color: var(--veil-border);
  }
  .veil-type-card.selected {
    background: color-mix(in srgb, var(--veil-brand) 12%, var(--veil-bg-elevated));
    border-color: var(--veil-brand);
  }
  .veil-type-icon {
    color: var(--veil-text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .veil-type-card.selected .veil-type-icon {
    color: var(--veil-brand);
  }
  .veil-type-info {
    flex: 1;
    min-width: 0;
  }
  .veil-type-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
  }
  .veil-type-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin-top: 2px;
  }
  .veil-type-radio {
    width: 18px;
    height: 18px;
    border-radius: var(--radius-full);
    border: 2px solid var(--veil-border);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: border-color var(--t-fast);
  }
  .veil-type-card.selected .veil-type-radio {
    border-color: var(--veil-brand);
  }
  .veil-radio-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    opacity: 0;
    transform: scale(0.5);
    transition: transform var(--t-fast), opacity var(--t-fast);
  }
  .veil-type-card.selected .veil-radio-dot {
    opacity: 1;
    transform: scale(1);
  }
  .veil-form-group {
    display: flex;
    flex-direction: column;
  }
  .veil-input-wrap {
    display: flex;
    align-items: center;
    position: relative;
  }
  .veil-input-prefix {
    position: absolute;
    left: 12px;
    color: var(--veil-text-muted);
    font-weight: 700;
    font-size: var(--text-base);
    pointer-events: none;
    display: flex;
    align-items: center;
  }
  .veil-channel-input {
    padding-left: 36px;
    width: 100%;
    height: 40px;
    border-radius: var(--radius-md);
    background: var(--veil-bg-subtle);
    border: 1px solid var(--veil-border);
    color: var(--veil-text-primary);
    font-size: var(--text-sm);
    font-weight: 500;
    transition: border-color var(--t-fast), background-color var(--t-fast);
    outline: none;
  }
  .veil-channel-input:hover {
    border-color: var(--veil-border-strong);
  }
  .veil-channel-input:focus {
    border-color: var(--veil-brand);
    background: var(--veil-bg-raised);
    outline: none;
    box-shadow: none;
  }
  .veil-form-error {
    color: var(--veil-danger);
    font-size: var(--text-xs);
    margin-top: var(--space-2);
  }
  .veil-e2ee-toggle-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    gap: var(--space-3);
  }
  .veil-e2ee-toggle-info {
    flex: 1;
  }
  .veil-e2ee-toggle-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .veil-e2ee-toggle-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin-top: 2px;
  }
  .veil-create-channel-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
    margin-top: var(--space-2);
  }
</style>

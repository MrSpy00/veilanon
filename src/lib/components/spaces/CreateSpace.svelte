<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import Icon from '../ui/Icon.svelte';

  type ModalMode = 'create' | 'join';
  let mode = $state<ModalMode>('create');

  // Create state
  let name = $state('');
  let creating = $state(false);
  let error = $state<string | null>(null);

  // Join state
  let joinQuery = $state('');
  let joining = $state(false);
  let publicSpaces = $state<Array<{ id: string; name: string; description?: string | null; iconHash?: string | null; memberCount: number; customLink?: string | null }>>([]);
  let searching = $state(false);

  function formatError(err: unknown): string {
    const msg = String(err).replace(/^Error:\s*/, '');
    if (msg.includes('not authenticated') || msg.includes('Unauthenticated')) {
      return 'Oturum bilgisi doğrulanamadı. Lütfen uygulamayı yeniden başlatın veya kimliğinizi açın.';
    }
    if (msg.includes('Space name must be')) {
      return 'Topluluk adı 1-64 karakter arasında olmalıdır.';
    }
    if (msg.includes('PermissionDenied') || msg.includes('permission denied')) {
      return 'Bu işlem için yetkiniz yok veya topluluktan yasaklandınız.';
    }
    if (msg.includes('Topluluk bulunamadı') || msg.includes('bağlantı geçersiz')) {
      return 'Topluluk bulunamadı veya davet bağlantısı geçersiz.';
    }
    return msg;
  }

  async function create() {
    const trimmed = name.trim();
    if (trimmed.length < 2 || creating) return;
    creating = true;
    error = null;
    try {
      const space = await spaceStore.createSpace(trimmed);
      toastStore.success('Topluluk oluşturuldu.');
      await spaceStore.loadSpaces();
      uiStore.navigate(space.id, null);
      uiStore.closeModal();
    } catch (err) {
      error = formatError(err);
    } finally {
      creating = false;
    }
  }

  async function searchSpaces() {
    searching = true;
    try {
      const results = await invoke<typeof publicSpaces>('spaces_search_public', { query: joinQuery });
      publicSpaces = results;
    } catch {
      publicSpaces = [];
    } finally {
      searching = false;
    }
  }

  async function joinSpace(idOrLink: string) {
    const trimmed = idOrLink.trim();
    if (!trimmed || joining) return;
    joining = true;
    error = null;
    try {
      const space = await spaceStore.joinPublic(trimmed);
      toastStore.success(`${space.name} topluluğuna katıldın!`);
      await spaceStore.loadChannels(space.id);
      void uiStore.navigateSpace(space.id);
      uiStore.closeModal();
    } catch (err) {
      error = formatError(err);
    } finally {
      joining = false;
    }
  }

  onMount(() => {
    void searchSpaces();
  });
</script>

<div class="veil-create-space">
  <!-- Mode Selector Tabs -->
  <div class="veil-modal-mode-tabs">
    <button
      type="button"
      class="veil-mode-tab"
      class:active={mode === 'create'}
      onclick={() => { mode = 'create'; error = null; }}
    >
      <Icon name="plus" size={15} />
      <span>Topluluk Oluştur</span>
    </button>
    <button
      type="button"
      class="veil-mode-tab"
      class:active={mode === 'join'}
      onclick={() => { mode = 'join'; error = null; searchSpaces(); }}
    >
      <Icon name="search" size={15} />
      <span>Topluluğa Katıl & Keşfet</span>
    </button>
  </div>

  {#if error}
    <div class="veil-alert-error" role="alert">{error}</div>
  {/if}

  {#if mode === 'create'}
    <div class="veil-tab-content">
      <h2 class="veil-settings-title">Yeni Bir Topluluk Başlat</h2>
      <p class="veil-create-desc">
        Topluluk; arkadaşlarınla metin ve ses kanalları üzerinden uçtan uca şifreli olarak iletişim kurabileceğin bağımsız bir alandır.
      </p>

      <div class="veil-form-group">
        <label class="veil-form-label" for="space-name">Topluluk Adı</label>
        <input
          id="space-name"
          class="veil-input"
          bind:value={name}
          placeholder="örn: Siber Güvenlik Topluluğu"
          maxlength={64}
          autocomplete="off"
          onkeydown={(e) => { if (e.key === 'Enter') create(); }}
        />
        <span class="veil-form-desc">1-64 karakter arası. İsmi daha sonra değiştirebilirsin.</span>
      </div>

      <div class="veil-modal-actions">
        <button class="btn btn-secondary" onclick={() => uiStore.closeModal()}>Vazgeç</button>
        <button class="btn btn-primary" onclick={create} disabled={name.trim().length < 2 || creating}>
          {creating ? 'Oluşturuluyor…' : 'Oluştur'}
        </button>
      </div>
    </div>
  {:else}
    <div class="veil-tab-content">
      <h2 class="veil-settings-title">Topluluk Ara veya Bağlantıyla Katıl</h2>
      <p class="veil-create-desc">
        Bir davet bağlantısı gir veya mevcut açık toplulukları arayarak katıl.
      </p>

      <div class="veil-form-group">
        <label class="veil-form-label" for="join-input">Davet Kodu / Özel Link veya Arama</label>
        <div class="veil-search-join-row">
          <input
            id="join-input"
            class="veil-input"
            bind:value={joinQuery}
            placeholder="örn: https://veilanon.com/join/kod veya topluluk adı"
            autocomplete="off"
            oninput={() => searchSpaces()}
            onkeydown={(e) => { if (e.key === 'Enter') joinSpace(joinQuery); }}
          />
          <button
            class="btn btn-primary"
            onclick={() => joinSpace(joinQuery)}
            disabled={!joinQuery.trim() || joining}
          >
            {joining ? 'Katılınıyor…' : 'Katıl'}
          </button>
        </div>
      </div>

      <div class="veil-public-spaces-header">
        <span>Keşfet & Açık Topluluklar</span>
        {#if searching}
          <div class="veil-spinner veil-spinner-xs"></div>
        {/if}
      </div>

      <div class="veil-public-spaces-list">
        {#each publicSpaces as space (space.id)}
          <div class="veil-public-space-card">
            <div class="veil-space-avatar">
              <Icon name="hash" size={18} />
            </div>
            <div class="veil-space-info">
              <div class="veil-space-title">{space.name}</div>
              <div class="veil-space-meta">
                {#if space.description}
                  <span class="veil-space-desc">{space.description}</span>
                {/if}
                <span class="veil-space-count">{space.memberCount || 1} üye</span>
              </div>
            </div>
            <button
              class="btn btn-secondary btn-sm"
              onclick={() => joinSpace(space.id)}
              disabled={joining}
            >
              Katıl
            </button>
          </div>
        {/each}

        {#if publicSpaces.length === 0 && !searching}
          <div class="veil-no-spaces">
            Aramanıza uygun açık topluluk bulunamadı. Davet kodunu doğrudan yukarıya yapıştırabilirsiniz.
          </div>
        {/if}
      </div>

      <div class="veil-modal-actions">
        <button class="btn btn-secondary" onclick={() => uiStore.closeModal()}>Kapat</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .veil-create-space {
    width: 100%;
    max-width: 460px;
  }

  .veil-modal-mode-tabs {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    background: var(--veil-bg-void);
    padding: 4px;
    border-radius: var(--radius-xl);
    border: 1px solid var(--veil-border-subtle);
  }

  .veil-mode-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: 8px 12px;
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .veil-mode-tab:hover {
    color: var(--veil-text-primary);
  }

  .veil-mode-tab.active {
    background: var(--veil-bg-elevated);
    color: var(--veil-brand);
    box-shadow: var(--shadow-sm);
  }

  .veil-tab-content {
    display: flex;
    flex-direction: column;
  }

  .veil-create-desc {
    color: var(--veil-text-muted);
    font-size: var(--text-sm);
    margin-bottom: var(--space-4);
    line-height: var(--leading-relaxed);
  }

  .veil-search-join-row {
    display: flex;
    gap: var(--space-2);
  }

  .veil-search-join-row .veil-input {
    flex: 1;
  }

  .veil-public-spaces-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--veil-text-muted);
    margin: var(--space-3) 0 var(--space-2);
  }

  .veil-public-spaces-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    max-height: 200px;
    overflow-y: auto;
    margin-bottom: var(--space-4);
  }

  .veil-public-space-card {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    transition: border-color 0.15s ease;
  }

  .veil-public-space-card:hover {
    border-color: var(--veil-border);
  }

  .veil-space-avatar {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--veil-brand);
    flex-shrink: 0;
  }

  .veil-space-info {
    flex: 1;
    min-width: 0;
  }

  .veil-space-title {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--veil-text-primary);
  }

  .veil-space-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .veil-space-desc {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 160px;
  }

  .veil-no-spaces {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    font-style: italic;
    padding: var(--space-3);
    text-align: center;
    background: var(--veil-bg-void);
    border-radius: var(--radius-lg);
  }

  .veil-modal-actions {
    display: flex;
    gap: var(--space-2);
    justify-content: flex-end;
    margin-top: var(--space-2);
  }

  .veil-alert-error {
    background: hsl(0 72% 62% / 0.1);
    border: 1px solid hsl(0 72% 62% / 0.3);
    border-radius: var(--radius-lg);
    padding: var(--space-3) var(--space-4);
    color: var(--veil-danger);
    font-size: var(--text-sm);
    margin-bottom: var(--space-4);
  }
</style>

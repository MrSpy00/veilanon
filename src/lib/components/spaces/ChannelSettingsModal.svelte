<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import { permissionsStore } from '$lib/stores/permissions';
  import {
    channelApi,
    roleApi,
    memberApi,
    type ChannelInfo,
    type RoleInfo,
    type MemberInfo,
    type ChannelOverrideItem,
    type ChannelType,
  } from '$lib/api/tauri';
  import {
    ALL_PERMISSIONS,
    PERMISSION_CATEGORIES,
  } from '$lib/utils/permissions';
  import Icon, { type IconName } from '../ui/Icon.svelte';
  import Avatar from '../ui/Avatar.svelte';
  import Toggle from '../ui/Toggle.svelte';
  import VeilSelect from '../ui/VeilSelect.svelte';

  const ui = $derived($uiStore);
  const data = $derived(
    (ui.modalData as { channelId?: string; spaceId?: string; tab?: string } | null) ?? null
  );

  const channelId = $derived(data?.channelId ?? ui.activeChannelId ?? '');
  const spaceId = $derived(data?.spaceId ?? ui.activeSpaceId ?? '');

  let activeTab = $state<'overview' | 'permissions'>('overview');
  const canManage = $derived($permissionsStore.isOwner || $permissionsStore.has('manage_channels'));

  // Channel details
  let channel = $state<ChannelInfo | null>(null);
  let name = $state('');
  let topic = $state('');
  let channelType = $state<ChannelType>('text');
  let isNsfw = $state(false);
  let isE2ee = $state(false);
  let slowModeSeconds = $state(0);
  let savingOverview = $state(false);

  // Roles & Members for permissions
  let roles = $state<RoleInfo[]>([]);
  let members = $state<MemberInfo[]>([]);
  let overrides = $state<ChannelOverrideItem[]>([]);
  let selectedTargetKey = $state<string>('role:@everyone'); // "role:<id>" or "member:<id>"
  let savingOverrides = $state(false);
  let loading = $state(true);

  // Add override dropdown
  let showAddPicker = $state(false);
  let searchTargetQuery = $state('');

  const defaultRole = $derived(roles.find((r) => r.isDefault) || roles[roles.length - 1] || null);
  const defaultRoleId = $derived(defaultRole ? defaultRole.id : spaceId);

  const SLOWMODE_OPTIONS = [
    { value: '0', label: 'Kapalı' },
    { value: '5', label: '5 saniye' },
    { value: '10', label: '10 saniye' },
    { value: '15', label: '15 saniye' },
    { value: '30', label: '30 saniye' },
    { value: '60', label: '1 dakika' },
    { value: '120', label: '2 dakika' },
    { value: '300', label: '5 dakika' },
    { value: '600', label: '10 dakika' },
    { value: '900', label: '15 dakika' },
    { value: '1800', label: '30 dakika' },
    { value: '3600', label: '1 saat' },
    { value: '7200', label: '2 saat' },
    { value: '21600', label: '6 saat' },
  ];

  onMount(async () => {
    if (!channelId || !spaceId) return;
    loading = true;
    try {
      const [channelsList, rls, mbrs, ovs] = await Promise.all([
        channelApi.list(spaceId).catch(() => []),
        roleApi.list(spaceId).catch(() => []),
        memberApi.list(spaceId).catch(() => []),
        channelApi.getOverrides(channelId).catch(() => []),
      ]);

      const found = channelsList.find((c) => c.id === channelId);
      if (found) {
        channel = found;
        name = found.name;
        topic = found.topic ?? '';
        channelType = found.channelType;
        isNsfw = found.isNsfw;
        isE2ee = found.isE2ee;
        slowModeSeconds = found.slowModeSeconds ?? 0;
      }
      roles = rls;
      members = mbrs;
      overrides = ovs;

      const defRole = rls.find((r) => r.isDefault) || rls[rls.length - 1];
      const defRoleId = defRole ? defRole.id : spaceId;
      selectedTargetKey = `role:${defRoleId}`;
    } catch {
      toastStore.error('Kanal ayarları yüklenemedi.');
    } finally {
      loading = false;
    }
  });

  // Target selection parsing
  const currentTargetType = $derived(
    selectedTargetKey.startsWith('member:') ? 'member' : 'role'
  );
  const currentTargetId = $derived(selectedTargetKey.replace(/^(role|member):/, ''));

  const currentOverride = $derived<ChannelOverrideItem>(
    overrides.find(
      (o) => o.targetType === currentTargetType && o.targetId === currentTargetId
    ) ?? {
      targetId: currentTargetId,
      targetType: currentTargetType,
      allow: [] as string[],
      deny: [] as string[],
    }
  );

  const selectedRole = $derived(
    currentTargetType === 'role' ? roles.find((r) => r.id === currentTargetId) ?? null : null
  );
  const selectedMember = $derived(
    currentTargetType === 'member' ? members.find((m) => m.userId === currentTargetId) ?? null : null
  );

  function getTriState(permId: string): 'allow' | 'deny' | 'inherit' {
    if (currentOverride.allow.includes(permId)) return 'allow';
    if (currentOverride.deny.includes(permId)) return 'deny';
    return 'inherit';
  }

  function setTriState(permId: string, state: 'allow' | 'deny' | 'inherit') {
    const existingIndex = overrides.findIndex(
      (o) => o.targetType === currentTargetType && o.targetId === currentTargetId
    );

    let nextAllow = currentOverride.allow.filter((p) => p !== permId);
    let nextDeny = currentOverride.deny.filter((p) => p !== permId);

    if (state === 'allow') {
      nextAllow.push(permId);
    } else if (state === 'deny') {
      nextDeny.push(permId);
    }

    const updatedItem: ChannelOverrideItem = {
      targetId: currentTargetId,
      targetType: currentTargetType,
      allow: nextAllow,
      deny: nextDeny,
    };

    if (existingIndex >= 0) {
      overrides = overrides.map((o, idx) => (idx === existingIndex ? updatedItem : o));
    } else {
      overrides = [...overrides, updatedItem];
    }
  }

  function removeCurrentOverride() {
    overrides = overrides.filter(
      (o) => !(o.targetType === currentTargetType && o.targetId === currentTargetId)
    );
    toastStore.info('Geçersiz kılma sıfırlandı. Değişiklikleri kaydedin.');
  }

  function addOverrideTarget(type: 'role' | 'member', id: string) {
    const key = `${type}:${id}`;
    if (!overrides.some((o) => o.targetType === type && o.targetId === id)) {
      overrides = [
        ...overrides,
        {
          targetId: id,
          targetType: type,
          allow: [],
          deny: [],
        },
      ];
    }
    selectedTargetKey = key;
    showAddPicker = false;
    searchTargetQuery = '';
  }

  async function saveOverview() {
    if (!channelId || savingOverview) return;
    const trimmed = name.trim();
    if (!trimmed) {
      toastStore.error('Kanal adı boş bırakılamaz.');
      return;
    }

    savingOverview = true;
    try {
      await channelApi.update({
        id: channelId,
        name: trimmed,
        position: channel?.position,
      });
      if (spaceId) {
        await spaceStore.loadChannels(spaceId);
      }
      toastStore.success('Kanal ayarları güncellendi.');
    } catch (err) {
      toastStore.error(`Güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      savingOverview = false;
    }
  }

  async function saveOverrides() {
    if (!channelId || savingOverrides) return;
    savingOverrides = true;
    try {
      await channelApi.updateOverrides({
        channelId,
        overrides: overrides.filter((o) => o.allow.length > 0 || o.deny.length > 0),
      });
      await permissionsStore.refresh(spaceId, channelId);
      toastStore.success('Kanal izinleri başarıyla kaydedildi.');
    } catch (err) {
      toastStore.error(`İzinler kaydedilemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      savingOverrides = false;
    }
  }

  async function deleteChannel() {
    if (!channelId) return;
    const ok = await uiStore.confirm(
      `"${name}" kanalını ve tüm mesajlarını silmek istediğinize emin misiniz? Bu işlem geri alınamaz.`,
      { title: 'Kanalı Sil', confirmLabel: 'Kalıcı Olarak Sil', danger: true }
    );
    if (!ok) return;
    try {
      await channelApi.delete(channelId);
      if (spaceId) {
        await spaceStore.loadChannels(spaceId);
      }
      toastStore.success('Kanal silindi.');
      uiStore.closeModal();
    } catch (err) {
      toastStore.error(`Silinemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  // Filtered available candidates to add override
  const availableRolesToAdd = $derived(
    roles.filter((r) => !overrides.some((o) => o.targetType === 'role' && o.targetId === r.id))
  );
  const availableMembersToAdd = $derived(
    members.filter((m) => !overrides.some((o) => o.targetType === 'member' && o.targetId === m.userId))
  );

  const filteredRoles = $derived(
    availableRolesToAdd.filter((r) =>
      !searchTargetQuery || r.name.toLowerCase().includes(searchTargetQuery.toLowerCase())
    )
  );
  const filteredMembers = $derived(
    availableMembersToAdd.filter(
      (m) =>
        !searchTargetQuery ||
        m.username.toLowerCase().includes(searchTargetQuery.toLowerCase()) ||
        m.displayName.toLowerCase().includes(searchTargetQuery.toLowerCase())
    )
  );
</script>

<div class="channel-settings-modal">
  <!-- Header Bar with Tabs -->
  <div class="modal-tab-bar">
    <button
      type="button"
      class="tab-btn"
      class:active={activeTab === 'overview'}
      onclick={() => (activeTab = 'overview')}
    >
      <Icon name="settings" size={15} />
      <span>Genel Bakış</span>
    </button>

    <button
      type="button"
      class="tab-btn"
      class:active={activeTab === 'permissions'}
      onclick={() => (activeTab = 'permissions')}
    >
      <Icon name="shield" size={15} />
      <span>İzinler & Geçersiz Kılmalar</span>
      {#if overrides.length > 0}
        <span class="tab-badge">{overrides.length}</span>
      {/if}
    </button>
  </div>

  {#if loading}
    <div class="loading-state">
      <div class="veil-spinner"></div>
      <span>Kanal yapılandırması yükleniyor…</span>
    </div>
  {:else if activeTab === 'overview'}
    <!-- ── GENEL BAKIŞ (OVERVIEW) ─────────────────────────────────────────── -->
    <div class="tab-content overview-pane">
      <div class="form-sections">
        <div class="veil-form-group">
          <label class="veil-form-label" for="ch-name-input">Kanal Adı</label>
          <div class="input-with-icon">
            <span class="input-type-icon">
              {channelType === 'voice' ? '🔊' : channelType === 'announcement' ? '📣' : channelType === 'forum' ? '💬' : '#'}
            </span>
            <input
              id="ch-name-input"
              type="text"
              class="veil-input"
              bind:value={name}
              placeholder="kanal-adi"
              maxlength={64}
              disabled={!canManage}
            />
          </div>
        </div>

        <div class="veil-form-group">
          <label class="veil-form-label" for="ch-topic-input">Kanal Konusu (Açıklama)</label>
          <textarea
            id="ch-topic-input"
            class="veil-input topic-textarea"
            bind:value={topic}
            placeholder="Bu kanalın amacı veya kuralları hakkında bilgi verin…"
            rows={3}
            maxlength={1024}
            disabled={!canManage}
          ></textarea>
        </div>

        <div class="veil-form-group">
          <label class="veil-form-label" for="ch-slowmode-select">Yavaş Mod (Slowmode)</label>
          <p class="form-desc">Üyelerin ardışık mesaj göndermeleri arasına bekleme süresi koyar.</p>
          <div class="slowmode-select-wrap">
            <VeilSelect
              options={SLOWMODE_OPTIONS}
              value={String(slowModeSeconds)}
              disabled={!canManage}
              onChange={(v) => (slowModeSeconds = Number(v))}
            />
          </div>
        </div>

        <div class="toggles-grid">
          <div class="toggle-card">
            <div class="toggle-info">
              <span class="toggle-title">Yaş Kısıtlamalı Kanal (NSFW)</span>
              <span class="toggle-desc">Bu kanala girerken 18 yaş doğrulama uyarısı gösterilir.</span>
            </div>
            <Toggle checked={isNsfw} disabled={!canManage} onChange={(v) => (isNsfw = v)} />
          </div>

          <div class="toggle-card">
            <div class="toggle-info">
              <span class="toggle-title">Uçtan Uca Şifreleme (MLS E2EE)</span>
              <span class="toggle-desc">Mesajlar yalnızca yetkili üyelerin cihazlarında çözülür.</span>
            </div>
            <span class="badge-e2ee" class:active={isE2ee}>
              {isE2ee ? 'Aktif' : 'Devre Dışı'}
            </span>
          </div>
        </div>
      </div>

      <!-- Action Footer -->
      {#if canManage}
        <div class="pane-footer">
          <button type="button" class="btn btn-danger" onclick={deleteChannel}>
            <Icon name="trash" size={14} />
            <span>Kanalı Sil</span>
          </button>
          <button
            type="button"
            class="btn btn-primary"
            onclick={saveOverview}
            disabled={!name.trim() || savingOverview}
          >
            {savingOverview ? 'Kaydediliyor…' : 'Değişiklikleri Kaydet'}
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <!-- ── İZİNLER & GEÇERSİZ KILMALAR (PERMISSIONS & OVERRIDES) ─────────────── -->
    <div class="tab-content permissions-layout">
      <!-- Left Sidebar: Roles & Members List -->
      <div class="overrides-sidebar">
        <div class="sidebar-header">
          <span class="sidebar-title">Roller & Üyeler</span>
          <button
            type="button"
            class="btn-icon btn-icon-sm"
            title="Rol veya Üye Ekle"
            onclick={() => (showAddPicker = !showAddPicker)}
          >
            <Icon name="plus" size={14} />
          </button>
        </div>

        <!-- Add Picker Dropdown -->
        {#if showAddPicker}
          <div class="add-target-popover">
            <div class="picker-search">
              <Icon name="search" size={12} />
              <input
                type="text"
                class="picker-input"
                placeholder="Rol veya üye ara…"
                bind:value={searchTargetQuery}
              />
            </div>
            <div class="picker-list">
              {#if filteredRoles.length > 0}
                <div class="picker-section-label">Roller</div>
                {#each filteredRoles as r}
                  <button
                    type="button"
                    class="picker-item"
                    onclick={() => addOverrideTarget('role', r.id)}
                  >
                    <span class="role-dot" style="background: {r.color || '#7c3aed'};"></span>
                    <span class="picker-name">{r.name}</span>
                  </button>
                {/each}
              {/if}

              {#if filteredMembers.length > 0}
                <div class="picker-section-label">Üyeler</div>
                {#each filteredMembers as m}
                  <button
                    type="button"
                    class="picker-item"
                    onclick={() => addOverrideTarget('member', m.userId)}
                  >
                    <Avatar name={m.displayName || m.username} hash={m.avatarHash} size="sm" />
                    <span class="picker-name">{m.displayName || m.username}</span>
                  </button>
                {/each}
              {/if}

              {#if filteredRoles.length === 0 && filteredMembers.length === 0}
                <div class="picker-empty">Eklenebilecek rol/üye bulunamadı.</div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Overrides List -->
        <div class="overrides-list">
          <!-- @everyone default item -->
          <button
            type="button"
            class="override-target-btn"
            class:active={selectedTargetKey === `role:${defaultRoleId}`}
            onclick={() => (selectedTargetKey = `role:${defaultRoleId}`)}
          >
            <div class="target-left">
              <Icon name="users" size={14} />
              <span class="target-name">@everyone</span>
            </div>
          </button>

          <!-- Specific Role Overrides -->
          {#each overrides.filter((o) => o.targetType === 'role' && o.targetId !== defaultRoleId) as ov}
            {@const roleObj = roles.find((r) => r.id === ov.targetId)}
            {#if roleObj}
              <button
                type="button"
                class="override-target-btn"
                class:active={selectedTargetKey === `role:${ov.targetId}`}
                onclick={() => (selectedTargetKey = `role:${ov.targetId}`)}
              >
                <div class="target-left">
                  <span class="role-dot" style="background: {roleObj.color || '#7c3aed'};"></span>
                  <span class="target-name">{roleObj.name}</span>
                </div>
              </button>
            {/if}
          {/each}

          <!-- Member Overrides -->
          {#each overrides.filter((o) => o.targetType === 'member') as ov}
            {@const memberObj = members.find((m) => m.userId === ov.targetId)}
            {#if memberObj}
              <button
                type="button"
                class="override-target-btn"
                class:active={selectedTargetKey === `member:${ov.targetId}`}
                onclick={() => (selectedTargetKey = `member:${ov.targetId}`)}
              >
                <div class="target-left">
                  <Avatar
                    name={memberObj.displayName || memberObj.username}
                    hash={memberObj.avatarHash}
                    size="sm"
                  />
                  <span class="target-name">{memberObj.displayName || memberObj.username}</span>
                </div>
              </button>
            {/if}
          {/each}
        </div>
      </div>

      <!-- Right Panel: Tri-State Matrix for Selected Target -->
      <div class="permissions-editor">
        <div class="editor-header">
          <div class="target-badge-info">
            {#if currentTargetType === 'role'}
              <span
                class="role-badge"
                style="--role-color: {selectedRole?.color || '#7c3aed'};"
              >
                <span class="role-dot" style="background: {selectedRole?.color || '#7c3aed'};"></span>
                <span>{selectedRole?.name || '@everyone'}</span>
              </span>
            {:else}
              <div class="member-badge">
                <Avatar
                  name={selectedMember?.displayName || selectedMember?.username || 'Üye'}
                  hash={selectedMember?.avatarHash}
                  size="sm"
                />
                <span>{selectedMember?.displayName || selectedMember?.username}</span>
              </div>
            {/if}
            <span class="editor-subtitle">İçin Kanal İzinleri</span>
          </div>

          <button
            type="button"
            class="btn btn-ghost btn-xs text-danger"
            onclick={removeCurrentOverride}
            title="Tüm geçersiz kılmaları kaldırıp varsayılana döndürür"
          >
            <Icon name="refresh-cw" size={12} />
            <span>Sıfırla</span>
          </button>
        </div>

        <!-- Permissions Categories Matrix -->
        <div class="matrix-scroll-area">
          {#each PERMISSION_CATEGORIES as cat}
            {@const catPerms = ALL_PERMISSIONS.filter((p) => p.category === cat.id)}
            {#if catPerms.length > 0}
              <div class="perm-category-block">
                <div class="category-title">
                  <Icon name={cat.icon} size={14} />
                  <span>{cat.label}</span>
                </div>

                <div class="perm-rows-list">
                  {#each catPerms as perm (perm.id)}
                    {@const state = getTriState(perm.id)}
                    <div class="perm-tri-row" class:danger-perm={perm.danger}>
                      <div class="perm-info">
                        <div class="perm-title-row">
                          <span class="perm-title">{perm.label}</span>
                          {#if perm.danger}
                            <span class="perm-danger-badge">Yetkili</span>
                          {/if}
                        </div>
                        <span class="perm-desc">{perm.desc}</span>
                      </div>

                      <!-- 3-Way Tri-State Button Matrix -->
                      <div class="tri-state-group" role="radiogroup" aria-label={perm.label}>
                        <button
                          type="button"
                          role="radio"
                          class="tri-btn deny"
                          class:active={state === 'deny'}
                          title="Reddet (Kullanıcı bu izne sahip olamaz)"
                          onclick={() => setTriState(perm.id, 'deny')}
                          aria-checked={state === 'deny'}
                        >
                          <Icon name="x" size={14} />
                        </button>

                        <button
                          type="button"
                          role="radio"
                          class="tri-btn inherit"
                          class:active={state === 'inherit'}
                          title="Devral (Rolün genel sunucu iznini kullan)"
                          onclick={() => setTriState(perm.id, 'inherit')}
                          aria-checked={state === 'inherit'}
                        >
                          <span class="inherit-dash">/</span>
                        </button>

                        <button
                          type="button"
                          role="radio"
                          class="tri-btn allow"
                          class:active={state === 'allow'}
                          title="İzin Ver (Bu kanalda koşulsuz izinli)"
                          onclick={() => setTriState(perm.id, 'allow')}
                          aria-checked={state === 'allow'}
                        >
                          <Icon name="check" size={14} />
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          {/each}
        </div>

        <!-- Overrides Save Footer -->
        <div class="editor-footer">
          <span class="footer-hint">Değişiklikleri geçerli kılmak için kaydetmeyi unutmayın.</span>
          <button
            type="button"
            class="btn btn-primary"
            onclick={saveOverrides}
            disabled={savingOverrides}
          >
            {savingOverrides ? 'Kaydediliyor…' : 'İzinleri Kaydet'}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .channel-settings-modal {
    display: flex;
    flex-direction: column;
    height: 640px;
    max-height: 85vh;
    color: var(--veil-text-primary, #ffffff);
  }

  .modal-tab-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
    padding: 0 var(--space-4) var(--space-3);
    margin-bottom: var(--space-4);
  }

  .tab-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--veil-text-secondary);
    font-size: var(--text-sm);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--t-fast);
  }

  .tab-btn:hover {
    background: var(--veil-bg-overlay);
    color: var(--veil-text-primary);
  }

  .tab-btn.active {
    background: var(--veil-bg-elevated);
    color: var(--veil-brand, #7c3aed);
    box-shadow: var(--shadow-sm);
  }

  .tab-badge {
    background: var(--veil-brand);
    color: #ffffff;
    font-size: 10px;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    font-weight: 700;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--space-3);
    color: var(--veil-text-muted);
  }

  .tab-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /* ── Overview Styles ────────────────────────────────────────── */
  .overview-pane {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0 var(--space-4);
    overflow-y: auto;
  }

  .form-sections {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .input-with-icon {
    display: flex;
    align-items: center;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    padding-left: var(--space-3);
  }

  .input-type-icon {
    font-size: var(--text-sm);
    color: var(--veil-text-muted);
    user-select: none;
  }

  .input-with-icon .veil-input {
    border: none;
    background: transparent;
  }

  .topic-textarea {
    resize: vertical;
    min-height: 80px;
  }

  .form-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    margin-top: 2px;
    margin-bottom: var(--space-2);
  }

  .slowmode-select-wrap {
    max-width: 320px;
    margin-top: var(--space-1);
  }

  .toggles-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--space-3);
    margin-top: var(--space-2);
  }

  .toggle-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
  }

  .toggle-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .toggle-title {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .toggle-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .badge-e2ee {
    font-size: var(--text-xs);
    padding: 3px 8px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-surface);
    color: var(--veil-text-muted);
    font-weight: 600;
  }

  .badge-e2ee.active {
    background: rgba(46, 204, 113, 0.15);
    color: #2ecc71;
    border: 1px solid rgba(46, 204, 113, 0.3);
  }

  .pane-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: var(--space-4);
    margin-top: var(--space-4);
    border-top: 1px solid var(--veil-border-subtle);
  }

  /* ── Permissions & Overrides Matrix Layout ───────────────────── */
  .permissions-layout {
    display: flex;
    gap: var(--space-4);
    height: 100%;
  }

  .overrides-sidebar {
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
    position: relative;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--veil-border-subtle);
    background: var(--veil-bg-elevated);
  }

  .sidebar-title {
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--veil-text-muted);
  }

  .add-target-popover {
    position: absolute;
    top: 38px;
    left: var(--space-2);
    right: var(--space-2);
    z-index: 50;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-xl);
    max-height: 260px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .picker-search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
  }

  .picker-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-size: var(--text-xs);
    color: var(--veil-text-primary);
  }

  .picker-list {
    overflow-y: auto;
    padding: var(--space-1);
  }

  .picker-section-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--veil-text-muted);
    padding: var(--space-1) var(--space-2);
  }

  .picker-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    padding: var(--space-1) var(--space-2);
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--veil-text-secondary);
    font-size: var(--text-xs);
    cursor: pointer;
    text-align: left;
  }

  .picker-item:hover {
    background: var(--veil-bg-elevated);
    color: var(--veil-text-primary);
  }

  .picker-empty {
    padding: var(--space-3);
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    text-align: center;
  }

  .overrides-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-1);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .override-target-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: var(--space-2) var(--space-2);
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    color: var(--veil-text-secondary);
    font-size: var(--text-xs);
    font-weight: 500;
    cursor: pointer;
    transition: all var(--t-fast);
  }

  .override-target-btn:hover {
    background: var(--veil-bg-overlay);
    color: var(--veil-text-primary);
  }

  .override-target-btn.active {
    background: color-mix(in srgb, var(--veil-brand) 15%, transparent);
    color: var(--veil-brand);
    font-weight: 700;
  }

  .target-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .target-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .role-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  /* ── Right Matrix Editor ─────────────────────────────────────── */
  .permissions-editor {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border-bottom: 1px solid var(--veil-border-subtle);
  }

  .target-badge-info {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .role-badge,
  .member-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-sm);
    font-weight: 700;
  }

  .editor-subtitle {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }

  .matrix-scroll-area {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .perm-category-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .category-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--veil-text-muted);
    border-bottom: 1px solid var(--veil-border-subtle);
    padding-bottom: 4px;
  }

  .perm-rows-list {
    display: flex;
    flex-direction: column;
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .perm-tri-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--veil-border-subtle);
  }

  .perm-tri-row:last-child {
    border-bottom: none;
  }

  .perm-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .perm-title-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .perm-title {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .perm-danger-badge {
    font-size: 10px;
    font-weight: 700;
    color: #e74c3c;
    background: rgba(231, 76, 60, 0.15);
    padding: 1px 5px;
    border-radius: var(--radius-sm);
  }

  .perm-desc {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
    line-height: 1.3;
  }

  /* ── 3-Way Tri-State Segmented Control ──────────────────────── */
  .tri-state-group {
    display: inline-flex;
    align-items: center;
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    padding: 2px;
    flex-shrink: 0;
    gap: 2px;
  }

  .tri-btn {
    width: 28px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--veil-text-muted);
    cursor: pointer;
    transition: all var(--t-fast);
  }

  .tri-btn:hover {
    color: var(--veil-text-primary);
  }

  .tri-btn.deny.active {
    background: #e74c3c;
    color: #ffffff;
    box-shadow: 0 0 6px rgba(231, 76, 60, 0.4);
  }

  .tri-btn.inherit.active {
    background: var(--veil-bg-elevated);
    color: var(--veil-text-primary);
    font-weight: 700;
  }

  .tri-btn.allow.active {
    background: #2ecc71;
    color: #ffffff;
    box-shadow: 0 0 6px rgba(46, 204, 113, 0.4);
  }

  .inherit-dash {
    font-size: var(--text-sm);
    font-weight: 700;
    line-height: 1;
  }

  .editor-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--veil-bg-elevated);
    border-top: 1px solid var(--veil-border-subtle);
  }

  .footer-hint {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
</style>

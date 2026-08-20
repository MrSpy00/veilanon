<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore } from '$lib/stores/notifications';
  import { roleApi, type RoleInfo } from '$lib/api/tauri';
  import Icon from '../ui/Icon.svelte';

  const ui = $derived($uiStore);
  const data = $derived((ui.modalData as { role?: RoleInfo | null; spaceId?: string } | null) ?? null);

  const role = $derived(data?.role ?? null);
  const spaceId = $derived(data?.spaceId ?? '');

  interface PermissionDef {
    id: string;
    label: string;
    desc: string;
    category: 'general' | 'moderation' | 'text' | 'voice';
    danger?: boolean;
  }

  const ALL_PERMISSIONS: PermissionDef[] = [
    // Genel & Yönetim
    { id: 'administrator', label: 'Yönetici (Administrator)', desc: 'Topluluktaki tüm izinleri koşulsuz sağlar ve kanal kısıtlamalarını atlar. (Tehlikeli)', category: 'general', danger: true },
    { id: 'manage_space', label: 'Topluluğu Yönet', desc: 'Topluluk adını, simgesini, bannerını ve genel ayarlarını düzenleyebilir.', category: 'general' },
    { id: 'view_audit_log', label: 'Denetim Kaydını Görüntüle', desc: 'Toplulukta gerçekleştirilen tüm moderasyon ve yönetim kayıtlarını inceleyebilir.', category: 'general' },
    { id: 'manage_roles', label: 'Rolleri Yönet', desc: 'Kendi rolünden daha düşük seviyedeki rolleri oluşturabilir, düzenleyebilir ve üyelere atayabilir.', category: 'general' },
    { id: 'manage_channels', label: 'Kanalları Yönet', desc: 'Yeni kanallar oluşturabilir, kanal adlarını ve izin kısıtlamalarını düzenleyebilir.', category: 'general' },
    { id: 'manage_invites', label: 'Davetleri Yönet', desc: 'Topluluk için davet bağlantıları oluşturabilir ve aktif davetleri iptal edebilir.', category: 'general' },
    { id: 'manage_webhooks', label: 'Webhook & Entegrasyonları Yönet', desc: 'Discord köprüsü ve harici bot webhook entegrasyonlarını yönetebilir.', category: 'general' },

    // Moderasyon & Üyeler
    { id: 'kick_members', label: 'Üyeleri At (Kick)', desc: 'Kendinden alt seviyedeki üyeleri topluluktan çıkarabilir.', category: 'moderation' },
    { id: 'ban_members', label: 'Üyeleri Yasakla (Ban)', desc: 'Kendinden alt seviyedeki üyeleri topluluktan kalıcı olarak yasaklar.', category: 'moderation', danger: true },
    { id: 'timeout_members', label: 'Üyeleri Sustur (Timeout)', desc: 'Belirli bir süre boyunca üyelerin mesaj göndermesini ve konuşmasını engeller.', category: 'moderation' },

    // Metin & Mesajlaşma
    { id: 'send_messages', label: 'Mesaj Gönder', desc: 'Metin kanallarına mesaj yazabilir.', category: 'text' },
    { id: 'read_messages', label: 'Mesaj Geçmişini Oku', desc: 'Kanalın geçmiş mesajlarını okuyabilir.', category: 'text' },
    { id: 'manage_messages', label: 'Mesajları Yönet', desc: 'Diğer kullanıcıların mesajlarını silebilir ve sabitleyebilir.', category: 'text' },
    { id: 'embed_links', label: 'Bağlantı Önizlemesi Ekle', desc: 'Mesajlarda paylaşılan bağlantıların zengin kart önizlemelerini gösterir.', category: 'text' },
    { id: 'attach_files', label: 'Dosya & Medya Ekle', desc: 'Resim, video, belge ve dosya yükleyebilir.', category: 'text' },
    { id: 'add_reactions', label: 'Tepki Ekle', desc: 'Mesajlara yeni emoji tepkileri ekleyebilir.', category: 'text' },
    { id: 'use_slash_commands', label: 'Eğik Çizgi (/) Komutlarını Kullan', desc: 'Uygulama ve bot slash komutlarını çalıştırabilir.', category: 'text' },
    { id: 'mention_everyone', label: '@everyone & @here Etiketle', desc: 'Kanal veya sunucudaki tüm üyelere anlık bildirim gönderebilir.', category: 'text' },
    { id: 'pin_messages', label: 'Mesajları Sabitle', desc: 'Önemli mesajları kanalın sabitlenenler paneline ekleyebilir.', category: 'text' },

    // Ses & Görüntü
    { id: 'connect_voice', label: 'Sese Bağlan', desc: 'Ses ve video kanallarına katılabilir.', category: 'voice' },
    { id: 'speak', label: 'Konuş', desc: 'Ses kanallarında mikrofonunu kullanarak konuşabilir.', category: 'voice' },
    { id: 'stream_video', label: 'Kamera Aç (Video)', desc: 'Ses kanallarında kamerasını yayına verebilir.', category: 'voice' },
    { id: 'share_screen', label: 'Ekran Paylaş', desc: 'Masaüstünü veya uygulama penceresini canlı yayına aktarabilir.', category: 'voice' },
    { id: 'mute_members', label: 'Üyeleri Sustur (Server Mute)', desc: 'Ses kanalındaki diğer üyelerin mikrofonunu kapatabilir.', category: 'voice' },
    { id: 'deafen_members', label: 'Üyeleri Sağırlaştır (Server Deafen)', desc: 'Ses kanalındaki diğer üyelerin ses duymasını engelleyebilir.', category: 'voice' },
    { id: 'move_members', label: 'Üyeleri Taşı', desc: 'Üyeleri bir ses kanalından diğerine aktarabilir veya bağlantısını kesebilir.', category: 'voice' },
    { id: 'use_voice_activity', label: 'Ses Etkinliği Kullan', desc: 'Bas-konuş zorunluluğu olmadan otomatik ses algılamasıyla konuşabilir.', category: 'voice' },
    { id: 'priority_speaker', label: 'Öncelikli Konuşmacı', desc: 'Konuştuğunda diğer üyelerin ses seviyesi otomatik olarak kısılır.', category: 'voice' },
  ];

  const CATEGORIES = [
    { id: 'all', label: 'Tümü' },
    { id: 'general', label: 'Genel & Yönetim' },
    { id: 'moderation', label: 'Moderasyon' },
    { id: 'text', label: 'Metin & Sohbet' },
    { id: 'voice', label: 'Ses & Video' },
  ];

  const PRESET_COLORS = [
    '#7c3aed', // Brand Purple
    '#5865f2', // Indigo
    '#3b82f6', // Blue
    '#06b6d4', // Cyan
    '#10b981', // Emerald
    '#84cc16', // Lime
    '#f59e0b', // Amber
    '#ef4444', // Red
    '#ec4899', // Pink
    '#d946ef', // Fuchsia
    '#64748b', // Slate
    '#94a3b8', // Silver
  ];

  let name = $state('');
  let color = $state('#7c3aed');
  let selected = $state<string[]>([]);
  let saving = $state(false);
  let activeTab = $state<'all' | 'general' | 'moderation' | 'text' | 'voice'>('all');
  let searchQuery = $state('');

  const isAdminSelected = $derived(selected.includes('administrator'));

  const filteredPermissions = $derived(
    ALL_PERMISSIONS.filter((p) => {
      const matchCat = activeTab === 'all' || p.category === activeTab;
      const q = searchQuery.trim().toLowerCase();
      const matchQuery = !q || p.label.toLowerCase().includes(q) || p.desc.toLowerCase().includes(q);
      return matchCat && matchQuery;
    })
  );

  onMount(() => {
    name = role?.name ?? '';
    color = role?.color ?? '#7c3aed';
    selected = [...(role?.permissions ?? ['send_messages', 'read_messages', 'connect_voice', 'speak'])];
  });

  function togglePermission(id: string) {
    selected = selected.includes(id) ? selected.filter(p => p !== id) : [...selected, id];
  }

  function selectAll() {
    selected = ALL_PERMISSIONS.map(p => p.id);
  }

  function deselectAll() {
    selected = [];
  }

  async function save() {
    if (!name.trim() || !spaceId || saving) return;
    saving = true;
    try {
      if (role) {
        await roleApi.update({ id: role.id, name: name.trim(), color, permissions: selected });
      } else {
        await roleApi.create({ spaceId, name: name.trim(), color, permissions: selected });
      }
      toastStore.success(role ? 'Rol güncellendi.' : 'Rol oluşturuldu.');
      uiStore.closeModal();
    } catch (err) {
      toastStore.error(`Rol kaydedilemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      saving = false;
    }
  }
</script>

<div class="veil-role-editor">
  <!-- Role Header Preview -->
  <div class="veil-role-head">
    <div class="veil-role-badge-preview" style="--role-color: {color};">
      <span class="veil-role-dot" style="background: {color};"></span>
      <span class="veil-role-preview-text">{name.trim() || 'Rol Önizleme'}</span>
      {#if role?.position}
        <span class="role-pos-pill">Seviye #{role.position}</span>
      {/if}
    </div>
    <h2 class="veil-settings-title" style="margin: 0;">{role ? 'Rolü Düzenle' : 'Yeni Rol Oluştur'}</h2>
  </div>

  <!-- Role Name & Color Grid -->
  <div class="role-meta-grid">
    <div class="veil-form-group">
      <label class="veil-form-label" for="role-name">Rol Adı</label>
      <input
        id="role-name"
        class="veil-input"
        bind:value={name}
        placeholder="örn: Moderatör, VIP, Sunucu Yöneticisi"
        maxlength={32}
        autocomplete="off"
      />
    </div>

    <div class="veil-form-group">
      <label class="veil-form-label" for="role-color-input">Rol Rengi</label>
      <div class="veil-color-row">
        <input
          id="role-color-input"
          class="veil-color-input"
          type="color"
          bind:value={color}
          aria-label="Özel renk seçici"
        />
        <input
          class="veil-input veil-color-text-input"
          type="text"
          bind:value={color}
          maxlength={9}
          placeholder="#7c3aed"
          aria-label="HEX renk kodu"
        />
      </div>
    </div>
  </div>

  <!-- Palette Swatches -->
  <div class="veil-color-presets">
    {#each PRESET_COLORS as presetColor}
      <button
        type="button"
        class="veil-color-swatch"
        class:active={color.toLowerCase() === presetColor.toLowerCase()}
        style="background: {presetColor};"
        title={presetColor}
        aria-label="Renk: {presetColor}"
        onclick={() => (color = presetColor)}
      >
        {#if color.toLowerCase() === presetColor.toLowerCase()}
          <Icon name="check" size={12} class="veil-swatch-check" />
        {/if}
      </button>
    {/each}
  </div>

  <!-- Administrator Master Alert -->
  {#if isAdminSelected}
    <div class="admin-notice-card">
      <Icon name="shield-alert" size={20} class="admin-icon" />
      <div class="admin-notice-content">
        <span class="admin-notice-title">Yönetici Yetkisi Aktif</span>
        <span class="admin-notice-desc">Bu role sahip üyeler tüm kanal izinlerini, kısıtlamaları ve moderasyon engellerini atlar.</span>
      </div>
    </div>
  {/if}

  <!-- Permissions Header & Controls -->
  <div class="veil-form-group">
    <div class="veil-perms-head">
      <div class="perms-title-row">
        <span class="veil-form-label" style="margin:0;">İzinler</span>
        <span class="perms-count-badge">{selected.length} / {ALL_PERMISSIONS.length}</span>
      </div>
      <div class="veil-perms-quick-btns">
        <button type="button" class="btn-link" onclick={selectAll}>Tümünü Seç</button>
        <span>·</span>
        <button type="button" class="btn-link" onclick={deselectAll}>Tümünü Kaldır</button>
      </div>
    </div>

    <!-- Category Tabs & Search Filter -->
    <div class="perms-filter-bar">
      <div class="perms-tabs">
        {#each CATEGORIES as cat}
          <button
            type="button"
            class="perm-tab-btn"
            class:active={activeTab === cat.id}
            onclick={() => (activeTab = cat.id as any)}
          >
            {cat.label}
          </button>
        {/each}
      </div>
      <div class="perms-search-box">
        <Icon name="search" size={13} />
        <input
          type="text"
          class="perms-search-input"
          placeholder="İzinlerde ara…"
          bind:value={searchQuery}
        />
      </div>
    </div>

    <!-- Permissions List -->
    <div class="veil-perms">
      {#if filteredPermissions.length === 0}
        <div class="no-perms-found">Arama kriterine uygun izin bulunamadı.</div>
      {:else}
        {#each filteredPermissions as perm (perm.id)}
          <label class="veil-perm-item" class:active={selected.includes(perm.id)} class:danger={perm.danger}>
            <input
              type="checkbox"
              checked={selected.includes(perm.id)}
              onchange={() => togglePermission(perm.id)}
            />
            <div class="veil-perm-info">
              <div class="perm-title-row">
                <span class="veil-perm-label">{perm.label}</span>
                {#if perm.danger}
                  <span class="perm-danger-tag">Yönetici</span>
                {/if}
              </div>
              <span class="veil-perm-desc">{perm.desc}</span>
            </div>
          </label>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Action Buttons -->
  <div class="veil-role-actions">
    <button class="btn btn-secondary" onclick={() => uiStore.closeModal()}>Vazgeç</button>
    <button class="btn btn-primary" onclick={save} disabled={!name.trim() || saving}>
      {saving ? 'Kaydediliyor…' : 'Rolü Kaydet'}
    </button>
  </div>
</div>

<style>
  .veil-role-editor {
    max-width: 580px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .veil-role-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
  }

  .veil-role-badge-preview {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--role-color) 16%, var(--veil-bg-surface, #1e1f22));
    border: 1px solid color-mix(in srgb, var(--role-color) 40%, transparent);
    color: var(--role-color);
    font-size: 12px;
    font-weight: 700;
  }

  .role-pos-pill {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.3);
    color: rgba(255, 255, 255, 0.85);
  }

  .veil-role-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .veil-role-preview-text {
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .role-meta-grid {
    display: grid;
    grid-template-columns: 1fr 160px;
    gap: 12px;
  }

  .veil-color-presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: -4px;
  }

  .veil-color-swatch {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.15s ease, border-color 0.15s ease;
    padding: 0;
  }

  .veil-color-swatch:hover {
    transform: scale(1.15);
  }

  .veil-color-swatch.active {
    border-color: #fff;
    box-shadow: 0 0 0 2px var(--veil-brand, #7c3aed);
  }

  :global(.veil-swatch-check) {
    color: #fff;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6));
  }

  .veil-color-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .veil-color-input {
    width: 38px;
    height: 34px;
    border: 1px solid var(--veil-border, rgba(255, 255, 255, 0.12));
    border-radius: 6px;
    background: var(--veil-bg-surface, #1e1f22);
    cursor: pointer;
    padding: 2px;
    flex-shrink: 0;
  }

  .veil-color-text-input {
    width: 100%;
    font-family: monospace;
    font-size: 13px;
    padding: 6px 8px;
  }

  /* Admin Warning Card */
  .admin-notice-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 8px;
    background: rgba(235, 77, 75, 0.12);
    border: 1px solid rgba(235, 77, 75, 0.4);
    color: #ffffff;
  }

  :global(.admin-icon) {
    color: #eb4d4b;
    flex-shrink: 0;
  }

  .admin-notice-content {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .admin-notice-title {
    font-size: 12.5px;
    font-weight: 700;
    color: #ff7675;
  }

  .admin-notice-desc {
    font-size: 11.5px;
    color: rgba(255, 255, 255, 0.85);
  }

  .veil-perms-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .perms-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .perms-count-badge {
    font-size: 11px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary, #ffffff);
  }

  .veil-perms-quick-btns {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted, #949ba4);
  }

  .btn-link {
    background: none;
    border: none;
    color: #5865f2;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }

  .btn-link:hover {
    text-decoration: underline;
  }

  .perms-filter-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
    flex-wrap: wrap;
  }

  .perms-tabs {
    display: flex;
    gap: 4px;
    background: rgba(0, 0, 0, 0.2);
    padding: 3px;
    border-radius: 6px;
  }

  .perm-tab-btn {
    background: none;
    border: none;
    padding: 4px 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted, #949ba4);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .perm-tab-btn:hover {
    color: var(--text-primary, #ffffff);
  }

  .perm-tab-btn.active {
    background: rgba(255, 255, 255, 0.15);
    color: var(--text-primary, #ffffff);
  }

  .perms-search-box {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-surface, #1e1f22);
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
  }

  .perms-search-input {
    background: none;
    border: none;
    outline: none;
    color: var(--text-primary, #ffffff);
    font-size: 11.5px;
    width: 110px;
  }

  .veil-perms {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 280px;
    overflow-y: auto;
    border: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
    border-radius: 8px;
    padding: 6px;
    background: var(--veil-bg-elevated, #2b2d31);
  }

  .no-perms-found {
    padding: 24px;
    text-align: center;
    font-size: 12px;
    color: var(--text-muted, #949ba4);
  }

  .veil-perm-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .veil-perm-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .veil-perm-item.active {
    background: rgba(88, 101, 242, 0.12);
  }

  .veil-perm-item.danger.active {
    background: rgba(235, 77, 75, 0.15);
  }

  .veil-perm-item input {
    margin-top: 3px;
    accent-color: #5865f2;
    cursor: pointer;
  }

  .veil-perm-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .perm-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .veil-perm-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary, #ffffff);
  }

  .perm-danger-tag {
    font-size: 9.5px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 3px;
    background: #eb4d4b;
    color: #ffffff;
    text-transform: uppercase;
  }

  .veil-perm-desc {
    font-size: 11.5px;
    color: var(--text-muted, #949ba4);
    line-height: 1.35;
  }

  .veil-role-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    padding-top: 10px;
    border-top: 1px solid var(--veil-border-subtle, rgba(255, 255, 255, 0.08));
  }
</style>

<script lang="ts">
  import Toggle from '../ui/Toggle.svelte';
  import VeilSelect from '../ui/VeilSelect.svelte';
  import Icon from '../ui/Icon.svelte';
  import type { IconName } from '$lib/types/icon';
  import Avatar from '../ui/Avatar.svelte';
  import BannerImage from '../ui/BannerImage.svelte';
  import BannerCropModal from '../ui/BannerCropModal.svelte';
  import ImageCropModal from '../ui/ImageCropModal.svelte';
  import { readLocalImageAsDataUrl } from '$lib/utils/image-loader';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { uiStore } from '$lib/stores/ui';
  import { spaceStore } from '$lib/stores/spaces';
  import { toastStore } from '$lib/stores/notifications';
  import { open } from '@tauri-apps/plugin-dialog';
  import { channelApi, roleApi, memberApi, inviteApi, mlsApi, discordApi, spaceApi, type ChannelInfo, type RoleInfo, type MemberInfo, type InviteInfo, type BanInfo, type ChannelType } from '$lib/api/tauri';
  import { streamerMode, maskInviteLink, maskUserId } from '$lib/stores/streamerMode';
  import { privacyShield, isShieldActive, isScreenShareActive } from '$lib/stores/privacyShield';
  import { permissionsStore } from '$lib/stores/permissions';
  import { authStore } from '$lib/stores/auth';
  import { copyText } from '$lib/utils/clipboard';

  let cropSrc = $state<string | null>(null);
  let iconCropSrc = $state<string | null>(null);

  const ui = $derived($uiStore);
  const spaces = $derived($spaceStore);
  const auth = $derived($authStore);
  const perms = $derived($permissionsStore);

  const spaceId = $derived((ui.modalData as { spaceId?: string } | null)?.spaceId ?? ui.activeSpaceId);
  const space = $derived(spaces.spaces.find(s => s.id === spaceId) ?? null);

  const isOwner = $derived(Boolean(space?.isOwner));
  const canManageSpace = $derived(isOwner || perms.has('manage_space'));
  const canManageChannels = $derived(isOwner || perms.has('manage_channels'));
  const canManageRoles = $derived(isOwner || perms.has('manage_roles'));
  const canKick = $derived(isOwner || perms.has('kick_members'));
  const canBan = $derived(isOwner || perms.has('ban_members'));
  const canModerate = $derived(isOwner || perms.has('moderate_members'));

  // Discord tarzı sekmeler: genel / kanallar / e2ee / roller / üyeler / davet / köprü
  type SettingsTab = 'general' | 'channels' | 'e2ee' | 'roles' | 'members' | 'invites' | 'bridge';
  let tab = $state<SettingsTab>('general');

  const visibleTabs = $derived.by(() => {
    const list: SettingsTab[] = ['general'];
    if (canManageChannels) list.push('channels', 'e2ee');
    if (canManageRoles) list.push('roles');
    list.push('members', 'invites');
    if (isOwner) list.push('bridge');
    return list;
  });

  const GROUPS: Array<{ label: string; ids: SettingsTab[] }> = [
    { label: 'TOPLULUK', ids: ['general', 'channels', 'e2ee'] },
    { label: 'KULLANICILAR & YETKİLER', ids: ['roles', 'members'] },
    { label: 'BAĞLANTILAR', ids: ['invites', 'bridge'] },
  ];

  const visibleGroups = $derived.by(() => {
    return GROUPS.map(g => ({
      label: g.label,
      ids: g.ids.filter(id => visibleTabs.includes(id))
    })).filter(g => g.ids.length > 0);
  });

  $effect(() => {
    const data = ui.modalData as { spaceId?: string; tab?: SettingsTab; channelId?: string } | null;
    if (data?.tab && visibleTabs.includes(data.tab)) {
      tab = data.tab;
    } else if (data?.channelId && canManageChannels) {
      tab = 'channels';
    } else if (!visibleTabs.includes(tab)) {
      tab = visibleTabs[0] ?? 'general';
    }
  });

  const TABS: Array<{ id: SettingsTab; label: string; icon: string }> = [
    { id: 'general', label: 'Genel', icon: 'info' },
    { id: 'channels', label: 'Kanallar', icon: 'hash' },
    { id: 'e2ee', label: 'E2EE Kanallar', icon: 'lock' },
    { id: 'roles', label: 'Roller', icon: 'shield' },
    { id: 'members', label: 'Üyeler', icon: 'users' },
    { id: 'invites', label: 'Bağlantılar', icon: 'link' },
    { id: 'bridge', label: 'Discord Köprüsü', icon: 'megaphone' },
  ];

  // Özel topluluk bağlantısı (bir kez alınabilir)
  let customLinkDraft = $state('');
  let customLinkBusy = $state(false);
  let customLinkError = $state<string | null>(null);

  const customLinkValue = $derived(space?.customLink ?? null);

  async function claimCustomLink() {
    if (!spaceId || !space?.isOwner || customLinkBusy) return;
    customLinkError = null;
    const link = customLinkDraft.trim().toLowerCase();
    if (link.length < 2 || link.length > 32 || !/^[a-z0-9-]+$/.test(link)) {
      customLinkError = 'Yalnızca küçük harf, rakam ve tire kullan (2-32 karakter).';
      return;
    }
    customLinkBusy = true;
    try {
      const updated = await spaceApi.setCustomLink(spaceId, link);
      spaceStore.applySpace(updated);
      customLinkDraft = '';
      toastStore.success('Özel bağlantın alındı: veilanon.com/join/' + link);
    } catch (err) {
      customLinkError = String(err).replace(/^Error:\s*/, '');
    } finally {
      customLinkBusy = false;
    }
  }

  async function copyCustomLink() {
    if (!customLinkValue) return;
    await copyText(`https://veilanon.com/join/${customLinkValue}`);
    toastStore.success('Özel bağlantı kopyalandı.');
  }

  // Üyeye rol atama (yalnızca sahip)
  let roleAssignFor = $state<MemberInfo | null>(null);
  let roleAssignDraft = $state<string[]>([]);
  let roleAssignBusy = $state(false);

  function openRoleAssign(m: MemberInfo) {
    roleAssignFor = m;
    roleAssignDraft = [...m.roleIds];
  }

  function toggleRoleAssign(roleId: string) {
    roleAssignDraft = roleAssignDraft.includes(roleId)
      ? roleAssignDraft.filter(r => r !== roleId)
      : [...roleAssignDraft, roleId];
  }

  async function saveRoleAssign() {
    if (!spaceId || !roleAssignFor || roleAssignBusy) return;
    const targetUserId = roleAssignFor.userId;
    const targetName = roleAssignFor.displayName || roleAssignFor.username;
    const newRoles = [...roleAssignDraft];
    roleAssignBusy = true;
    try {
      await memberApi.update({ spaceId, userId: targetUserId, roleIds: newRoles });
      members = members.map(m => m.userId === targetUserId ? { ...m, roleIds: newRoles } : m);
      toastStore.success(`${targetName} için roller güncellendi.`);
      roleAssignFor = null;
      await reload();
    } catch (err) {
      toastStore.error(`Roller kaydedilemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      roleAssignBusy = false;
    }
  }

  async function deleteRole(role: RoleInfo) {
    const ok = await uiStore.confirm(
      `"${role.name}" rolünü silmek istiyor musun? Bu rolü taşıyan üyelerin rolleri kaldırılır.`,
      { title: 'Rolü Sil', confirmLabel: 'Sil', danger: true }
    );
    if (!ok) return;
    try {
      await roleApi.deleteRole(role.id);
      toastStore.success('Rol silindi.');
      await reload();
    } catch (err) {
      toastStore.error(`Rol silinemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  let channels = $state<ChannelInfo[]>([]);
  let roles = $state<RoleInfo[]>([]);
  let members = $state<MemberInfo[]>([]);
  let invites = $state<InviteInfo | null>(null);
  let bans = $state<BanInfo[]>([]);
  let loading = $state(true);

  // New channel form
  let newChannelName = $state('');
  let newChannelType = $state<ChannelType>('text');
  let newChannelE2ee = $state(false);

  // E2EE üye daveti
  let inviteChannelId = $state<string | null>(null);
  let inviteMemberId = $state('');
  let inviteKeyPackage = $state('');
  let inviteBusy = $state(false);

  // Kendi E2EE anahtarı
  let myKeyPackage = $state<string | null>(null);
  let mySigner = $state<string | null>(null);

  // Discord köprüsü
  let bridgeChannelId = $state('');
  let bridgeUrl = $state('');
  let bridgeInfo = $state<{ channelId: string; maskedUrl: string } | null>(null);
  let bridgeBusy = $state(false);

  const e2eeChannels = $derived(channels.filter(c => c.isE2ee));

  // Topluluk görünümü: ad, açıklama, ikon, banner
  let nameDraft = $state('');
  let committedName = $state('');
  let descDraft = $state('');
  let committedDesc = $state('');
  let savingName = $state(false);
  let savingDesc = $state(false);
  let mediaBusy = $state(false);

  $effect(() => {
    if (space && space.name !== committedName) {
      committedName = space.name;
      nameDraft = space.name;
    }
    if (space && (space.description ?? '') !== committedDesc) {
      committedDesc = space.description ?? '';
      descDraft = space.description ?? '';
    }
  });

  async function saveSpaceName() {
    const trimmed = nameDraft.trim();
    if (!spaceId || !trimmed || trimmed === committedName || savingName) return;
    savingName = true;
    try {
      const updated = await spaceApi.update({ id: spaceId, name: trimmed });
      spaceStore.applySpace(updated);
      committedName = trimmed;
      toastStore.success('Topluluk adı güncellendi.');
    } catch (err) {
      toastStore.error(`Güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      savingName = false;
    }
  }

  async function saveSpaceDesc() {
    if (!spaceId || savingDesc) return;
    const trimmed = descDraft.trim();
    if (trimmed === committedDesc) return;
    savingDesc = true;
    try {
      const updated = await spaceApi.update({ id: spaceId, description: trimmed || null });
      spaceStore.applySpace(updated);
      committedDesc = trimmed;
      toastStore.success('Açıklama güncellendi.');
    } catch (err) {
      toastStore.error(`Güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
    } finally {
      savingDesc = false;
    }
  }

  async function refreshSpaceAfterMedia() {
    if (!spaceId) return;
    try {
      const updated = await spaceApi.update({ id: spaceId });
      spaceStore.applySpace(updated);
    } catch { /* keep previous */ }
  }

  async function changeSpaceIcon() {
    if (!spaceId || mediaBusy) return;
    const selected = await open({
      title: 'Topluluk ikonu seç',
      multiple: false,
      filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    });
    if (!selected || typeof selected !== 'string') return;
    if (selected.toLowerCase().endsWith('.gif')) {
      mediaBusy = true;
      try {
        await spaceApi.setIcon(spaceId, selected);
        await refreshSpaceAfterMedia();
        toastStore.success('Animasyonlu topluluk ikonu güncellendi.');
      } catch {
        toastStore.error('İkon yüklenemedi.');
      } finally {
        mediaBusy = false;
      }
      return;
    }
    iconCropSrc = await readLocalImageAsDataUrl(selected);
  }

  async function handleSpaceIconCropSave(croppedDataUrl: string) {
    iconCropSrc = null;
    if (!spaceId) return;
    mediaBusy = true;
    try {
      await spaceApi.setIcon(spaceId, croppedDataUrl);
      await refreshSpaceAfterMedia();
      toastStore.success('Topluluk ikonu güncellendi.');
    } catch {
      toastStore.error('İkon yüklenemedi.');
    } finally {
      mediaBusy = false;
    }
  }

  async function changeSpaceBanner() {
    if (!spaceId || mediaBusy) return;
    const selected = await open({
      title: 'Topluluk bannerı seç',
      multiple: false,
      filters: [{ name: 'Görseller', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
    });
    if (!selected || typeof selected !== 'string') return;
    if (selected.toLowerCase().endsWith('.gif')) {
      mediaBusy = true;
      try {
        await spaceApi.setBanner(spaceId, selected);
        await refreshSpaceAfterMedia();
        toastStore.success('Animasyonlu topluluk bannerı güncellendi.');
      } catch {
        toastStore.error('Banner yüklenemedi.');
      } finally {
        mediaBusy = false;
      }
      return;
    }
    cropSrc = await readLocalImageAsDataUrl(selected);
  }

  async function handleSpaceBannerCropSave(croppedDataUrl: string) {
    cropSrc = null;
    if (!spaceId) return;
    mediaBusy = true;
    try {
      await spaceApi.setBanner(spaceId, croppedDataUrl);
      await refreshSpaceAfterMedia();
      toastStore.success('Topluluk bannerı güncellendi.');
    } catch {
      toastStore.error('Banner yüklenemedi.');
    } finally {
      mediaBusy = false;
    }
  }

  async function removeSpaceIcon() {
    if (!spaceId) return;
    const ok = await uiStore.confirm('Topluluk ikonunu kaldırmak istiyor musun?', {
      title: 'İkonu Kaldır',
      confirmLabel: 'Kaldır',
      danger: true,
    });
    if (!ok) return;
    try {
      const updated = await spaceApi.update({ id: spaceId, iconHash: null });
      spaceStore.applySpace(updated);
      toastStore.success('İkon kaldırıldı.');
    } catch {
      toastStore.error('İkon kaldırılamadı.');
    }
  }

  async function removeSpaceBanner() {
    if (!spaceId) return;
    const ok = await uiStore.confirm('Topluluk bannerını kaldırmak istiyor musun?', {
      title: 'Bannerı Kaldır',
      confirmLabel: 'Kaldır',
      danger: true,
    });
    if (!ok) return;
    try {
      const updated = await spaceApi.update({ id: spaceId, bannerHash: null });
      spaceStore.applySpace(updated);
      toastStore.success('Banner kaldırıldı.');
    } catch {
      toastStore.error('Banner kaldırılamadı.');
    }
  }

  // Modal açılışında veya topluluk değişince verileri yükle — onMount yerine
  // $effect kullanmak, modalData/spaceId geç set edilse bile yüklemeyi garantiler.
  $effect(() => {
    const id = spaceId;
    if (!id) return;
    loading = true;
    void fetchAll(id);
  });

  const ROLE_PERMISSIONS = [
    { id: 'manage_channels', label: 'Kanalları yönet' },
    { id: 'manage_roles', label: 'Rolleri yönet' },
    { id: 'manage_members', label: 'Üyeleri yönet' },
    { id: 'manage_invites', label: 'Davetleri yönet' },
    { id: 'manage_messages', label: 'Mesajları yönet' },
    { id: 'kick_members', label: 'Üyeleri at' },
    { id: 'ban_members', label: 'Üyeleri yasakla' },
    { id: 'send_messages', label: 'Mesaj gönder' },
  ];

  async function fetchAll(id: string) {
    try {
      const [chs, rls, mbrs, bns] = await Promise.all([
        channelApi.list(id).catch(() => []),
        roleApi.list(id).catch(() => []),
        memberApi.list(id).catch(() => []),
        memberApi.listBans(id).catch(() => []),
      ]);
      channels = chs;
      roles = rls;
      members = mbrs;
      bans = bns;
    } catch {
      toastStore.error('Topluluk bilgileri yüklenemedi.');
    } finally {
      loading = false;
    }
  }

  async function reloadBans() {
    if (!spaceId) return;
    try {
      bans = await memberApi.listBans(spaceId);
    } catch { /* keep previous */ }
  }

  async function moveRoleUp(index: number) {
    if (index <= 0 || !spaceId) return;
    const newRoles = [...roles];
    const temp = newRoles[index - 1];
    newRoles[index - 1] = newRoles[index];
    newRoles[index] = temp;
    roles = newRoles;
    try {
      await roleApi.reorder({ spaceId, roleIds: newRoles.map(r => r.id) });
      toastStore.success('Rol hiyerarşisi güncellendi.');
    } catch (err) {
      toastStore.error(`Hiyerarşi güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
      await reload();
    }
  }

  async function moveRoleDown(index: number) {
    if (index >= roles.length - 1 || !spaceId) return;
    const newRoles = [...roles];
    const temp = newRoles[index + 1];
    newRoles[index + 1] = newRoles[index];
    newRoles[index] = temp;
    roles = newRoles;
    try {
      await roleApi.reorder({ spaceId, roleIds: newRoles.map(r => r.id) });
      toastStore.success('Rol hiyerarşisi güncellendi.');
    } catch (err) {
      toastStore.error(`Hiyerarşi güncellenemedi: ${String(err).replace(/^Error:\s*/, '')}`);
      await reload();
    }
  }

  async function kickMember(m: MemberInfo) {
    if (!spaceId) return;
    const ok = await uiStore.confirm(
      `${m.displayName || m.username} kullanıcısını topluluktan atmak istiyor musun?`,
      { title: 'Üyeyi At', confirmLabel: 'At', danger: true }
    );
    if (!ok) return;
    try {
      await memberApi.kick({ spaceId, userId: m.userId });
      toastStore.success('Üye atıldı.');
      await reload();
    } catch (err) {
      toastStore.error(`Atılamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function banMember(m: MemberInfo) {
    if (!spaceId) return;
    const reason = await uiStore.promptInput(
      'Yasaklama nedeni (isteğe bağlı):',
      { title: `${m.displayName || m.username} kullanıcısını yasakla`, confirmLabel: 'Yasakla' }
    );
    if (reason === null) return;
    try {
      await memberApi.ban({ spaceId, userId: m.userId, reason: reason.trim() || null });
      toastStore.success('Üye yasaklandı.');
      await reload();
    } catch (err) {
      toastStore.error(`Yasaklanamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function timeoutMember(m: MemberInfo) {
    if (!spaceId) return;
    const minutes = await uiStore.promptInput(
      'Susturma süresi (dakika, 0 = kaldır):',
      { title: `${m.displayName || m.username} kullanıcısını sustur`, confirmLabel: 'Sustur', defaultValue: '30' }
    );
    if (minutes === null) return;
    const n = Number(minutes.trim());
    if (!Number.isFinite(n) || n < 0) {
      toastStore.error('Geçerli bir dakika değeri gir.');
      return;
    }
    const until = n === 0 ? null : Math.floor(Date.now() / 1000) + n * 60;
    try {
      await memberApi.timeout({ spaceId, userId: m.userId, until });
      toastStore.success(n === 0 ? 'Susturma kaldırıldı.' : `${n} dakikalık susturma uygulandı.`);
    } catch (err) {
      toastStore.error(`Susturulamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function unbanMember(b: BanInfo) {
    if (!spaceId) return;
    try {
      await memberApi.unban({ spaceId, userId: b.userId });
      toastStore.success('Yasak kaldırıldı.');
      await reloadBans();
    } catch (err) {
      toastStore.error(`Kaldırılamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function transferOwnershipToMember(m: MemberInfo) {
    if (!spaceId || !space?.isOwner) return;
    const ok = await uiStore.confirm(
      `"${space.name}" topluluğunun kuruculuk sahipliğini @${m.displayName || m.username} kullanıcısına devretmek istediğine emin misin?\n\nBu işlem geri alınamaz. Devrettikten sonra sunucu sahibi @${m.displayName || m.username} olacak.`,
      {
        title: 'Topluluk Sahipliğini Devret',
        confirmLabel: 'Sahipliği Devret',
        danger: true,
      }
    );
    if (!ok) return;
    try {
      await spaceStore.transferOwnership(spaceId, m.userId);
      toastStore.success(`Topluluk sahipliği ${m.displayName || m.username} kullanıcısına devredildi.`);
      await reload();
    } catch (err) {
      toastStore.error(`Sahiplik aktarılamadı: ${String(err).replace(/^Error:\s*/, '')}`);
    }
  }

  async function reload() {
    if (!spaceId) return;
    loading = true;
    await fetchAll(spaceId);
  }

  async function createChannel() {
    const name = newChannelName.trim();
    if (!name || !spaceId) return;
    try {
      await spaceStore.createChannel(spaceId, name, newChannelType, undefined, newChannelE2ee);
      newChannelName = '';
      newChannelE2ee = false;
      toastStore.success('Kanal oluşturuldu.');
      await reload();
    } catch {
      toastStore.error('Kanal oluşturulamadı.');
    }
  }

  async function generateMyKeyPackage(channelId: string) {
    try {
      const kp = await mlsApi.createKeyPackage(channelId);
      myKeyPackage = kp.keyPackage;
      mySigner = kp.signerPrivate;
      toastStore.success('Davet kodun üretildi. Sahibine ilet.');
    } catch {
      toastStore.error('Davet kodu üretilemedi.');
    }
  }

  async function consumeMyWelcome(channelId: string) {
    if (!myKeyPackage || !mySigner) {
      toastStore.error('Önce "Davet kodun" butonuyla kod üret.');
      return;
    }
    try {
      await mlsApi.consumeWelcome({ channelId, keyPackage: myKeyPackage, signerPrivate: mySigner });
      toastStore.success('E2EE anahtarı alındı. Kanal şifreli mesajlaşmaya hazır.');
      myKeyPackage = null;
      mySigner = null;
    } catch (err) {
      toastStore.error(String(err).replace(/^Error:\s*/, ''));
    }
  }

  async function sendInvite() {
    if (!inviteChannelId || !inviteMemberId || !inviteKeyPackage.trim() || inviteBusy) return;
    inviteBusy = true;
    try {
      await mlsApi.addMember({
        channelId: inviteChannelId,
        userId: inviteMemberId,
        keyPackage: inviteKeyPackage.trim(),
      });
      toastStore.success('Üye E2EE kanalına eklendi.');
      inviteKeyPackage = '';
      inviteMemberId = '';
      inviteChannelId = null;
    } catch (err) {
      toastStore.error(String(err).replace(/^Error:\s*/, ''));
    } finally {
      inviteBusy = false;
    }
  }

  async function loadBridge() {
    if (!bridgeChannelId) return;
    bridgeInfo = null;
    try {
      bridgeInfo = await discordApi.getWebhook(bridgeChannelId);
      if (bridgeInfo) bridgeUrl = '';
    } catch {
      bridgeInfo = null;
    }
  }

  async function saveBridge() {
    if (!bridgeChannelId || !bridgeUrl.trim() || bridgeBusy) return;
    bridgeBusy = true;
    try {
      bridgeInfo = await discordApi.setWebhook({ channelId: bridgeChannelId, webhookUrl: bridgeUrl.trim() });
      bridgeUrl = '';
      toastStore.success('Discord köprüsü ayarlandı.');
    } catch (err) {
      toastStore.error(String(err).replace(/^Error:\s*/, ''));
    } finally {
      bridgeBusy = false;
    }
  }

  async function clearBridge() {
    if (!bridgeChannelId) return;
    try {
      await discordApi.clearWebhook(bridgeChannelId);
      bridgeInfo = null;
      toastStore.success('Discord köprüsü kaldırıldı.');
    } catch {
      toastStore.error('Köprü kaldırılamadı.');
    }
  }

  async function deleteChannel(channelId: string) {
    const ok = await uiStore.confirm(
      'Bu kanalı silmek istediğine emin misin?',
      { title: 'Kanalı Sil', confirmLabel: 'Sil', danger: true }
    );
    if (!ok) return;
    try {
      await channelApi.delete(channelId);
      toastStore.success('Kanal silindi.');
      await reload();
    } catch {
      toastStore.error('Kanal silinemedi.');
    }
  }

  async function createInvite() {
    if (!spaceId) return;
    try {
      invites = await inviteApi.create({ spaceId });
      toastStore.success('Davet linki oluşturuldu.');
    } catch {
      toastStore.error('Davet oluşturulamadı.');
    }
  }

  const INVITE_BASE = 'https://veilanon.com/invite/';

  async function copyInvite() {
    if (invites) {
      await copyText(`${INVITE_BASE}${invites.code}`);
      toastStore.success('Davet linki kopyalandı.');
    }
  }

  async function copyServerLink() {
    if (!spaceId) return;
    await copyText(`https://veilanon.com/server/${spaceId}`);
    toastStore.success('Topluluk bağlantısı kopyalandı.');
  }

  async function leaveOrDeleteThisSpace() {
    if (!space) return;
    if (space.isOwner) {
      const ok = await uiStore.confirm(
        `"${space.name}" topluluğunu kalıcı olarak silmek istediğine emin misin? Bu işlem tüm kanalları ve mesajları yok eder.`,
        { title: 'Topluluğu Sil', confirmLabel: 'Sil', danger: true }
      );
      if (!ok) return;
      try {
        await spaceStore.deleteSpace(space.id);
        uiStore.closeModal();
        uiStore.navigate(null, null);
        toastStore.success('Topluluk silindi.');
      } catch (err) {
        toastStore.error(`Silinemedi: ${String(err).replace(/^Error:\s*/, '')}`);
      }
    } else {
      const ok = await uiStore.confirm(
        `"${space.name}" topluluğundan ayrılmak istediğine emin misin?`,
        { title: 'Topluluktan Ayrıl', confirmLabel: 'Ayrıl', danger: true }
      );
      if (!ok) return;
      try {
        await spaceStore.leaveSpace(space.id);
        uiStore.closeModal();
        uiStore.navigate(null, null);
        toastStore.success('Topluluktan ayrıldın.');
      } catch (err) {
        toastStore.error(`Ayrılamadı: ${String(err).replace(/^Error:\s*/, '')}`);
      }
    }
  }
</script>

{#if !space}
  <p class="veil-empty-inline">Topluluk bulunamadı.</p>
{:else}
  <div class="veil-space-settings-layout">
    <nav class="veil-space-settings-nav" aria-label="Topluluk ayarları sekmeleri">
      <div class="veil-space-nav-header">
        <Avatar name={space.name} hash={space.iconHash} size="md" />
        <div class="veil-space-nav-header-info">
          <div class="veil-space-nav-title">{space.name}</div>
          <div class="veil-space-nav-sub">{space.memberCount} üye{space.isOwner ? ' · Sahip' : ''}</div>
        </div>
      </div>

      <div class="veil-space-nav-scroll">
        {#each visibleGroups as group (group.label)}
          <div class="veil-settings-section-label">{group.label}</div>
          {#each TABS.filter(t => group.ids.includes(t.id)) as t (t.id)}
            <button
              class="veil-settings-nav-item"
              class:active={tab === t.id}
              onclick={() => (tab = t.id)}
              aria-current={tab === t.id ? 'page' : undefined}
            >
              <span class="veil-settings-nav-icon" aria-hidden="true"><Icon name={t.icon as IconName} size={17} /></span>
              <span class="veil-settings-nav-label">{t.label}</span>
            </button>
          {/each}
        {/each}
      </div>
    </nav>

    <div class="veil-space-settings-content" tabindex="-1">
      {#if loading}
        <div class="veil-spinner" style="margin: 4rem auto;"></div>
      {:else}
        <!-- ── GENEL ─────────────────────────────────────────────── -->
        {#if tab === 'general'}
        <div class="veil-settings-group">
          <div class="veil-settings-group-label">Topluluk Görünümü</div>
          <div class="veil-space-hero">
            {#if space.bannerHash}
              <BannerImage hash={space.bannerHash} alt="" class="veil-space-hero-banner" />
            {:else}
              <div class="veil-space-hero-banner veil-space-hero-placeholder" aria-hidden="true"></div>
            {/if}
            <div class="veil-space-hero-overlay">
              <Avatar name={space.name} hash={space.iconHash} size="lg" />
              <div class="veil-space-hero-info">
                <div class="veil-space-hero-name">{space.name}</div>
                <div class="veil-space-hero-meta">{space.memberCount} üye{space.isOwner ? ' · sahibi' : ''}</div>
              </div>
            </div>
          </div>
          {#if canManageSpace}
            <div class="veil-space-media-actions">
              <button class="btn btn-secondary btn-sm" onclick={changeSpaceIcon} disabled={mediaBusy}>
                <Icon name="camera" size={13} />
                İkon Değiştir
              </button>
              {#if space.iconHash}
                <button class="btn btn-ghost btn-sm" onclick={removeSpaceIcon}>İkonu Kaldır</button>
              {/if}
              <button class="btn btn-secondary btn-sm" onclick={changeSpaceBanner} disabled={mediaBusy}>
                <Icon name="image" size={13} />
                Banner Yükle
              </button>
              {#if space.bannerHash}
                <button class="btn btn-ghost btn-sm" onclick={removeSpaceBanner}>Bannerı Kaldır</button>
              {/if}
            </div>
          {/if}
          <div class="veil-settings-row veil-settings-row-stack">
            <div class="veil-settings-row-info">
              <label class="veil-settings-row-label" for="space-name-input">Topluluk Adı</label>
              <p class="veil-settings-row-desc">Topluluklarında görünen ad (en fazla 64 karakter).</p>
            </div>
            <form class="veil-displayname-form" onsubmit={(e) => { e.preventDefault(); if (canManageSpace) saveSpaceName(); }}>
              <input
                id="space-name-input"
                class="veil-input"
                bind:value={nameDraft}
                maxlength={64}
                autocomplete="off"
                placeholder="Topluluk adı"
                disabled={!canManageSpace}
              />
              {#if canManageSpace}
                <button class="btn btn-primary btn-sm" type="submit" disabled={savingName || !nameDraft.trim() || nameDraft.trim() === committedName}>
                  {savingName ? 'Kaydediliyor…' : 'Kaydet'}
                </button>
              {/if}
            </form>
          </div>
          <div class="veil-settings-row veil-settings-row-stack">
            <div class="veil-settings-row-info">
              <label class="veil-settings-row-label" for="space-desc-input">Açıklama</label>
              <p class="veil-settings-row-desc">Yeni üyelerin topluluğu tanıması için kısa bir tanıtım (en fazla 300 karakter). Yalnızca cihazında saklanır.</p>
            </div>
            <textarea
              id="space-desc-input"
              class="veil-input veil-bio-input"
              bind:value={descDraft}
              rows={2}
              maxlength={300}
              placeholder="Topluluğun hakkında…"
              disabled={!canManageSpace}
            ></textarea>
            {#if canManageSpace}
              <div class="veil-bio-actions">
                <span class="veil-bio-count">{descDraft.length}/300</span>
                <button class="btn btn-primary btn-sm" onclick={saveSpaceDesc} disabled={savingDesc || descDraft.trim() === committedDesc}>
                  {savingDesc ? 'Kaydediliyor…' : 'Kaydet'}
                </button>
              </div>
            {/if}
          </div>

          <div class="veil-settings-divider"></div>
          <div class="veil-settings-group-label" style="color: var(--veil-danger); margin-top: var(--space-4);">Tehlikeli Bölge</div>
          <div class="veil-settings-row">
            <div class="veil-settings-row-info">
              <div class="veil-settings-row-label" style="color: var(--veil-danger);">
                {space.isOwner ? 'Topluluğu Sil' : 'Topluluktan Ayrıl'}
              </div>
              <p class="veil-settings-row-desc">
                {space.isOwner
                  ? 'Topluluğu ve tüm kanallarını kalıcı olarak siler. Bu işlem geri alınamaz.'
                  : 'Bu topluluktan ayrılırsın. Tekrar katılmak için davet linkine veya açık topluluk listesine ihtiyacın olur.'}
              </p>
            </div>
            <button class="btn btn-danger btn-sm" onclick={leaveOrDeleteThisSpace}>
              <Icon name={space.isOwner ? 'trash' : 'log-out'} size={14} />
              {space.isOwner ? 'Topluluğu Sil' : 'Topluluktan Ayrıl'}
            </button>
          </div>
        </div>
      {/if}

      <!-- ── KANALLAR ──────────────────────────────────────────── -->
      {#if tab === 'channels'}
        <div class="veil-settings-group">
          <div class="veil-settings-group-label">Kanallar</div>
          {#each channels.filter(c => c.channelType !== 'category') as ch (ch.id)}
            <div class="veil-settings-row">
              <div class="veil-settings-row-info">
                <div class="veil-settings-row-label">
                  <span class="veil-channel-type-badge">{ch.channelType === 'voice' ? '🔊' : ch.channelType === 'announcement' ? '📣' : ch.channelType === 'forum' ? '💬' : '#'}</span>
                  {ch.name}
                  {#if ch.isNsfw}
                    <span class="badge-nsfw">NSFW</span>
                  {/if}
                </div>
                <div class="veil-settings-row-desc">
                  {ch.channelType === 'voice' ? 'Ses kanalı' : ch.channelType === 'announcement' ? 'Duyuru kanalı' : ch.channelType === 'forum' ? 'Forum kanalı' : 'Metin kanalı'}{ch.isE2ee ? ' · E2EE' : ''}
                </div>
              </div>
              <div class="veil-channel-actions">
                {#if canManageChannels}
                  <button
                    class="btn btn-secondary btn-sm"
                    title="Kanalı düzenle ve izinleri ayarla"
                    onclick={() => uiStore.openModal('channel-edit', { spaceId: space?.id, channelId: ch.id })}
                  >
                    <Icon name="settings" size={13} />
                    Düzenle & İzinler
                  </button>
                  <button class="btn-icon" title="Kanalı sil" aria-label="{ch.name} kanalını sil" onclick={() => deleteChannel(ch.id)}>
                    <Icon name="trash" size={16} />
                  </button>
                {/if}
              </div>
            </div>
          {/each}

          {#if canManageChannels}
            <div class="veil-create-channel">
              <input
                class="veil-input"
                bind:value={newChannelName}
                placeholder="Yeni kanal adı"
                aria-label="Yeni kanal adı"
                maxlength={64}
              />
              <VeilSelect
                options={[
                  { value: 'text', label: 'Metin' },
                  { value: 'voice', label: 'Ses' },
                  { value: 'announcement', label: 'Duyuru' },
                  { value: 'forum', label: 'Forum' },
                ]}
                value={newChannelType}
                onChange={(v) => (newChannelType = v as ChannelType)}
              />
              <button class="btn btn-primary btn-sm" onclick={createChannel} disabled={!newChannelName.trim()}>Ekle</button>
            </div>
            <label class="veil-e2ee-toggle">
              <input
                type="checkbox"
                checked={newChannelE2ee}
                onchange={(e) => (newChannelE2ee = (e.currentTarget as HTMLInputElement).checked)}
              />
              <span>Uçtan uca şifreli kanal (MLS)</span>
            </label>
            {#if newChannelE2ee}
              <p class="veil-settings-row-desc veil-note">
                E2EE kanallarda mesaj içeriği yalnızca üye cihazlarında çözülür; sunucu yalnızca şifreli veri saklar.
              </p>
            {/if}
          {/if}
        </div>
      {/if}

      <!-- ── E2EE KANALLAR ─────────────────────────────────────── -->
      {#if tab === 'e2ee'}
        {#if e2eeChannels.length > 0}
          <div class="veil-settings-group">
            <div class="veil-settings-group-label">E2EE Kanallar</div>
            {#each e2eeChannels as ch (ch.id)}
              <div class="veil-settings-row veil-e2ee-row">
                <div class="veil-settings-row-info">
                  <div class="veil-settings-row-label">
                    <Icon name="lock" size={13} class="veil-e2ee-mini" />
                    {ch.name}
                  </div>
                  <div class="veil-settings-row-desc">
                    {ch.channelType === 'voice' ? 'Ses kanalı' : 'Metin kanalı'} · MLS E2EE
                  </div>
                </div>
                <div class="veil-e2ee-actions">
                  <button class="btn btn-secondary btn-sm" onclick={() => generateMyKeyPackage(ch.id)} title="Katılmak için sahibine göndereceğin davet kodunu üret">
                    Davet kodun
                  </button>
                  <button class="btn btn-secondary btn-sm" onclick={() => consumeMyWelcome(ch.id)} title="Sahip seni ekledikten sonra anahtarı al">
                    Anahtarını al
                  </button>
                  {#if space.isOwner}
                    <button class="btn btn-secondary btn-sm" onclick={() => (inviteChannelId = inviteChannelId === ch.id ? null : ch.id)}>
                      Üye ekle
                    </button>
                  {/if}
                </div>
              </div>
              {#if inviteChannelId === ch.id}
                <div class="veil-e2ee-invite veil-pop-in">
                  <VeilSelect
                    options={members.map(m => ({ value: m.userId, label: m.displayName || m.username }))}
                    value={inviteMemberId}
                    label="Üye"
                    onChange={(v) => (inviteMemberId = v)}
                  />
                  <textarea
                    class="veil-input veil-e2ee-kp"
                    bind:value={inviteKeyPackage}
                    rows={3}
                    placeholder="Üyenin davet kodu (key package) — üye 'Davet kodun' ile üretir ve sana iletir"
                    aria-label="Davet kodu"
                  ></textarea>
                  <button class="btn btn-primary btn-sm" onclick={sendInvite} disabled={!inviteMemberId || !inviteKeyPackage.trim() || inviteBusy}>
                    {inviteBusy ? 'Ekleniyor…' : 'E2EE\'ye ekle'}
                  </button>
                </div>
              {/if}
              {#if myKeyPackage}
                <div class="veil-e2ee-mykp">
                  <code>{myKeyPackage.slice(0, 64)}…</code>
                  <button
                    class="btn-icon"
                    title="Kodu kopyala"
                    aria-label="Davet kodunu kopyala"
                    onclick={async () => { await copyText(myKeyPackage!); toastStore.success('Kod kopyalandı.'); }}
                  >
                    <Icon name="copy" size={14} />
                  </button>
                </div>
              {/if}
            {/each}
          </div>
        {:else}
          <p class="veil-empty-inline">Henüz E2EE kanal yok. Kanallar sekmesinden "Uçtan uca şifreli" seçerek oluşturabilirsin.</p>
        {/if}
      {/if}

      <!-- ── ROLLER ────────────────────────────────────────────── -->
      {#if tab === 'roles'}
        <div class="veil-settings-group">
          <div class="veil-settings-group-header-row">
            <div>
              <div class="veil-settings-group-label">Roller & Hiyerarşi ({roles.length})</div>
              <p class="veil-settings-row-desc" style="margin-top:2px;">
                Listenin üstündeki roller daha yüksek yetkiye sahiptir. Ok butonlarıyla rol sırasını değiştirebilirsiniz.
              </p>
            </div>
            {#if canManageRoles}
              <button
                class="btn btn-primary btn-sm"
                onclick={() => uiStore.openModal('role-editor', { role: null, spaceId: space.id })}
              >
                <Icon name="plus" size={14} />
                Yeni Rol
              </button>
            {/if}
          </div>

          <div class="roles-hierarchy-list">
            {#each roles as role, index (role.id)}
              <div class="veil-settings-row veil-role-row">
                <!-- Reorder controls -->
                {#if canManageRoles}
                  <div class="role-reorder-controls">
                    <button
                      type="button"
                      class="btn-icon btn-icon-xs"
                      disabled={index === 0}
                      onclick={() => moveRoleUp(index)}
                      title="Yukarı taşı (Daha yüksek yetki)"
                      aria-label="Yukarı taşı"
                    >
                      <Icon name="arrow-up" size={12} />
                    </button>
                    <button
                      type="button"
                      class="btn-icon btn-icon-xs"
                      disabled={index === roles.length - 1}
                      onclick={() => moveRoleDown(index)}
                      title="Aşağı taşı (Daha düşük yetki)"
                      aria-label="Aşağı taşı"
                    >
                      <Icon name="arrow-down" size={12} />
                    </button>
                  </div>
                {/if}

                <div class="veil-settings-row-info">
                  <div class="veil-settings-row-label">
                    {#if role.color}
                      <span class="veil-role-dot" style="background:{role.color};" aria-hidden="true"></span>
                    {/if}
                    <span class="role-name-text">{role.name}</span>
                    <span class="role-rank-badge">#{roles.length - index}</span>
                    {#if role.permissions.includes('administrator')}
                      <span class="role-admin-badge" title="Tüm izinlere koşulsuz sahip">Yönetici</span>
                    {/if}
                  </div>
                  <div class="veil-settings-row-desc">
                    {role.permissions.includes('administrator') ? 'Tüm izinlere sahip (Admin)' : `${role.permissions.length} aktif izin`}
                  </div>
                </div>

                {#if canManageRoles}
                  <div class="veil-member-actions">
                    <button
                      class="btn btn-secondary btn-sm"
                      onclick={() => uiStore.openModal('role-editor', { role, spaceId: space.id })}
                    >
                      Düzenle
                    </button>
                    <button
                      class="btn-icon veil-role-delete"
                      title="Rolü sil"
                      aria-label="{role.name} rolünü sil"
                      onclick={() => deleteRole(role)}
                    >
                      <Icon name="trash" size={14} />
                    </button>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- ── ÜYELER ────────────────────────────────────────────── -->
      {#if tab === 'members'}
        <div class="veil-settings-group">
          <div class="veil-settings-group-label">Üyeler ({members.length})</div>
          {#each members as m (m.userId)}
            <div class="veil-settings-row veil-member-row">
              <div class="veil-settings-row-info">
                <div class="veil-settings-row-label">
                  <Avatar name={m.displayName || m.username} size="sm" hash={m.avatarHash} />
                  <span class="veil-member-name-text">{m.displayName || m.username}</span>
                </div>
                <div class="veil-settings-row-desc">
                  @{m.username}
                  {#if m.roleIds.length > 0}
                    <span class="veil-member-role-chips">
                      {#each m.roleIds.slice(0, 3) as rid (rid)}
                        {@const role = roles.find(r => r.id === rid)}
                        {#if role}
                          <span class="veil-member-role-chip" class:colored={!!role.color} style={role.color ? `--role-color:${role.color}` : ''}>
                            {role.name}
                          </span>
                        {/if}
                      {/each}
                      {#if m.roleIds.length > 3}+{m.roleIds.length - 3}{/if}
                    </span>
                  {/if}
                </div>
              </div>
              <div class="veil-member-actions">
                {#if m.userId === space?.ownerId}
                  <span class="veil-owner-badge" title="Topluluk Kurucusu & Sahibi">
                    👑 Sahip
                  </span>
                {/if}
                {#if canManageRoles && m.userId !== space.ownerId}
                  <button class="btn btn-secondary btn-sm" title="Rolleri Yönet" onclick={() => openRoleAssign(m)}>
                    <Icon name="shield" size={13} />
                    Roller
                  </button>
                {/if}
                {#if isOwner && m.userId !== space.ownerId}
                  <button class="btn btn-secondary btn-sm" title="Sahipliği Devret" onclick={() => transferOwnershipToMember(m)}>
                    <Icon name="key" size={13} />
                    Sahipliği Aktar
                  </button>
                {/if}
                {#if canModerate && m.userId !== space.ownerId}
                  <button class="btn btn-secondary btn-sm" title="Geçici sustur" onclick={() => timeoutMember(m)}>
                    <Icon name="moon" size={13} />
                  </button>
                {/if}
                {#if canKick && m.userId !== space.ownerId}
                  <button class="btn btn-secondary btn-sm" title="Topluluktan at" onclick={() => kickMember(m)}>
                    <Icon name="logout" size={13} />
                  </button>
                {/if}
                {#if canBan && m.userId !== space.ownerId}
                  <button class="btn btn-danger btn-sm" title="Yasakla" onclick={() => banMember(m)}>
                    <Icon name="x" size={13} />
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        {#if bans.length > 0}
          <div class="veil-settings-group">
            <div class="veil-settings-group-label">Yasaklılar ({bans.length})</div>
            {#each bans as b (b.userId)}
              <div class="veil-settings-row">
                <div class="veil-settings-row-info">
                  <div class="veil-settings-row-label">{b.displayName || b.username}</div>
                  <div class="veil-settings-row-desc">
                    @{b.username}
                    {#if b.reason}<span class="veil-ban-reason">· {b.reason}</span>{/if}
                  </div>
                </div>
                {#if canBan}
                  <button class="btn btn-secondary btn-sm" onclick={() => unbanMember(b)}>Yasağı Kaldır</button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      {/if}

      <!-- ── DAVET & BAĞLANTI ──────────────────────────────────── -->
      {#if tab === 'invites'}
        <div class="veil-settings-group">
          <div class="veil-settings-group-label">Özel Topluluk Bağlantısı</div>
          <p class="veil-settings-row-desc veil-note">
            Topluluğun için kısa ve akılda kalıcı bir bağlantı al: <code>veilanon.com/join/<b>adın</b></code>.
            Bu bağlantı <strong>yalnızca bir kez alınabilir</strong> ve sonradan değiştirilemez ya da iptal edilemez —
            paylaşan herkes bu bağlantıyla topluluğa katılabilir.
          </p>
          {#if customLinkValue}
            <div class="veil-invite-code" aria-label="Özel topluluk bağlantısı">
              <code data-streamer-mask="invite" data-auto-protect="secret">
                {$streamerMode.enabled && $streamerMode.hideInviteLinks ? maskInviteLink(customLinkValue) : `veilanon.com/join/${customLinkValue}`}
              </code>
              <button class="btn btn-secondary btn-sm" onclick={copyCustomLink} title="Bağlantıyı kopyala">
                <Icon name="copy" size={13} />
                Kopyala
              </button>
            </div>
            <p class="veil-settings-row-desc veil-note">Bu bağlantı bir kez alındı, değiştirilemez.</p>
          {:else if space?.isOwner}
            <div class="veil-create-channel">
              <input
                class="veil-input veil-mono-input"
                bind:value={customLinkDraft}
                placeholder="topluluk-adin (küçük harf, rakam, tire)"
                aria-label="Özel bağlantı adı"
                maxlength={32}
                autocomplete="off"
                spellcheck={false}
                onkeydown={(e) => { if (e.key === 'Enter') claimCustomLink(); }}
              />
              <button class="btn btn-primary btn-sm" onclick={claimCustomLink} disabled={customLinkBusy || !customLinkDraft.trim()}>
                {customLinkBusy ? 'Alınıyor…' : 'Bağlantıyı Al'}
              </button>
            </div>
            {#if customLinkError}
              <p class="veil-form-error" role="alert">{customLinkError}</p>
            {/if}
            <p class="veil-settings-row-desc veil-note">
              Bağlantıyı almazsan davetlerde standart (rastgele kodlu) bağlantı kullanılır — aşağıdan oluşturabilirsin.
            </p>
          {:else}
            <p class="veil-settings-row-desc">Bu topluluk sahibi özel bağlantı almamış. Davet linkiyle katılabilirsin.</p>
          {/if}
        </div>

        <div class="veil-settings-group">
          <div class="veil-settings-group-label">Davet Linki</div>
          {#if invites}
            <div class="veil-invite-code" aria-label="Davet linki">
              <code data-streamer-mask="invite" data-auto-protect="secret">
                {$streamerMode.enabled && $streamerMode.hideInviteLinks ? maskInviteLink(invites.code) : `${INVITE_BASE}${invites.code}`}
              </code>
              <button class="btn btn-secondary btn-sm" onclick={copyInvite} title="Linki kopyala">
                <Icon name="copy" size={13} />
                Kopyala
              </button>
            </div>
            <p class="veil-settings-row-desc veil-note">Bu linki paylaşan herkes topluluğa katılabilir.</p>
          {:else}
            <button class="btn btn-secondary btn-sm" onclick={createInvite}>Davet linki oluştur</button>
          {/if}
          <div class="veil-server-link">
            <button class="btn btn-secondary btn-sm" onclick={copyServerLink} title="veilanon://server/... bağlantısını kopyala">
              <Icon name="link" size={13} />
              Topluluk bağlantısını kopyala
            </button>
            <span class="veil-settings-row-desc">Yüklü veilanon'da bu topluluğu doğrudan açar.</span>
          </div>
        </div>
      {/if}

      <!-- ── DISCORD KÖPRÜSÜ ───────────────────────────────────── -->
      {#if tab === 'bridge'}
        <div class="veil-settings-group">
          <div class="veil-settings-group-label">Discord Köprüsü</div>
          <p class="veil-settings-row-desc veil-note">
            Köprü, Discord webhook'uyla mesajlarını dışarı yansıtır. Köprüden geçen mesajlar
            Discord tarafında E2EE korumasına SAHİP DEĞİLDİR ve "[köprü]" etiketiyle gönderilir.
          </p>
          <div class="veil-bridge-row">
            <VeilSelect
              options={channels.filter(c => c.channelType === 'text' || c.channelType === 'announcement' || c.channelType === 'forum').map(c => ({ value: c.id, label: `# ${c.name}` }))}
              value={bridgeChannelId}
              label="Kanal"
              onChange={(v) => { bridgeChannelId = v; void loadBridge(); }}
            />
          </div>
          {#if bridgeChannelId}
            {#if bridgeInfo}
              <div class="veil-bridge-status">
                <span class="veil-ai-ok">
                  <span class="veil-ai-dot" aria-hidden="true"></span>
                  Köprü aktif: {bridgeInfo.maskedUrl}
                </span>
                <button class="btn btn-secondary btn-sm" onclick={clearBridge}>Kaldır</button>
              </div>
            {:else}
              <div class="veil-bridge-row">
                <input
                  class="veil-input"
                  data-auto-protect="secret"
                  bind:value={bridgeUrl}
                  placeholder="https://discord.com/api/webhooks/…"
                  aria-label="Discord webhook URL"
                  autocomplete="off"
                  spellcheck={false}
                />
                <button class="btn btn-primary btn-sm" onclick={saveBridge} disabled={!bridgeUrl.trim() || bridgeBusy}>
                  {bridgeBusy ? 'Kaydediliyor…' : 'Bağla'}
                </button>
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    {/if}
    </div>

    {#if roleAssignFor}
      <div class="veil-role-assign veil-pop-in" role="dialog" aria-label="Rol ata">
        <div class="veil-role-assign-head">
          <div class="veil-role-assign-title">
            <Avatar name={roleAssignFor.displayName || roleAssignFor.username} size="sm" hash={roleAssignFor.avatarHash} />
            <span>{roleAssignFor.displayName || roleAssignFor.username} için roller</span>
          </div>
          <button class="btn-icon" aria-label="Kapat" onclick={() => (roleAssignFor = null)}>
            <Icon name="x" size={14} />
          </button>
        </div>
        {#if roles.length === 0}
          <p class="veil-settings-row-desc">Henüz rol yok. Önce Roller sekmesinden rol oluştur.</p>
        {:else}
          <div class="veil-role-assign-list">
            {#each roles as role (role.id)}
              <label class="veil-perm-item">
                <input
                  type="checkbox"
                  checked={roleAssignDraft.includes(role.id)}
                  onchange={() => toggleRoleAssign(role.id)}
                />
                <span class="veil-role-assign-name">
                  {#if role.color}
                    <span class="veil-role-dot" style="background:{role.color};" aria-hidden="true"></span>
                  {/if}
                  {role.name}
                </span>
              </label>
            {/each}
          </div>
        {/if}
        <div class="veil-role-assign-actions">
          <button class="btn btn-secondary btn-sm" onclick={() => (roleAssignFor = null)}>Vazgeç</button>
          <button class="btn btn-primary btn-sm" onclick={saveRoleAssign} disabled={roleAssignBusy}>
            {roleAssignBusy ? 'Kaydediliyor…' : 'Kaydet'}
          </button>
        </div>
      </div>
    {/if}

    {#if iconCropSrc}
      <ImageCropModal
        src={iconCropSrc}
        shape="circle"
        aspectRatio={1}
        title="Topluluk İkonunu Ayarla"
        onSave={handleSpaceIconCropSave}
        onClose={() => { iconCropSrc = null; }}
      />
    {/if}

    {#if cropSrc}
      <BannerCropModal
        src={cropSrc}
        aspectRatio={2.4}
        title="Topluluk Bannerını Ayarla"
        hasAvatarPreview={true}
        avatarName={space?.name}
        avatarHash={space?.iconHash}
        onSave={handleSpaceBannerCropSave}
        onClose={() => { cropSrc = null; }}
      />
    {/if}
  </div>
{/if}

<style>
  .veil-empty-inline { color: var(--veil-text-muted); padding: var(--space-4); }
  .veil-create-channel {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
  .veil-role-dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: var(--radius-full);
    margin-right: var(--space-1);
  }
  .veil-invite-code {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-lg);
  }
  .veil-invite-code code {
    flex: 1;
    font-family: var(--font-mono);
    letter-spacing: 0.08em;
    user-select: text;
    word-break: break-all;
  }
  .veil-e2ee-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-3);
    font-size: var(--text-sm);
    color: var(--veil-text-secondary);
    cursor: pointer;
  }
  .veil-e2ee-toggle input { accent-color: var(--veil-brand); }
  .veil-e2ee-row { flex-wrap: wrap; }
  .veil-e2ee-actions { display: flex; gap: var(--space-1); flex-wrap: wrap; }
  .veil-e2ee-invite {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    margin-bottom: var(--space-2);
  }
  .veil-e2ee-kp { resize: vertical; font-family: var(--font-mono); font-size: var(--text-xs); }
  .veil-e2ee-mykp {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-2);
  }
  .veil-e2ee-mykp code {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--veil-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .veil-bridge-row { display: flex; gap: var(--space-2); align-items: flex-end; margin-top: var(--space-2); }
  .veil-bridge-row .veil-input { flex: 1; min-width: 0; font-family: var(--font-mono); font-size: var(--text-xs); }
  .veil-bridge-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--veil-bg-elevated);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
  }
  .veil-ai-ok { display: inline-flex; align-items: center; gap: var(--space-1); color: var(--veil-success); font-size: var(--text-sm); }
  .veil-ai-dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--veil-success);
    display: inline-block;
  }
  .veil-member-row { flex-wrap: wrap; }
  .veil-member-actions { display: flex; gap: var(--space-1); flex-shrink: 0; }
  .veil-ban-reason { color: var(--veil-text-muted); }
  .veil-server-link {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .veil-space-hero {
    position: relative;
    border-radius: var(--radius-xl);
    overflow: hidden;
    border: 1px solid var(--veil-border-subtle);
    margin-bottom: var(--space-4);
    background: var(--veil-bg-surface);
    width: 100%;
    aspect-ratio: 2.8 / 1;
    min-height: 150px;
    max-height: 200px;
    box-shadow: var(--shadow-sm);
  }
  :global(.veil-space-hero-banner) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    display: block;
  }
  .veil-space-hero-placeholder {
    width: 100%;
    height: 100%;
    background:
      radial-gradient(120% 160% at 15% 0%, var(--veil-brand-subtle), transparent 55%),
      linear-gradient(160deg, var(--veil-bg-surface), var(--veil-bg-void));
  }
  .veil-space-hero-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(to top, rgba(10, 12, 18, 0.95) 0%, rgba(10, 12, 18, 0.6) 45%, transparent 100%);
    display: flex;
    align-items: flex-end;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    z-index: 2;
  }
  .veil-space-hero-overlay :global(.veil-avatar) {
    border: 3px solid rgba(255, 255, 255, 0.25);
    border-radius: var(--radius-full);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);
    flex-shrink: 0;
  }
  .veil-space-hero-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .veil-space-hero-name {
    font-size: var(--text-xl);
    font-weight: 700;
    letter-spacing: var(--tracking-tight);
    color: #ffffff;
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.85);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-space-hero-meta {
    font-size: var(--text-xs);
    font-weight: 600;
    color: rgba(255, 255, 255, 0.8);
    text-shadow: 0 1px 4px rgba(0, 0, 0, 0.85);
  }
  .veil-owner-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 9px;
    border-radius: var(--radius-full);
    background: rgba(250, 179, 135, 0.16);
    border: 1px solid rgba(250, 179, 135, 0.4);
    color: #fab387;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
    user-select: none;
  }
  .veil-space-media-actions {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    flex-wrap: wrap;
  }
  .veil-bio-input { resize: vertical; min-height: 56px; }
  .veil-bio-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }
  .veil-bio-count { font-size: var(--text-xs); color: var(--veil-text-muted); font-variant-numeric: tabular-nums; }
  .veil-displayname-form { display: flex; gap: var(--space-2); }
  .veil-displayname-form .veil-input { flex: 1; min-width: 0; }

  /* Discord tarzı dikey sol menü ve içerik düzeni */
  .veil-space-settings-layout {
    display: flex;
    min-height: min(640px, 78dvh);
    max-height: min(640px, 78dvh);
    margin: -1.25rem;
    overflow: hidden;
  }
  .veil-space-settings-nav {
    width: 230px;
    flex-shrink: 0;
    background: var(--veil-bg-raised);
    border-right: 1px solid var(--veil-border-subtle);
    display: flex;
    flex-direction: column;
    padding: var(--space-4) var(--space-3);
  }
  .veil-space-nav-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-2) var(--space-4) var(--space-2);
    border-bottom: 1px solid var(--veil-border-subtle);
    margin-bottom: var(--space-2);
  }
  .veil-space-nav-header-info { min-width: 0; }
  .veil-space-nav-title {
    font-weight: 700;
    font-size: var(--text-base);
    color: var(--veil-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-space-nav-sub {
    font-size: var(--text-xs);
    color: var(--veil-text-muted);
  }
  .veil-space-nav-scroll {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .veil-space-settings-content {
    flex: 1;
    min-width: 0;
    padding: var(--space-5) var(--space-6);
    overflow-y: auto;
  }

  .veil-channel-type-badge { display: inline-block; width: 18px; text-align: center; margin-right: 2px; }
  .veil-role-row { flex-wrap: wrap; }
  .veil-role-delete { color: var(--veil-text-muted); }
  .veil-role-delete:hover { background: var(--veil-danger); color: #fff; }

  .veil-member-name-text { margin-left: var(--space-2); vertical-align: middle; }
  .veil-member-role-chips {
    display: inline-flex;
    gap: 3px;
    flex-wrap: wrap;
    margin-left: var(--space-1);
    vertical-align: middle;
  }
  .veil-member-role-chip {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px;
    font-size: var(--text-xs);
    font-weight: 600;
    border-radius: var(--radius-full);
    background: var(--veil-bg-overlay);
    color: var(--veil-text-muted);
  }
  .veil-member-role-chip.colored {
    color: var(--role-color, var(--veil-brand));
    background: color-mix(in srgb, var(--role-color, var(--veil-brand)) 12%, transparent);
  }

  /* Rol atama paneli (üye satırından açılır) */
  .veil-role-assign {
    position: fixed;
    z-index: 1200;
    right: var(--space-5);
    bottom: var(--space-5);
    width: 320px;
    max-width: calc(100vw - var(--space-6));
    background: color-mix(in srgb, var(--veil-bg-raised) 96%, transparent);
    backdrop-filter: blur(14px);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-xl);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .veil-role-assign-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
  .veil-role-assign-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
    font-size: var(--text-sm);
    min-width: 0;
  }
  .veil-role-assign-title span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-role-assign-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 260px;
    overflow-y: auto;
  }
  .veil-role-assign-name { display: inline-flex; align-items: center; gap: var(--space-1); }
  .veil-settings-group-header-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
  }
  .role-reorder-controls {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-right: var(--space-2);
  }
  .btn-icon-xs {
    width: 20px;
    height: 20px;
    padding: 0;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--veil-bg-overlay);
    color: var(--veil-text-muted);
    border: none;
    cursor: pointer;
    transition: all var(--t-fast);
  }
  .btn-icon-xs:hover:not(:disabled) {
    background: var(--veil-bg-surface);
    color: var(--veil-text-primary);
  }
  .btn-icon-xs:disabled {
    opacity: 0.25;
    cursor: not-allowed;
  }
  .role-name-text {
    font-weight: 600;
  }
  .role-rank-badge {
    font-size: 10.5px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    background: var(--veil-bg-overlay);
    color: var(--veil-text-muted);
    margin-left: var(--space-1);
  }
  .role-admin-badge {
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 4px;
    background: #eb4d4b;
    color: #ffffff;
    text-transform: uppercase;
    margin-left: var(--space-1);
  }
</style>

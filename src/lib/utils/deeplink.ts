/**
 * veilanon deep-link ve web bağlantısı işleme motoru.
 *
 * Desteklenen URL biçimleri (hem `veilanon://` protokolü hem `https://veilanon.com` vb. alan adları):
 *   veilanon://invite/CODE veya /join/KOD veya /c/KOD → topluluk daveti / katılma
 *   veilanon://u/USERNAME veya /user/KULLANICI veya /profile/KULLANICI → kullanıcı profili
 *   veilanon://friend/USERNAME veya /friend-request/KULLANICI veya /add-friend/KULLANICI → arkadaşlık
 *   veilanon://server/SPACE_ID veya /space/SPACE_ID veya /spaces/SPACE_ID → topluluğa git
 *   veilanon://channel/CHANNEL_ID veya /channels/SPACE_ID/CHANNEL_ID → kanala git
 *   veilanon://message/CH/ID veya /channels/SPACE_ID/CH/MSG → belirli mesaja git ve odaklan
 *   veilanon://dm/DM_ID veya /direct/DM_ID → direkt mesaja git
 */
import { get } from 'svelte/store';
import { uiStore } from '$lib/stores/ui';
import { spaceStore } from '$lib/stores/spaces';
import { toastStore } from '$lib/stores/notifications';
import { socialApi } from '$lib/api/tauri';
import type { UserProfileInfo } from '$lib/api/tauri';

export type DeepLinkAction =
  | { kind: 'invite'; code: string }
  | { kind: 'profile'; username: string }
  | { kind: 'friend'; username: string }
  | { kind: 'server'; spaceId: string }
  | { kind: 'channel'; channelId: string; spaceId?: string }
  | { kind: 'message'; channelId: string; messageId: string; spaceId?: string }
  | { kind: 'dm'; dmId: string }
  | { kind: 'unknown'; raw: string };

const APP_DOMAINS = [
  'veilanon.com',
  'www.veilanon.com',
  'veilanon.com.tr',
  'www.veilanon.com.tr',
  'veilanon.online',
  'www.veilanon.online',
  'veilanon.info',
  'www.veilanon.info',
  'localhost',
  '127.0.0.1',
];

function stripAppHost(rawUrl: string): string {
  try {
    const u = new URL(rawUrl);
    if (APP_DOMAINS.includes(u.hostname.toLowerCase())) {
      return u.pathname + u.search;
    }
  } catch {
    return rawUrl;
  }
  return rawUrl;
}

export function parseDeepLink(raw: string): DeepLinkAction {
  const clean = raw.trim();
  if (!clean) return { kind: 'unknown', raw: '' };

  const fromWeb = clean.startsWith('http://') || clean.startsWith('https://');
  const stripped = fromWeb ? stripAppHost(clean) : clean;

  // Query parametresi varsa önce kontrol et (örn: /invite?code=ABC123DEF veya ?uri=veilanon://...)
  if (stripped.includes('?')) {
    try {
      const qIndex = stripped.indexOf('?');
      const searchStr = stripped.slice(qIndex);
      const params = new URLSearchParams(searchStr);
      const uriParam = params.get('uri') || params.get('url');
      if (uriParam) {
        return parseDeepLink(uriParam);
      }
      const codeParam = params.get('code');
      if (codeParam) {
        return { kind: 'invite', code: codeParam };
      }
    } catch { /* ignored */ }
  }

  const path = stripped
    .replace(/^veilanon:\/\//i, '')
    .replace(/^\/+/, '')
    .split('?')[0];

  const segs = path.split('/').filter(Boolean);
  if (segs.length === 0) return { kind: 'unknown', raw: clean };

  const root = segs[0].toLowerCase();
  const first = segs[1] ? decodeURIComponent(segs[1]) : '';
  const second = segs[2] ? decodeURIComponent(segs[2]) : '';
  const third = segs[3] ? decodeURIComponent(segs[3]) : '';

  // 1. Davet / Katılma
  if ((root === 'invite' || root === 'join' || root === 'c') && first) {
    return { kind: 'invite', code: first };
  }

  // 2. Profil
  if ((root === 'u' || root === 'user' || root === 'profile') && first) {
    return { kind: 'profile', username: first.replace(/^@/, '') };
  }

  // 3. Arkadaşlık
  if ((root === 'friend' || root === 'friend-request' || root === 'add-friend') && first) {
    return { kind: 'friend', username: first.replace(/^@/, '') };
  }

  // 4. Discord-style /channels/SPACE_ID/CHANNEL_ID(/MESSAGE_ID)
  if (root === 'channels' && first && second) {
    if (third) {
      return { kind: 'message', spaceId: first, channelId: second, messageId: third };
    }
    return { kind: 'channel', spaceId: first, channelId: second };
  }

  // 5. Sunucu / Topluluk
  if ((root === 'server' || root === 'space' || root === 'spaces') && first) {
    return { kind: 'server', spaceId: first };
  }

  // 6. Kanal
  if (root === 'channel' && first) {
    return { kind: 'channel', channelId: first };
  }

  // 7. Mesaj
  if (root === 'message' && first && second) {
    return { kind: 'message', channelId: first, messageId: second };
  }
  if (root === 'm' && first) {
    return { kind: 'message', channelId: '', messageId: first };
  }

  // 8. DM / Direkt Mesaj
  if ((root === 'dm' || root === 'direct') && first) {
    return { kind: 'dm', dmId: first };
  }

  // 9. Doğrudan tek segment @username desteği
  if (root.startsWith('@') && root.length > 1) {
    return { kind: 'profile', username: root.replace(/^@/, '') };
  }

  return { kind: 'unknown', raw: clean };
}

let lastHandledLink = '';
let lastHandledTime = 0;

export async function handleDeepLink(raw: string): Promise<void> {
  const clean = raw.trim();
  const now = Date.now();
  if (clean && clean === lastHandledLink && now - lastHandledTime < 4000) {
    return;
  }
  lastHandledLink = clean;
  lastHandledTime = now;

  const action = parseDeepLink(raw);
  switch (action.kind) {
    case 'invite': {
      try {
        let space;
        try {
          // Önce standart davet kodu olarak çöz
          space = await spaceStore.redeem(action.code);
        } catch {
          // Başarısız olursa özel kısa bağlantı / vanity link veya açık topluluk olarak katılmayı dene
          space = await spaceStore.joinPublic(action.code);
        }
        toastStore.success(`Topluluğa katıldın: ${space.name}`);
        uiStore.navigate(space.id, null);
        await spaceStore.loadChannels(space.id);
      } catch {
        toastStore.error('Davet veya topluluk bağlantısı geçersiz ya da süresi dolmuş.');
      }
      break;
    }
    case 'profile':
    case 'friend': {
      try {
        const p: UserProfileInfo = await socialApi.resolveUsername(action.username);
        uiStore.openModal('user-profile', {
          userId: p.userId,
          username: p.username,
          displayName: p.displayName,
          avatarHash: p.avatarHash,
          onlineStatus: p.onlineStatus,
        });
      } catch {
        toastStore.error(`@${action.username} kullanıcısı bulunamadı.`);
      }
      break;
    }
    case 'server': {
      await spaceStore.loadSpaces();
      const space = get(spaceStore).spaces.find(s => s.id === action.spaceId);
      if (space) {
        uiStore.navigate(space.id, null);
        await spaceStore.loadChannels(space.id);
      } else {
        // Kullanıcı henüz üye değilse, katılmayı dene
        try {
          let joined;
          try {
            joined = await spaceStore.joinPublic(action.spaceId);
          } catch {
            joined = await spaceStore.redeem(action.spaceId);
          }
          toastStore.success(`Topluluğa katıldın: ${joined.name}`);
          uiStore.navigate(joined.id, null);
          await spaceStore.loadChannels(joined.id);
        } catch {
          toastStore.error('Topluluk bulunamadı veya üye değilsin.');
        }
      }
      break;
    }
    case 'channel': {
      await spaceStore.loadSpaces();
      await Promise.all(get(spaceStore).spaces.map(s => spaceStore.loadChannels(s.id)));
      const state = get(spaceStore);

      // Eğer spaceId biliniyorsa doğrudan oraya git
      if (action.spaceId && state.channelsBySpace[action.spaceId]) {
        const ch = state.channelsBySpace[action.spaceId].find(c => c.id === action.channelId);
        if (ch) {
          uiStore.navigate(action.spaceId, ch.id);
          return;
        }
      }

      // Değilse tüm kanallarda ara
      for (const spaceId of Object.keys(state.channelsBySpace)) {
        const ch = state.channelsBySpace[spaceId].find(c => c.id === action.channelId);
        if (ch) {
          uiStore.navigate(spaceId, ch.id);
          return;
        }
      }
      toastStore.error('Kanal bulunamadı.');
      break;
    }
    case 'message': {
      await spaceStore.loadSpaces();
      await Promise.all(get(spaceStore).spaces.map(s => spaceStore.loadChannels(s.id)));
      const state = get(spaceStore);

      let foundSpaceId = action.spaceId;
      let foundChannelId = action.channelId;

      if (!foundSpaceId) {
        for (const spaceId of Object.keys(state.channelsBySpace)) {
          const ch = state.channelsBySpace[spaceId].find(c => c.id === action.channelId);
          if (ch) {
            foundSpaceId = spaceId;
            foundChannelId = ch.id;
            break;
          }
        }
      }

      if (foundSpaceId && foundChannelId) {
        uiStore.navigate(foundSpaceId, foundChannelId);
        setTimeout(() => {
          const el = document.getElementById(`msg-${action.messageId}`);
          if (el) {
            el.scrollIntoView({ behavior: 'smooth', block: 'center' });
            el.classList.add('veil-msg-highlight');
            setTimeout(() => el.classList.remove('veil-msg-highlight'), 2500);
          }
        }, 500);
        return;
      }

      toastStore.error('Kanal veya mesaj bulunamadı.');
      break;
    }
    case 'dm': {
      await spaceStore.loadDms();
      const state = get(spaceStore);
      const dm = state.dmChannels.find(d => d.id === action.dmId);
      if (dm) {
        uiStore.navigateDm(dm.id);
      } else {
        // ID doğrudan kanal ID'si olarak açılmayı denensin
        uiStore.navigateDm(action.dmId);
      }
      break;
    }
    default:
      toastStore.info('Bu bağlantı tanınmadı.');
  }
}

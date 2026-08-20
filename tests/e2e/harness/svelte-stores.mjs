/**
 * VeilAnon E2E Test Harness — Svelte Reactive Store Mocks
 * High-fidelity pure-JS implementation of VeilAnon's reactive stores
 * with full subscriptions, masking, disappearing message countdowns,
 * keyboard navigation, and settings persistence.
 */

// Simple Svelte-like writable store
export function createWritable(initialValue) {
  let value = initialValue;
  const subscribers = new Set();

  function subscribe(run) {
    subscribers.add(run);
    run(value);
    return () => subscribers.delete(run);
  }

  function set(newValue) {
    value = newValue;
    subscribers.forEach(fn => fn(value));
  }

  function update(fn) {
    set(fn(value));
  }

  function get() {
    return value;
  }

  return { subscribe, set, update, get };
}

// ── Streamer Mode Store Mock ─────────────────────────────────────────

export function createStreamerModeMock(initialEnabled = false) {
  const store = createWritable({
    enabled: initialEnabled,
    autoEnableOnScreenShare: true,
    preset: 'streamer_balanced',
    maskStyle: 'asterisks',
    hideAccountDetails: true,
    hideUserIds: true,
    hideInviteLinks: true,
    hideDmContent: true,
    hideVoiceParticipants: false,
    suppressNotificationPopups: true,
    suppressAudioAlerts: true,
    blurMediaAttachments: true,
    hideSystemDiagnostics: true,
    hideServerDetails: true,
  });

  return {
    ...store,
    toggle(forced) {
      store.update(s => ({ ...s, enabled: forced !== undefined ? forced : !s.enabled }));
    },
    setEnabled(enabled) {
      store.update(s => ({ ...s, enabled }));
    },
    setPreset(preset) {
      store.update(s => ({ ...s, preset }));
    },
    setMaskStyle(maskStyle) {
      store.update(s => ({ ...s, maskStyle }));
    },
    updateSetting(key, val) {
      store.update(s => ({ ...s, [key]: val }));
    },
    maskText(text, style, length) {
      if (!text) return '';
      const state = store.get();
      if (!state.enabled) return text;
      const resolvedStyle = style || state.maskStyle;
      const len = length ?? Math.min(Math.max(text.length, 6), 16);
      if (resolvedStyle === 'bullets') return '•'.repeat(len);
      if (resolvedStyle === 'hidden') return '[GİZLENDİ]';
      return '*'.repeat(len);
    },
    maskEmail(email) {
      if (!email) return '';
      const state = store.get();
      if (!state.enabled || !state.hideAccountDetails) return email;
      if (state.maskStyle === 'hidden') return '[GİZLİ E-POSTA]';
      if (state.maskStyle === 'bullets') return '••••••••••@•••••••.•••';
      return '**********@*******.***';
    },
    maskUserId(id) {
      if (!id) return '';
      const state = store.get();
      if (!state.enabled || !state.hideUserIds) return id;
      if (state.maskStyle === 'hidden') return '[GİZLİ KİMLİK]';
      if (state.maskStyle === 'bullets') return '••••-••••-••••';
      return '****-****-****';
    },
    maskInviteLink(link) {
      if (!link) return '';
      const state = store.get();
      if (!state.enabled || !state.hideInviteLinks) return link;
      if (state.maskStyle === 'hidden') return '[GİZLİ DAVET BAĞLANTISI]';
      if (state.maskStyle === 'bullets') return 'veilanon://join/••••••••';
      return 'veilanon://join/********';
    },
    maskPath(path) {
      if (!path) return '';
      const state = store.get();
      if (!state.enabled || !state.hideSystemDiagnostics) return path;
      if (state.maskStyle === 'hidden') return '[GİZLİ DOSYA YOLU]';
      if (state.maskStyle === 'bullets') return '••••/••••/••••/logs';
      return '****/****/****/logs';
    },
  };
}

// ── Privacy Shield Store Mock ────────────────────────────────────────

export function createPrivacyShieldMock(streamerStore, initialAutoShield = true) {
  const config = createWritable({
    autoShieldEnabled: initialAutoShield,
    protectEmailsAndIds: true,
    protectPasswordsAndKeys: true,
    protectInvites: true,
    protectWebhooks: true,
    autoHideTimeoutSeconds: 5,
    maskStyle: 'bullets',
    blurMediaOnShare: true,
    protectClipboard: true,
  });

  const isScreenSharing = createWritable(false);
  const revealedSecrets = new Map();

  return {
    config,
    isScreenSharing,
    setScreenSharing(active) {
      isScreenSharing.set(active);
    },
    isShieldActive() {
      const cfg = config.get();
      const sharing = isScreenSharing.get();
      const streamer = streamerStore ? streamerStore.get().enabled : false;
      return streamer || (cfg.autoShieldEnabled && sharing);
    },
    revealSecret(key, durationSeconds = 5) {
      const expiry = Date.now() + (durationSeconds * 1000);
      revealedSecrets.set(key, expiry);
    },
    hideSecret(key) {
      revealedSecrets.delete(key);
    },
    isSecretRevealed(key) {
      const exp = revealedSecrets.get(key);
      return exp ? exp > Date.now() : false;
    },
    formatSecret(val, key, forceMask = false) {
      if (!val) return '';
      if (!forceMask && this.isSecretRevealed(key)) return val;
      const cfg = config.get();
      const active = this.isShieldActive();
      if (!forceMask && !active && !cfg.autoShieldEnabled) return val;
      if (cfg.maskStyle === 'bullets') return '•'.repeat(Math.min(Math.max(val.length, 8), 24));
      if (cfg.maskStyle === 'hidden') return '[GİZLENDİ]';
      return '*'.repeat(Math.min(Math.max(val.length, 8), 24));
    },
  };
}

// ── Trusted Domains Store Mock ───────────────────────────────────────

export function createTrustedDomainsMock() {
  const defaultDomains = [
    'github.com',
    'google.com',
    'youtube.com',
    'aegissoft.com.tr',
    'veilanon.com',
    'wikipedia.org',
  ];

  const store = createWritable({
    trustedDomains: [...defaultDomains],
    directRedirectForTrusted: true,
    alwaysOpenWithoutPrompt: false,
  });

  function extractDomain(urlOrDomain) {
    try {
      let clean = urlOrDomain.trim().toLowerCase();
      if (!clean.startsWith('http://') && !clean.startsWith('https://')) {
        clean = 'https://' + clean;
      }
      const parsed = new URL(clean);
      return parsed.hostname.replace(/^www\./, '');
    } catch {
      return urlOrDomain.trim().toLowerCase().replace(/^www\./, '');
    }
  }

  return {
    ...store,
    extractDomain,
    isTrusted(urlOrDomain) {
      const s = store.get();
      if (s.alwaysOpenWithoutPrompt) return true;
      const domain = extractDomain(urlOrDomain);
      return s.trustedDomains.some(td => {
        const cleanTd = td.toLowerCase().trim().replace(/^www\./, '');
        return domain === cleanTd || domain.endsWith('.' + cleanTd);
      });
    },
    shouldDirectRedirect(urlOrDomain) {
      const s = store.get();
      if (s.alwaysOpenWithoutPrompt) return true;
      return s.directRedirectForTrusted && this.isTrusted(urlOrDomain);
    },
    addTrustedDomain(domain) {
      const clean = extractDomain(domain);
      if (!clean) return;
      store.update(s => {
        if (s.trustedDomains.includes(clean)) return s;
        return { ...s, trustedDomains: [...s.trustedDomains, clean] };
      });
    },
    removeTrustedDomain(domain) {
      const clean = extractDomain(domain);
      store.update(s => ({
        ...s,
        trustedDomains: s.trustedDomains.filter(d => d !== clean),
      }));
    },
  };
}

// ── UI & Navigation Store Mock ───────────────────────────────────────

export function createUiStoreMock() {
  const store = createWritable({
    theme: 'dark',
    accentColor: null,
    isAmoled: false,
    activeSpaceId: null,
    activeChannelId: null,
    activeDmId: null,
    openModal: null,
    modalData: null,
    settingsTab: 'account',
    presence: 'online',
    compactMode: false,
    confirmDialog: null,
    inputDialog: null,
    replyTo: null,
  });

  return {
    ...store,
    setTheme(theme) {
      store.update(s => ({ ...s, theme }));
    },
    setAccentColor(color) {
      store.update(s => ({ ...s, accentColor: color }));
    },
    setAmoledMode(enabled) {
      store.update(s => ({ ...s, isAmoled: enabled }));
    },
    navigate(spaceId, channelId) {
      store.update(s => ({
        ...s,
        activeSpaceId: spaceId,
        activeChannelId: channelId,
        activeDmId: null,
      }));
    },
    navigateDm(dmId) {
      store.update(s => ({
        ...s,
        activeSpaceId: null,
        activeChannelId: null,
        activeDmId: dmId,
      }));
    },
    openModal(modal, data = null) {
      store.update(s => ({
        ...s,
        openModal: modal,
        modalData: data,
        settingsTab: (data && data.tab) || s.settingsTab,
      }));
    },
    closeModal() {
      store.update(s => ({ ...s, openModal: null, modalData: null }));
    },
    setPresence(presence) {
      store.update(s => ({ ...s, presence }));
    },
    setCompactMode(compact) {
      store.update(s => ({ ...s, compactMode: compact }));
    },
    setReplyTo(replyTo) {
      store.update(s => ({ ...s, replyTo }));
    },
  };
}

// ── Message Store Mock with Disappearing Messages & Countdowns ───────

export function createMessageStoreMock() {
  const store = createWritable({
    byChannel: {},
    queuedMessages: [],
  });

  return {
    ...store,
    sendMessage(channelId, content, replyToId = null, disappearSeconds = null) {
      const nowSec = Math.floor(Date.now() / 1000);
      const disappearsAt = disappearSeconds ? nowSec + disappearSeconds : null;
      const msg = {
        id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        channelId,
        senderId: 'user-self',
        senderName: 'VeilUser',
        content,
        messageType: 'text',
        status: 'sent',
        replyToId,
        pinned: false,
        reactions: [],
        attachments: [],
        createdAt: nowSec,
        disappearsAt,
        isOwn: true,
      };

      store.update(s => ({
        ...s,
        byChannel: {
          ...s.byChannel,
          [channelId]: [...(s.byChannel[channelId] || []), msg],
        },
      }));
      return msg;
    },

    queueOfflineMessage(channelId, content, replyToId = null, disappearSeconds = null) {
      const nowSec = Math.floor(Date.now() / 1000);
      const msg = {
        id: `queued-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        channelId,
        senderId: 'user-self',
        content,
        status: 'queued',
        replyToId,
        createdAt: nowSec,
        disappearSeconds,
        disappearsAt: null, // Computed upon flush
      };

      store.update(s => ({
        ...s,
        queuedMessages: [...s.queuedMessages, msg],
      }));
      return msg;
    },

    flushOfflineQueue() {
      const state = store.get();
      const flushed = [];
      const nowSec = Math.floor(Date.now() / 1000);

      for (const qMsg of state.queuedMessages) {
        const sent = {
          ...qMsg,
          status: 'sent',
          createdAt: nowSec,
          disappearsAt: qMsg.disappearSeconds ? nowSec + qMsg.disappearSeconds : null,
          isOwn: true,
        };
        flushed.push(sent);
      }

      store.update(s => {
        const nextByChannel = { ...s.byChannel };
        for (const msg of flushed) {
          nextByChannel[msg.channelId] = [...(nextByChannel[msg.channelId] || []), msg];
        }
        return {
          byChannel: nextByChannel,
          queuedMessages: [],
        };
      });
      return flushed;
    },

    getRemainingSeconds(message, currentTimestampSec = Math.floor(Date.now() / 1000)) {
      if (!message.disappearsAt) return null;
      const diff = message.disappearsAt - currentTimestampSec;
      return Math.max(0, diff);
    },

    purgeExpiredMessages(currentTimestampSec = Math.floor(Date.now() / 1000)) {
      let purgedCount = 0;
      store.update(s => {
        const updated = {};
        for (const [chId, msgs] of Object.entries(s.byChannel)) {
          const remaining = msgs.filter(m => {
            if (m.disappearsAt && m.disappearsAt <= currentTimestampSec) {
              purgedCount++;
              return false;
            }
            return true;
          });
          updated[chId] = remaining;
        }
        return { ...s, byChannel: updated };
      });
      return purgedCount;
    },

    addReaction(channelId, messageId, emoji, userId = 'user-self') {
      store.update(s => {
        const msgs = s.byChannel[channelId] || [];
        const nextMsgs = msgs.map(m => {
          if (m.id !== messageId) return m;
          const rxList = [...m.reactions];
          const found = rxList.find(r => r.emoji === emoji);
          if (found) {
            if (!found.userIds.includes(userId)) {
              found.userIds.push(userId);
              found.count = found.userIds.length;
            }
          } else {
            rxList.push({ emoji, userIds: [userId], count: 1 });
          }
          return { ...m, reactions: rxList };
        });
        return { ...s, byChannel: { ...s.byChannel, [channelId]: nextMsgs } };
      });
    },

    deleteMessage(channelId, messageId) {
      store.update(s => {
        const msgs = s.byChannel[channelId] || [];
        return {
          ...s,
          byChannel: {
            ...s.byChannel,
            [channelId]: msgs.filter(m => m.id !== messageId),
          },
        };
      });
    },
  };
}

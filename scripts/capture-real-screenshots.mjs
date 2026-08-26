import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import puppeteer from 'puppeteer-core';

const BRAVE_PATH = 'C:\\Program Files\\BraveSoftware\\Brave-Browser-Beta\\Application\\brave.exe';
const BUILD_DIR = path.resolve('build');
const PORT = 4173;
const SCREENSHOTS_DIR = path.resolve('screenshots');
const ARTIFACT_DIR = 'C:\\Users\\mrSpy\\.gemini\\antigravity\\brain\\5b34953e-8edf-4444-b05a-ddfd449476c5\\screenshots';

if (!fs.existsSync(SCREENSHOTS_DIR)) fs.mkdirSync(SCREENSHOTS_DIR, { recursive: true });
if (!fs.existsSync(ARTIFACT_DIR)) fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.wasm': 'application/wasm',
  '.woff2': 'font/woff2',
  '.task': 'application/octet-stream',
};

function createStaticServer() {
  return http.createServer((req, res) => {
    let reqPath = req.url.split('?')[0];
    if (reqPath === '/' || reqPath === '') reqPath = '/index.html';
    let filePath = path.join(BUILD_DIR, reqPath);

    if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
      filePath = path.join(BUILD_DIR, 'index.html');
    }

    const ext = path.extname(filePath).toLowerCase();
    const contentType = MIME_TYPES[ext] || 'application/octet-stream';

    try {
      const content = fs.readFileSync(filePath);
      res.writeHead(200, {
        'Content-Type': contentType,
        'Cache-Control': 'no-cache',
      });
      res.end(content);
    } catch (err) {
      res.writeHead(404);
      res.end('Not found');
    }
  });
}

function makeSvgAvatar(name, bg = '#7c3aed', fg = '#ffffff') {
  const initials = name.slice(0, 2).toUpperCase();
  return `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><defs><linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%"><stop offset="0%" stop-color="${encodeURIComponent(bg)}"/><stop offset="100%" stop-color="${encodeURIComponent(bg)}dd"/></linearGradient></defs><rect width="128" height="128" rx="64" fill="url(%23g)"/><text x="50%" y="54%" font-family="system-ui,-apple-system,sans-serif" font-size="44" font-weight="700" fill="${encodeURIComponent(fg)}" text-anchor="middle" dominant-baseline="middle">${initials}</text></svg>`;
}

const SVG_MAP = {
  hash_alex: makeSvgAvatar('Alex Rivers', '#8b5cf6', '#ffffff'),
  hash_cipher: makeSvgAvatar('Cipher Fox', '#06b6d4', '#0f172a'),
  hash_starlight: makeSvgAvatar('Nova Starlight', '#ec4899', '#ffffff'),
  hash_nexus: makeSvgAvatar('Nexus Echo', '#3b82f6', '#ffffff'),
  hash_glitch: makeSvgAvatar('Glitch Zero', '#10b981', '#064e3b'),
  hash_phantom: makeSvgAvatar('Phantom Dev', '#64748b', '#ffffff'),
  space_icon_veil: makeSvgAvatar('VeilAnon Community', '#7c3aed', '#ffffff'),
  space_icon_sec: makeSvgAvatar('Security Lab', '#0284c7', '#ffffff'),
  space_icon_oss: makeSvgAvatar('Open Source', '#059669', '#ffffff'),
};

const MOCK_DATA = {
  self: {
    id: 'usr_me_001',
    username: 'alex_veil',
    displayName: 'Alex Rivers',
    avatarHash: 'hash_alex',
    bannerHash: null,
    bio: '🛡️ Zero-Knowledge Enthusiast & VeilAnon Contributor',
    customStatus: 'Coding with end-to-end privacy 🔒',
    deviceId: 'dev_primary_node',
    publicKey: {
      dh_public_key: 'dh_pub_e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
      signing_public_key: 'sign_pub_ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb',
      fingerprint: '3b8a:92c1:7d44:ee89:12a0:fc88:65bc:4410',
    },
    recoveryCode: 'VEIL-7X9K-MN42-LP88',
  },
  users: [
    {
      userId: 'usr_me_001',
      username: 'alex_veil',
      displayName: 'Alex Rivers',
      avatarHash: 'hash_alex',
      roleIds: ['role_admin'],
      onlineStatus: 'online',
    },
    {
      userId: 'usr_002',
      username: 'cipher_fox',
      displayName: 'Cipher Fox',
      avatarHash: 'hash_cipher',
      roleIds: ['role_core', 'role_mod'],
      onlineStatus: 'online',
    },
    {
      userId: 'usr_003',
      username: 'starlight',
      displayName: 'Nova Starlight',
      avatarHash: 'hash_starlight',
      roleIds: ['role_core'],
      onlineStatus: 'dnd',
    },
    {
      userId: 'usr_004',
      username: 'nexus_echo',
      displayName: 'Nexus Echo',
      avatarHash: 'hash_nexus',
      roleIds: ['role_member'],
      onlineStatus: 'away',
    },
    {
      userId: 'usr_005',
      username: 'glitch_zero',
      displayName: 'Glitch Zero',
      avatarHash: 'hash_glitch',
      roleIds: ['role_member'],
      onlineStatus: 'online',
    },
    {
      userId: 'usr_006',
      username: 'phantom_dev',
      displayName: 'Phantom Dev',
      avatarHash: 'hash_phantom',
      roleIds: ['role_member'],
      onlineStatus: 'offline',
    },
  ],
  spaces: [
    {
      id: 'space_veilanon_hq',
      name: 'VeilAnon Community',
      iconHash: 'space_icon_veil',
      ownerId: 'usr_me_001',
      memberCount: 142,
      isOwner: true,
      myRoles: ['role_admin'],
      description: 'Gizlilik odaklı, sıfır-bilgi mimarili açık kaynaklı topluluk alanı.',
      customLink: 'veilanon-official',
    },
    {
      id: 'space_cybersec',
      name: 'CyberSec & Cryptography',
      iconHash: 'space_icon_sec',
      ownerId: 'usr_002',
      memberCount: 89,
      isOwner: false,
      myRoles: ['role_core'],
      description: 'E2EE, Tor Routing, Post-Quantum KEM ve Güvenlik Protokolleri.',
      customLink: 'crypto-lab',
    },
    {
      id: 'space_open_source',
      name: 'Open Source Vanguard',
      iconHash: 'space_icon_oss',
      ownerId: 'usr_003',
      memberCount: 310,
      isOwner: false,
      myRoles: ['role_member'],
      description: 'Gözetimsiz ve özgür yazılım geliştiricileri merkezi.',
    },
  ],
  channels: {
    space_veilanon_hq: [
      {
        id: 'ch_announcements',
        spaceId: 'space_veilanon_hq',
        name: 'duyurular',
        channelType: 'announcement',
        position: 1,
        isNsfw: false,
        isE2ee: true,
        unreadCount: 0,
        mentioned: false,
        lastMessageId: 'msg_001',
        topic: 'VeilAnon v0.0.1 güncellemeleri ve önemli güvenlik bültenleri.',
      },
      {
        id: 'ch_general',
        spaceId: 'space_veilanon_hq',
        name: 'genel-sohbet',
        channelType: 'text',
        position: 2,
        isNsfw: false,
        isE2ee: true,
        unreadCount: 0,
        mentioned: false,
        lastMessageId: 'msg_005',
        topic: 'Topluluk ana sohbet kanalı — Uçtan Uca Şifreli 🔒',
      },
      {
        id: 'ch_crypto_dev',
        spaceId: 'space_veilanon_hq',
        name: 'kriptografi-lab',
        channelType: 'text',
        position: 3,
        isNsfw: false,
        isE2ee: true,
        unreadCount: 2,
        mentioned: true,
        lastMessageId: 'msg_010',
        topic: 'Noise Protocol, Argon2id ve X3DH anahtar değişim tartışmaları.',
      },
      {
        id: 'ch_voice_main',
        spaceId: 'space_veilanon_hq',
        name: 'Ana Ses & Video Odası',
        channelType: 'voice',
        position: 4,
        isNsfw: false,
        isE2ee: true,
        unreadCount: 0,
        mentioned: false,
        lastMessageId: null,
        topic: 'E2EE LiveKit Tabanlı Düşük Gecikmeli Ses Odası',
      },
    ],
  },
  dms: [
    {
      id: 'dm_cipher_fox',
      spaceId: null,
      name: 'Cipher Fox',
      channelType: 'dm',
      position: 1,
      isNsfw: false,
      isE2ee: true,
      unreadCount: 0,
      mentioned: false,
      lastMessageId: 'msg_dm_003',
      avatarHash: 'hash_cipher',
      onlineStatus: 'online',
      peerId: 'usr_002',
    },
    {
      id: 'dm_nova_starlight',
      spaceId: null,
      name: 'Nova Starlight',
      channelType: 'dm',
      position: 2,
      isNsfw: false,
      isE2ee: true,
      unreadCount: 1,
      mentioned: false,
      lastMessageId: 'msg_dm_010',
      avatarHash: 'hash_starlight',
      onlineStatus: 'dnd',
      peerId: 'usr_003',
    },
  ],
  friends: [
    {
      userId: 'usr_002',
      username: 'cipher_fox',
      displayName: 'Cipher Fox',
      avatarHash: 'hash_cipher',
      status: 'friends',
      onlineStatus: 'online',
    },
    {
      userId: 'usr_003',
      username: 'starlight',
      displayName: 'Nova Starlight',
      avatarHash: 'hash_starlight',
      status: 'friends',
      onlineStatus: 'dnd',
    },
    {
      userId: 'usr_004',
      username: 'nexus_echo',
      displayName: 'Nexus Echo',
      avatarHash: 'hash_nexus',
      status: 'friends',
      onlineStatus: 'away',
    },
    {
      userId: 'usr_005',
      username: 'glitch_zero',
      displayName: 'Glitch Zero',
      avatarHash: 'hash_glitch',
      status: 'friends',
      onlineStatus: 'online',
    },
  ],
  roles: [
    {
      id: 'role_admin',
      spaceId: 'space_veilanon_hq',
      name: 'Founder / Admin',
      color: '#8b5cf6',
      permissions: ['administrator'],
      position: 1,
      isDefault: false,
    },
  ],
  messages: {
    ch_general: [
      {
        id: 'msg_001',
        channelId: 'ch_general',
        senderId: 'usr_002',
        senderName: 'Cipher Fox',
        senderAvatarHash: 'hash_cipher',
        senderRoleColor: '#06b6d4',
        content: 'Selamlar herkese! VeilAnon üzerinde yeni uçtan uca şifreli oturum başlattık. Tüm mesaj paketleri istemci tarafında **Argon2id + ChaCha20-Poly1305** ile mühürleniyor. 🛡️⚡',
        messageType: 'text',
        status: 'sent',
        replyToId: null,
        pinned: true,
        reactions: [{ emoji: '🔥', userIds: ['usr_me_001', 'usr_003'], count: 2 }],
        attachments: [],
        editedAt: null,
        createdAt: Math.floor(Date.now() / 1000) - 3600,
        disappearsAt: null,
      },
      {
        id: 'msg_002',
        channelId: 'ch_general',
        senderId: 'usr_me_001',
        senderName: 'Alex Rivers',
        senderAvatarHash: 'hash_alex',
        senderRoleColor: '#8b5cf6',
        content: 'Harika görünüyor! Kod bloklarındaki syntax highlighting ve gizlilik kalkanı kontrolleri de çok akıcı çalışıyor:\n\n```rust\n// Zero-Knowledge Identity Proof\npub fn verify_zero_knowledge(proof: &ZkProof, pubkey: &PublicKey) -> bool {\n    crypto::verify_snark_proof(proof, pubkey).is_ok()\n}\n```',
        messageType: 'text',
        status: 'sent',
        replyToId: null,
        pinned: false,
        reactions: [{ emoji: '⚡', userIds: ['usr_me_001', 'usr_002'], count: 2 }],
        attachments: [],
        editedAt: null,
        createdAt: Math.floor(Date.now() / 1000) - 1200,
        disappearsAt: null,
      },
    ],
    ch_crypto_dev: [
      {
        id: 'msg_c_001',
        channelId: 'ch_crypto_dev',
        senderId: 'usr_002',
        senderName: 'Cipher Fox',
        senderAvatarHash: 'hash_cipher',
        senderRoleColor: '#06b6d4',
        content: 'Post-Quantum Kyber-1024 hibrit anahtar değişim protokolü entegrasyonu tamamlandı. Ağdaki tüm anahtar rotasyonları mükemmel ileri gizlilik (PFS) garantisi sunuyor.',
        messageType: 'text',
        status: 'sent',
        replyToId: null,
        pinned: true,
        reactions: [{ emoji: '🔐', userIds: ['usr_me_001', 'usr_003'], count: 2 }],
        attachments: [],
        editedAt: null,
        createdAt: Math.floor(Date.now() / 1000) - 1800,
        disappearsAt: null,
      },
    ],
    dm_cipher_fox: [
      {
        id: 'msg_dm_001',
        channelId: 'dm_cipher_fox',
        senderId: 'usr_002',
        senderName: 'Cipher Fox',
        senderAvatarHash: 'hash_cipher',
        senderRoleColor: null,
        content: 'Selam Alex! Yeni güvenlik denetim raporunu inceledin mi? Tüm Tor SOCKS5 çıkış düğümleri doğrulanmış durumda.',
        messageType: 'text',
        status: 'sent',
        replyToId: null,
        pinned: false,
        reactions: [],
        attachments: [],
        editedAt: null,
        createdAt: Math.floor(Date.now() / 1000) - 1800,
        disappearsAt: null,
      },
    ],
  },
  settings: {
    presenceVisibility: 'everyone',
    showReadReceipts: true,
    showTypingIndicator: true,
    autoDownloadMedia: false,
    linkPreviews: true,
    notificationPreview: 'sender',
    telemetryEnabled: false,
    localAiEnabled: false,
    discordBridgeEnabled: false,
    showJoinDate: false,
    networkPrivacy: {
      mode: 'tor',
      proxyHost: '127.0.0.1',
      proxyPort: 9050,
      strictMode: true,
      routeAppOnly: true,
      autoStartTor: true,
      verifyExitNode: true,
    },
    theme: 'dark',
    fontSize: 15,
    reduceMotion: false,
    compactMode: false,
    accentColor: '#8b5cf6',
    amoledMode: false,
    presetThemeId: 'veil-origin',
    customThemeName: 'Cyber Velvet',
    desktopNotifications: true,
    notificationSound: true,
    noiseSuppression: true,
    echoCancellation: true,
    autoUnlock: true,
    dmPrivacy: 'friends',
  },
  about: {
    appName: 'veilanon',
    version: '0.0.1',
    description: 'Privacy-first, open-source desktop communication platform by aegisSoft',
    developer: 'aegisSoft',
    developerUrl: 'https://www.aegissoft.com.tr/',
    developerGithub: 'https://github.com/aegissoft',
    projectGithub: 'https://github.com/MrSpy00/veilanon',
    supportUrl: 'https://github.com/MrSpy00/veilanon/issues',
    license: 'AGPL-3.0',
    buildDate: '2026-08-26',
    rustVersion: '1.80.0',
    platform: 'windows',
  },
};

function getMockInjection(authenticated = true, activeTab = 'account') {
  return `
    window.__TAURI_INTERNALS__ = {
      transformCallback: (fn) => 1,
      unregisterCallback: () => {},
      convertFileSrc: (p) => p,
      invoke: async (cmd, args = {}) => {
        if (cmd === 'log_client_error') return null;
        if (cmd === 'get_identity_hint') return ${authenticated ? JSON.stringify({ hasIdentity: true, username: 'alex_veil', displayName: 'Alex Rivers' }) : JSON.stringify({ hasIdentity: false, username: null, displayName: null })};
        if (cmd === 'try_auto_unlock') return ${authenticated ? JSON.stringify(MOCK_DATA.self) : 'null'};
        if (cmd === 'create_identity' || cmd === 'load_identity') return ${JSON.stringify(MOCK_DATA.self)};
        if (cmd === 'get_avatar' || cmd === 'spaces_get_icon') {
          const hash = args.hash || args.iconHash;
          return ${JSON.stringify(SVG_MAP)}[hash] || ${JSON.stringify(SVG_MAP.hash_alex)};
        }
        if (cmd === 'get_settings') return ${JSON.stringify(MOCK_DATA.settings)};
        if (cmd === 'spaces_list') return ${JSON.stringify(MOCK_DATA.spaces)};
        if (cmd === 'channels_list') {
          const sid = args.spaceId || 'space_veilanon_hq';
          return ${JSON.stringify(MOCK_DATA.channels)}[sid] || ${JSON.stringify(MOCK_DATA.channels.space_veilanon_hq)};
        }
        if (cmd === 'dm_list') return ${JSON.stringify(MOCK_DATA.dms)};
        if (cmd === 'members_list') return ${JSON.stringify(MOCK_DATA.users)};
        if (cmd === 'roles_list') return ${JSON.stringify(MOCK_DATA.roles)};
        if (cmd === 'friends_list') return ${JSON.stringify(MOCK_DATA.friends)};
        if (cmd === 'load_messages') {
          const cid = args.channelId || 'ch_general';
          return ${JSON.stringify(MOCK_DATA.messages)}[cid] || ${JSON.stringify(MOCK_DATA.messages.ch_general)};
        }
        if (cmd === 'get_about_info') return ${JSON.stringify(MOCK_DATA.about)};
        if (cmd === 'get_diagnostics') return {
          appVersion: '0.0.1',
          platform: 'windows (x86_64)',
          supabaseConfigured: true,
          supabaseReachable: true,
          livekitConfigured: true,
          r2Configured: true,
          realtimeConnected: true,
          messageCount: 489,
          friendCount: 5,
          spaceCount: 3,
          queuedCount: 0,
          fileCount: 24,
          databaseSizeBytes: 2048576,
          logDirectory: 'C:\\\\Users\\\\AppData\\\\Local\\\\veilanon\\\\logs'
        };
        if (cmd === 'check_tor_status') return {
          running: true,
          connected: true,
          version: '0.4.8.10',
          circuit_established: true,
          bootstrap_percentage: 100,
          socks_port: 9050,
          exit_node_ip: '185.220.101.5',
          exit_node_country: 'Switzerland (CH)'
        };
        if (cmd === 'check_ip_leak') return {
          leaked: false,
          detected_ip: '185.220.101.5',
          dns_servers: ['1.1.1.1 (Cloudflare DoH)', '9.9.9.9 (Quad9 DoH)'],
          webrtc_leak: false,
          tor_exit: true
        };
        if (cmd === 'check_doh_status') return { active: true, provider: 'Cloudflare Zero-Log DoH', latency_ms: 14 };
        if (cmd === 'check_multi_doh_status') return [
          { provider: 'Cloudflare', latency_ms: 12, secure: true },
          { provider: 'Quad9', latency_ms: 16, secure: true },
          { provider: 'Mullvad DoH', latency_ms: 22, secure: true }
        ];
        if (cmd === 'get_pinned_messages') return [${JSON.stringify(MOCK_DATA.messages.ch_general[0])}];
        if (cmd === 'join_voice_channel' || cmd === 'get_livekit_token') return {
          token: 'mock_jwt_token',
          url: 'wss://livekit.veilanon.network',
          roomName: 'room_voice_main',
          isE2ee: true,
          e2eeScope: 'space',
          e2eeKey: 'k_e2ee_mock_secret_key_8829'
        };
        return true;
      }
    };

    try {
      localStorage.setItem('veilanon-theme', 'dark');
      localStorage.setItem('veilanon-preset', 'veil-origin');
      localStorage.setItem('veilanon-accent', '#8b5cf6');
      localStorage.setItem('veilanon-settings-tab', ${JSON.stringify(activeTab)});
    } catch(e) {}
  `;
}

async function captureShot(page, filename, authenticated, actionFn, delay = 1200, activeTab = 'account') {
  console.log(`[Capture] Starting ${filename}...`);
  await page.evaluateOnNewDocument(getMockInjection(authenticated, activeTab));
  await page.goto(`http://localhost:${PORT}/`, { waitUntil: 'networkidle0' });
  await new Promise(r => setTimeout(r, 600));

  if (actionFn) {
    await page.evaluate(actionFn);
  }

  await new Promise(r => setTimeout(r, delay));

  const localPath = path.join(SCREENSHOTS_DIR, filename);
  const artifactPath = path.join(ARTIFACT_DIR, filename);

  await page.screenshot({ path: localPath, fullPage: false, type: 'png' });
  fs.copyFileSync(localPath, artifactPath);
  console.log(`[Capture] Successfully written ${filename}`);
}

async function main() {
  const server = createStaticServer();
  server.listen(PORT, async () => {
    console.log(`[Server] Static server listening at http://localhost:${PORT}`);

    try {
      const browser = await puppeteer.launch({
        executablePath: BRAVE_PATH,
        headless: true,
        defaultViewport: {
          width: 1920,
          height: 1080,
          deviceScaleFactor: 2,
        },
        args: [
          '--no-sandbox',
          '--disable-setuid-sandbox',
          '--disable-web-security',
          '--force-device-scale-factor=2',
          '--font-render-hinting=max',
          '--enable-font-antialiasing',
        ],
      });

      console.log('[Browser] Brave initialized.');
      const page = await browser.newPage();

      // ── SCREEN 1: Onboarding / Welcome ─────────────────────────────────────
      await captureShot(page, '01_onboarding_welcome.png', false, null, 800);

      // ── SCREEN 2: Onboarding / Create Identity ──────────────────────────────
      await captureShot(page, '02_onboarding_create_identity.png', false, async () => {
        const btn = Array.from(document.querySelectorAll('button')).find(b => b.textContent.includes('Yeni Kimlik Oluştur'));
        if (btn) btn.click();
        await new Promise(r => setTimeout(r, 300));
        const inputs = document.querySelectorAll('input');
        if (inputs[0]) { inputs[0].value = 'alex_rivers'; inputs[0].dispatchEvent(new Event('input', { bubbles: true })); }
        if (inputs[1]) { inputs[1].value = 'Alex Rivers'; inputs[1].dispatchEvent(new Event('input', { bubbles: true })); }
        if (inputs[2]) { inputs[2].value = 'CorrectHorseBatteryStaple!2026'; inputs[2].dispatchEvent(new Event('input', { bubbles: true })); }
        if (inputs[3]) { inputs[3].value = 'CorrectHorseBatteryStaple!2026'; inputs[3].dispatchEvent(new Event('input', { bubbles: true })); }
      }, 800);

      // ── SCREEN 3: Community Main Chat (#genel-sohbet) ──────────────────────
      await captureShot(page, '03_community_channels_chat.png', true, async () => {
        const spaceIcons = document.querySelectorAll('.veil-space-icon');
        if (spaceIcons.length > 0) spaceIcons[0].click();
      }, 1500);

      // ── SCREEN 4: Kriptografi Lab Channel (#kriptografi-lab) ───────────────
      await captureShot(page, '04_kripto_lab_channel.png', true, async () => {
        const spaceIcons = document.querySelectorAll('.veil-space-icon');
        if (spaceIcons.length > 0) spaceIcons[0].click();
        await new Promise(r => setTimeout(r, 600));
        const channels = Array.from(document.querySelectorAll('.veil-channel-item'));
        const cryptoCh = channels.find(c => c.textContent.includes('kriptografi-lab'));
        if (cryptoCh) cryptoCh.click();
      }, 1500);

      // ── SCREEN 5: Direct Messages (1:1 E2EE DM with Cipher Fox) ────────────
      await captureShot(page, '05_direct_messages_e2ee.png', true, async () => {
        const homeLogo = document.querySelector('.veil-sidebar-logo');
        if (homeLogo) homeLogo.click();
        await new Promise(r => setTimeout(r, 500));
        const dmItems = document.querySelectorAll('.veil-channel-item');
        if (dmItems.length > 0) dmItems[0].click();
      }, 1500);

      // ── SCREEN 6: Voice & Video Room Stage ─────────────────────────────────
      await captureShot(page, '06_voice_video_room.png', true, async () => {
        const spaceIcons = document.querySelectorAll('.veil-space-icon');
        if (spaceIcons.length > 0) spaceIcons[0].click();
        await new Promise(r => setTimeout(r, 600));
        const voiceCh = Array.from(document.querySelectorAll('.veil-channel-item')).find(c => c.textContent.includes('Ana Ses'));
        if (voiceCh) voiceCh.click();
      }, 1500);

      // ── SCREEN 7: Settings - Privacy & Network (Tor / Anti-Leak) ───────────
      await captureShot(page, '07_settings_privacy_network.png', true, async () => {
        const settingsBtn = document.querySelector('.veil-bottom-bar-settings, button[title*="Ayarlar"]');
        if (settingsBtn) settingsBtn.click();
        await new Promise(r => setTimeout(r, 500));
        const btn = Array.from(document.querySelectorAll('.veil-settings-nav-item')).find(el => el.textContent.includes('Gizlilik'));
        if (btn) btn.click();
      }, 1500, 'privacy');

      // ── SCREEN 8: Settings - Themes & Customization (AMOLED / Accents) ──────
      await captureShot(page, '08_settings_appearance_themes.png', true, async () => {
        const settingsBtn = document.querySelector('.veil-bottom-bar-settings, button[title*="Ayarlar"]');
        if (settingsBtn) settingsBtn.click();
        await new Promise(r => setTimeout(r, 500));
        const btn = Array.from(document.querySelectorAll('.veil-settings-nav-item')).find(el => el.textContent.includes('Görünüm'));
        if (btn) btn.click();
      }, 1500, 'appearance');

      // ── SCREEN 9: Settings - Security & Keys ───────────────────────────────
      await captureShot(page, '09_settings_security_keys.png', true, async () => {
        const settingsBtn = document.querySelector('.veil-bottom-bar-settings, button[title*="Ayarlar"]');
        if (settingsBtn) settingsBtn.click();
        await new Promise(r => setTimeout(r, 500));
        const btn = Array.from(document.querySelectorAll('.veil-settings-nav-item')).find(el => el.textContent.includes('Güvenlik'));
        if (btn) btn.click();
      }, 1500, 'security');

      // ── SCREEN 10: Home Dashboard & Friends Overview ───────────────────────
      await captureShot(page, '10_home_friends_overview.png', true, async () => {
        const homeLogo = document.querySelector('.veil-sidebar-logo');
        if (homeLogo) homeLogo.click();
      }, 1500);

      await browser.close();
      server.close();
      console.log('[All Done] Captured all 10 authentic screenshots in 2x HiDPI!');
      process.exit(0);
    } catch (err) {
      console.error('[Error] Capture failed:', err);
      server.close();
      process.exit(1);
    }
  });
}

main();

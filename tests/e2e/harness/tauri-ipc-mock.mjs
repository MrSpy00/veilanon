/**
 * VeilAnon E2E Test Harness — Tauri IPC Router Mock
 * Maps frontend `invoke` calls to realistic backend domain handlers
 * according to PROJECT.md interface contracts.
 */
import {
  TorMockProvider,
  IpTraceMockProvider,
  DohMockProvider,
  PwnedPasswordsMockProvider,
  UrlHausMockProvider,
  MultiDohBenchmarkMockProvider,
  LinkPreviewMockProvider,
  ClockSkewMockProvider,
} from './mock-providers.mjs';
import { generateDeterministicSvgAvatar, sha256Hex } from './crypto-utils.mjs';
import { DEFAULT_APP_SETTINGS } from './types.mjs';

export class TauriIpcMockRouter {
  constructor() {
    this.torProvider = new TorMockProvider();
    this.ipTraceProvider = new IpTraceMockProvider();
    this.dohProvider = new DohMockProvider();
    this.pwnedPasswordsProvider = new PwnedPasswordsMockProvider();
    this.urlHausProvider = new UrlHausMockProvider();
    this.multiDohProvider = new MultiDohBenchmarkMockProvider();
    this.linkPreviewProvider = new LinkPreviewMockProvider();
    this.clockSkewProvider = new ClockSkewMockProvider();

    this.currentIdentity = null;
    this.settings = { ...DEFAULT_APP_SETTINGS };
    this.spaces = [];
    this.channels = [];
    this.messages = new Map(); // channelId -> Message[]
    this.diagnosticsLogs = ['[2026-08-18 00:00:01] INFO veilanon::init starting desktop client v0.0.1'];
  }

  async invoke(command, args = {}) {
    switch (command) {
      // ── Privacy Tools (Zero-Key APIs) ───────────────────────────
      case 'check_tor_status':
        return this.torProvider.checkTorStatus();

      case 'check_ip_leak':
        return this.ipTraceProvider.checkIpLeak();

      case 'check_doh_status':
        return this.dohProvider.checkDohStatus();

      case 'check_multi_doh_status':
        return this.multiDohProvider.checkMultiDohStatus();

      case 'check_password_pwned': {
        const prefix = args.prefix_5_hex || (args.input && args.input.prefix_5_hex) || args.prefix5Hex;
        return this.pwnedPasswordsProvider.checkPasswordRange(prefix);
      }

      case 'scan_urlhaus': {
        const url = args.url || (args.input && args.input.url);
        return this.urlHausProvider.scanUrl(url);
      }

      case 'fetch_link_preview': {
        const url = args.url || (args.input && args.input.url);
        return this.linkPreviewProvider.fetchLinkPreview(url);
      }

      case 'generate_privacy_avatar': {
        const seed = args.seed || (args.input && args.input.seed) || '';
        return generateDeterministicSvgAvatar(seed);
      }

      case 'detect_clock_skew':
        return this.clockSkewProvider.detectClockSkew();

      // ── Identity & Auth ─────────────────────────────────────────
      case 'create_identity': {
        const input = args.input || args;
        if (!input.username || !input.passphrase) {
          throw new Error('Username and passphrase are required');
        }
        const id = `user-${sha256Hex(input.username).slice(0, 12)}`;
        const recoveryCode = `VEIL-${Math.random().toString(36).slice(2, 6).toUpperCase()}-${Math.random().toString(36).slice(2, 6).toUpperCase()}-${Math.random().toString(36).slice(2, 6).toUpperCase()}`;
        const avatarSvg = generateDeterministicSvgAvatar(input.username);

        this.currentIdentity = {
          id,
          username: input.username,
          displayName: input.displayName || input.username,
          avatarHash: sha256Hex(avatarSvg).slice(0, 16),
          deviceId: `dev-${Math.random().toString(36).slice(2, 8)}`,
          publicKey: {
            dh_public_key: `dh-pub-${sha256Hex(input.username).slice(0, 32)}`,
            signing_public_key: `sign-pub-${sha256Hex(input.username).slice(0, 32)}`,
            fingerprint: sha256Hex(input.username).slice(0, 40),
          },
          recoveryCode,
        };
        return this.currentIdentity;
      }

      case 'load_identity': {
        const passphrase = args.passphrase;
        if (!this.currentIdentity) {
          throw new Error('No identity found in local keychain');
        }
        if (passphrase === 'wrong_password') {
          throw new Error('Invalid passphrase / Authentication failed');
        }
        return this.currentIdentity;
      }

      case 'recover_identity': {
        const { recoveryCode, newPassphrase } = args;
        if (!recoveryCode || !newPassphrase) {
          throw new Error('Recovery code and new passphrase are required');
        }
        if (!recoveryCode.startsWith('VEIL-')) {
          throw new Error('Invalid recovery code format');
        }
        if (this.currentIdentity) {
          this.currentIdentity.recoveryCode = recoveryCode;
        }
        return this.currentIdentity || {
          id: 'recovered-user-id',
          username: 'recovered_user',
          displayName: 'Recovered User',
          deviceId: 'dev-recovered',
          publicKey: {
            dh_public_key: 'dh-pub-rec',
            signing_public_key: 'sign-pub-rec',
            fingerprint: 'fp-rec',
          },
        };
      }

      case 'get_identity_hint':
        return {
          hasIdentity: !!this.currentIdentity,
          username: this.currentIdentity ? this.currentIdentity.username : null,
          displayName: this.currentIdentity ? this.currentIdentity.displayName : null,
          avatarHash: this.currentIdentity ? this.currentIdentity.avatarHash : null,
        };

      // ── Messages & Disappearing Messages ────────────────────────
      case 'send_message': {
        const input = args.input || args;
        const nowSec = Math.floor(Date.now() / 1000);
        const disappearsAt = input.disappearSeconds || input.disappear_seconds
          ? nowSec + (input.disappearSeconds || input.disappear_seconds)
          : null;

        const msg = {
          id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
          channelId: input.channelId || input.channel_id,
          senderId: this.currentIdentity ? this.currentIdentity.id : 'user-self',
          senderName: this.currentIdentity ? this.currentIdentity.displayName : 'VeilUser',
          senderAvatarHash: this.currentIdentity ? this.currentIdentity.avatarHash : null,
          senderRoleColor: null,
          content: input.content,
          messageType: 'text',
          status: 'sent',
          replyToId: input.replyToId || input.reply_to_id || null,
          pinned: false,
          reactions: [],
          attachments: input.attachments || [],
          editedAt: null,
          createdAt: nowSec,
          disappearsAt,
        };

        const chId = msg.channelId;
        if (!this.messages.has(chId)) this.messages.set(chId, []);
        this.messages.get(chId).push(msg);
        return msg;
      }

      case 'load_messages': {
        const chId = args.channelId || args.channel_id;
        return this.messages.get(chId) || [];
      }

      case 'delete_message': {
        const msgId = args.messageId || args.message_id;
        for (const [chId, list] of this.messages.entries()) {
          this.messages.set(chId, list.filter(m => m.id !== msgId));
        }
        return true;
      }

      // ── Settings & Diagnostics ──────────────────────────────────
      case 'get_settings':
        return this.settings;

      case 'save_settings': {
        const newSettings = args.settings || args.input || args;
        this.settings = { ...this.settings, ...newSettings };
        return this.settings;
      }

      case 'get_diagnostics':
        return {
          version: '0.0.1',
          platform: 'windows',
          rust_version: '1.80.0',
          logs: this.diagnosticsLogs,
          active_sessions: 1,
        };

      case 'open_log_folder':
        return true;

      default:
        throw new Error(`Unknown command: ${command}`);
    }
  }
}

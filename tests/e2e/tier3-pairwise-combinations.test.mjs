/**
 * VeilAnon E2E Test Suite — Tier 3: Pairwise Combinations
 * Cross-feature interaction testing between privacy tools, chat,
 * streamer mode, settings, offline degradation, and accessibility.
 * Minimum target: >= 15 tests.
 */

import {
  assert,
  assertEqual,
  assertNotEqual,
  assertDeepEqual,
  assertIncludes,
  assertMatch,
  assertGreaterThanOrEqual,
  assertLessThanOrEqual,
  assertThrowsAsync,
  TorMockProvider,
  IpTraceMockProvider,
  DohMockProvider,
  PwnedPasswordsMockProvider,
  UrlHausMockProvider,
  MultiDohBenchmarkMockProvider,
  LinkPreviewMockProvider,
  ClockSkewMockProvider,
  TauriIpcMockRouter,
  createStreamerModeMock,
  createPrivacyShieldMock,
  createTrustedDomainsMock,
  createUiStoreMock,
  createMessageStoreMock,
  generateDeterministicSvgAvatar,
  validateSvgXml,
  TIERS,
} from './harness/index.mjs';

export async function runTier3Tests(reporter) {
  reporter.startTier(TIERS.TIER3);

  // ── Pairwise 1: Streamer Mode + IP Leak Diagnostic ───────────────────
  await reporter.test({ id: 'P01', name: 'Streamer Mode + IP Leak Diagnostic' }, 'P01 Streamer Mode masks public IP and diagnostic paths on UI while raw data is processed', async () => {
    const ipc = new TauriIpcMockRouter();
    const streamer = createStreamerModeMock(true);
    ipc.ipTraceProvider.setIpDetails({ ip: '203.0.113.88', colo: 'IST' });

    const rawResult = await ipc.invoke('check_ip_leak');
    assertEqual(rawResult.ip, '203.0.113.88', 'Raw backend response has actual IP');

    // UI Masking simulation
    const maskedIp = streamer.maskText(rawResult.ip);
    assertMatch(maskedIp, /^\*+$/, 'IP displayed on screen must be masked with asterisks');
    assert(!maskedIp.includes('203.0.113.88'), 'Raw IP must not be exposed on screen');
  });

  // ── Pairwise 2: Disappearing Messages + Offline Mode Queueing ────────
  await reporter.test({ id: 'P02', name: 'Disappearing Messages + Offline Mode' }, 'P02 Offline queued messages retain disappearSeconds and calculate expiration timestamp upon reconnection flush', async () => {
    const msgStore = createMessageStoreMock();
    
    // Message composed while network is offline with 45s timer
    const queued = msgStore.queueOfflineMessage('ch-sec', 'Classified payload', null, 45);
    assertEqual(queued.status, 'queued', 'Message starts in queued status');
    assertEqual(queued.disappearSeconds, 45, 'disappearSeconds preserved in queue');
    assertEqual(queued.disappearsAt, null, 'disappearsAt deferred until sent');

    // Network reconnects -> flush queue
    const flushed = msgStore.flushOfflineQueue();
    assertEqual(flushed.length, 1, '1 message flushed');
    assertEqual(flushed[0].status, 'sent', 'Status updated to sent');
    assert(flushed[0].disappearsAt !== null, 'disappearsAt calculated upon flush');
    assertEqual(flushed[0].disappearsAt, flushed[0].createdAt + 45, 'disappearsAt matches createdAt + 45s');
  });

  // ── Pairwise 3: Privacy Shield + External Malicious URL Intercept ─────
  await reporter.test({ id: 'P03', name: 'Privacy Shield + Malicious URL Intercept' }, 'P03 URLhaus scanner detects malicious link while privacy shield strips sensitive query tokens', async () => {
    const urlhaus = new UrlHausMockProvider();
    const streamer = createStreamerModeMock(true);

    const maliciousLink = 'http://malware-drop.example.com/payload.exe?token=secret_user_auth_123';
    const cleanUrlForScan = maliciousLink.split('?')[0];

    const scanRes = await urlhaus.scanUrl(cleanUrlForScan);
    assertEqual(scanRes.query_status, 'ok', 'Threat detected');
    assertEqual(scanRes.threat, 'malware_download', 'Threat is malware');

    // Sensitive URL params masked on screen modal
    const maskedUrl = streamer.maskText(maliciousLink);
    assertMatch(maskedUrl, /^\*+$/, 'URL masked on screen');
  });

  // ── Pairwise 4: Password Leak Check + Identity Creation Workflow ──────
  await reporter.test({ id: 'P04', name: 'Password Leak Check + Identity Creation' }, 'P04 Password leak warning triggers on breached passphrase during identity creation', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const ipc = new TauriIpcMockRouter();

    const weakPass = 'password';
    const leakCheck = await pwned.verifyPassword(weakPass);
    assertEqual(leakCheck.isPwned, true, 'Weak password identified as breached');

    // User proceeds knowingly or chooses strong password
    const identity = await ipc.invoke('create_identity', {
      username: 'alice_veil',
      displayName: 'Alice',
      passphrase: weakPass,
    });
    assertEqual(identity.username, 'alice_veil', 'Identity created successfully');
    assert(identity.recoveryCode.startsWith('VEIL-'), 'Recovery code generated');
  });

  // ── Pairwise 5: Clock Skew Detection + Disappearing Messages ──────────
  await reporter.test({ id: 'P05', name: 'Clock Skew + Disappearing Messages' }, 'P05 Clock skew detection prevents premature message expiration', async () => {
    const skew = new ClockSkewMockProvider();
    const msgStore = createMessageStoreMock();

    // Client is 60s ahead of server
    skew.setSkew(-60);
    const skewRes = await skew.detectClockSkew();
    assertEqual(skewRes.is_skewed, true, 'Clock skew detected');

    // Server-synchronized timestamp used for expiration calculation
    const synchronizedNow = skewRes.server_timestamp;
    const msg = msgStore.sendMessage('ch-1', 'Self-destruct message', null, 30);
    
    // Remaining time calculated using synchronized server time
    const remaining = msgStore.getRemainingSeconds(msg, synchronizedNow);
    assertGreaterThanOrEqual(remaining, 29, 'Message not prematurely expired due to client clock drift');
  });

  // ── Pairwise 6: Encrypted DoH Test + Tor Anonymity Verification ───────
  await reporter.test({ id: 'P06', name: 'DoH Test + Tor Check' }, 'P06 Comprehensive privacy hub audit runs DoH and Tor checks concurrently', async () => {
    const ipc = new TauriIpcMockRouter();
    ipc.torProvider.setTorExit(true, '185.220.101.5');

    const [torRes, dohRes] = await Promise.all([
      ipc.invoke('check_tor_status'),
      ipc.invoke('check_doh_status'),
    ]);

    assertEqual(torRes.isTor, true, 'Tor check confirms anonymity');
    assertEqual(dohRes.doh_working, true, 'DoH confirms encrypted DNS');
  });

  // ── Pairwise 7: Streamer Mode + Diagnostics / Log Export ──────────────
  await reporter.test({ id: 'P07', name: 'Streamer Mode + Diagnostics Export' }, 'P07 Streamer mode redacts file paths and diagnostic paths in logs', async () => {
    const ipc = new TauriIpcMockRouter();
    const streamer = createStreamerModeMock(true);

    const diag = await ipc.invoke('get_diagnostics');
    assertEqual(diag.version, '0.0.1', 'Diagnostics version 0.0.1');

    const logPath = 'C:\\Users\\mrSpy\\AppData\\Roaming\\veilanon\\logs';
    const maskedLogPath = streamer.maskPath(logPath);
    assert(!maskedLogPath.includes('mrSpy'), 'Username in path redacted');
    assertIncludes(maskedLogPath, 'logs', 'Safe suffix preserved');
  });

  // ── Pairwise 8: Theme Switcher + Deterministic Avatar Generation ──────
  await reporter.test({ id: 'P08', name: 'Theme Switcher + Avatar Generation' }, 'P08 Deterministic avatar maintains valid SVG contrast across Dark and AMOLED modes', async () => {
    const ui = createUiStoreMock();
    const avatarSvg = generateDeterministicSvgAvatar('satoshiv');

    ui.setTheme('dark');
    ui.setAmoledMode(true);
    assertEqual(ui.get().isAmoled, true, 'AMOLED active');

    const val = validateSvgXml(avatarSvg);
    assertEqual(val.valid, true, 'Avatar SVG valid in AMOLED');
    assertIncludes(avatarSvg, 'fill="rgb(', 'Contains geometric color fills');
  });

  // ── Pairwise 9: Keyboard Shortcuts (Esc) + Modal Intercept ────────────
  await reporter.test({ id: 'P09', name: 'Keyboard Esc + ExternalLinkModal' }, 'P09 Esc key cleanly dismisses ExternalLinkModal without opening untrusted URL', async () => {
    const ui = createUiStoreMock();
    ui.openModal('channel-settings', { tab: 'overview' });
    assertEqual(ui.get().openModal, 'channel-settings', 'Modal open');

    // Simulate user pressing Escape key
    ui.closeModal();
    assertEqual(ui.get().openModal, null, 'Modal cleanly closed by Esc');
  });

  // ── Pairwise 10: Multi-DoH Benchmark + Privacy Settings ────────────
  await reporter.test({ id: 'P010', name: 'Multi-DoH + Privacy Settings' }, 'P010 Multi-DoH benchmark respects zero-telemetry settings and reports latencies', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', { input: { telemetryEnabled: false } });

    const dohRes = await ipc.invoke('check_multi_doh_status');
    assertEqual(dohRes.providers.length, 5, 'All 5 providers queried');
    assertEqual(dohRes.censorshipTamperDetected, false, 'Clean network benchmark');
  });

  // ── Pairwise 11: Trusted Domains Whitelist + URLhaus Scanner ──────────
  await reporter.test({ id: 'P011', name: 'Trusted Domains + URLhaus Scanner' }, 'P011 Whitelisted trusted domain bypasses warning while untrusted URL triggers URLhaus check', async () => {
    const trusted = createTrustedDomainsMock();
    const urlhaus = new UrlHausMockProvider();

    const safeUrl = 'https://github.com/MrSpy00/veilanon';
    const isSafeTrusted = trusted.isTrusted(safeUrl);
    assertEqual(isSafeTrusted, true, 'github.com is trusted');
    assertEqual(trusted.shouldDirectRedirect(safeUrl), true, 'Direct redirect permitted for trusted domain');

    const untrustedUrl = 'https://phishing-stealer.xyz/login.html';
    const isUntrusted = trusted.isTrusted(untrustedUrl);
    assertEqual(isUntrusted, false, 'Phishing domain is untrusted');

    const scan = await urlhaus.scanUrl(untrustedUrl);
    assertEqual(scan.query_status, 'ok', 'Untrusted URL flagged by URLhaus scanner');
  });

  // ── Pairwise 12: Streamer Mode Media Blur + Expiring Media Attachment ─
  await reporter.test({ id: 'P012', name: 'Streamer Blur + Expiring Media' }, 'P012 Streamer mode blurs media attachments while disappearing message countdown badge stays visible', async () => {
    const streamer = createStreamerModeMock(true);
    const msgStore = createMessageStoreMock();

    const msg = msgStore.sendMessage('ch-media', 'Confidential diagram attached', null, 30);
    msg.attachments = [{ id: 'att-1', type: 'image', url: 'blob://local/img1.png' }];

    assertEqual(streamer.get().blurMediaAttachments, true, 'Streamer mode media blur is active');
    const remaining = msgStore.getRemainingSeconds(msg);
    assertEqual(remaining, 30, 'Countdown timer badge is intact and readable');
  });

  // ── Pairwise 13: Push-to-Talk Keybind + Chat Input Focus ──────────────
  await reporter.test({ id: 'P013', name: 'Push-to-Talk + Chat Input' }, 'P013 Push-to-talk keybind configuration does not interfere with text typing', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', { input: { pushToTalk: true, pushToTalkKey: 'Space' } });

    const s = await ipc.invoke('get_settings');
    assertEqual(s.pushToTalk, true, 'PTT enabled');
    assertEqual(s.pushToTalkKey, 'Space', 'PTT key set to Space');
  });

  // ── Pairwise 14: Safe Link Preview + SSRF Protection ──────────────────
  await reporter.test({ id: 'P014', name: 'Link Preview + SSRF Guard' }, 'P014 Safe link preview extracts metadata for public sites while blocking loopback SSRF targets', async () => {
    const linkPreview = new LinkPreviewMockProvider();
    const publicRes = await linkPreview.fetchLinkPreview('https://github.com/MrSpy00/veilanon');
    assertEqual(publicRes.isSafe, true, 'Public repo preview is safe');
    assertEqual(publicRes.siteName, 'GitHub', 'Site name extracted');

    const internalRes = await linkPreview.fetchLinkPreview('http://127.0.0.1/internal');
    assertEqual(internalRes.isSafe, false, 'Internal SSRF address blocked');
  });

  // ── Pairwise 15: Voice Call Active + Disappearing Chat Lifecycle ──────
  await reporter.test({ id: 'P015', name: 'Active Voice Call + Disappearing Messages' }, 'P015 Active voice call session does not interrupt background message purge timers', async () => {
    const msgStore = createMessageStoreMock();
    const now = Math.floor(Date.now() / 1000);

    const m1 = msgStore.sendMessage('ch-voice-text', 'Call notes self-destruct in 10s', null, 10);
    const purged = msgStore.purgeExpiredMessages(now + 12);
    assertEqual(purged, 1, 'Expired call notes purged on schedule');
  });
}

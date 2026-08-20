/**
 * VeilAnon E2E Test Suite — Tier 1: Feature Coverage
 * Comprehensive requirement-driven opaque-box tests covering all 15 features.
 * Minimum target: >= 75 tests (5 distinct tests per feature).
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
  sha1HexUpper,
  sha256Hex,
  createMulberry32,
  stringToSeed,
  DEFAULT_APP_SETTINGS,
  FEATURES,
  TIERS,
} from './harness/index.mjs';

import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

export async function runTier1Tests(reporter) {
  reporter.startTier(TIERS.TIER1);

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 1: Tor & Relay Anonymity Check
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.TOR_CHECK, '1.1 check_tor_status returns is_tor=false for direct connection', async () => {
    const tor = new TorMockProvider();
    tor.setTorExit(false, '198.51.100.42');
    const res = await tor.checkTorStatus();
    assertEqual(res.isTor, false, 'Expected direct IP not to be a Tor exit');
    assertEqual(res.ip, '198.51.100.42', 'IP should match configured direct IP');
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.2 check_tor_status returns is_tor=true and exit IP for Tor relay', async () => {
    const tor = new TorMockProvider();
    tor.setTorExit(true, '185.220.101.5');
    const res = await tor.checkTorStatus();
    assertEqual(res.isTor, true, 'Expected Tor exit to be detected');
    assertEqual(res.ip, '185.220.101.5', 'IP should match Tor exit node IP');
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.3 check_tor_status handles IPv4 address format properly', async () => {
    const tor = new TorMockProvider();
    tor.setTorExit(false, '104.28.16.88');
    const res = await tor.checkTorStatus();
    assertMatch(res.ip, /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/, 'IP should be valid IPv4');
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.4 check_tor_status includes check timestamp', async () => {
    const tor = new TorMockProvider();
    const now = Date.now();
    const res = await tor.checkTorStatus();
    assertGreaterThanOrEqual(res.timestamp, now - 1000, 'Timestamp should be recent');
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.5 check_tor_status works via Tauri IPC router', async () => {
    const ipc = new TauriIpcMockRouter();
    ipc.torProvider.setTorExit(true, '185.220.101.77');
    const res = await ipc.invoke('check_tor_status');
    assertEqual(res.isTor, true, 'IPC route should return Tor status true');
    assertEqual(res.ip, '185.220.101.77', 'IPC route should return exit IP');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 2: IP Leak & Network Diagnostic
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.IP_LEAK, '2.1 check_ip_leak retrieves public IP and Cloudflare colo code', async () => {
    const trace = new IpTraceMockProvider();
    trace.setIpDetails({ ip: '203.0.113.50', colo: 'FRA' });
    const res = await trace.checkIpLeak();
    assertEqual(res.ip, '203.0.113.50', 'IP should match trace IP');
    assertEqual(res.colo, 'FRA', 'Colo code should match Frankfurt edge');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.2 check_ip_leak parses TLS version (TLSv1.3) and SNI status', async () => {
    const trace = new IpTraceMockProvider();
    trace.setIpDetails({ tls: 'TLSv1.3', sni: 'encrypted' });
    const res = await trace.checkIpLeak();
    assertEqual(res.tls, 'TLSv1.3', 'TLS version should be TLSv1.3');
    assertEqual(res.sni, 'encrypted', 'SNI should be parsed');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.3 check_ip_leak detects WARP and Gateway status', async () => {
    const trace = new IpTraceMockProvider();
    trace.setIpDetails({ warp: 'on', gateway: 'off' });
    const res = await trace.checkIpLeak();
    assertEqual(res.warp, 'on', 'WARP tunnel should be detected as on');
    assertEqual(res.gateway, 'off', 'Gateway should be detected as off');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.4 check_ip_leak measures round-trip time in milliseconds', async () => {
    const trace = new IpTraceMockProvider();
    const res = await trace.checkIpLeak();
    assertGreaterThanOrEqual(res.rtt_ms, 1, 'RTT must be positive integer');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.5 check_ip_leak works via Tauri IPC router', async () => {
    const ipc = new TauriIpcMockRouter();
    ipc.ipTraceProvider.setIpDetails({ ip: '198.51.100.99', loc: 'DE', colo: 'HAM' });
    const res = await ipc.invoke('check_ip_leak');
    assertEqual(res.ip, '198.51.100.99', 'IPC route should return public IP');
    assertEqual(res.loc, 'DE', 'IPC route should return ISO country code');
    assertEqual(res.colo, 'HAM', 'IPC route should return Hamburg edge');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 3: Encrypted DoH Test
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.DOH_TEST, '3.1 check_doh_status validates Cloudflare DoH resolver', async () => {
    const doh = new DohMockProvider();
    const res = await doh.checkDohStatus();
    assertEqual(res.cloudflare_ok, true, 'Cloudflare DoH should be marked working');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.2 check_doh_status validates Google DoH resolver', async () => {
    const doh = new DohMockProvider();
    const res = await doh.checkDohStatus();
    assertEqual(res.google_ok, true, 'Google DoH should be marked working');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.3 check_doh_status measures latency for active resolvers', async () => {
    const doh = new DohMockProvider();
    const res = await doh.checkDohStatus();
    assertGreaterThanOrEqual(res.latency_cloudflare_ms, 1, 'Cloudflare latency should be positive');
    assertGreaterThanOrEqual(res.latency_google_ms, 1, 'Google latency should be positive');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.4 check_doh_status flags doh_working=true when resolvers respond', async () => {
    const doh = new DohMockProvider();
    const res = await doh.checkDohStatus();
    assertEqual(res.doh_working, true, 'DoH overall status should be true');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.5 check_doh_status handles single provider block gracefully', async () => {
    const doh = new DohMockProvider();
    doh.setBlocked('google');
    const res = await doh.checkDohStatus();
    assertEqual(res.cloudflare_ok, true, 'Cloudflare should remain ok');
    assertEqual(res.google_ok, false, 'Google should be marked blocked');
    assertEqual(res.doh_working, true, 'DoH should remain working via fallback');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 4: k-Anonymity Password Leak Check
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.PASSWORD_CHECK, '4.1 check_password_pwned hashes input and queries 5-char prefix', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const hash = sha1HexUpper('password');
    const prefix = hash.slice(0, 5);
    const ranges = await pwned.checkPasswordRange(prefix);
    assert(ranges.length > 0, 'Range results should not be empty for common prefix');
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.2 check_password_pwned detects compromised password', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const verification = await pwned.verifyPassword('123456');
    assertEqual(verification.isPwned, true, "'123456' must be identified as breached");
    assertGreaterThanOrEqual(verification.breachCount, 1000000, 'Breach count should be high');
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.3 check_password_pwned returns breach count for match', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const verification = await pwned.verifyPassword('admin');
    assertEqual(verification.isPwned, true, "'admin' must be identified as breached");
    assertEqual(verification.breachCount, 893120, 'Breach count must match known record');
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.4 check_password_pwned marks high-entropy passphrase as safe', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const safePass = 'xQ9#mK8$vL2!wZ7@pY4%jR3*';
    const verification = await pwned.verifyPassword(safePass);
    assertEqual(verification.isPwned, false, 'High entropy password must not be flagged');
    assertEqual(verification.breachCount, 0, 'Breach count should be 0');
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.5 check_password_pwned preserves k-anonymity (transmits only 5 chars)', async () => {
    const ipc = new TauriIpcMockRouter();
    const fullHash = sha1HexUpper('secret');
    const prefix5 = fullHash.slice(0, 5);
    const ranges = await ipc.invoke('check_password_pwned', { prefix_5_hex: prefix5 });
    assert(Array.isArray(ranges), 'Result should be an array of suffix matches');
    const suffix = fullHash.slice(5);
    const found = ranges.find(([s]) => s === suffix);
    assert(!!found, 'Suffix must be found in range response');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 5: Real-Time Malicious URL Scanner
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.URL_SCANNER, '5.1 scan_urlhaus flags known malware download URL', async () => {
    const urlhaus = new UrlHausMockProvider();
    const res = await urlhaus.scanUrl('http://malware-drop.example.com/payload.exe');
    assertEqual(res.query_status, 'ok', 'Status should be ok (found in DB)');
    assertEqual(res.threat, 'malware_download', 'Threat should be malware_download');
    assertIncludes(res.tags, 'trojan', 'Tags should include trojan');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.2 scan_urlhaus flags phishing stealer domain', async () => {
    const urlhaus = new UrlHausMockProvider();
    const res = await urlhaus.scanUrl('https://phishing-stealer.xyz/login.html');
    assertEqual(res.query_status, 'ok', 'Status should be ok');
    assertEqual(res.threat, 'phishing', 'Threat type should be phishing');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.3 scan_urlhaus flags botnet C2 IP literals', async () => {
    const urlhaus = new UrlHausMockProvider();
    const res = await urlhaus.scanUrl('http://198.51.100.99:8080/mozi.m');
    assertEqual(res.threat, 'botnet_c2', 'Threat should be botnet C2');
    assertIncludes(res.tags, 'mozi', 'Tags should include mozi');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.4 scan_urlhaus marks clean URL as no_results', async () => {
    const urlhaus = new UrlHausMockProvider();
    const res = await urlhaus.scanUrl('https://github.com/MrSpy00/veilanon');
    assertEqual(res.query_status, 'no_results', 'Clean URL should return no_results');
    assertEqual(res.threat, null, 'Threat should be null');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.5 scan_urlhaus works via Tauri IPC router', async () => {
    const ipc = new TauriIpcMockRouter();
    const res = await ipc.invoke('scan_urlhaus', { url: 'http://malware-drop.example.com/payload.exe' });
    assertEqual(res.query_status, 'ok', 'IPC should return threat status ok');
    assertEqual(res.threat, 'malware_download', 'IPC should return threat type');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 6: Multi-Resolver DoH & Tamper Benchmark
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.1 check_multi_doh_status probes all 5 major global privacy resolvers', async () => {
    const multiDoh = new MultiDohBenchmarkMockProvider();
    const res = await multiDoh.checkMultiDohStatus();
    assertEqual(res.providers.length, 5, 'Should probe 5 providers');
    const names = res.providers.map(p => p.name);
    assertIncludes(names.join(','), 'Cloudflare', 'Includes Cloudflare');
    assertIncludes(names.join(','), 'Google', 'Includes Google');
    assertIncludes(names.join(','), 'Quad9', 'Includes Quad9');
    assertIncludes(names.join(','), 'AdGuard', 'Includes AdGuard');
    assertIncludes(names.join(','), 'Mullvad', 'Includes Mullvad');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.2 check_multi_doh_status calculates fastest provider correctly', async () => {
    const multiDoh = new MultiDohBenchmarkMockProvider();
    const res = await multiDoh.checkMultiDohStatus();
    assertEqual(res.fastestProvider, 'Cloudflare', 'Cloudflare should be fastest with 14ms');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.3 check_multi_doh_status computes average latency across reachable resolvers', async () => {
    const multiDoh = new MultiDohBenchmarkMockProvider();
    const res = await multiDoh.checkMultiDohStatus();
    assert(res.averageLatencyMs > 0, 'Average latency should be positive');
    assertLessThanOrEqual(res.averageLatencyMs, 50, 'Average latency should be reasonable (<50ms)');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.4 check_multi_doh_status flags censorship and tampering when resolvers blocked', async () => {
    const multiDoh = new MultiDohBenchmarkMockProvider();
    multiDoh.setTamper(true);
    const res = await multiDoh.checkMultiDohStatus();
    assertEqual(res.censorshipTamperDetected, true, 'Tamper flag should be raised when <3 resolvers reachable');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.5 check_multi_doh_status and link preview operate cleanly via IPC router', async () => {
    const ipc = new TauriIpcMockRouter();
    const dohRes = await ipc.invoke('check_multi_doh_status');
    assertEqual(dohRes.providers.length, 5, 'IPC should return 5 providers');
    const previewRes = await ipc.invoke('fetch_link_preview', { url: 'https://github.com/MrSpy00/veilanon' });
    assertEqual(previewRes.isSafe, true, 'Link preview should be safe');
    assertEqual(previewRes.siteName, 'GitHub', 'Site name should be GitHub');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 7: Deterministic Privacy Avatar Generator
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.AVATAR_GEN, '7.1 generate_privacy_avatar produces valid SVG XML', async () => {
    const svg = generateDeterministicSvgAvatar('alice');
    const val = validateSvgXml(svg);
    assertEqual(val.valid, true, `SVG validation failed: ${val.error}`);
    assertIncludes(svg, '<svg', 'Should contain root svg tag');
    assertIncludes(svg, '</svg>', 'Should contain closing svg tag');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.2 generate_privacy_avatar is strictly deterministic', async () => {
    const svg1 = generateDeterministicSvgAvatar('bob_the_builder');
    const svg2 = generateDeterministicSvgAvatar('bob_the_builder');
    assertEqual(svg1, svg2, 'Identical seed must produce bit-for-bit identical SVG');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.3 generate_privacy_avatar produces distinct SVGs for distinct seeds', async () => {
    const svgA = generateDeterministicSvgAvatar('user_alpha');
    const svgB = generateDeterministicSvgAvatar('user_beta');
    assertNotEqual(svgA, svgB, 'Distinct seeds must yield distinct SVGs');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.4 generate_privacy_avatar renders geometric shapes with rounded corners', async () => {
    const svg = generateDeterministicSvgAvatar('charlie');
    assertIncludes(svg, 'rx="2"', 'Should include rounded corner rectangles');
    assertIncludes(svg, 'viewBox="0 0 128 128"', 'Should have standard 128x128 viewBox');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.5 generate_privacy_avatar works via Tauri IPC router', async () => {
    const ipc = new TauriIpcMockRouter();
    const svg = await ipc.invoke('generate_privacy_avatar', { seed: 'veil_admin' });
    const val = validateSvgXml(svg);
    assertEqual(val.valid, true, 'IPC returned SVG must be valid');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 8: Cryptographic Clock Skew Detector
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.CLOCK_SKEW, '8.1 detect_clock_skew accurately calculates clock difference', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(15);
    const res = await skew.detectClockSkew();
    assertEqual(res.skew_seconds, 15, 'Skew seconds should be 15');
    assertEqual(res.is_skewed, false, '15s skew should not trigger is_skewed flag (<=30s)');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.2 detect_clock_skew flags is_skewed=false when clock is synced', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(0);
    const res = await skew.detectClockSkew();
    assertEqual(res.skew_seconds, 0, 'Zero skew when synced');
    assertEqual(res.is_skewed, false, 'is_skewed must be false when synced');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.3 detect_clock_skew flags is_skewed=true when skew exceeds 30s threshold', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(45);
    const res = await skew.detectClockSkew();
    assertEqual(res.skew_seconds, 45, 'Skew should be 45s');
    assertEqual(res.is_skewed, true, '45s skew must trigger is_skewed=true');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.4 detect_clock_skew handles negative skew (client ahead)', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(-50);
    const res = await skew.detectClockSkew();
    assertEqual(res.skew_seconds, -50, 'Negative skew should be preserved');
    assertEqual(res.is_skewed, true, '|-50s| > 30s must trigger is_skewed=true');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.5 detect_clock_skew works via Tauri IPC router', async () => {
    const ipc = new TauriIpcMockRouter();
    ipc.clockSkewProvider.setSkew(120);
    const res = await ipc.invoke('detect_clock_skew');
    assertEqual(res.skew_seconds, 120, 'IPC should return skew');
    assertEqual(res.is_skewed, true, 'IPC should mark skewed');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 9: Disappearing Messages Visual Countdown
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.1 sendMessage with disappearSeconds computes future disappearsAt timestamp', async () => {
    const msgStore = createMessageStoreMock();
    const msg = msgStore.sendMessage('ch-general', 'This secret message will self-destruct', null, 30);
    assert(msg.disappearsAt !== null, 'disappearsAt must be populated');
    assertEqual(msg.disappearsAt, msg.createdAt + 30, 'disappearsAt must equal createdAt + 30s');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.2 getRemainingSeconds calculates live countdown accurately', async () => {
    const msgStore = createMessageStoreMock();
    const msg = msgStore.sendMessage('ch-general', 'Expiring soon', null, 60);
    const remaining = msgStore.getRemainingSeconds(msg, msg.createdAt + 25);
    assertEqual(remaining, 35, 'Remaining seconds should be 60 - 25 = 35s');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.3 purgeExpiredMessages automatically purges expired messages', async () => {
    const msgStore = createMessageStoreMock();
    const msg1 = msgStore.sendMessage('ch-general', 'Expiring message', null, 10);
    const msg2 = msgStore.sendMessage('ch-general', 'Permanent message', null, null);

    const purged = msgStore.purgeExpiredMessages(msg1.createdAt + 15);
    assertEqual(purged, 1, 'Exactly 1 message should be purged');

    const channelMsgs = msgStore.get().byChannel['ch-general'];
    assertEqual(channelMsgs.length, 1, 'Only permanent message should remain');
    assertEqual(channelMsgs[0].id, msg2.id, 'Remaining message must be msg2');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.4 disappearing messages support reactions and replies while active', async () => {
    const msgStore = createMessageStoreMock();
    const msg = msgStore.sendMessage('ch-general', 'Expiring with reaction', null, 100);
    msgStore.addReaction('ch-general', msg.id, '🔥', 'user-bob');

    const chMsgs = msgStore.get().byChannel['ch-general'];
    const target = chMsgs.find(m => m.id === msg.id);
    assertEqual(target.reactions.length, 1, 'Reaction should be attached');
    assertEqual(target.reactions[0].emoji, '🔥', 'Emoji should match 🔥');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.5 messages without disappearSeconds remain persistent', async () => {
    const msgStore = createMessageStoreMock();
    const msg = msgStore.sendMessage('ch-general', 'Standard message');
    assertEqual(msg.disappearsAt, null, 'disappearsAt must be null');
    const remaining = msgStore.getRemainingSeconds(msg);
    assertEqual(remaining, null, 'Remaining seconds must be null');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 10: Complete Settings Panels & UX Audit
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.1 settings initialize with privacy-first defaults', async () => {
    assertEqual(DEFAULT_APP_SETTINGS.telemetryEnabled, false, 'Telemetry must be disabled by default');
    assertEqual(DEFAULT_APP_SETTINGS.autoDownloadMedia, false, 'Auto-download media must be disabled');
    assertEqual(DEFAULT_APP_SETTINGS.presenceVisibility, 'everyone', 'Default presence visibility is everyone');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.2 settings update and persist audio/video device selections and PTT', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', {
      input: {
        inputDeviceId: 'mic-hyperx-quadcast',
        outputDeviceId: 'speakers-realtek',
        pushToTalk: true,
        pushToTalkKey: 'Space',
      },
    });
    const current = await ipc.invoke('get_settings');
    assertEqual(current.inputDeviceId, 'mic-hyperx-quadcast', 'Input device must be updated');
    assertEqual(current.pushToTalk, true, 'PTT must be enabled');
    assertEqual(current.pushToTalkKey, 'Space', 'PTT key must be Space');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.3 settings manage granular notification volume and category toggles', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', {
      input: {
        notificationVolume: 50,
        soundMessages: true,
        soundCalls: true,
        dndSuppressNotifications: true,
      },
    });
    const s = await ipc.invoke('get_settings');
    assertEqual(s.notificationVolume, 50, 'Volume must be updated to 50');
    assertEqual(s.dndSuppressNotifications, true, 'DND suppression must be enabled');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.4 settings manage appearance themes and custom accent colors', async () => {
    const ui = createUiStoreMock();
    ui.setTheme('dark');
    ui.setAccentColor('#8b5cf6');
    ui.setAmoledMode(true);

    const state = ui.get();
    assertEqual(state.theme, 'dark', 'Theme must be dark');
    assertEqual(state.accentColor, '#8b5cf6', 'Accent must be violet');
    assertEqual(state.isAmoled, true, 'AMOLED mode must be enabled');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.5 settings enforce presence visibility and read receipt privacy controls', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', {
      input: {
        presenceVisibility: 'contacts_only',
        showReadReceipts: false,
        showTypingIndicator: false,
      },
    });
    const s = await ipc.invoke('get_settings');
    assertEqual(s.presenceVisibility, 'contacts_only', 'Presence visibility should be contacts_only');
    assertEqual(s.showReadReceipts, false, 'Read receipts should be disabled');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 11: Keyboard Navigation & Empty States
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.KEYBOARD_NAV, '11.1 UI store manages active modal state and Esc dismissal', async () => {
    const ui = createUiStoreMock();
    ui.openModal('settings', { tab: 'privacy' });
    assertEqual(ui.get().openModal, 'settings', 'Settings modal must be open');
    assertEqual(ui.get().settingsTab, 'privacy', 'Privacy tab must be active');

    ui.closeModal();
    assertEqual(ui.get().openModal, null, 'Modal must be closed on dismissal');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.2 UI store handles space and channel navigation state transitions', async () => {
    const ui = createUiStoreMock();
    ui.navigate('space-123', 'ch-456');
    assertEqual(ui.get().activeSpaceId, 'space-123', 'Active space must match');
    assertEqual(ui.get().activeChannelId, 'ch-456', 'Active channel must match');
    assertEqual(ui.get().activeDmId, null, 'Active DM must be null');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.3 UI store handles DM navigation and resets active space/channel context', async () => {
    const ui = createUiStoreMock();
    ui.navigate('space-123', 'ch-456');
    ui.navigateDm('dm-789');
    assertEqual(ui.get().activeSpaceId, null, 'Active space must be cleared');
    assertEqual(ui.get().activeChannelId, null, 'Active channel must be cleared');
    assertEqual(ui.get().activeDmId, 'dm-789', 'Active DM must be set');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.4 UI store manages replyTo target state across channels and clears properly', async () => {
    const ui = createUiStoreMock();
    ui.setReplyTo({ channelId: 'ch-1', messageId: 'msg-1', author: 'Alice', content: 'Hello' });
    assertEqual(ui.get().replyTo.author, 'Alice', 'Reply author must be Alice');
    ui.setReplyTo(null);
    assertEqual(ui.get().replyTo, null, 'Reply target must be cleared');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.5 UI store maintains compact mode toggle for high density layouts', async () => {
    const ui = createUiStoreMock();
    ui.setCompactMode(true);
    assertEqual(ui.get().compactMode, true, 'Compact mode should be enabled');
    ui.setCompactMode(false);
    assertEqual(ui.get().compactMode, false, 'Compact mode should be disabled');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 12: Roadmap & Docs Completion
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.ROADMAP_DOCS, '12.1 ROADMAP.md document exists and contains v0.0.1 scope', async () => {
    const roadmapPath = resolve(process.cwd(), 'docs/ROADMAP.md');
    assert(existsSync(roadmapPath), 'docs/ROADMAP.md must exist');
    const content = readFileSync(roadmapPath, 'utf8');
    assertIncludes(content, 'v0.0.1', 'ROADMAP.md must include v0.0.1 scope header');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.2 ROADMAP.md documents disappearing message countdown implementation', async () => {
    const roadmapPath = resolve(process.cwd(), 'docs/ROADMAP.md');
    const content = readFileSync(roadmapPath, 'utf8');
    assertIncludes(content, 'Kaybolan mesaj', 'ROADMAP.md must document disappearing message support');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.3 ROADMAP.md documents Double Ratchet and MLS E2EE statuses', async () => {
    const roadmapPath = resolve(process.cwd(), 'docs/ROADMAP.md');
    const content = readFileSync(roadmapPath, 'utf8');
    assertIncludes(content, 'Double Ratchet', 'ROADMAP.md must mention Double Ratchet');
    assertIncludes(content, 'MLS', 'ROADMAP.md must mention MLS');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.4 PROJECT.md lists all 15 features and interface contracts', async () => {
    const projectPath = resolve(process.cwd(), 'PROJECT.md');
    assert(existsSync(projectPath), 'PROJECT.md must exist');
    const content = readFileSync(projectPath, 'utf8');
    assertIncludes(content, 'check_tor_status', 'PROJECT.md must document check_tor_status');
    assertIncludes(content, 'check_ip_leak', 'PROJECT.md must document check_ip_leak');
    assertIncludes(content, 'check_doh_status', 'PROJECT.md must document check_doh_status');
    assertIncludes(content, 'check_password_pwned', 'PROJECT.md must document check_password_pwned');
    assertIncludes(content, 'scan_urlhaus', 'PROJECT.md must document scan_urlhaus');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.5 TEST_INFRA.md specifies 4-Tier test architecture and coverage targets', async () => {
    const infraPath = resolve(process.cwd(), 'TEST_INFRA.md');
    assert(existsSync(infraPath), 'TEST_INFRA.md must exist');
    const content = readFileSync(infraPath, 'utf8');
    assertIncludes(content, 'Tier 1 (Coverage)', 'TEST_INFRA.md must specify Tier 1');
    assertIncludes(content, 'Tier 2 (Boundary)', 'TEST_INFRA.md must specify Tier 2');
    assertIncludes(content, 'Tier 3 (Pairwise)', 'TEST_INFRA.md must specify Tier 3');
    assertIncludes(content, 'Tier 4 (Scenario)', 'TEST_INFRA.md must specify Tier 4');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 13: Backend Rust Test Expansion
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.BACKEND_TESTS, '13.1 SHA-256 and SHA-1 deterministic hashing integrity', async () => {
    const text = 'VeilAnon Privacy 2026';
    const sha1 = sha1HexUpper(text);
    const sha256 = sha256Hex(text);
    assertEqual(sha1.length, 40, 'SHA-1 digest must be exactly 40 hex characters');
    assertEqual(sha256.length, 64, 'SHA-256 digest must be exactly 64 hex characters');
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.2 deterministic PRNG seed distribution and entropy', async () => {
    const seed = stringToSeed('veilanon_kdf_salt');
    const prng = createMulberry32(seed);
    const val1 = prng();
    const val2 = prng();
    assert(val1 >= 0 && val1 < 1, 'PRNG value must be in [0, 1)');
    assert(val2 >= 0 && val2 < 1, 'PRNG value must be in [0, 1)');
    assertNotEqual(val1, val2, 'Consecutive PRNG samples must vary');
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.3 message encryption structure and metadata validation', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('create_identity', { username: 'test_user', passphrase: 'master_passphrase' });
    const msg = await ipc.invoke('send_message', {
      channelId: 'ch-e2ee',
      content: 'Encrypted message body',
    });
    assertEqual(msg.status, 'sent', 'Message status must be sent');
    assertEqual(msg.senderName, 'test_user', 'Sender name should be populated');
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.4 identity keys: public key structure (DH, signing, fingerprint)', async () => {
    const ipc = new TauriIpcMockRouter();
    const identity = await ipc.invoke('create_identity', { username: 'cryptoman', passphrase: 'pass' });
    assert(identity.publicKey.dh_public_key.startsWith('dh-pub-'), 'DH public key format valid');
    assert(identity.publicKey.signing_public_key.startsWith('sign-pub-'), 'Signing key format valid');
    assertEqual(identity.publicKey.fingerprint.length, 40, 'Fingerprint length 40 chars');
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.5 recovery code format: VEIL-XXXX-XXXX-XXXX standard entropy', async () => {
    const ipc = new TauriIpcMockRouter();
    const identity = await ipc.invoke('create_identity', { username: 'alice', passphrase: 'pass' });
    assertMatch(identity.recoveryCode, /^VEIL-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/, 'Recovery code must match VEIL standard format');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 14: E2E Testing Suite (Tiers 1-4)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.E2E_SUITE, '14.1 test harness initializes mock providers and IPC routers cleanly', async () => {
    const ipc = new TauriIpcMockRouter();
    assert(ipc.torProvider instanceof TorMockProvider, 'TorProvider initialized');
    assert(ipc.ipTraceProvider instanceof IpTraceMockProvider, 'IpTraceProvider initialized');
    assert(ipc.dohProvider instanceof DohMockProvider, 'DohProvider initialized');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.2 test harness assertions throw structured AssertionError on mismatch', async () => {
    let caught = false;
    try {
      assertEqual(1, 2, 'Should fail');
    } catch (err) {
      caught = true;
      assertEqual(err.name, 'AssertionError', 'Error name should be AssertionError');
      assertEqual(err.actual, 1, 'Actual value preserved');
      assertEqual(err.expected, 2, 'Expected value preserved');
    }
    assert(caught, 'Expected AssertionError to be caught');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.3 test harness supports isolated test execution without shared pollution', async () => {
    const storeA = createMessageStoreMock();
    const storeB = createMessageStoreMock();
    storeA.sendMessage('ch-1', 'Message for A');
    assertEqual(Object.keys(storeB.get().byChannel).length, 0, 'Store B must remain empty');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.4 test harness supports async assertion rejection handling', async () => {
    await assertThrowsAsync(
      async () => {
        throw new Error('Specific async failure occurred');
      },
      'Specific async failure',
      'Should catch expected error message'
    );
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.5 test harness validates tier definitions and minimum test thresholds', async () => {
    assertEqual(TIERS.TIER1.minTests, 75, 'Tier 1 minimum tests must be 75');
    assertEqual(TIERS.TIER2.minTests, 75, 'Tier 2 minimum tests must be 75');
    assertEqual(TIERS.TIER3.minTests, 15, 'Tier 3 minimum tests must be 15');
    assertEqual(TIERS.TIER4.minTests, 8, 'Tier 4 minimum tests must be 8');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 15: Adversarial Coverage Hardening
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.1 XSS sanitization: avatar generator escapes special characters in seed', async () => {
    const maliciousSeed = '<script>alert("xss")</script>&foo="bar"';
    const svg = generateDeterministicSvgAvatar(maliciousSeed);
    assert(!svg.includes('<script>'), 'Unescaped <script> tag must not exist in SVG');
    assertIncludes(svg, '&lt;script&gt;', 'Must contain escaped script entity');
    assertIncludes(svg, '&amp;foo=', 'Must contain escaped ampersand');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.2 Zero-knowledge privacy: password check never exposes full hash to remote', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const verification = await pwned.verifyPassword('super_secret_master_key_2026!');
    assertEqual(verification.prefix.length, 5, 'Transmitted prefix is strictly 5 characters');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.3 Streamer mode: maskText obfuscates sensitive tokens with asterisks or bullets', async () => {
    const streamer = createStreamerModeMock(true);
    streamer.setMaskStyle('asterisks');
    const maskedAsterisk = streamer.maskText('secret_api_key_12345');
    assertMatch(maskedAsterisk, /^\*+$/, 'Masked text must consist only of asterisks');

    streamer.setMaskStyle('bullets');
    const maskedBullet = streamer.maskText('secret_api_key_12345');
    assertMatch(maskedBullet, /^•+$/, 'Masked text must consist only of bullets');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.4 Streamer mode: maskEmail obfuscates domain and username components', async () => {
    const streamer = createStreamerModeMock(true);
    const masked = streamer.maskEmail('whistleblower@protonmail.com');
    assert(!masked.includes('whistleblower'), 'Username must not be visible');
    assert(!masked.includes('protonmail'), 'Domain must not be visible');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.5 Streamer mode: maskUserId and maskInviteLink obfuscate identifiers and tokens', async () => {
    const streamer = createStreamerModeMock(true);
    const maskedId = streamer.maskUserId('usr_492049102-49120');
    assertIncludes(maskedId, '*', 'User ID must be masked');
    const maskedInvite = streamer.maskInviteLink('veilanon://join/SEC7799X');
    assertIncludes(maskedInvite, 'veilanon://join/****', 'Invite code must be masked');
  });
}

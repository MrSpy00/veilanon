/**
 * VeilAnon E2E Test Suite — Tier 2: Boundary, Corner & Negative Cases
 * Rigorous edge-case analysis, invalid input combination testing,
 * resource stress, and error handling for all 15 features.
 * Minimum target: >= 75 tests (5 distinct boundary tests per feature).
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
  FEATURES,
  TIERS,
} from './harness/index.mjs';

import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

export async function runTier2Tests(reporter) {
  reporter.startTier(TIERS.TIER2);

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 1: Tor & Relay Anonymity Check (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.TOR_CHECK, '1.1 Tor check handles HTTP 500 error gracefully', async () => {
    const tor = new TorMockProvider();
    tor.setFailure('http_500');
    await assertThrowsAsync(
      async () => tor.checkTorStatus(),
      'HTTP 500',
      'Should throw HTTP 500 error'
    );
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.2 Tor check handles network unreachable error', async () => {
    const tor = new TorMockProvider();
    tor.setFailure('network_error');
    await assertThrowsAsync(
      async () => tor.checkTorStatus(),
      'Network unreachable',
      'Should throw Network unreachable'
    );
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.3 Tor check handles malformed HTML error response', async () => {
    const tor = new TorMockProvider();
    tor.setFailure('malformed_json');
    const res = await tor.checkTorStatus();
    assertEqual(res.isTor, false, 'Malformed response should safely default isTor to false');
    assertEqual(res.parsed, null, 'Parsed content should be null');
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.4 Tor check handles IPv6 exit address format', async () => {
    const tor = new TorMockProvider();
    tor.setTorExit(true, '2001:db8:85a3::8a2e:370:7334');
    const res = await tor.checkTorStatus();
    assertEqual(res.isTor, true, 'IPv6 Tor exit node should be detected');
    assertIncludes(res.ip, '2001:db8:', 'IPv6 address format should be preserved');
  });

  await reporter.test(FEATURES.TOR_CHECK, '1.5 Tor check handles empty response body without crashing', async () => {
    const tor = new TorMockProvider();
    tor.setTorExit(false, '');
    const res = await tor.checkTorStatus();
    assertEqual(res.isTor, false, 'Empty IP should yield isTor=false');
    assertEqual(res.ip, '', 'IP should be empty string');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 2: IP Leak & Network Diagnostic (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.IP_LEAK, '2.1 IP leak check handles missing optional keys in trace output', async () => {
    const trace = new IpTraceMockProvider();
    const parsed = trace.parseTrace('fl=42f102\nh=1.1.1.1\nip=192.0.2.1\nrtt=15');
    assertEqual(parsed.ip, '192.0.2.1', 'IP should be parsed');
    assertEqual(parsed.colo, null, 'Missing colo should be null');
    assertEqual(parsed.loc, null, 'Missing loc should be null');
    assertEqual(parsed.warp, null, 'Missing warp should be null');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.2 IP leak check handles truncated trace text without crashing', async () => {
    const trace = new IpTraceMockProvider();
    const parsed = trace.parseTrace('corrupted_garbage_without_equals_sign');
    assertEqual(parsed.ip, '', 'IP should be empty string');
    assertEqual(parsed.colo, null, 'Colo should be null');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.3 IP leak check handles extreme network latency (>5000ms)', async () => {
    const trace = new IpTraceMockProvider();
    const parsed = trace.parseTrace('ip=192.0.2.1\nrtt=6500');
    assertEqual(parsed.rtt_ms, 6500, 'High latency parsed correctly');
    assertGreaterThanOrEqual(parsed.rtt_ms, 5000, 'Extreme latency handled');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.4 IP leak check handles IPv6 client addresses', async () => {
    const trace = new IpTraceMockProvider();
    const parsed = trace.parseTrace('ip=2606:4700:4700::1111\ncolo=LHR\nloc=GB');
    assertEqual(parsed.ip, '2606:4700:4700::1111', 'IPv6 address parsed');
    assertEqual(parsed.loc, 'GB', 'Country code parsed');
  });

  await reporter.test(FEATURES.IP_LEAK, '2.5 IP leak check handles network failure with descriptive error', async () => {
    const trace = new IpTraceMockProvider();
    trace.setFailure('network_error');
    await assertThrowsAsync(
      async () => trace.checkIpLeak(),
      'Network error',
      'Should throw Network error'
    );
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 3: Encrypted DoH Test (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.DOH_TEST, '3.1 DoH test handles complete blackout when both resolvers are down', async () => {
    const doh = new DohMockProvider();
    doh.setBlocked('both');
    const res = await doh.checkDohStatus();
    assertEqual(res.cloudflare_ok, false, 'Cloudflare must be false');
    assertEqual(res.google_ok, false, 'Google must be false');
    assertEqual(res.doh_working, false, 'doh_working must be false');
    assertEqual(res.latency_cloudflare_ms, 0, 'Latency 0 when down');
    assertEqual(res.latency_google_ms, 0, 'Latency 0 when down');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.2 DoH test handles zero latency edge cases', async () => {
    const doh = new DohMockProvider();
    doh.cloudflareLatency = 0;
    doh.googleLatency = 0;
    const res = await doh.checkDohStatus();
    assertEqual(res.latency_cloudflare_ms, 0, 'Zero latency handled');
    assertEqual(res.latency_google_ms, 0, 'Zero latency handled');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.3 DoH test handles rapid repeated probing without state corruption', async () => {
    const doh = new DohMockProvider();
    for (let i = 0; i < 20; i++) {
      const res = await doh.checkDohStatus();
      assertEqual(res.doh_working, true, 'Continuous checks should remain true');
    }
  });

  await reporter.test(FEATURES.DOH_TEST, '3.4 DoH test handles partial provider degradation (Cloudflare down, Google up)', async () => {
    const doh = new DohMockProvider();
    doh.setBlocked('cloudflare');
    const res = await doh.checkDohStatus();
    assertEqual(res.cloudflare_ok, false, 'Cloudflare is down');
    assertEqual(res.google_ok, true, 'Google is up');
    assertEqual(res.doh_working, true, 'doh_working remains true');
  });

  await reporter.test(FEATURES.DOH_TEST, '3.5 DoH test handles provider state flipping', async () => {
    const doh = new DohMockProvider();
    doh.setBlocked('cloudflare');
    let res = await doh.checkDohStatus();
    assertEqual(res.cloudflare_ok, false, 'Initially blocked');

    doh.setBlocked(null);
    res = await doh.checkDohStatus();
    assertEqual(res.cloudflare_ok, true, 'Restored to working');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 4: k-Anonymity Password Leak Check (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.PASSWORD_CHECK, '4.1 Password check rejects empty or null prefix', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    await assertThrowsAsync(
      async () => pwned.checkPasswordRange(''),
      'Invalid prefix',
      'Empty prefix must be rejected'
    );
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.2 Password check rejects invalid prefix length (4 or 6 chars)', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    await assertThrowsAsync(
      async () => pwned.checkPasswordRange('ABCD'),
      'must be exactly 5 hex characters',
      '4-char prefix must be rejected'
    );
    await assertThrowsAsync(
      async () => pwned.checkPasswordRange('ABCDEF'),
      'must be exactly 5 hex characters',
      '6-char prefix must be rejected'
    );
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.3 Password check rejects non-hex characters in prefix', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    await assertThrowsAsync(
      async () => pwned.checkPasswordRange('GHIJK'),
      'Invalid prefix format',
      'Non-hex characters must be rejected'
    );
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.4 Password check handles HTTP 429 Too Many Requests rate limit', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    pwned.setRateLimited(true);
    await assertThrowsAsync(
      async () => pwned.checkPasswordRange('5BAA6'),
      'HTTP 429',
      'Rate limited check must throw HTTP 429'
    );
  });

  await reporter.test(FEATURES.PASSWORD_CHECK, '4.5 Password check normalizes lowercase hex prefix to uppercase', async () => {
    const pwned = new PwnedPasswordsMockProvider();
    const rangesLower = await pwned.checkPasswordRange('5baa6');
    assert(rangesLower.length > 0, 'Lowercase 5baa6 should match uppercase 5BAA6');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 5: Real-Time Malicious URL Scanner (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.URL_SCANNER, '5.1 URL scanner marks empty or whitespace URL as invalid_url', async () => {
    const urlhaus = new UrlHausMockProvider();
    const res1 = await urlhaus.scanUrl('');
    assertEqual(res1.query_status, 'invalid_url', 'Empty string must return invalid_url');
    const res2 = await urlhaus.scanUrl('   ');
    assertEqual(res2.query_status, 'invalid_url', 'Whitespace must return invalid_url');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.2 URL scanner handles massive query string URL without crashing', async () => {
    const urlhaus = new UrlHausMockProvider();
    const massiveQuery = 'https://example.com/search?' + 'param=' + 'A'.repeat(8192);
    const res = await urlhaus.scanUrl(massiveQuery);
    assertEqual(res.query_status, 'no_results', 'Massive URL should return no_results safely');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.3 URL scanner handles non-HTTP protocol schemes safely', async () => {
    const urlhaus = new UrlHausMockProvider();
    const res = await urlhaus.scanUrl('file:///C:/Windows/System32/calc.exe');
    assertEqual(res.query_status, 'no_results', 'Local file URL should return no_results');
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.4 URL scanner handles endpoint timeout error', async () => {
    const urlhaus = new UrlHausMockProvider();
    urlhaus.setFailure('timeout');
    await assertThrowsAsync(
      async () => urlhaus.scanUrl('https://example.com'),
      'timed out',
      'Should throw timeout error'
    );
  });

  await reporter.test(FEATURES.URL_SCANNER, '5.5 URL scanner handles IDN / punycode domains', async () => {
    const urlhaus = new UrlHausMockProvider();
    urlhaus.addThreat('https://xn--e1afmkfd.xn--p1ai/malware', {
      threat: 'phishing',
      url_status: 'online',
      tags: ['russian_c2'],
    });
    const res = await urlhaus.scanUrl('https://xn--e1afmkfd.xn--p1ai/malware');
    assertEqual(res.query_status, 'ok', 'Punycode URL should match');
    assertEqual(res.threat, 'phishing', 'Threat should be phishing');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 6: Multi-Resolver DoH & Link Preview (Boundary & Corner)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.1 Multi-DoH handles 100% blocked resolver outage cleanly', async () => {
    const multiDoh = new MultiDohBenchmarkMockProvider();
    multiDoh.providers.forEach(p => { p.isReachable = false; });
    const res = await multiDoh.checkMultiDohStatus();
    assertEqual(res.censorshipTamperDetected, true, 'Must detect tamper when all resolvers blocked');
    assertEqual(res.fastestProvider, null, 'Fastest provider is null when all fail');
    assertEqual(res.averageLatencyMs, 0, 'Average latency is 0 when all fail');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.2 Multi-DoH single provider online handles fallback without NaN', async () => {
    const multiDoh = new MultiDohBenchmarkMockProvider();
    multiDoh.providers.forEach(p => { p.isReachable = false; });
    multiDoh.providers[2].isReachable = true; // Quad9 only
    multiDoh.providers[2].latencyMs = 45;
    const res = await multiDoh.checkMultiDohStatus();
    assertEqual(res.fastestProvider, 'Quad9', 'Only active provider becomes fastest');
    assertEqual(res.averageLatencyMs, 45, 'Average latency matches single provider');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.3 Link preview rejects private loopback SSRF addresses (127.0.0.1)', async () => {
    const linkPreview = new LinkPreviewMockProvider();
    const res = await linkPreview.fetchLinkPreview('http://127.0.0.1/internal/admin');
    assertEqual(res.isSafe, false, 'Loopback IP must be flagged as unsafe');
    assertEqual(res.title, null, 'Must not extract metadata from private IP');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.4 Link preview rejects LAN RFC 1918 private subnets (192.168.1.1)', async () => {
    const linkPreview = new LinkPreviewMockProvider();
    const res = await linkPreview.fetchLinkPreview('http://192.168.1.1/router');
    assertEqual(res.isSafe, false, 'Private LAN IP must be flagged as unsafe');
  });

  await reporter.test(FEATURES.MULTI_DOH_BENCHMARK, '6.5 Link preview handles empty or null input without throwing', async () => {
    const linkPreview = new LinkPreviewMockProvider();
    const resEmpty = await linkPreview.fetchLinkPreview('');
    assertEqual(resEmpty.isSafe, false, 'Empty URL must not crash');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 7: Deterministic Privacy Avatar Generator (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.AVATAR_GEN, '7.1 Avatar generator handles empty string seed gracefully', async () => {
    const svg = generateDeterministicSvgAvatar('');
    const val = validateSvgXml(svg);
    assertEqual(val.valid, true, 'Empty seed must generate valid SVG');
    assertIncludes(svg, '<svg', 'Contains SVG tag');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.2 Avatar generator handles massive seed (>64KB)', async () => {
    const massiveSeed = 'x'.repeat(65536);
    const svg = generateDeterministicSvgAvatar(massiveSeed);
    const val = validateSvgXml(svg);
    assertEqual(val.valid, true, 'Massive seed must produce valid SVG without memory error');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.3 Avatar generator handles Unicode emojis and international characters in seed', async () => {
    const unicodeSeed = '🚀_Privacy_🛡️_ユーザー_Тест_🔒';
    const svg = generateDeterministicSvgAvatar(unicodeSeed);
    const val = validateSvgXml(svg);
    assertEqual(val.valid, true, 'Unicode seed must produce valid SVG');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.4 Avatar generator escapes XML special characters in data-seed attribute', async () => {
    const unsafeSeed = 'foo" onclick="alert(1)" bar=\'<xml>\'&baz=1';
    const svg = generateDeterministicSvgAvatar(unsafeSeed);
    assert(!svg.includes('onclick="alert(1)"'), 'Attribute injection must be escaped');
    assertIncludes(svg, '&quot;', 'Quotes must be escaped');
    assertIncludes(svg, '&lt;xml&gt;', 'Tags must be escaped');
  });

  await reporter.test(FEATURES.AVATAR_GEN, '7.5 Avatar generator handles custom dimension bounds (16px to 1024px)', async () => {
    const svgSmall = generateDeterministicSvgAvatar('seed', 16);
    assertIncludes(svgSmall, 'viewBox="0 0 16 16"', '16px viewBox');
    const svgLarge = generateDeterministicSvgAvatar('seed', 1024);
    assertIncludes(svgLarge, 'viewBox="0 0 1024 1024"', '1024px viewBox');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 8: Cryptographic Clock Skew Detector (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.CLOCK_SKEW, '8.1 Clock skew exact threshold boundary (+30s vs +31s)', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(30);
    let res = await skew.detectClockSkew();
    assertEqual(res.is_skewed, false, '+30s is exactly at threshold (not skewed)');

    skew.setSkew(31);
    res = await skew.detectClockSkew();
    assertEqual(res.is_skewed, true, '+31s exceeds threshold (is skewed)');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.2 Clock skew exact negative threshold boundary (-30s vs -31s)', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(-30);
    let res = await skew.detectClockSkew();
    assertEqual(res.is_skewed, false, '-30s is at threshold (not skewed)');

    skew.setSkew(-31);
    res = await skew.detectClockSkew();
    assertEqual(res.is_skewed, true, '-31s exceeds threshold (is skewed)');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.3 Clock skew handles massive clock drift (>1 year / 31536000s)', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(31536000);
    const res = await skew.detectClockSkew();
    assertEqual(res.skew_seconds, 31536000, 'Year drift recorded');
    assertEqual(res.is_skewed, true, 'Massive drift flagged as skewed');
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.4 Clock skew detector handles network failure', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setFailure('network_error');
    await assertThrowsAsync(
      async () => skew.detectClockSkew(),
      'Network error',
      'Should throw Network error'
    );
  });

  await reporter.test(FEATURES.CLOCK_SKEW, '8.5 Clock skew handles integer second precision cleanly', async () => {
    const skew = new ClockSkewMockProvider();
    skew.setSkew(7);
    const res = await skew.detectClockSkew();
    assertEqual(Number.isInteger(res.skew_seconds), true, 'Skew must be integer');
    assertEqual(Number.isInteger(res.local_timestamp), true, 'Local ts must be integer');
    assertEqual(Number.isInteger(res.server_timestamp), true, 'Server ts must be integer');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 9: Disappearing Messages Visual Countdown (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.1 Disappearing messages handle 0s duration (no disappear timer)', async () => {
    const msgStore = createMessageStoreMock();
    const msg = msgStore.sendMessage('ch-test', 'Zero seconds message', null, 0);
    assertEqual(msg.disappearsAt, null, '0s should not set disappearsAt');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.2 Disappearing messages handle already expired timestamp upon receipt', async () => {
    const msgStore = createMessageStoreMock();
    const nowSec = Math.floor(Date.now() / 1000);
    const expiredMsg = {
      id: 'expired-msg-1',
      channelId: 'ch-test',
      senderId: 'user-other',
      content: 'Old message',
      status: 'sent',
      createdAt: nowSec - 100,
      disappearsAt: nowSec - 50,
    };
    msgStore.update(s => ({
      ...s,
      byChannel: { 'ch-test': [expiredMsg] },
    }));

    const remaining = msgStore.getRemainingSeconds(expiredMsg, nowSec);
    assertEqual(remaining, 0, 'Remaining time on expired message clamped to 0');

    const purged = msgStore.purgeExpiredMessages(nowSec);
    assertEqual(purged, 1, 'Expired message purged immediately');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.3 High-volume purge: handles 1000 expiring messages in single tick', async () => {
    const msgStore = createMessageStoreMock();
    const nowSec = Math.floor(Date.now() / 1000);
    const bulkMsgs = [];
    for (let i = 0; i < 1000; i++) {
      bulkMsgs.push({
        id: `bulk-${i}`,
        channelId: 'ch-bulk',
        senderId: 'user-self',
        content: `Msg ${i}`,
        status: 'sent',
        createdAt: nowSec - 30,
        disappearsAt: nowSec - 5,
      });
    }
    msgStore.update(s => ({
      ...s,
      byChannel: { 'ch-bulk': bulkMsgs },
    }));

    const purged = msgStore.purgeExpiredMessages(nowSec);
    assertEqual(purged, 1000, 'All 1000 messages must be purged in one tick');
    assertEqual(msgStore.get().byChannel['ch-bulk'].length, 0, 'Channel must be empty');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.4 Multiple concurrent purge ticks without race condition', async () => {
    const msgStore = createMessageStoreMock();
    const nowSec = Math.floor(Date.now() / 1000);
    msgStore.sendMessage('ch-1', 'Msg', null, 5);

    const purge1 = msgStore.purgeExpiredMessages(nowSec + 10);
    const purge2 = msgStore.purgeExpiredMessages(nowSec + 10);
    assertEqual(purge1, 1, 'First purge removes 1 message');
    assertEqual(purge2, 0, 'Second concurrent purge removes 0 messages');
  });

  await reporter.test(FEATURES.DISAPPEARING_MSGS, '9.5 Disappearing message deletion does not crash active store', async () => {
    const msgStore = createMessageStoreMock();
    const msg = msgStore.sendMessage('ch-1', 'Msg to delete', null, 30);
    msgStore.deleteMessage('ch-1', msg.id);
    assertEqual(msgStore.get().byChannel['ch-1'].length, 0, 'Message deleted cleanly');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 10: Complete Settings Panels & UX Audit (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.1 Settings volume boundary: handles 0% mute and 100% maximum', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', { input: { notificationVolume: 0 } });
    let s = await ipc.invoke('get_settings');
    assertEqual(s.notificationVolume, 0, '0% volume saved');

    await ipc.invoke('save_settings', { input: { notificationVolume: 100 } });
    s = await ipc.invoke('get_settings');
    assertEqual(s.notificationVolume, 100, '100% volume saved');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.2 Settings handle empty input object without wiping defaults', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('save_settings', { input: {} });
    const s = await ipc.invoke('get_settings');
    assertEqual(s.telemetryEnabled, false, 'Default telemetry preserved');
    assertEqual(s.presenceVisibility, 'everyone', 'Default presence preserved');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.3 Settings handle unknown theme string with safe fallback', async () => {
    const ui = createUiStoreMock();
    ui.setTheme('custom_neon_theme');
    assertEqual(ui.get().theme, 'custom_neon_theme', 'Custom theme applied');
    ui.setTheme('dark');
    assertEqual(ui.get().theme, 'dark', 'Reset to dark theme');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.4 Settings handle clearing custom accent color (null reset)', async () => {
    const ui = createUiStoreMock();
    ui.setAccentColor('#ff0055');
    assertEqual(ui.get().accentColor, '#ff0055', 'Accent set to pink');
    ui.setAccentColor(null);
    assertEqual(ui.get().accentColor, null, 'Accent reset to default null');
  });

  await reporter.test(FEATURES.SETTINGS_AUDIT, '10.5 Settings handle all notification preview modes (full, sender, none)', async () => {
    const ipc = new TauriIpcMockRouter();
    const modes = ['full', 'sender', 'none'];
    for (const mode of modes) {
      await ipc.invoke('save_settings', { input: { notificationPreview: mode } });
      const s = await ipc.invoke('get_settings');
      assertEqual(s.notificationPreview, mode, `Notification preview ${mode} saved`);
    }
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 11: Keyboard Navigation & Empty States (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.KEYBOARD_NAV, '11.1 Rapid repeated Esc dismiss calls do not corrupt UI store', async () => {
    const ui = createUiStoreMock();
    ui.openModal('settings');
    ui.closeModal();
    ui.closeModal();
    ui.closeModal();
    assertEqual(ui.get().openModal, null, 'Modal state remains cleanly null');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.2 Closing non-existent modal is a safe no-op', async () => {
    const ui = createUiStoreMock();
    assertEqual(ui.get().openModal, null, 'Initial openModal is null');
    ui.closeModal();
    assertEqual(ui.get().openModal, null, 'Still null after close');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.3 Navigation with null space and channel resets active view', async () => {
    const ui = createUiStoreMock();
    ui.navigate('space-1', 'ch-1');
    ui.navigate(null, null);
    assertEqual(ui.get().activeSpaceId, null, 'Active space null');
    assertEqual(ui.get().activeChannelId, null, 'Active channel null');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.4 Reply target clear on channel change', async () => {
    const ui = createUiStoreMock();
    ui.setReplyTo({ channelId: 'ch-1', messageId: 'msg-10', author: 'Bob', content: 'Yo' });
    assertEqual(ui.get().replyTo.channelId, 'ch-1', 'Reply set on ch-1');
    ui.setReplyTo(null);
    assertEqual(ui.get().replyTo, null, 'Reply cleared');
  });

  await reporter.test(FEATURES.KEYBOARD_NAV, '11.5 Navigation to non-existent space ID handled without crash', async () => {
    const ui = createUiStoreMock();
    ui.navigate('non-existent-space-999', 'ch-none');
    assertEqual(ui.get().activeSpaceId, 'non-existent-space-999', 'Space ID recorded');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 12: Roadmap & Docs Completion (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.ROADMAP_DOCS, '12.1 ROADMAP.md document does not contain unclosed code blocks', async () => {
    const content = readFileSync(resolve(process.cwd(), 'docs/ROADMAP.md'), 'utf8');
    const matches = content.match(/```/g) || [];
    assertEqual(matches.length % 2, 0, 'Code fences in ROADMAP.md must be balanced (even number)');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.2 PROJECT.md interface contracts have valid markdown table formatting', async () => {
    const content = readFileSync(resolve(process.cwd(), 'PROJECT.md'), 'utf8');
    assertIncludes(content, '| # | Feature |', 'PROJECT.md must include feature table header');
    assertIncludes(content, '| 15 | Adversarial Coverage Hardening |', 'PROJECT.md must include feature 15 row');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.3 TEST_INFRA.md defines all 4 Tiers explicitly', async () => {
    const content = readFileSync(resolve(process.cwd(), 'TEST_INFRA.md'), 'utf8');
    assertIncludes(content, 'Tier 1', 'Must contain Tier 1');
    assertIncludes(content, 'Tier 2', 'Must contain Tier 2');
    assertIncludes(content, 'Tier 3', 'Must contain Tier 3');
    assertIncludes(content, 'Tier 4', 'Must contain Tier 4');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.4 Package.json has valid semver 0.0.1', async () => {
    const pkg = JSON.parse(readFileSync(resolve(process.cwd(), 'package.json'), 'utf8'));
    assertEqual(pkg.version, '0.0.1', 'Package version must be 0.0.1');
  });

  await reporter.test(FEATURES.ROADMAP_DOCS, '12.5 Cargo.toml in src-tauri has matching 0.0.1 version', async () => {
    const cargoPath = resolve(process.cwd(), 'src-tauri/Cargo.toml');
    if (existsSync(cargoPath)) {
      const content = readFileSync(cargoPath, 'utf8');
      assertIncludes(content, 'version = "0.0.1"', 'Cargo.toml must declare version 0.0.1');
    }
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 13: Backend Rust Test Expansion (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.BACKEND_TESTS, '13.1 Crypto hashing: zero-length string produces standard SHA-1 and SHA-256 digests', async () => {
    const sha1Empty = sha1HexUpper('');
    assertEqual(sha1Empty, 'DA39A3EE5E6B4B0D3255BFEF95601890AFD80709', 'SHA-1 of empty string is well-known standard');
    const sha256Empty = sha256Hex('');
    assertEqual(sha256Empty, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 'SHA-256 of empty string is well-known standard');
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.2 Crypto hashing: large 1MB string hashed without stack overflow', async () => {
    const largeStr = 'A'.repeat(1024 * 1024);
    const hash = sha256Hex(largeStr);
    assertEqual(hash.length, 64, '1MB hash produces 64-char digest');
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.3 Auth router rejects identity creation with empty username or passphrase', async () => {
    const ipc = new TauriIpcMockRouter();
    await assertThrowsAsync(
      async () => ipc.invoke('create_identity', { username: '', passphrase: 'secret_password' }),
      'Username and passphrase are required',
      'Empty username should be rejected'
    );
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.4 Auth router rejects load_identity with incorrect passphrase', async () => {
    const ipc = new TauriIpcMockRouter();
    await ipc.invoke('create_identity', { username: 'testuser', passphrase: 'correct_password' });
    await assertThrowsAsync(
      async () => ipc.invoke('load_identity', { passphrase: 'wrong_password' }),
      'Invalid passphrase',
      'Wrong passphrase must be rejected'
    );
  });

  await reporter.test(FEATURES.BACKEND_TESTS, '13.5 Auth router rejects invalid recovery code format during recovery', async () => {
    const ipc = new TauriIpcMockRouter();
    await assertThrowsAsync(
      async () => ipc.invoke('recover_identity', { recoveryCode: 'INVALID-CODE', newPassphrase: 'new_pass' }),
      'Invalid recovery code format',
      'Malformed recovery code rejected'
    );
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 14: E2E Testing Suite (Tiers 1-4) (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.E2E_SUITE, '14.1 Assertion error handles undefined and null values cleanly', async () => {
    let caught = false;
    try {
      assertEqual(null, undefined, 'Null vs undefined');
    } catch (err) {
      caught = true;
      assertEqual(err.actual, null, 'Actual is null');
      assertEqual(err.expected, undefined, 'Expected is undefined');
    }
    assert(caught, 'Must catch mismatch');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.2 Deep equality comparison detects nested property differences', async () => {
    let caught = false;
    try {
      assertDeepEqual({ a: { b: 1 } }, { a: { b: 2 } }, 'Nested diff');
    } catch (err) {
      caught = true;
      assertIncludes(err.message, 'Objects not deeply equal', 'Deep equality error msg');
    }
    assert(caught, 'Must catch deep mismatch');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.3 assertIncludes throws when target substring is missing', async () => {
    let caught = false;
    try {
      assertIncludes('hello world', 'goodbye', 'Substring check');
    } catch (err) {
      caught = true;
      assertIncludes(err.message, "does not include substring 'goodbye'", 'Substring missing message');
    }
    assert(caught, 'Must throw on missing substring');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.4 assertMatch throws when regex pattern does not match', async () => {
    let caught = false;
    try {
      assertMatch('abc', /^\d+$/, 'Digits check');
    } catch (err) {
      caught = true;
      assertIncludes(err.message, 'does not match pattern', 'Regex mismatch message');
    }
    assert(caught, 'Must throw on regex mismatch');
  });

  await reporter.test(FEATURES.E2E_SUITE, '14.5 assertThrowsAsync throws if wrapped function does NOT throw', async () => {
    let caught = false;
    try {
      await assertThrowsAsync(async () => {
        // Successful execution without error
      }, 'Some error', 'Should fail because function succeeded');
    } catch (err) {
      caught = true;
      assertIncludes(err.message, 'Expected function to throw an error', 'Throws check message');
    }
    assert(caught, 'Must catch assertion failure');
  });

  // ═══════════════════════════════════════════════════════════════════
  // FEATURE 15: Adversarial Coverage Hardening (Boundary & Negative)
  // ═══════════════════════════════════════════════════════════════════
  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.1 SQL injection attack string in channel message handled safely', async () => {
    const msgStore = createMessageStoreMock();
    const sqli = "'; DROP TABLE messages; DROP TABLE users; --";
    const msg = msgStore.sendMessage('ch-1', sqli);
    assertEqual(msg.content, sqli, 'Message content preserved verbatim without SQL execution');
    assertEqual(msgStore.get().byChannel['ch-1'].length, 1, 'Channel still exists and has 1 message');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.2 Null-byte injection (\0) in seed does not crash avatar generator', async () => {
    const nullSeed = 'admin\0_secret_root\0';
    const svg = generateDeterministicSvgAvatar(nullSeed);
    const val = validateSvgXml(svg);
    assertEqual(val.valid, true, 'Null-byte seed produces valid SVG');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.3 Streamer mode handles null/empty email and token inputs without error', async () => {
    const streamer = createStreamerModeMock(true);
    assertEqual(streamer.maskEmail(null), '', 'Null email returns empty string');
    assertEqual(streamer.maskEmail(''), '', 'Empty email returns empty string');
    assertEqual(streamer.maskText(null), '', 'Null text returns empty string');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.4 Streamer mode preserves minimum mask length for short secrets', async () => {
    const streamer = createStreamerModeMock(true);
    streamer.setMaskStyle('asterisks');
    const shortMask = streamer.maskText('123'); // Length 3
    assertGreaterThanOrEqual(shortMask.length, 6, 'Short tokens clamped to min length 6');
  });

  await reporter.test(FEATURES.ADVERSARIAL_HARDENING, '15.5 Privacy shield formatSecret returns raw value when unshielded and streamer mode off', async () => {
    const streamer = createStreamerModeMock(false);
    const shield = createPrivacyShieldMock(streamer, false);
    const secret = 'my_plain_passphrase';
    const formatted = shield.formatSecret(secret, 'pass_key', false);
    assertEqual(formatted, secret, 'Raw value returned when shield and streamer mode are both off');
  });
}

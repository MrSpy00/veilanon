/**
 * VeilAnon E2E Test Harness — Mock API Providers
 * Simulates zero-key privacy network endpoints with realistic protocols,
 * error injection, rate-limiting, and k-anonymity ranges.
 */
import { sha1HexUpper, sha256Hex } from './crypto-utils.mjs';

/**
 * 1. Tor & Relay Anonymity Check Provider
 */
export class TorMockProvider {
  constructor() {
    this.isTor = false;
    this.currentIp = '198.51.100.42';
    this.failureMode = null; // 'network_error' | 'http_500' | 'timeout' | 'malformed_json'
    this.latencyMs = 20;
  }

  setTorExit(isTor, ip = '185.220.101.5') {
    this.isTor = isTor;
    this.currentIp = ip;
    this.failureMode = null;
  }

  setFailure(mode) {
    this.failureMode = mode;
  }

  async checkTorStatus() {
    if (this.failureMode === 'network_error') {
      throw new Error('Network unreachable: Failed to connect to check.torproject.org');
    }
    if (this.failureMode === 'http_500') {
      throw new Error('HTTP 500 Internal Server Error from check.torproject.org');
    }
    if (this.failureMode === 'malformed_json') {
      return { raw: '<html><body>Service Unavailable</body></html>', parsed: null, isTor: false, ip: '' };
    }
    return {
      isTor: this.isTor,
      ip: this.currentIp,
      timestamp: Date.now(),
    };
  }
}

/**
 * 2. IP Leak & Cloudflare CDN Trace Provider
 */
export class IpTraceMockProvider {
  constructor() {
    this.ip = '203.0.113.195';
    this.colo = 'IST';
    this.loc = 'TR';
    this.tls = 'TLSv1.3';
    this.sni = 'plaintext';
    this.warp = 'off';
    this.gateway = 'off';
    this.failureMode = null;
  }

  setIpDetails(details) {
    Object.assign(this, details);
  }

  setFailure(mode) {
    this.failureMode = mode;
  }

  getTraceText() {
    return [
      `fl=42f102`,
      `h=1.1.1.1`,
      `ip=${this.ip}`,
      `ts=${(Date.now() / 1000).toFixed(3)}`,
      `visit_scheme=https`,
      `uag=veilanon/0.0.1`,
      `colo=${this.colo}`,
      `loc=${this.loc}`,
      `tls=${this.tls}`,
      `sni=${this.sni}`,
      `warp=${this.warp}`,
      `gateway=${this.gateway}`,
      `rtt=18`,
    ].join('\n');
  }

  parseTrace(traceText) {
    const lines = traceText.split('\n');
    const result = {
      ip: '',
      colo: null,
      loc: null,
      tls: null,
      sni: null,
      warp: null,
      gateway: null,
      rtt_ms: 18,
    };

    for (const line of lines) {
      const idx = line.indexOf('=');
      if (idx === -1) continue;
      const key = line.slice(0, idx).trim();
      const val = line.slice(idx + 1).trim();
      if (key === 'ip') result.ip = val;
      else if (key === 'colo') result.colo = val;
      else if (key === 'loc') result.loc = val;
      else if (key === 'tls') result.tls = val;
      else if (key === 'sni') result.sni = val;
      else if (key === 'warp') result.warp = val;
      else if (key === 'gateway') result.gateway = val;
      else if (key === 'rtt') result.rtt_ms = parseInt(val, 10) || 18;
    }
    return result;
  }

  async checkIpLeak() {
    if (this.failureMode === 'network_error') {
      throw new Error('Network error: trace request failed');
    }
    const trace = this.getTraceText();
    return this.parseTrace(trace);
  }
}

/**
 * 3. Encrypted DoH (DNS-over-HTTPS) Provider
 */
export class DohMockProvider {
  constructor() {
    this.cloudflareOk = true;
    this.googleOk = true;
    this.cloudflareLatency = 14;
    this.googleLatency = 22;
    this.blockedProvider = null; // 'cloudflare' | 'google' | 'both'
  }

  setBlocked(provider) {
    this.blockedProvider = provider;
  }

  async checkDohStatus() {
    const cfOk = this.blockedProvider !== 'cloudflare' && this.blockedProvider !== 'both' && this.cloudflareOk;
    const gOk = this.blockedProvider !== 'google' && this.blockedProvider !== 'both' && this.googleOk;

    return {
      cloudflare_ok: cfOk,
      google_ok: gOk,
      latency_cloudflare_ms: cfOk ? this.cloudflareLatency : 0,
      latency_google_ms: gOk ? this.googleLatency : 0,
      doh_working: cfOk || gOk,
    };
  }
}

/**
 * 4. k-Anonymity Password Leak Check Provider (pwnedpasswords range API)
 */
export class PwnedPasswordsMockProvider {
  constructor() {
    // In-memory database of common breached hashes
    this.knownHashes = new Map();
    this.seedCommonPasswords();
    this.rateLimited = false;
  }

  seedCommonPasswords() {
    const common = [
      { pass: 'password', count: 3861493 },
      { pass: '123456', count: 24230577 },
      { pass: '12345678', count: 4501230 },
      { pass: 'qwerty', count: 4120931 },
      { pass: 'admin', count: 893120 },
      { pass: 'secret', count: 182390 },
      { pass: 'hunter2', count: 19842 },
      { pass: 'veilAnon2026!', count: 3 },
      { pass: 'p@ssword123', count: 5410 },
      { pass: 'letmein', count: 241092 },
    ];

    for (const item of common) {
      const fullHash = sha1HexUpper(item.pass);
      const prefix = fullHash.slice(0, 5);
      const suffix = fullHash.slice(5);
      if (!this.knownHashes.has(prefix)) {
        this.knownHashes.set(prefix, []);
      }
      this.knownHashes.get(prefix).push({ suffix, count: item.count });
    }
  }

  setRateLimited(limited) {
    this.rateLimited = limited;
  }

  /**
   * Add custom breached hash to mock database
   * @param {string} rawPassword 
   * @param {number} count 
   */
  addCustomBreachedPassword(rawPassword, count = 100) {
    const fullHash = sha1HexUpper(rawPassword);
    const prefix = fullHash.slice(0, 5);
    const suffix = fullHash.slice(5);
    if (!this.knownHashes.has(prefix)) {
      this.knownHashes.set(prefix, []);
    }
    this.knownHashes.get(prefix).push({ suffix, count });
  }

  /**
   * Query range endpoint with 5-character hex prefix
   * @param {string} prefix5Hex 
   * @returns {Promise<Array<[string, number]>>}
   */
  async checkPasswordRange(prefix5Hex) {
    if (this.rateLimited) {
      throw new Error('HTTP 429 Too Many Requests: Rate limit exceeded');
    }
    if (!prefix5Hex || typeof prefix5Hex !== 'string') {
      throw new Error('Invalid prefix: prefix must be a 5-character hex string');
    }
    const cleanPrefix = prefix5Hex.trim().toUpperCase();
    if (!/^[0-9A-F]{5}$/.test(cleanPrefix)) {
      throw new Error(`Invalid prefix format '${prefix5Hex}': must be exactly 5 hex characters`);
    }

    const matches = this.knownHashes.get(cleanPrefix) || [];
    return matches.map(m => [m.suffix, m.count]);
  }

  /**
   * Full client-side k-Anonymity verification
   * @param {string} password 
   * @returns {Promise<{ isPwned: boolean, breachCount: number, prefix: string }>}
   */
  async verifyPassword(password) {
    if (!password) return { isPwned: false, breachCount: 0, prefix: '' };
    const fullHash = sha1HexUpper(password);
    const prefix = fullHash.slice(0, 5);
    const suffix = fullHash.slice(5);

    const rangeResults = await this.checkPasswordRange(prefix);
    const match = rangeResults.find(([s]) => s === suffix);

    return {
      isPwned: !!match,
      breachCount: match ? match[1] : 0,
      prefix,
    };
  }
}

/**
 * 5. Real-Time Malicious URL Scanner Provider (URLhaus API)
 */
export class UrlHausMockProvider {
  constructor() {
    this.threatDatabase = new Map();
    this.seedThreats();
    this.failureMode = null;
  }

  seedThreats() {
    this.threatDatabase.set('http://malware-drop.example.com/payload.exe', {
      query_status: 'ok',
      url_status: 'online',
      threat: 'malware_download',
      tags: ['exe', 'trojan', 'redline'],
    });
    this.threatDatabase.set('https://phishing-stealer.xyz/login.html', {
      query_status: 'ok',
      url_status: 'online',
      threat: 'phishing',
      tags: ['credential_stealer', 'discord_token'],
    });
    this.threatDatabase.set('http://198.51.100.99:8080/mozi.m', {
      query_status: 'ok',
      url_status: 'offline',
      threat: 'botnet_c2',
      tags: ['mozi', 'botnet', 'ddos'],
    });
  }

  addThreat(url, details) {
    this.threatDatabase.set(url, {
      query_status: 'ok',
      url_status: details.url_status || 'online',
      threat: details.threat || 'malware_download',
      tags: details.tags || ['suspicious'],
    });
  }

  setFailure(mode) {
    this.failureMode = mode;
  }

  async scanUrl(url) {
    if (this.failureMode === 'timeout') {
      throw new Error('URLhaus scanner request timed out');
    }
    if (!url || typeof url !== 'string' || !url.trim()) {
      return {
        query_status: 'invalid_url',
        url_status: null,
        threat: null,
        tags: [],
      };
    }

    const cleanUrl = url.trim();
    if (this.threatDatabase.has(cleanUrl)) {
      return this.threatDatabase.get(cleanUrl);
    }

    // Default clean URL response
    return {
      query_status: 'no_results',
      url_status: null,
      threat: null,
      tags: [],
    };
  }
}

/**
 * 6. Multi-Provider DNS-over-HTTPS (DoH) Benchmark Provider
 */
export class MultiDohBenchmarkMockProvider {
  constructor() {
    this.providers = [
      { name: 'Cloudflare', endpoint: 'https://cloudflare-dns.com/dns-query', isReachable: true, latencyMs: 14 },
      { name: 'Google', endpoint: 'https://dns.google/resolve', isReachable: true, latencyMs: 22 },
      { name: 'Quad9', endpoint: 'https://dns.quad9.net/dns-query', isReachable: true, latencyMs: 28 },
      { name: 'AdGuard', endpoint: 'https://dns.adguard-dns.com/dns-query', isReachable: true, latencyMs: 35 },
      { name: 'Mullvad', endpoint: 'https://dns.mullvad.net/dns-query', isReachable: true, latencyMs: 40 },
    ];
    this.tamperSimulation = false;
  }

  setTamper(tamper) {
    this.tamperSimulation = tamper;
    if (tamper) {
      this.providers[0].isReachable = false;
      this.providers[1].isReachable = false;
      this.providers[2].isReachable = false;
    } else {
      this.providers.forEach(p => { p.isReachable = true; });
    }
  }

  async checkMultiDohStatus() {
    const reachable = this.providers.filter(p => p.isReachable);
    const avg = reachable.length > 0
      ? Math.round(reachable.reduce((a, b) => a + b.latencyMs, 0) / reachable.length)
      : 0;
    const fastest = reachable.length > 0
      ? [...reachable].sort((a, b) => a.latencyMs - b.latencyMs)[0].name
      : null;

    return {
      providers: this.providers,
      fastestProvider: fastest,
      averageLatencyMs: avg,
      censorshipTamperDetected: reachable.length < 3,
    };
  }
}

/**
 * 7. Privacy-Preserving Link Preview Provider (SSRF-Filtered Proxy)
 */
export class LinkPreviewMockProvider {
  constructor() {
    this.previews = new Map([
      ['https://github.com/MrSpy00/veilanon', {
        url: 'https://github.com/MrSpy00/veilanon',
        title: 'GitHub - MrSpy00/veilanon',
        description: 'Privacy-first, open-source desktop communication platform',
        image: 'https://veilanon.com/banner.png',
        siteName: 'GitHub',
        favicon: 'https://github.com/favicon.ico',
        isSafe: true,
      }],
      ['http://127.0.0.1/internal', {
        url: 'http://127.0.0.1/internal',
        title: null,
        description: null,
        image: null,
        siteName: null,
        favicon: null,
        isSafe: false,
      }],
    ]);
  }

  async fetchLinkPreview(url) {
    if (!url || url.includes('127.0.0.1') || url.includes('localhost') || url.includes('192.168.')) {
      return { url, title: null, description: null, image: null, siteName: null, favicon: null, isSafe: false };
    }
    return this.previews.get(url) || {
      url,
      title: 'Example Page Title',
      description: 'Example page description for preview card',
      image: null,
      siteName: 'Example',
      favicon: null,
      isSafe: true,
    };
  }
}

/**
 * 8. Cryptographic Clock Skew Detector Provider (WorldTimeAPI)
 */
export class ClockSkewMockProvider {
  constructor() {
    this.artificialSkewSeconds = 0;
    this.failureMode = null;
    this.skewThresholdSeconds = 30;
  }

  setSkew(seconds) {
    this.artificialSkewSeconds = seconds;
  }

  setFailure(mode) {
    this.failureMode = mode;
  }

  async detectClockSkew() {
    if (this.failureMode === 'network_error') {
      throw new Error('Network error: WorldTimeAPI unreachable');
    }
    const localTimestamp = Math.floor(Date.now() / 1000);
    const serverTimestamp = localTimestamp + this.artificialSkewSeconds;
    const skewSeconds = serverTimestamp - localTimestamp;
    const isSkewed = Math.abs(skewSeconds) > this.skewThresholdSeconds;

    return {
      local_timestamp: localTimestamp,
      server_timestamp: serverTimestamp,
      skew_seconds: skewSeconds,
      is_skewed: isSkewed,
    };
  }
}

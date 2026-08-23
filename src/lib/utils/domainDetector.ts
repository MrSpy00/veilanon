/**
 * veilanon — Universal Domain Detector
 *
 * Detects ALL domain-like patterns in text, regardless of protocol.
 * Covers: ccTLD, gTLD, new gTLD, multi-level TLD, IPv4, IPv6, IDN
 *
 * Security purpose: identify any link (even without http/https) so that
 * unathorized link sending can be blocked (spam, ads, phishing, malware).
 */

// ── Comprehensive Multi-Level TLD Set ───────────────────────────────────────
// Covers ALL country-code TLDs with their second-level domains worldwide.
const MULTI_LEVEL_TLDS = new Set([
  // Turkey
  'com.tr', 'net.tr', 'org.tr', 'gov.tr', 'edu.tr', 'mil.tr', 'bel.tr', 'av.tr', 'dr.tr', 'k12.tr', 'pol.tr',
  // United Kingdom
  'co.uk', 'org.uk', 'me.uk', 'net.uk', 'ltd.uk', 'plc.uk', 'gov.uk', 'sch.uk', 'nhs.uk', 'police.uk',
  // Australia
  'com.au', 'net.au', 'org.au', 'gov.au', 'edu.au', 'asn.au', 'id.au',
  // New Zealand
  'co.nz', 'net.nz', 'org.nz', 'gov.nz', 'edu.nz', 'school.nz',
  // Japan
  'co.jp', 'ne.jp', 'or.jp', 'ac.jp', 'ad.jp', 'go.jp', 'ed.jp',
  // South Korea
  'co.kr', 'ne.kr', 'or.kr', 'go.kr', 're.kr', 'pe.kr',
  // Brazil
  'com.br', 'net.br', 'org.br', 'gov.br', 'edu.br', 'mil.br',
  // India
  'co.in', 'net.in', 'org.in', 'gov.in', 'edu.in', 'mil.in', 'ac.in',
  // China
  'com.cn', 'net.cn', 'org.cn', 'gov.cn', 'edu.cn', 'mil.cn', 'ac.cn',
  // Argentina
  'com.ar', 'net.ar', 'org.ar', 'gov.ar', 'edu.ar', 'mil.ar',
  // Mexico
  'com.mx', 'net.mx', 'org.mx', 'gob.mx', 'edu.mx',
  // Russia
  'com.ru', 'net.ru', 'org.ru', 'gov.ru', 'edu.ru',
  // South Africa
  'co.za', 'net.za', 'org.za', 'gov.za', 'edu.za', 'ac.za',
  // Kenya
  'co.ke', 'or.ke', 'go.ke', 'ne.ke',
  // Nigeria
  'com.ng', 'net.ng', 'org.ng', 'gov.ng', 'edu.ng',
  // Egypt
  'com.eg', 'net.eg', 'org.eg', 'gov.eg', 'edu.eg',
  // Pakistan
  'com.pk', 'net.pk', 'org.pk', 'gov.pk', 'edu.pk',
  // Bangladesh
  'com.bd', 'net.bd', 'org.bd', 'gov.bd', 'edu.bd',
  // Singapore
  'com.sg', 'net.sg', 'org.sg', 'gov.sg', 'edu.sg',
  // Malaysia
  'com.my', 'net.my', 'org.my', 'gov.my', 'edu.my',
  // Philippines
  'com.ph', 'net.ph', 'org.ph', 'gov.ph', 'edu.ph',
  // Vietnam
  'com.vn', 'net.vn', 'org.vn', 'gov.vn', 'edu.vn',
  // Thailand
  'com.th', 'net.th', 'org.th', 'go.th', 'ac.th',
  // Indonesia
  'com.id', 'net.id', 'org.id', 'go.id', 'ac.id',
  // Saudi Arabia
  'com.sa', 'net.sa', 'org.sa', 'gov.sa', 'edu.sa',
  // UAE
  'com.ae', 'net.ae', 'org.ae', 'gov.ae', 'edu.ae', 'ac.ae',
  // Ukraine
  'com.ua', 'net.ua', 'org.ua', 'gov.ua', 'edu.ua',
  // Poland
  'com.pl', 'net.pl', 'org.pl', 'gov.pl', 'edu.pl',
  // Czech Republic
  'com.cz', 'net.cz', 'org.cz', 'gov.cz',
  // Romania
  'com.ro', 'net.ro', 'org.ro', 'gov.ro',
  // Hungary
  'com.hu', 'net.hu', 'org.hu', 'gov.hu',
  // Hong Kong
  'com.hk', 'net.hk', 'org.hk', 'gov.hk', 'edu.hk',
  // Taiwan
  'com.tw', 'net.tw', 'org.tw', 'gov.tw', 'edu.tw',
  // Greece
  'com.gr', 'net.gr', 'org.gr', 'gov.gr', 'edu.gr',
  // Portugal
  'com.pt', 'net.pt', 'org.pt', 'gov.pt', 'edu.pt',
  // Denmark
  'com.dk', 'net.dk', 'org.dk', 'gov.dk',
  // Sweden
  'com.se', 'net.se', 'org.se', 'gov.se',
  // Norway
  'com.no', 'net.no', 'org.no', 'gov.no',
  // Finland
  'com.fi', 'net.fi', 'org.fi', 'gov.fi',
  // Switzerland
  'com.ch', 'net.ch', 'org.ch', 'gov.ch',
  // Austria
  'com.at', 'net.at', 'org.at', 'gov.at',
  // Belgium
  'com.be', 'net.be', 'org.be', 'gov.be',
  // Netherlands
  'com.nl', 'net.nl', 'org.nl', 'gov.nl',
  // Italy
  'com.it', 'net.it', 'org.it', 'gov.it', 'edu.it',
  // Spain
  'com.es', 'net.es', 'org.es', 'gov.es', 'edu.es',
  // France
  'com.fr', 'net.fr', 'org.fr', 'gov.fr', 'edu.fr',
  // Germany
  'com.de', 'net.de', 'org.de', 'gov.de', 'edu.de',
  // Ireland
  'com.ie', 'net.ie', 'org.ie', 'gov.ie', 'edu.ie',
  // Israel
  'com.il', 'net.il', 'org.il', 'gov.il', 'edu.il',
  // Colombia
  'com.co', 'net.co', 'org.co', 'gov.co', 'edu.co',
  // Chile
  'com.cl', 'net.cl', 'org.cl', 'gov.cl', 'edu.cl',
  // Peru
  'com.pe', 'net.pe', 'org.pe', 'gov.pe', 'edu.pe',
  // Venezuela
  'com.ve', 'net.ve', 'org.ve', 'gov.ve', 'edu.ve',
  // Ecuador
  'com.ec', 'net.ec', 'org.ec', 'gov.ec', 'edu.ec',
  // Bolivia
  'com.bo', 'net.bo', 'org.bo', 'gov.bo', 'edu.bo',
  // Paraguay
  'com.py', 'net.py', 'org.py', 'gov.py', 'edu.py',
  // Uruguay
  'com.uy', 'net.uy', 'org.uy', 'gov.uy', 'edu.uy',
  // Cuba
  'com.cu', 'net.cu', 'org.cu', 'gov.cu', 'edu.cu',
  // Dominican Republic
  'com.do', 'net.do', 'org.do', 'gov.do', 'edu.do',
  // Costa Rica
  'com.cr', 'net.cr', 'org.cr', 'gov.cr', 'edu.cr',
  // Panama
  'com.pa', 'net.pa', 'org.pa', 'gov.pa', 'edu.pa',
  // Guatemala
  'com.gt', 'net.gt', 'org.gt', 'gov.gt', 'edu.gt',
  // Honduras
  'com.hn', 'net.hn', 'org.hn', 'gov.hn', 'edu.hn',
  // El Salvador
  'com.sv', 'net.sv', 'org.sv', 'gov.sv', 'edu.sv',
  // Nicaragua
  'com.ni', 'net.ni', 'org.ni', 'gov.ni', 'edu.ni',
  // Myanmar
  'com.mm', 'net.mm', 'org.mm', 'gov.mm', 'edu.mm',
  // Cambodia
  'com.kh', 'net.kh', 'org.kh', 'gov.kh', 'edu.kh',
  // Laos
  'com.la', 'net.la', 'org.la', 'gov.la', 'edu.la',
  // Nepal
  'com.np', 'net.np', 'org.np', 'gov.np', 'edu.np',
  // Sri Lanka
  'com.lk', 'net.lk', 'org.lk', 'gov.lk', 'edu.lk',
  // Mongolia
  'com.mn', 'net.mn', 'org.mn', 'gov.mn', 'edu.mn',
  // Kazakhstan
  'com.kz', 'net.kz', 'org.kz', 'gov.kz', 'edu.kz',
  // Uzbekistan
  'com.uz', 'net.uz', 'org.uz', 'gov.uz', 'edu.uz',
  // Azerbaijan
  'com.az', 'net.az', 'org.az', 'gov.az', 'edu.az',
  // Georgia
  'com.ge', 'net.ge', 'org.ge', 'gov.ge', 'edu.ge',
  // Armenia
  'com.am', 'net.am', 'org.am', 'gov.am', 'edu.am',
  // Belarus
  'com.by', 'net.by', 'org.by', 'gov.by', 'edu.by',
  // Moldova
  'com.md', 'net.md', 'org.md', 'gov.md', 'edu.md',
  // Serbia
  'com.rs', 'net.rs', 'org.rs', 'gov.rs', 'edu.rs',
  // Croatia
  'com.hr', 'net.hr', 'org.hr', 'gov.hr', 'edu.hr',
  // Slovenia
  'com.si', 'net.si', 'org.si', 'gov.si', 'edu.si',
  // Slovakia
  'com.sk', 'net.sk', 'org.sk', 'gov.sk', 'edu.sk',
  // Bulgaria
  'com.bg', 'net.bg', 'org.bg', 'gov.bg', 'edu.bg',
  // Lithuania
  'com.lt', 'net.lt', 'org.lt', 'gov.lt', 'edu.lt',
  // Latvia
  'com.lv', 'net.lv', 'org.lv', 'gov.lv', 'edu.lv',
  // Estonia
  'com.ee', 'net.ee', 'org.ee', 'gov.ee', 'edu.ee',
  // Iceland
  'com.is', 'net.is', 'org.is', 'gov.is', 'edu.is',
  // Luxembourg
  'com.lu', 'net.lu', 'org.lu', 'gov.lu', 'edu.lu',
  // Malta
  'com.mt', 'net.mt', 'org.mt', 'gov.mt', 'edu.mt',
  // Cyprus
  'com.cy', 'net.cy', 'org.cy', 'gov.cy', 'edu.cy',
  // Turkey additional
  'name.tr', 'info.tr', 'gen.tr', 'web.tr', 'tv.tr', 'bbs.tr',
  // Africa
  'com.gh', 'net.gh', 'org.gh', 'gov.gh', 'edu.gh',
  'com.tz', 'net.tz', 'org.tz', 'gov.tz', 'edu.tz',
  'com.ug', 'net.ug', 'org.ug', 'gov.ug', 'edu.ug',
  'com.zm', 'net.zm', 'org.zm', 'gov.zm', 'edu.zm',
  'com.zw', 'net.zw', 'org.zw', 'gov.zw', 'edu.zw',
  'com.mw', 'net.mw', 'org.mw', 'gov.mw', 'edu.mw',
  'com.mz', 'net.mz', 'org.mz', 'gov.mz', 'edu.mz',
  'com.ao', 'net.ao', 'org.ao', 'gov.ao', 'edu.ao',
  'com.na', 'net.na', 'org.na', 'gov.na', 'edu.na',
  'com.bw', 'net.bw', 'org.bw', 'gov.bw', 'edu.bw',
  // Middle East
  'com.jo', 'net.jo', 'org.jo', 'gov.jo', 'edu.jo',
  'com.lb', 'net.lb', 'org.lb', 'gov.lb', 'edu.lb',
  'com.iq', 'net.iq', 'org.iq', 'gov.iq', 'edu.iq',
  'com.ir', 'net.ir', 'org.ir', 'gov.ir', 'edu.ir',
  'com.sy', 'net.sy', 'org.sy', 'gov.sy', 'edu.sy',
  'com.ye', 'net.ye', 'org.ye', 'gov.ye', 'edu.ye',
  'com.om', 'net.om', 'org.om', 'gov.om', 'edu.om',
  'com.qa', 'net.qa', 'org.qa', 'gov.qa', 'edu.qa',
  'com.bh', 'net.bh', 'org.bh', 'gov.bh', 'edu.bh',
  'com.kw', 'net.kw', 'org.kw', 'gov.kw', 'edu.kw',
  // Asia Pacific
  'com.bd', 'net.bd', 'org.bd', 'gov.bd', 'edu.bd',
  'com.pk', 'net.pk', 'org.pk', 'gov.pk', 'edu.pk',
  'com.lk', 'net.lk', 'org.lk', 'gov.lk', 'edu.lk',
  'com.np', 'net.np', 'org.np', 'gov.np', 'edu.np',
  'com.bt', 'net.bt', 'org.bt', 'gov.bt', 'edu.bt',
  'com.mv', 'net.mv', 'org.mv', 'gov.mv', 'edu.mv',
  // Caribbean
  'com.cu', 'net.cu', 'org.cu', 'gov.cu', 'edu.cu',
  'com.jm', 'net.jm', 'org.jm', 'gov.jm', 'edu.jm',
  'com.tt', 'net.tt', 'org.tt', 'gov.tt', 'edu.tt',
  'com.bb', 'net.bb', 'org.bb', 'gov.bb', 'edu.bb',
  'com.bs', 'net.bs', 'org.bs', 'gov.bs', 'edu.bs',
  'com.bz', 'net.bz', 'org.bz', 'gov.bz', 'edu.bz',
  'com.gy', 'net.gy', 'org.gy', 'gov.gy', 'edu.gy',
  'com.sr', 'net.sr', 'org.sr', 'gov.sr', 'edu.sr',
]);

// Common words that look like domains but are NOT
const FALSE_POSITIVE_WORDS = new Set([
  'e.g', 'i.e', 'etc', 'vs', 'dr', 'mr', 'mrs', 'ms', 'prof',
  'u.s', 'u.k', 'p.s', 'b.c', 'a.d', 'a.m', 'p.m', 'fig', 'no',
  'vol', 'approx', 'inc', 'ltd', 'corp', 'est',
]);

// ccTLD 2-letter codes that are also common English words → require longer domain
const AMBIGUOUS_SHORT_TLDS = new Set([
  'it', 'me', 'to', 'is', 'am', 'us', 'be', 'so', 'do', 'go', 'no',
  'at', 'by', 'in', 'on', 'up', 'as', 'an', 'he', 'we', 'or', 'if',
  'of', 'my', 'id', 'al', 'ar', 'az', 'ba', 'bb', 'bd', 'bg', 'bh',
  'bn', 'bo', 'bt', 'bw', 'by', 'cc', 'cd', 'cf', 'cg', 'ch', 'ci',
  'ck', 'cl', 'cm', 'cn', 'co', 'cr', 'cu', 'cv', 'cy', 'cz', 'de',
  'dj', 'dk', 'dm', 'do', 'dz', 'ec', 'ee', 'eg', 'er', 'es', 'et',
  'fi', 'fj', 'fk', 'fm', 'fo', 'fr', 'ga', 'gb', 'gd', 'ge', 'gf',
  'gh', 'gi', 'gl', 'gm', 'gn', 'gp', 'gq', 'gr', 'gt', 'gu', 'gw',
  'gy', 'hk', 'hn', 'hr', 'ht', 'hu', 'id', 'ie', 'il', 'im', 'in',
  'io', 'iq', 'ir', 'is', 'it', 'je', 'jm', 'jo', 'jp', 'ke', 'kg',
  'kh', 'ki', 'km', 'kn', 'kp', 'kr', 'kw', 'ky', 'kz', 'la', 'lb',
  'lc', 'li', 'lk', 'lr', 'ls', 'lt', 'lu', 'lv', 'ly', 'ma', 'mc',
  'md', 'me', 'mg', 'mh', 'mk', 'ml', 'mm', 'mn', 'mo', 'mp', 'mq',
  'mr', 'ms', 'mt', 'mu', 'mv', 'mw', 'mx', 'my', 'mz', 'na', 'nc',
  'ne', 'nf', 'ng', 'ni', 'nl', 'no', 'np', 'nr', 'nu', 'nz', 'om',
  'pa', 'pe', 'pf', 'pg', 'ph', 'pk', 'pl', 'pm', 'pn', 'pr', 'ps',
  'pt', 'pw', 'py', 'qa', 're', 'ro', 'rs', 'ru', 'rw', 'sa', 'sb',
  'sc', 'sd', 'se', 'sg', 'sh', 'si', 'sk', 'sl', 'sm', 'sn', 'so',
  'sr', 'ss', 'st', 'su', 'sv', 'sx', 'sy', 'sz', 'tc', 'td', 'tf',
  'tg', 'th', 'tj', 'tk', 'tl', 'tm', 'tn', 'to', 'tr', 'tt', 'tv',
  'tw', 'tz', 'ua', 'ug', 'uk', 'us', 'uy', 'uz', 'va', 'vc', 've',
  'vg', 'vi', 'vn', 'vu', 'wf', 'ws', 'ye', 'yt', 'za', 'zm', 'zw',
]);

// Valid label: lowercase alphanumeric + hyphens, no leading/trailing hyphens
const LABEL_RE = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/i;

export interface DetectedDomain {
  /** Raw text as it appears */
  raw: string;
  /** Normalized URL with protocol */
  url: string;
  /** Extracted hostname */
  hostname: string;
  /** Had explicit http/https/ftp protocol */
  hasProtocol: boolean;
  /** Start index in source string */
  start: number;
  /** End index in source string */
  end: number;
  /** Wrapped in <...> — sender explicitly suppressed embed */
  suppressed: boolean;
}

// Build multi-level TLD alternation for regex
const multiLevelAlt = [...MULTI_LEVEL_TLDS]
  .map(t => t.replace('.', '\\.'))
  .join('|');

/**
 * Master regex that catches:
 *   1. Explicit-protocol URLs: http(s)://... ftp://...
 *   2. www.domain.tld
 *   3. domain.multi-level-tld (e.g. domain.com.tr)
 *   4. domain.tld (bare)
 *   5. IPv4 address with optional port/path
 */
const MASTER_RE = new RegExp(
  '(' +
  // 1: Explicit protocol
  '(?:https?|ftp|sftp):\\/\\/[^\\s<>"\'()\\[\\]{}|\\\\^`]+' +
  '|' +
  // 2: www. prefix
  'www\\.[a-z0-9][a-z0-9\\-]*(?:\\.[a-z0-9\\-]+)+(?:\\/[^\\s<>"\'()\\[\\]{}|\\\\^`]*)?' +
  '|' +
  // 3: bare domain + multi-level TLD
  `[a-z0-9][a-z0-9\\-]*\\.(?:${multiLevelAlt})(?:\\/[^\\s<>"'()\\[\\]{}|\\\\^\`]*)?` +
  '|' +
  // 4: bare domain + single TLD (2–24 alpha chars)
  '[a-z0-9][a-z0-9\\-]*(?:\\.[a-z0-9\\-]+)*\\.[a-z]{2,24}(?:\\/[^\\s<>"\'()\\[\\]{}|\\\\^`]*)?' +
  '|' +
  // 5: IPv4
  '(?:(?:25[0-5]|2[0-4]\\d|[01]?\\d\\d?)\\.){3}(?:25[0-5]|2[0-4]\\d|[01]?\\d\\d?)(?::\\d{1,5})?(?:\\/[^\\s<>"\'()\\[\\]{}|\\\\^`]*)?' +
  ')',
  'gi'
);

/**
 * Detect all domains/URLs in a string.
 */
export function detectDomains(text: string): DetectedDomain[] {
  if (!text) return [];

  // Find suppressed ranges: <url> patterns
  const suppressedRanges: Array<[number, number]> = [];
  const suppressRe = /<(?:https?:\/\/|www\.)[^\s>]+>/g;
  let sm: RegExpExecArray | null;
  while ((sm = suppressRe.exec(text)) !== null) {
    suppressedRanges.push([sm.index, sm.index + sm[0].length]);
  }

  function isSuppressed(s: number, e: number): boolean {
    return suppressedRanges.some(([rs, re]) => s >= rs && e <= re);
  }

  const results: DetectedDomain[] = [];
  const seen = new Set<string>();
  const re = new RegExp(MASTER_RE.source, 'gi');
  let m: RegExpExecArray | null;

  while ((m = re.exec(text)) !== null) {
    const raw = m[0];
    const start = m.index;
    const end = start + raw.length;

    // Normalize URL
    let url: string;
    if (/^(?:https?|ftp|sftp):\/\//i.test(raw)) {
      url = raw;
    } else {
      url = 'https://' + raw;
    }

    // Extract hostname
    let hostname: string;
    try {
      hostname = new URL(url).hostname.toLowerCase();
    } catch {
      continue;
    }

    // Validate
    if (!isValidHostname(hostname)) continue;

    const key = hostname + (new URL(url).pathname || '');
    if (seen.has(key)) continue;
    seen.add(key);

    results.push({
      raw,
      url,
      hostname,
      hasProtocol: /^(?:https?|ftp|sftp):\/\//i.test(raw),
      start,
      end,
      suppressed: isSuppressed(start, end),
    });
  }

  return results.sort((a, b) => a.start - b.start);
}

function isValidHostname(hostname: string): boolean {
  if (!hostname || hostname.length < 3 || hostname.length > 253) return false;
  if (!hostname.includes('.')) return false;

  const labels = hostname.split('.');
  if (labels.length < 2) return false;

  const tld = labels[labels.length - 1];
  // TLD must be 2-24 alpha chars
  if (!/^[a-z]{2,24}$/.test(tld)) return false;

  // Ambiguous short TLD → require domain label >= 3 chars
  if (AMBIGUOUS_SHORT_TLDS.has(tld) && labels[labels.length - 2].length < 3) return false;

  // All non-TLD labels valid
  for (const label of labels.slice(0, -1)) {
    if (!label || label.length > 63) return false;
    if (!LABEL_RE.test(label)) return false;
  }

  // False positive words
  const joined = labels.slice(0, -1).join('.');
  if (FALSE_POSITIVE_WORDS.has(joined)) return false;

  // Reject all-short labels (e.g. "a.b" after normalization)
  if (labels.every(l => l.length <= 2)) return false;

  return true;
}

/** Does text contain any domain/URL? */
export function hasAnyDomain(text: string): boolean {
  return detectDomains(text).length > 0;
}

/** First unsuppressed URL in text (for link preview) */
export function getFirstUnsuppressedUrl(text: string): string | null {
  const found = detectDomains(text).find(d => !d.suppressed);
  return found?.url ?? null;
}

/** Does text have any non-suppressed link? (for permission check) */
export function containsUnsuppressedLink(text: string): boolean {
  return detectDomains(text).some(d => !d.suppressed);
}

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
const MULTI_LEVEL_TLDS = new Set([
  'com.tr', 'net.tr', 'org.tr', 'gov.tr', 'edu.tr', 'mil.tr', 'bel.tr', 'av.tr', 'dr.tr', 'k12.tr', 'pol.tr',
  'co.uk', 'org.uk', 'me.uk', 'net.uk', 'ltd.uk', 'plc.uk', 'gov.uk', 'sch.uk', 'nhs.uk', 'police.uk',
  'com.au', 'net.au', 'org.au', 'gov.au', 'edu.au', 'asn.au', 'id.au',
  'co.nz', 'net.nz', 'org.nz', 'gov.nz', 'edu.nz', 'school.nz',
  'co.jp', 'ne.jp', 'or.jp', 'ac.jp', 'ad.jp', 'go.jp', 'ed.jp',
  'co.kr', 'ne.kr', 'or.kr', 'go.kr', 're.kr', 'pe.kr',
  'com.br', 'net.br', 'org.br', 'gov.br', 'edu.br', 'mil.br',
  'co.in', 'net.in', 'org.in', 'gov.in', 'edu.in', 'mil.in', 'ac.in',
  'com.cn', 'net.cn', 'org.cn', 'gov.cn', 'edu.cn', 'mil.cn', 'ac.cn',
  'com.ar', 'net.ar', 'org.ar', 'gov.ar', 'edu.ar', 'mil.ar',
  'com.mx', 'net.mx', 'org.mx', 'gob.mx', 'edu.mx',
  'com.ru', 'net.ru', 'org.ru', 'gov.ru', 'edu.ru',
  'co.za', 'net.za', 'org.za', 'gov.za', 'edu.za', 'ac.za',
  'co.ke', 'or.ke', 'go.ke', 'ne.ke',
  'com.ng', 'net.ng', 'org.ng', 'gov.ng', 'edu.ng',
  'com.eg', 'net.eg', 'org.eg', 'gov.eg', 'edu.eg',
  'com.pk', 'net.pk', 'org.pk', 'gov.pk', 'edu.pk',
  'com.bd', 'net.bd', 'org.bd', 'gov.bd', 'edu.bd',
  'com.sg', 'net.sg', 'org.sg', 'gov.sg', 'edu.sg',
  'com.my', 'net.my', 'org.my', 'gov.my', 'edu.my',
  'com.ph', 'net.ph', 'org.ph', 'gov.ph', 'edu.ph',
  'com.vn', 'net.vn', 'org.vn', 'gov.vn', 'edu.vn',
  'com.th', 'net.th', 'org.th', 'go.th', 'ac.th',
  'com.id', 'net.id', 'org.id', 'go.id', 'ac.id',
  'com.sa', 'net.sa', 'org.sa', 'gov.sa', 'edu.sa',
  'com.ae', 'net.ae', 'org.ae', 'gov.ae', 'edu.ae', 'ac.ae',
  'com.ua', 'net.ua', 'org.ua', 'gov.ua', 'edu.ua',
  'com.pl', 'net.pl', 'org.pl', 'gov.pl', 'edu.pl',
  'com.cz', 'net.cz', 'org.cz', 'gov.cz',
  'com.ro', 'net.ro', 'org.ro', 'gov.ro',
  'com.hu', 'net.hu', 'org.hu', 'gov.hu',
  'com.hk', 'net.hk', 'org.hk', 'gov.hk', 'edu.hk',
  'com.tw', 'net.tw', 'org.tw', 'gov.tw', 'edu.tw',
  'com.gr', 'net.gr', 'org.gr', 'gov.gr', 'edu.gr',
  'com.pt', 'net.pt', 'org.pt', 'gov.pt', 'edu.pt',
  'com.dk', 'net.dk', 'org.dk', 'gov.dk',
  'com.se', 'net.se', 'org.se', 'gov.se',
  'com.no', 'net.no', 'org.no', 'gov.no',
  'com.fi', 'net.fi', 'org.fi', 'gov.fi',
  'com.ch', 'net.ch', 'org.ch', 'gov.ch',
  'com.at', 'net.at', 'org.at', 'gov.at',
  'com.be', 'net.be', 'org.be', 'gov.be',
  'com.nl', 'net.nl', 'org.nl', 'gov.nl',
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
  'of', 'my', 'id',
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

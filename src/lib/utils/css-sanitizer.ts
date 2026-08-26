/**
 * veilanon — CSS Sanitizer & Security Validator
 * Blocks malicious CSS vectors, dangerous protocols, and HTML injections.
 */

export interface SanitizationResult {
  safe: string;
  warnings: string[];
  isValid: boolean;
}

export const MAX_CSS_LENGTH = 24_000;

const BLOCKED_PATTERNS: Array<{ regex: RegExp; message: string }> = [
  { regex: /@import\s+[^;]+;/gi, message: '@import kuralı güvenlik nedeniyle engellendi.' },
  { regex: /javascript\s*:/gi, message: 'javascript: protokolü engellendi.' },
  { regex: /vbscript\s*:/gi, message: 'vbscript: protokolü engellendi.' },
  { regex: /expression\s*\([^)]*\)/gi, message: 'CSS expression() engellendi.' },
  { regex: /behavior\s*:[^;]+;/gi, message: 'behavior kuralı engellendi.' },
  { regex: /-moz-binding\s*:[^;]+;/gi, message: '-moz-binding kuralı engellendi.' },
  { regex: /<\s*script[^>]*>[\s\S]*?<\s*\/\s*script\s*>/gi, message: '<script> etiketleri engellendi.' },
  { regex: /<\s*(script|iframe|object|embed|style|svg|img|link|meta)\b[^>]*>/gi, message: 'HTML etiketleri engellendi.' },
];

/**
 * Validates URLs used inside `url(...)` to ensure they only use safe protocols.
 */
function sanitizeUrls(css: string, warnings: string[]): string {
  return css.replace(/url\(\s*['"]?([^'")]+)['"]?\s*\)/gi, (match, rawUrl: string) => {
    const trimmed = rawUrl.trim();
    if (
      trimmed.startsWith('https://') ||
      trimmed.startsWith('http://') ||
      trimmed.startsWith('data:image/') ||
      trimmed.startsWith('blob:') ||
      trimmed.startsWith('tauri://') ||
      trimmed.startsWith('/') ||
      trimmed.startsWith('./')
    ) {
      return `url("${trimmed}")`;
    }
    warnings.push(`Güvensiz URL protokolü engellendi: ${trimmed.slice(0, 30)}...`);
    return 'none';
  });
}

/**
 * Sanitizes user-provided CSS text.
 */
export function sanitizeCss(rawCss: string): SanitizationResult {
  const warnings: string[] = [];

  if (!rawCss || typeof rawCss !== 'string') {
    return { safe: '', warnings: [], isValid: true };
  }

  // 1. Length check
  let processed = rawCss;
  if (processed.length > MAX_CSS_LENGTH) {
    warnings.push(`CSS uzunluğu ${MAX_CSS_LENGTH} karakter sınırını aştı (kırpıldı).`);
    processed = processed.slice(0, MAX_CSS_LENGTH);
  }

  // 2. Blocked pattern removal
  for (const { regex, message } of BLOCKED_PATTERNS) {
    if (regex.test(processed)) {
      warnings.push(message);
      processed = processed.replace(regex, '');
    }
  }

  // 3. URL protocol verification
  processed = sanitizeUrls(processed, warnings);

  return {
    safe: processed.trim(),
    warnings,
    isValid: warnings.length === 0,
  };
}

/**
 * Validates a background media URL (Image / Video).
 */
export function validateMediaUrl(url: string): { isValid: boolean; error: string | null } {
  if (!url || typeof url !== 'string') return { isValid: true, error: null };
  const trimmed = url.trim();
  if (!trimmed) return { isValid: true, error: null };

  if (trimmed.length > 2048) {
    return { isValid: false, error: 'URL uzunluğu 2048 karakter sınırını aşıyor.' };
  }

  const isSafe =
    trimmed.startsWith('https://') ||
    trimmed.startsWith('http://') ||
    trimmed.startsWith('data:') ||
    trimmed.startsWith('blob:') ||
    trimmed.startsWith('tauri://') ||
    trimmed.startsWith('/') ||
    trimmed.startsWith('file://');

  if (!isSafe) {
    return { isValid: false, error: 'Yalnızca güvenli protokoller (https, data, blob, file) desteklenir.' };
  }

  return { isValid: true, error: null };
}

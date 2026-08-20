/**
 * veilanon — Theme Application Engine & Customization Helpers
 * Applies presets, accent colors, AMOLED layers, and custom CSS nodes to the DOM.
 */

import { getPresetById, type ThemePreset, type ThemeTokens } from '$lib/themes/presets';
import { sanitizeCss } from './css-sanitizer';

export interface ThemeExportData {
  version: number;
  id?: string;
  name: string;
  presetThemeId: string;
  customCss: string;
  customCssEnabled: boolean;
  customBgImage: string;
  customBgVideo: string;
  customBgOpacity: number;
  accentColor?: string | null;
  exportedAt?: string;
}

export interface SavedTheme {
  id: string;
  name: string;
  presetThemeId: string;
  customCss: string;
  customCssEnabled: boolean;
  accentColor: string | null;
  customBgImage: string;
  customBgVideo: string;
  customBgOpacity: number;
  savedAt: string;
}

const SAVED_THEMES_KEY = 'veilanon-saved-themes';

const CUSTOM_STYLE_NODE_ID = 'veilanon-custom-theme';
const PRESET_STYLE_NODE_ID = 'veilanon-preset-tokens';

/**
 * Calculates whether a color is bright or dark to choose the best contrast foreground.
 */
export function getContrastForeground(hexOrHsl: string): '#ffffff' | '#000000' {
  if (!hexOrHsl) return '#ffffff';
  let hex = hexOrHsl.trim();

  // If hex format #RRGGBB
  if (hex.startsWith('#')) {
    if (hex.length === 4) {
      hex = '#' + hex[1] + hex[1] + hex[2] + hex[2] + hex[3] + hex[3];
    }
    const r = parseInt(hex.slice(1, 3), 16) || 0;
    const g = parseInt(hex.slice(3, 5), 16) || 0;
    const b = parseInt(hex.slice(5, 7), 16) || 0;
    // Relative luminance
    const yiq = (r * 299 + g * 587 + b * 114) / 1000;
    return yiq >= 140 ? '#000000' : '#ffffff';
  }

  // If HSL format hsl(h, s%, l%)
  const match = hex.match(/hsl\(\s*\d+\s*,\s*\d+%\s*,\s*(\d+)%\s*\)/i);
  if (match && match[1]) {
    const l = parseInt(match[1], 10);
    return l >= 60 ? '#000000' : '#ffffff';
  }

  return '#ffffff';
}

/**
 * Applies preset CSS tokens dynamically to the root element.
 */
export function applyThemeTokensToDom(
  presetId: string,
  isDark: boolean,
  isAmoled: boolean,
  accentColor: string | null = null
) {
  if (typeof document === 'undefined') return;

  const preset = getPresetById(presetId);
  const root = document.documentElement;
  root.setAttribute('data-preset', preset.id);

  const tokens: ThemeTokens = isDark ? preset.dark : preset.light;

  // Let's create or update a dedicated style tag for preset tokens
  let styleNode = document.getElementById(PRESET_STYLE_NODE_ID) as HTMLStyleElement | null;
  if (!styleNode) {
    styleNode = document.createElement('style');
    styleNode.id = PRESET_STYLE_NODE_ID;
    document.head.appendChild(styleNode);
  }

  const cssDeclarations = Object.entries(tokens)
    .map(([key, val]) => `  ${key}: ${val};`)
    .join('\n');

  styleNode.textContent = `:root {\n${cssDeclarations}\n}`;

  // If user has an explicit accent override, apply it on top
  if (accentColor) {
    const fg = getContrastForeground(accentColor);
    root.style.setProperty('--veil-brand', accentColor);
    root.style.setProperty('--veil-brand-hover', `color-mix(in srgb, ${accentColor} 88%, black)`);
    root.style.setProperty('--veil-brand-active', `color-mix(in srgb, ${accentColor} 80%, black)`);
    root.style.setProperty('--veil-brand-subtle', `color-mix(in srgb, ${accentColor} 12%, transparent)`);
    root.style.setProperty('--veil-brand-border', `color-mix(in srgb, ${accentColor} 25%, transparent)`);
    root.style.setProperty('--veil-brand-foreground', fg);
  } else {
    root.style.removeProperty('--veil-brand');
    root.style.removeProperty('--veil-brand-hover');
    root.style.removeProperty('--veil-brand-active');
    root.style.removeProperty('--veil-brand-subtle');
    root.style.removeProperty('--veil-brand-border');
    root.style.removeProperty('--veil-brand-foreground');
  }

  // Handle AMOLED attribute
  if (isAmoled && isDark) {
    root.setAttribute('data-amoled', 'true');
  } else {
    root.removeAttribute('data-amoled');
  }
}

/**
 * Applies or removes the custom CSS style node from document head.
 */
export function applyCustomCssNode(css: string, enabled: boolean) {
  if (typeof document === 'undefined') return;

  let node = document.getElementById(CUSTOM_STYLE_NODE_ID) as HTMLStyleElement | null;

  if (!enabled || !css.trim()) {
    if (node) node.textContent = '';
    return;
  }

  if (!node) {
    node = document.createElement('style');
    node.id = CUSTOM_STYLE_NODE_ID;
    document.head.appendChild(node);
  }

  const sanitized = sanitizeCss(css);
  node.textContent = sanitized.safe;
}

/**
 * Removes the custom CSS style node completely.
 */
export function clearCustomCssNode() {
  if (typeof document === 'undefined') return;
  const node = document.getElementById(CUSTOM_STYLE_NODE_ID);
  if (node) node.remove();
}

/**
 * Generates an starter template for users to edit in Theme Studio.
 */
export function getStarterCssTemplate(): string {
  return `/* ============================================================
 * veilanon — Kişisel Tema CSS Şablonu
 * Tüm değişkenler :root bloğu içinde tanımlanmalıdır.
 * ============================================================ */

:root {
  /* ── Vurgu (Brand) Renkleri ──────────────────────────────── */
  --veil-brand:            #7c3aed;
  --veil-brand-hover:      #8b5cf6;
  --veil-brand-active:     #6d28d9;
  --veil-brand-subtle:     rgba(124, 58, 237, 0.12);
  --veil-brand-border:     rgba(124, 58, 237, 0.25);
  --veil-brand-foreground: #ffffff; /* Buton içi metin rengi (#fff veya #000) */

  /* ── Katmanlı Arka Planlar ──────────────────────────────── */
  --veil-bg-void:          #0b0d13; /* En derin arka plan (pencere) */
  --veil-bg-base:          #11141d; /* Ana gövde katmanı */
  --veil-bg-elevated:      #171b26; /* Kartlar ve paneller */
  --veil-bg-surface:       #1e2332; /* Giriş alanları (inputs) */
  --veil-bg-overlay:       #262c3e; /* Hover durumları */
  --veil-bg-raised:        #2f364c; /* Sağ tık menüleri */

  /* ── Tipografi & Metin Renkleri ─────────────────────────── */
  --veil-text-primary:     #f1f5f9; /* Başlıklar ve ana metin */
  --veil-text-secondary:   #94a3b8; /* İkincil açıklamalar */
  --veil-text-muted:       #64748b; /* Pasif ve küçük etiketler */
  --veil-text-disabled:    #475569; /* Devre dışı alanlar */

  /* ── Kenarlıklar (Borders) ──────────────────────────────── */
  --veil-border:           #242b3d;
  --veil-border-subtle:    #191e2b;
  --veil-border-focus:     #7c3aed;

  /* ── Navigasyon & Yan Panel ─────────────────────────────── */
  --veil-sidebar-bg:       #090a0f;
  --veil-channel-bg:       #0e1118;

  /* ── Özel Efektler ──────────────────────────────────────── */
  --veil-theme-glow:       rgba(124, 58, 237, 0.1);
}
`;
}

/**
 * Generates an AI prompt template tailored for generating veilanon CSS themes safely.
 */
export function generateAiThemePrompt(targetIdea: string = ''): string {
  const ideaClause = targetIdea.trim()
    ? `Tema Fikri / Konsepti:\n"${targetIdea.trim()}"\n\n`
    : 'Tema Fikri / Konsepti: [Buraya istediğin temayı yaz, örn: "Cyberpunk neon pembe ve kömür siyahı yüzeyler", "Lo-Fi sıcak kahve ve krem", "Matrix neon yeşili"]\n\n';

  return `Sen bir "veilanon" masaüstü iletişim platformu için premium CSS teması oluşturan uzmansın.

${ideaClause}Lütfen aşağıdaki kurallara KESİNLİKLE uyarak geçerli ve güvenli bir CSS çıktısı üret:

1. KESİNLİKLE @import, javascript:, expression(), vbscript:, behavior veya HTML etiketleri (<script>, <iframe> vs.) KULLANMA.
2. Yalnızca aşağıdaki semantik CSS değişkenlerini içeren bir \`:root { ... }\` bloğu yaz.
3. Yüksek kontrast oranına (WCAG 4.5:1) dikkat et; metinler arka planlar üzerinde mükemmel şekilde okunabilir olmalı.
4. Açık renkli brand renklerinde (örn. sarı, açık cyan), \`--veil-brand-foreground: #000000;\` kullan. Koyu veya canlı renklerde \`--veil-brand-foreground: #ffffff;\` kullan.

Değiştirilecek Token Sözleşmesi:
\`\`\`css
:root {
  --veil-brand: [Hex/HSL];
  --veil-brand-hover: [Hex/HSL];
  --veil-brand-active: [Hex/HSL];
  --veil-brand-subtle: [RGBA/Hex];
  --veil-brand-border: [RGBA/Hex];
  --veil-brand-foreground: [#ffffff veya #000000];

  --veil-bg-void: [Hex];
  --veil-bg-base: [Hex];
  --veil-bg-elevated: [Hex];
  --veil-bg-surface: [Hex];
  --veil-bg-overlay: [Hex];
  --veil-bg-raised: [Hex];

  --veil-text-primary: [Hex];
  --veil-text-secondary: [Hex];
  --veil-text-muted: [Hex];
  --veil-text-disabled: [Hex];

  --veil-border: [Hex];
  --veil-border-subtle: [Hex];
  --veil-border-focus: [Hex];

  --veil-sidebar-bg: [Hex];
  --veil-channel-bg: [Hex];
  --veil-theme-glow: [RGBA];
}
\`\`\`

Yalnızca CSS kod bloğu olarak çıktı ver, gereksiz uzun açıklamalar ekleme.`;
}

/**
 * Exports current custom theme configuration as a versioned JSON string.
 */
export function exportThemeJson(data: ThemeExportData): string {
  const exportPayload: ThemeExportData = {
    version: 1,
    id: data.id || `custom-theme-${Date.now()}`,
    name: data.name || 'Kişisel Tema',
    presetThemeId: data.presetThemeId || 'veil-origin',
    customCss: data.customCss || '',
    customCssEnabled: data.customCssEnabled ?? true,
    customBgImage: data.customBgImage || '',
    customBgVideo: data.customBgVideo || '',
    customBgOpacity: data.customBgOpacity ?? 0.26,
    accentColor: data.accentColor ?? null,
    exportedAt: new Date().toISOString(),
  };

  return JSON.stringify(exportPayload, null, 2);
}

/**
 * Parses and validates an imported theme JSON string.
 */
export function importThemeJson(jsonStr: string): { data: ThemeExportData | null; error: string | null } {
  try {
    const parsed = JSON.parse(jsonStr);
    if (!parsed || typeof parsed !== 'object') {
      return { data: null, error: 'Geçersiz JSON formatı.' };
    }

    const customCss = typeof parsed.customCss === 'string' ? parsed.customCss : '';
    const sanitized = sanitizeCss(customCss);

    const themeData: ThemeExportData = {
      version: typeof parsed.version === 'number' ? parsed.version : 1,
      id: typeof parsed.id === 'string' ? parsed.id : `imported-${Date.now()}`,
      name: typeof parsed.name === 'string' ? parsed.name.slice(0, 50) : 'İçe Aktarılan Tema',
      presetThemeId: typeof parsed.presetThemeId === 'string' ? parsed.presetThemeId : 'veil-origin',
      customCss: sanitized.safe,
      customCssEnabled: parsed.customCssEnabled !== false,
      customBgImage: typeof parsed.customBgImage === 'string' ? parsed.customBgImage : '',
      customBgVideo: typeof parsed.customBgVideo === 'string' ? parsed.customBgVideo : '',
      customBgOpacity: typeof parsed.customBgOpacity === 'number' ? Math.max(0, Math.min(0.6, parsed.customBgOpacity)) : 0.26,
      accentColor: typeof parsed.accentColor === 'string' ? parsed.accentColor : null,
    };

    return { data: themeData, error: null };
  } catch (err) {
    return { data: null, error: 'JSON dosyası okunamadı veya bozuk: ' + String(err) };
  }
}

export function generateThemeId(): string {
  return `theme-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function saveNamedTheme(theme: SavedTheme): void {
  const themes = getSavedThemes();
  const existing = themes.findIndex(t => t.id === theme.id);
  if (existing >= 0) {
    themes[existing] = theme;
  } else {
    themes.push(theme);
  }
  try {
    localStorage.setItem(SAVED_THEMES_KEY, JSON.stringify(themes));
  } catch { /* quota exceeded */ }
}

export function getSavedThemes(): SavedTheme[] {
  try {
    const raw = localStorage.getItem(SAVED_THEMES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed as SavedTheme[];
  } catch { /* corrupt data */ }
  return [];
}

export function deleteSavedTheme(id: string): void {
  const themes = getSavedThemes().filter(t => t.id !== id);
  try {
    localStorage.setItem(SAVED_THEMES_KEY, JSON.stringify(themes));
  } catch { /* quota exceeded */ }
}

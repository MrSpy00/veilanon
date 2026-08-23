/**
 * veilanon — SAFE markdown renderer.
 *
 * SECURITY CONTRACT:
 *  - Never renders raw HTML from message content. Every token of user text is escaped.
 *  - Only our own spans/links are emitted; hrefs are restricted to http:/https:.
 *  - Spoilers toggle via CSS class (`.revealed`) — content stays hidden until clicked.
 *  - Render with `{@html renderMarkdown(content)}` — safe because all user text is escaped.
 */

import { trustedDomainsStore } from '$lib/stores/trustedDomains';

const ESCAPE_RE = /[&<>"']/g;
const ESCAPE_MAP: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

export function escapeHtml(text: string): string {
  return text.replace(ESCAPE_RE, c => ESCAPE_MAP[c]);
}

/**
 * Restrict links to http/https + bare domains; anything else renders as plain text.
 * Universal domain detection — covers ALL TLDs worldwide (ccTLD, gTLD, new gTLD, multi-level).
 */
function sanitizeUrl(url: string): string | null {
  const trimmed = url.trim();
  if (/^https?:\/\//i.test(trimmed)) return trimmed;
  if (/^www\./i.test(trimmed)) return `https://${trimmed}`;
  // Bare domain pattern: e.g. "veilanon.com", "example.co.uk", "site.com.tr"
  // Matches any valid hostname with at least one dot and a TLD (2-63 chars for new gTLDs)
  if (/^[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?)*\.[a-z]{2,63}(?:\/[^\s<>"'()\\\]]*)?$/i.test(trimmed)) return `https://${trimmed}`;
  return null;
}

interface Token {
  type: 'text' | 'bold' | 'italic' | 'inline_code' | 'link' | 'image' | 'spoiler' | 'mention';
  text: string;
  href?: string;
  alt?: string;
}

/**
 * Tokenize message content. Line-based for code blocks; inline regex for the rest.
 * Auto-links raw URLs and markdown-formatted links.
 */
function tokenize(content: string): Token[] {
  const tokens: Token[] = [];
  // Split code blocks first so their contents are never re-processed.
  const parts = content.split(/```([\s\S]*?)```/g);
  parts.forEach((part, idx) => {
    if (idx % 2 === 1) {
      tokens.push({ type: 'inline_code', text: part });
      return;
    }
    // Inline processing: markdown + raw URLs + bare domains (universal TLD detection)
    const inlineRe =
      /(\*\*([^*]+)\*\*)|(\*([^*]+)\*)|(`([^`]+)`)|(\|\|([^|]+)\|\|)|(@[A-Za-z0-9_]{2,32})|(!\[([^\]]*)\]\(([^)]+)\))|(\[([^\]]+)\]\(([^)]+)\))|(https?:\/\/[^\s<>"'()]+|www\.[^\s<>"'()]+|[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9\-]*[a-z0-9])?)*\.[a-z]{2,63}(?:\/[^\s<>"'()]+)?)/gi;
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = inlineRe.exec(part)) !== null) {
      if (m.index > last) {
        tokens.push({ type: 'text', text: part.slice(last, m.index) });
      }
      if (m[2] !== undefined) tokens.push({ type: 'bold', text: m[2] });
      else if (m[4] !== undefined) tokens.push({ type: 'italic', text: m[4] });
      else if (m[6] !== undefined) tokens.push({ type: 'inline_code', text: m[6] });
      else if (m[8] !== undefined) tokens.push({ type: 'spoiler', text: m[8] });
      else if (m[9] !== undefined) tokens.push({ type: 'mention', text: m[9] });
      else if (m[11] !== undefined) tokens.push({ type: 'image', text: m[10] ?? '', href: m[12], alt: m[11] });
      else if (m[14] !== undefined) tokens.push({ type: 'link', text: m[14], href: m[15] });
      else if (m[16] !== undefined) {
        const raw = m[16];
        const href = /^https?:\/\//i.test(raw) ? raw : `https://${raw}`;
        tokens.push({ type: 'link', text: raw, href });
      }
      last = m.index + m[0].length;
    }
    if (last < part.length) {
      tokens.push({ type: 'text', text: part.slice(last) });
    }
  });
  return tokens;
}

function renderToken(token: Token): string {
  const text = escapeHtml(token.text);
  switch (token.type) {
    case 'bold':
      return `<strong>${text}</strong>`;
    case 'italic':
      return `<em>${text}</em>`;
    case 'inline_code': {
      // Code block vs inline: multi-line → block container.
      if (token.text.includes('\n')) {
        return `<pre class="veil-code-block">${escapeHtml(token.text.trim())}</pre>`;
      }
      return `<code class="veil-code">${text}</code>`;
    }
    case 'link': {
      const href = sanitizeUrl(token.href ?? '');
      if (!href) return text;
      const isTrusted = trustedDomainsStore.isTrusted(href);
      return `<a href="${escapeHtml(href)}" class="veil-markdown-link ${isTrusted ? 'trusted' : 'external'}" data-external-url="${escapeHtml(href)}" target="_blank" rel="noreferrer noopener">${text}</a>`;
    }
    case 'image': {
      const href = sanitizeUrl(token.href ?? '');
      if (!href) return escapeHtml(token.alt ?? '');
      const alt = escapeHtml(token.alt ?? 'gif');
      return `<img class="veil-chat-image" src="${escapeHtml(href)}" alt="${alt}" loading="lazy" referrerpolicy="no-referrer">`;
    }
    case 'spoiler':
      return `<span class="veil-spoiler" tabindex="0" data-spoiler>${text}</span>`;
    case 'mention':
      return `<span class="veil-mention">${text}</span>`;
    case 'text':
      return text;
  }
}

/**
 * Render markdown-ish content to SAFE HTML (all user text escaped).
 * Single \n -> <br>, double \n\n -> paragraph gap, trailing/leading blank lines trimmed.
 */
export function renderMarkdown(content: string): string {
  if (!content) return '';
  let normalized = content.replace(/\r\n/g, '\n').replace(/\n{3,}/g, '\n\n').trim();
  if (!normalized) return '';
  const tokens = tokenize(normalized);
  const html = tokens.map(renderToken).join('');
  return html.replace(/\n/g, '<br>');
}

/**
 * Click-to-reveal for spoilers. Attach ONCE to a container (event delegation).
 * Returns a cleanup function.
 */
export function setupSpoilerReveal(container: HTMLElement): () => void {
  function onClick(e: Event) {
    const target = e.target as HTMLElement | null;
    const spoiler = target?.closest?.('.veil-spoiler') as HTMLElement | null;
    if (!spoiler || !container.contains(spoiler)) return;
    if (spoiler.classList.contains('revealed')) {
      spoiler.classList.remove('revealed');
    } else {
      spoiler.classList.add('revealed');
    }
  }
  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    const target = e.target as HTMLElement | null;
    const spoiler = target?.closest?.('.veil-spoiler') as HTMLElement | null;
    if (!spoiler || !container.contains(spoiler)) return;
    e.preventDefault();
    spoiler.classList.toggle('revealed');
  }
  container.addEventListener('click', onClick);
  container.addEventListener('keydown', onKeydown);
  return () => {
    container.removeEventListener('click', onClick);
    container.removeEventListener('keydown', onKeydown);
  };
}

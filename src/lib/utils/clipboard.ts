/**
 * veilanon — Universal Clipboard Utility
 *
 * Provides resilient clipboard copy and paste operations across all OS platforms
 * (Windows, macOS, Linux) and WebView security contexts.
 */

/**
 * Robust text copy: tries navigator.clipboard first, falls back to document.execCommand('copy').
 */
export async function copyText(text: string): Promise<boolean> {
  if (!text) return false;

  // 1. Try modern Async Clipboard API
  if (typeof navigator !== 'undefined' && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      // Fallback below
    }
  }

  // 2. Legacy fallback: textarea selection + execCommand
  if (typeof document !== 'undefined') {
    try {
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.left = '-999999px';
      textarea.style.top = '-999999px';
      textarea.setAttribute('readonly', '');
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      textarea.setSelectionRange(0, text.length);
      const success = document.execCommand('copy');
      document.body.removeChild(textarea);
      if (success) return true;
    } catch {
      // Failed
    }
  }

  return false;
}

/**
 * Extracts image file/blob from a ClipboardEvent (Ctrl+V / Paste)
 */
export function extractImageFromClipboard(event: ClipboardEvent): File | null {
  const items = event.clipboardData?.items;
  if (!items) return null;

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile();
      if (file) return file;
    }
  }

  const files = event.clipboardData?.files;
  if (files && files.length > 0) {
    for (let i = 0; i < files.length; i++) {
      if (files[i].type.startsWith('image/')) {
        return files[i];
      }
    }
  }

  return null;
}

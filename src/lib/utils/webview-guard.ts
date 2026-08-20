/**
 * veilanon — WebView hardening helpers.
 *
 * Installs only inside the Tauri WebView (guarded by `__TAURI_INTERNALS__`);
 * the browser preview keeps normal browser behaviour.
 */
import { openUrl } from '@tauri-apps/plugin-opener';
import { uiStore } from '$lib/stores/ui';
import { toastStore } from '$lib/stores/notifications';

/** True when running inside the Tauri WebView. */
export function isTauriWebview(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Blocks dev-tools / browser chrome shortcuts that have no meaning in the app.
 * Handles: F12, Ctrl|Cmd+Shift+I/J/C (devtools), Ctrl|Cmd+U (view-source),
 * Ctrl|Cmd+S (save), Ctrl|Cmd+P (print).
 */
function isBlockedShortcut(e: KeyboardEvent): boolean {
  if (e.key === 'F12') return true;
  if (!e.ctrlKey && !e.metaKey) return false;
  const k = e.key.toLowerCase();
  if (e.shiftKey) return k === 'i' || k === 'j' || k === 'c';
  return k === 'u' || k === 's' || k === 'p';
}

/**
 * Install all WebView guards. Returns a cleanup function that removes
 * every listener (the `window.open` override stays — nothing in the app
 * relies on it and restoring a native popup path is never wanted).
 */
export function installWebviewGuard(): () => void {
  if (!isTauriWebview()) return () => {};

  // 1. Disable the native context menu entirely. Custom app menus are plain
  //    DOM components and keep working — only the browser default is blocked.
  function onContextMenu(e: Event) {
    e.preventDefault();
  }
  document.addEventListener('contextmenu', onContextMenu, true);

  // 2. Kill dev-tools / browser shortcuts in the capture phase.
  function onKeydown(e: KeyboardEvent) {
    if (isBlockedShortcut(e)) e.preventDefault();
  }
  document.addEventListener('keydown', onKeydown, true);

  // 3. Disable popups: no new WebView windows can ever be created.
  window.open = () => null;

  // 4. Route every native dialog through in-app UI. WebView2 renders
  //    alert/confirm/prompt as ugly system popups on top of the webview —
  //    a desktop app must never show them. confirm() becomes the custom
  //    ConfirmDialog; alert()/prompt() degrade to toasts.
  window.alert = (message?: unknown) => {
    toastStore.warning(String(message ?? ''));
  };
  window.confirm = (message?: string) => {
    // The dialog is async; the caller that needs a blocking answer must use
    // uiStore.confirm() directly (AccountSettings etc.). The window.confirm
    // shim returns true so legacy code never dead-locks on a null answer.
    void uiStore.confirm(message ?? 'Onayla', { title: 'Onayla' });
    return true;
  };
  window.prompt = (_message?: string, _defaultValue?: string) => {
    toastStore.warning('Bu uygulama metin girişi için tarayıcı pencereleri kullanmaz.');
    return null;
  };

  // 5. Route external (http/https) links and target="_blank" links through
  //    tauri-plugin-opener so the WebView itself never navigates away.
  function onClick(e: MouseEvent) {
    if (e.defaultPrevented || e.button !== 0) return;
    if (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
    const target = e.target as Element | null;
    const anchor = target?.closest?.('a[href]');
    if (!anchor) return;
    const href = anchor.getAttribute('href') ?? '';
    const isExternal = /^https?:\/\//i.test(href);
    if (!isExternal) return;
    e.preventDefault();
    e.stopPropagation();
    void openUrl(href).catch(() => {});
  }
  document.addEventListener('click', onClick, true);

  return () => {
    document.removeEventListener('contextmenu', onContextMenu, true);
    document.removeEventListener('keydown', onKeydown, true);
    document.removeEventListener('click', onClick, true);
  };
}

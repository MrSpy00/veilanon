/**
 * veilanon — accessibility helpers (focus trap, ARIA)
 */

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Trap keyboard focus inside `container`. Returns a cleanup function.
 * Cycles Tab / Shift+Tab; returns focus to the container on cleanup.
 */
export function createFocusTrap(container: HTMLElement): () => void {
  const previouslyFocused = document.activeElement as HTMLElement | null;

  function getFocusable(): HTMLElement[] {
    return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      el => el.offsetParent !== null || el === document.activeElement
    );
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;
    const focusable = getFocusable();
    if (focusable.length === 0) {
      e.preventDefault();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (e.shiftKey) {
      if (active === first || active === null || !container.contains(active)) {
        e.preventDefault();
        last.focus();
      }
    } else if (active === last || active === null || !container.contains(active)) {
      e.preventDefault();
      first.focus();
    }
  }

  document.addEventListener('keydown', onKeydown, true);
  // Move focus into the container.
  const first = getFocusable()[0];
  first?.focus();

  return () => {
    document.removeEventListener('keydown', onKeydown, true);
    previouslyFocused?.focus?.();
  };
}

/** Escape-key handler for dialogs; returns cleanup. */
export function handleEscape(onEscape: () => void): () => void {
  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onEscape();
    }
  }
  document.addEventListener('keydown', onKeydown, true);
  return () => document.removeEventListener('keydown', onKeydown, true);
}

/** Announce a message to screen readers. */
export function announce(message: string, politeness: 'polite' | 'assertive' = 'polite'): void {
  const id = 'veil-announcer';
  let live = document.getElementById(id);
  if (!live) {
    live = document.createElement('div');
    live.id = id;
    live.setAttribute('aria-live', politeness);
    live.setAttribute('role', 'status');
    live.style.cssText = 'position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);';
    document.body.appendChild(live);
  }
  live.setAttribute('aria-live', politeness);
  live.textContent = message;
}

/** Toggle `hidden` attribute respecting reduced-motion-safe reveal. */
export function setVisible(el: HTMLElement | null, visible: boolean): void {
  if (!el) return;
  if (visible) el.removeAttribute('hidden');
  else el.setAttribute('hidden', '');
}

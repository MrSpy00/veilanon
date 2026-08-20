<script lang="ts">
  /**
   * UserStatusMenu — profile trigger + presence dropdown.
   * Shows avatar, display name and current status dot; opens a menu to pick
   * presence (online / away / dnd / invisible) and set a custom status.
   * Focus-trapped, Esc / outside-click close, reduced-motion aware.
   */
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { identityApi, presenceApi, socialApi, type PresenceStatus } from '$lib/api/tauri';
  import { authStore } from '$lib/stores/auth';
  import { uiStore } from '$lib/stores/ui';
  import { toastStore } from '$lib/stores/notifications';
  import { createFocusTrap, handleEscape, announce } from '$lib/utils/accessibility';
  import Avatar from './Avatar.svelte';
  import Icon, { type IconName } from './Icon.svelte';

  const STATUS_LABELS: Record<PresenceStatus, string> = {
    online: 'Çevrimiçi',
    away: 'Boşta',
    dnd: 'Rahatsız Etme',
    offline: 'Çevrimdışı',
    invisible: 'Görünmez',
  };

  const STATUS_OPTIONS = [
    { value: 'online', label: 'Çevrimiçi' },
    { value: 'away', label: 'Boşta' },
    { value: 'dnd', label: 'Rahatsız Etme' },
    { value: 'invisible', label: 'Görünmez' },
  ] as const;

  type StatusValue = (typeof STATUS_OPTIONS)[number]['value'];

  let {
    name = 'veilanon',
    username = '',
    avatarHash = null,
    presence = 'online',
    placement = 'up',
    showLabel = true,
    actions = [],
    class: className = '',
  }: {
    name?: string;
    username?: string;
    avatarHash?: string | null;
    presence?: PresenceStatus;
    placement?: 'up' | 'down' | 'right';
    showLabel?: boolean;
    /** Extra menu actions rendered below the status list (settings, mic…). */
    actions?: Array<{ icon: IconName; label: string; danger?: boolean; onClick: () => void }>;
    class?: string;
  } = $props();

  const uid = `veil-status-menu-${Math.random().toString(36).slice(2, 9)}`;

  // svelte-ignore state_referenced_locally
  let current = $state<PresenceStatus>(presence);
  $effect(() => {
    current = presence;
  });
  let open = $state(false);
  let customStatus = $state('');
  let draft = $state('');
  let reduced = $state(false);

  $effect(() => {
    const id = $authStore.identity?.id;
    if (id) {
      socialApi.getUserProfile(id).then((p) => {
        if (p?.customStatus) {
          customStatus = p.customStatus;
          draft = p.customStatus;
        }
      }).catch(() => {});
    }
  });

  let rootEl = $state<HTMLDivElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);
  let trapCleanup: (() => void) | null = null;

  $effect(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reduced = mq.matches;
    const onChange = (e: MediaQueryListEvent) => (reduced = e.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  // Outside click + Esc while open
  $effect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootEl && !rootEl.contains(e.target as Node)) close();
    };
    document.addEventListener('mousedown', onDown);
    const escCleanup = handleEscape(close);
    return () => {
      document.removeEventListener('mousedown', onDown);
      escCleanup();
    };
  });

  // Focus trap + viewport clamping while open
  $effect(() => {
    if (!open) return;
    let cancelled = false;
    const onResize = () => positionMenu();
    window.addEventListener('resize', onResize);
    tick().then(() => {
      if (cancelled) return;
      positionMenu();
      if (menuEl) trapCleanup = createFocusTrap(menuEl);
    });
    return () => {
      cancelled = true;
      window.removeEventListener('resize', onResize);
      trapCleanup?.();
      trapCleanup = null;
    };
  });

  // Cleanup on unmount
  $effect(() => () => {
    trapCleanup?.();
  });

  function toggle() {
    if (open) close();
    else openMenu();
  }

  async function openMenu() {
    open = true;
  }

  function close() {
    if (!open) return;
    open = false;
    triggerEl?.focus();
  }

  /** Anchor the fixed-position menu next to the trigger, clamped to the viewport. */
  function positionMenu() {
    if (!menuEl || !triggerEl) return;
    const r = triggerEl.getBoundingClientRect();
    const m = menuEl.getBoundingClientRect();
    const gap = 8;
    const pad = 8;
    let top: number;
    let left: number;

    if (placement === 'up') {
      top = r.top - m.height - gap;
      left = Math.min(r.left, window.innerWidth - m.width - pad);
    } else if (placement === 'down') {
      top = r.bottom + gap;
      left = Math.min(r.left, window.innerWidth - m.width - pad);
    } else {
      // right
      top = Math.min(r.top, window.innerHeight - m.height - pad);
      left = r.right + gap;
      if (left + m.width > window.innerWidth - pad) {
        left = Math.max(pad, r.left - m.width - gap);
      }
    }
    menuEl.style.top = `${Math.max(pad, top)}px`;
    menuEl.style.left = `${Math.max(pad, left)}px`;
  }

  function selectStatus(value: StatusValue) {
    current = value;
    uiStore.setPresence(value);
    presenceApi.update(value).catch(() => {});
    announce(`${STATUS_LABELS[value]} durumu ayarlandı`);
    close();
  }

  function applyCustomStatus() {
    const text = draft.trim();
    customStatus = text;
    const identity = $authStore.identity;
    if (identity) {
      identityApi.updateProfile({
        displayName: identity.displayName || name,
        customStatus: text,
      }).then(() => {
        authStore.initialize();
        presenceApi.update(current).catch(() => {});
        if (text) {
          toastStore.success(`Durum güncellendi: "${text}"`);
          announce(`Durum ayarlandı: ${text}`);
        } else {
          toastStore.info('Özel durum temizlendi.');
        }
      }).catch(() => {});
    }
    close();
  }

  function clearCustomStatus() {
    draft = '';
    applyCustomStatus();
  }

  function onMenuKeydown(e: KeyboardEvent) {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const items = Array.from(
      menuEl?.querySelectorAll<HTMLElement>('[role="menuitemradio"]') ?? []
    );
    if (items.length === 0) return;
    const idx = items.indexOf(document.activeElement as HTMLElement);
    const next =
      e.key === 'ArrowDown'
        ? (idx + 1) % items.length
        : (idx - 1 + items.length) % items.length;
    items[next]?.focus();
  }
</script>

<div
  class="veil-status-menu-wrap veil-status-menu-{placement} {className}"
  bind:this={rootEl}
>
  <button
    class="veil-status-trigger"
    type="button"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={uid}
    bind:this={triggerEl}
    onclick={toggle}
  >
    <Avatar name={name} hash={avatarHash} presence={current} size={showLabel ? 'md' : 'lg'} />
    {#if showLabel}
      <span class="veil-status-trigger-label">
        <span class="veil-user-name">{name}</span>
        <span class="veil-status-trigger-sub" title={customStatus || STATUS_LABELS[current]}>
          {#if customStatus}
            <span class="veil-custom-status-text">{customStatus}</span>
          {:else}
            {STATUS_LABELS[current]}
          {/if}
        </span>
      </span>
    {/if}
  </button>

  {#if open}
    <div
      id={uid}
      class="veil-status-menu"
      role="menu"
      aria-label="Durum"
      tabindex="-1"
      bind:this={menuEl}
      onkeydown={onMenuKeydown}
      transition:fade={{ duration: reduced ? 0 : 150 }}
    >
      <div class="veil-status-header">
        <Avatar name={name} hash={avatarHash} presence={current} size="lg" />
        <div class="veil-status-header-info">
          <div class="veil-status-header-name">{name}</div>
          {#if username}
            <div class="veil-status-header-tag">@{username}</div>
          {/if}
          <div class="veil-status-header-current">
            {#if customStatus}
              {customStatus}
            {:else}
              {STATUS_LABELS[current] ?? 'Çevrimdışı'}
            {/if}
          </div>
        </div>
      </div>

      {#each STATUS_OPTIONS as opt (opt.value)}
        <button
          class="veil-status-option"
          class:active={current === opt.value}
          type="button"
          role="menuitemradio"
          aria-checked={current === opt.value}
          onclick={() => selectStatus(opt.value)}
        >
          <span class="veil-status-dot {opt.value}" aria-hidden="true"></span>
          <span class="veil-status-option-label">{opt.label}</span>
          {#if current === opt.value}
            <Icon name="check" size={16} class="veil-status-check" />
          {/if}
        </button>
      {/each}

      <div class="veil-status-divider" role="separator"></div>

      <div class="veil-status-custom">
        <input
          class="veil-status-input"
          type="text"
          maxlength="64"
          placeholder="Durum ayarla"
          aria-label="Durum ayarla"
          bind:value={draft}
          onkeydown={(e) => {
            if (e.key === 'Enter') applyCustomStatus();
          }}
        />
        {#if draft.trim()}
          <button
            class="btn-icon"
            type="button"
            title="Durumu temizle"
            aria-label="Durumu temizle"
            onclick={clearCustomStatus}
          >
            <Icon name="x" size={14} />
          </button>
        {/if}
        <button
          class="btn-icon"
          type="button"
          title="Durumu kaydet"
          aria-label="Durumu kaydet"
          disabled={!draft.trim() && !customStatus}
          onclick={applyCustomStatus}
        >
          <Icon name="check" size={16} />
        </button>
      </div>

      {#if actions.length > 0}
        <div class="veil-status-divider" role="separator"></div>
        {#each actions as action (action.label)}
          <button
            class="veil-status-option"
            class:danger={action.danger}
            type="button"
            onclick={() => { close(); action.onClick(); }}
          >
            <Icon name={action.icon} size={16} class="veil-status-action-icon" />
            <span class="veil-status-option-label">{action.label}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

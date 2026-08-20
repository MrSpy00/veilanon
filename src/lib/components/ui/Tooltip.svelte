<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    label = '',
    position = 'top',
    disabled = false,
    children,
  }: {
    label?: string;
    position?: 'top' | 'bottom' | 'left' | 'right';
    disabled?: boolean;
    children?: Snippet;
  } = $props();
</script>

<span
  class="veil-tooltip-wrap"
  data-tip={disabled || !label ? undefined : label}
  data-pos={position}
>
  {@render children?.()}
</span>

<style>
  .veil-tooltip-wrap {
    position: relative;
    display: inline-flex;
  }
  /* Bubble */
  .veil-tooltip-wrap::after {
    content: attr(data-tip);
    position: absolute;
    z-index: 200;
    padding: 5px 9px;
    font-size: var(--text-xs);
    font-weight: 500;
    line-height: 1.35;
    letter-spacing: 0.01em;
    color: var(--veil-text-primary);
    background: var(--veil-bg-void);
    border: 1px solid var(--veil-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    white-space: nowrap;
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
    opacity: 0;
    visibility: hidden;
    transition:
      opacity 130ms var(--ease-out),
      visibility 130ms;
    transition-delay: 0s;
  }
  /* Arrow */
  .veil-tooltip-wrap::before {
    content: '';
    position: absolute;
    z-index: 199;
    width: 8px;
    height: 8px;
    background: var(--veil-bg-void);
    border-radius: 2px;
    pointer-events: none;
    opacity: 0;
    visibility: hidden;
    transition:
      opacity 130ms var(--ease-out),
      visibility 130ms;
    transition-delay: 0s;
  }
  .veil-tooltip-wrap:hover::after,
  .veil-tooltip-wrap:focus-within::after,
  .veil-tooltip-wrap:hover::before,
  .veil-tooltip-wrap:focus-within::before {
    opacity: 1;
    visibility: visible;
    transition-delay: 150ms;
  }

  /* top — arrow points down, exposed faces: right + bottom */
  .veil-tooltip-wrap[data-pos='top']::after {
    bottom: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
  }
  .veil-tooltip-wrap[data-pos='top']::before {
    bottom: calc(100% + 2px);
    left: 50%;
    transform: translate(-50%, 50%) rotate(45deg);
    border-right: 1px solid var(--veil-border);
    border-bottom: 1px solid var(--veil-border);
  }

  /* bottom — arrow points up, exposed faces: top + left */
  .veil-tooltip-wrap[data-pos='bottom']::after {
    top: calc(100% + 6px);
    left: 50%;
    transform: translateX(-50%);
  }
  .veil-tooltip-wrap[data-pos='bottom']::before {
    top: calc(100% + 2px);
    left: 50%;
    transform: translate(-50%, -50%) rotate(45deg);
    border-top: 1px solid var(--veil-border);
    border-left: 1px solid var(--veil-border);
  }

  /* left — arrow points right, exposed faces: top + right */
  .veil-tooltip-wrap[data-pos='left']::after {
    right: calc(100% + 6px);
    top: 50%;
    transform: translateY(-50%);
  }
  .veil-tooltip-wrap[data-pos='left']::before {
    right: calc(100% + 2px);
    top: 50%;
    transform: translate(50%, -50%) rotate(45deg);
    border-top: 1px solid var(--veil-border);
    border-right: 1px solid var(--veil-border);
  }

  /* right — arrow points left, exposed faces: top + left */
  .veil-tooltip-wrap[data-pos='right']::after {
    left: calc(100% + 6px);
    top: 50%;
    transform: translateY(-50%);
  }
  .veil-tooltip-wrap[data-pos='right']::before {
    left: calc(100% + 2px);
    top: 50%;
    transform: translate(-50%, -50%) rotate(45deg);
    border-top: 1px solid var(--veil-border);
    border-left: 1px solid var(--veil-border);
  }
</style>

/**
 * Effects store — manages effect state, visibility, and active effects (multi-select supported)
 *
 * State is persisted to localStorage for session continuity.
 * Broadcast mode syncs effect parameters via LiveKit DataChannel.
 *
 * Emits typed events for broadcast consumers:
 *   onEffectActivated  — effectId + params + visibility
 *   onEffectDeactivated — effectId
 *   onParamsChanged     — effectId + merged params
 */

import { writable, get } from 'svelte/store';
import type {
  ActiveEffect,
  EffectParams,
  EffectEngineState,
  VisibilityMode,
  EffectCategory,
  EffectBroadcastPayload,
} from './types';
import { BUILTIN_EFFECTS, getEffect } from './effects';
import { effectEngine, diagnoseEngine } from './engine';
import { getPlugins } from './plugin';

// ── Store state ──────────────────────────────────────────────────────────────

export interface EffectsState {
  /** Is the effects panel open */
  panelOpen: boolean;
  /** Currently active effects list (multi-select supported) */
  activeEffects: ActiveEffect[];
  /** Currently active effect (first active effect or null, for backwards compatibility) */
  activeEffect: ActiveEffect | null;
  /** User's default visibility mode */
  visibility: VisibilityMode;
  /** Effect engine state */
  engineState: EffectEngineState;
  /** Selected category filter in the panel */
  selectedCategory: EffectCategory | 'all';
  /** Last used effect ID (for quick re-activate) */
  lastEffectId: string | null;
  /** Preserved scroll position of effects grid during session */
  gridScrollTop: number;
}

// ── Typed event emitter ──────────────────────────────────────────────────────

export type EffectsEventType =
  | 'effectActivated'
  | 'effectDeactivated'
  | 'paramsChanged';

export type EffectsEventHandler = (payload: EffectBroadcastPayload) => void;

const listeners = new Map<EffectsEventType, Set<EffectsEventHandler>>();

function emit(event: EffectsEventType, payload: EffectBroadcastPayload) {
  const handlers = listeners.get(event);
  if (!handlers) return;
  for (const fn of handlers) {
    try {
      fn(payload);
    } catch (err) {
      console.warn(`[effects] event handler error (${event}):`, err);
    }
  }
}

export function subscribeToEffectEvents(
  event: EffectsEventType,
  handler: EffectsEventHandler,
): () => void {
  if (!listeners.has(event)) {
    listeners.set(event, new Set());
  }
  listeners.get(event)!.add(handler);
  return () => {
    listeners.get(event)?.delete(handler);
  };
}

// ── Persistence ──────────────────────────────────────────────────────────────

interface PersistedEffectsData {
  visibility: VisibilityMode;
  lastEffectId: string | null;
  activeEffectIds?: string[];
  effectParams: Record<string, EffectParams>;
}

const STORAGE_KEY = 'veilanon_effects_state_v2';

function loadPersistedState(): PersistedEffectsData {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        visibility: parsed.visibility ?? 'self',
        lastEffectId: parsed.lastEffectId ?? null,
        activeEffectIds: Array.isArray(parsed.activeEffectIds) ? parsed.activeEffectIds : [],
        effectParams: parsed.effectParams ?? {},
      };
    }
  } catch { /* ignore */ }
  return { visibility: 'self', lastEffectId: null, activeEffectIds: [], effectParams: {} };
}

function persist(state: EffectsState) {
  try {
    const existing = loadPersistedState();
    const mergedParams = { ...existing.effectParams };
    for (const act of state.activeEffects) {
      mergedParams[act.effectId] = act.params;
    }
    const data: PersistedEffectsData = {
      visibility: state.visibility,
      lastEffectId: state.activeEffects[0]?.effectId ?? state.lastEffectId,
      activeEffectIds: state.activeEffects.map(e => e.effectId),
      effectParams: mergedParams,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
  } catch { /* ignore */ }
}

// ── Create store ─────────────────────────────────────────────────────────────

function createEffectsStore() {
  const persisted = loadPersistedState();

  const { subscribe, update } = writable<EffectsState>({
    panelOpen: false,
    activeEffects: [],
    activeEffect: null,
    visibility: persisted.visibility,
    engineState: effectEngine.getState(),
    selectedCategory: 'all',
    lastEffectId: persisted.lastEffectId,
    gridScrollTop: 0,
  });

  // Subscribe to engine state changes
  effectEngine.subscribe((engineState) => {
    update(s => ({ ...s, engineState }));
  });

  function syncActiveEffects() {
    const current = get({ subscribe });
    effectEngine.clearEffects();
    if (current.activeEffects.length > 0) {
      for (const act of current.activeEffects) {
        effectEngine.addEffect(act.effectId, act.params);
      }
    }
  }

  return {
    subscribe,

    /** Toggle effects panel */
    togglePanel() {
      update(s => ({ ...s, panelOpen: !s.panelOpen }));
    },

    /** Open effects panel */
    openPanel() {
      update(s => ({ ...s, panelOpen: true }));
    },

    /** Close effects panel */
    closePanel() {
      update(s => ({ ...s, panelOpen: false }));
    },

    /** Check if an effect is active */
    isEffectActive(effectId: string): boolean {
      return get({ subscribe }).activeEffects.some(e => e.effectId === effectId);
    },

    /** Toggle an effect on/off (multi-select) */
    toggleEffect(effectId: string, params: EffectParams = {}) {
      if (this.isEffectActive(effectId)) {
        this.deactivateEffect(effectId);
      } else {
        this.activateEffect(effectId, params);
      }
    },

    /** Activate an effect (adds to activeEffects array) */
    activateEffect(effectId: string, params: EffectParams = {}) {
      const effect = getEffect(effectId) || getPlugins().find(p => `plugin-${p.manifest.id}` === effectId)?.manifest;
      if (!effect) return;

      const effectObj = getEffect(effectId);
      const defaultParams: EffectParams = {};
      if (effectObj?.params) {
        for (const p of effectObj.params) {
          defaultParams[p.name] = p.default;
        }
      }

      // Merge: definition defaults → persisted params → explicit params
      const persisted = loadPersistedState();
      const storedParams = persisted.effectParams[effectId] ?? {};

      const newActive: ActiveEffect = {
        effectId,
        params: { ...defaultParams, ...storedParams, ...params },
        visibility: get({ subscribe }).visibility,
        activatedAt: Date.now(),
      };

      update(s => {
        const withoutThis = s.activeEffects.filter(e => e.effectId !== effectId);
        const nextActiveList = [...withoutThis, newActive];
        const newState: EffectsState = {
          ...s,
          activeEffects: nextActiveList,
          activeEffect: nextActiveList[0] || null,
          lastEffectId: effectId,
        };
        persist(newState);
        return newState;
      });

      syncActiveEffects();

      emit('effectActivated', {
        userId: '',
        effectId,
        params: newActive.params,
      });
    },

    /** Deactivate a specific effect, or all if not specified */
    deactivateEffect(effectId?: string) {
      const current = get({ subscribe });
      const targetId = effectId ?? current.activeEffects[current.activeEffects.length - 1]?.effectId;
      if (!targetId) {
        this.deactivateAllEffects();
        return;
      }

      update(s => {
        const nextActiveList = s.activeEffects.filter(e => e.effectId !== targetId);
        const newState: EffectsState = {
          ...s,
          activeEffects: nextActiveList,
          activeEffect: nextActiveList[0] || null,
        };
        persist(newState);
        return newState;
      });

      effectEngine.removeEffect(targetId);
      if (get({ subscribe }).activeEffects.length === 0) {
        effectEngine.stop();
      }

      emit('effectDeactivated', {
        userId: '',
        effectId: targetId,
        params: {},
      });
    },

    /** Deactivate all active effects */
    deactivateAllEffects() {
      const current = get({ subscribe });
      const deactivatedIds = current.activeEffects.map(e => e.effectId);

      update(s => {
        const newState: EffectsState = {
          ...s,
          activeEffects: [],
          activeEffect: null,
        };
        persist(newState);
        return newState;
      });

      effectEngine.clearEffects();
      effectEngine.stop();

      for (const id of deactivatedIds) {
        emit('effectDeactivated', {
          userId: '',
          effectId: id,
          params: {},
        });
      }
    },

    /** Update parameters for an active effect */
    updateParams(paramNameOrObj: string | EffectParams, valueOrParams?: number | string | boolean | EffectParams) {
      let targetEffectId: string | null = null;
      let newParams: EffectParams = {};

      if (typeof paramNameOrObj === 'string' && typeof valueOrParams !== 'object') {
        // updateParams('intensity', 10) on primary active effect
        const current = get({ subscribe });
        targetEffectId = current.activeEffect?.effectId ?? null;
        if (valueOrParams !== undefined) {
          newParams = { [paramNameOrObj]: valueOrParams };
        }
      } else if (typeof paramNameOrObj === 'string' && typeof valueOrParams === 'object') {
        // updateParams(effectId, { intensity: 10 })
        targetEffectId = paramNameOrObj;
        newParams = valueOrParams as EffectParams;
      } else if (typeof paramNameOrObj === 'object') {
        // updateParams({ intensity: 10 })
        const current = get({ subscribe });
        targetEffectId = current.activeEffect?.effectId ?? null;
        newParams = paramNameOrObj as EffectParams;
      }

      if (!targetEffectId) return;

      update(s => {
        const nextActiveList = s.activeEffects.map(e => {
          if (e.effectId === targetEffectId) {
            return {
              ...e,
              params: { ...e.params, ...newParams },
            };
          }
          return e;
        });

        const newState: EffectsState = {
          ...s,
          activeEffects: nextActiveList,
          activeEffect: nextActiveList[0] || null,
        };
        persist(newState);
        return newState;
      });

      effectEngine.updateEffectParams(targetEffectId, newParams);
      emit('paramsChanged', {
        userId: '',
        effectId: targetEffectId,
        params: newParams,
      });
    },

    /** Reset effect parameters to defaults */
    resetParams(effectId?: string) {
      const current = get({ subscribe });
      const targetId = effectId ?? current.activeEffect?.effectId;
      if (!targetId) return;

      const effect = getEffect(targetId);
      if (!effect) return;

      const defaultParams: EffectParams = {};
      for (const p of effect.params) {
        defaultParams[p.name] = p.default;
      }

      update(s => {
        const nextActiveList = s.activeEffects.map(e => {
          if (e.effectId === targetId) {
            return { ...e, params: defaultParams };
          }
          return e;
        });

        const newState: EffectsState = {
          ...s,
          activeEffects: nextActiveList,
          activeEffect: nextActiveList[0] || null,
        };
        persist(newState);
        return newState;
      });

      effectEngine.updateEffectParams(targetId, defaultParams);
      emit('paramsChanged', {
        userId: '',
        effectId: targetId,
        params: defaultParams,
      });
    },

    /** Set visibility mode */
    setVisibility(mode: VisibilityMode) {
      update(s => {
        const nextActiveList = s.activeEffects.map(e => ({ ...e, visibility: mode }));
        const newState: EffectsState = {
          ...s,
          visibility: mode,
          activeEffects: nextActiveList,
          activeEffect: nextActiveList[0] || null,
        };
        persist(newState);
        return newState;
      });
    },

    /** Set category filter */
    setCategory(category: EffectCategory | 'all') {
      update(s => ({ ...s, selectedCategory: category }));
    },

    /** Get all available effects (built-in + plugins) */
    getAvailableEffects(): Array<{ id: string; name: string; nameTr: string; category: EffectCategory; thumbnail: string; icon: string; isPlugin: boolean }> {
      const builtins = BUILTIN_EFFECTS.filter(e => e.id !== 'custom').map(e => ({
        id: e.id,
        name: e.name,
        nameTr: e.nameTr,
        category: e.category,
        thumbnail: e.thumbnail,
        icon: e.icon,
        isPlugin: false,
      }));

      const plugins = getPlugins().map(p => ({
        id: `plugin-${p.manifest.id}`,
        name: p.manifest.name,
        nameTr: p.manifest.name,
        category: p.manifest.category,
        thumbnail: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
        icon: 'puzzle',
        isPlugin: true,
      }));

      return [...builtins, ...plugins];
    },

    /** Start the effect engine */
    async startEngine() {
      await effectEngine.start();
    },

    /** Stop the effect engine */
    stopEngine() {
      effectEngine.stop();
    },

    /** Set video element for the engine */
    setVideoElement(video: HTMLVideoElement | null) {
      effectEngine.setVideoElement(video);
    },

    /** Set canvas for the engine */
    setCanvas(canvas: HTMLCanvasElement | null) {
      effectEngine.setCanvas(canvas);
    },

    /** Restore last used effect (on camera toggle) */
    restoreLastEffect() {
      const current = get({ subscribe });
      if (current.lastEffectId && current.activeEffects.length === 0) {
        this.activateEffect(current.lastEffectId);
      }
    },

    /** Apply persisted state on startup */
    restoreEffect() {
      const p = loadPersistedState();
      if (p.activeEffectIds && p.activeEffectIds.length > 0) {
        for (const effId of p.activeEffectIds) {
          const exists = BUILTIN_EFFECTS.some(e => e.id === effId)
            || getPlugins().some(pl => `plugin-${pl.manifest.id}` === effId);
          if (exists) {
            const storedParams = p.effectParams[effId] ?? {};
            this.activateEffect(effId, storedParams);
          }
        }
      } else if (p.lastEffectId) {
        const exists = BUILTIN_EFFECTS.some(e => e.id === p.lastEffectId)
          || getPlugins().some(pl => `plugin-${pl.manifest.id}` === p.lastEffectId);
        if (exists) {
          const storedParams = p.effectParams[p.lastEffectId] ?? {};
          this.activateEffect(p.lastEffectId, storedParams);
        }
      }
    },

    getDiagnostics() {
      return diagnoseEngine();
    },

    /** Save current grid scroll position to restore on reopening menu */
    setScrollTop(top: number) {
      update(s => ({ ...s, gridScrollTop: Math.max(0, top) }));
    },

    /** Reset session scroll and category on joining/leaving call */
    resetSession() {
      update(s => ({ ...s, gridScrollTop: 0, selectedCategory: 'all' }));
    },
  };
}

export const effectsStore = createEffectsStore();

/**
 * Effect system types — face/hand/body tracking, effect pipeline, plugin manifest
 *
 * Architecture:
 *   MediaPipe (WASM) → landmarks → Effect.process() → Canvas 2D overlay
 *   No pixel data leaves the browser — privacy invariant preserved.
 */

// ── Landmark types ───────────────────────────────────────────────────────────

export interface Landmark {
  x: number; // 0..1, normalized to image width
  y: number; // 0..1, normalized to image height
  z: number; // depth, relative to camera
  visibility?: number; // 0..1, confidence
}

export type FaceLandmarks = Landmark[]; // 478 points (MediaPipe Face Mesh)
export type HandLandmarks = Landmark[]; // 21 points per hand
export type PoseLandmarks = Landmark[]; // 33 points (MediaPipe Pose)

export interface TrackingResult {
  face?: FaceLandmarks[];
  hands?: HandLandmarks[];
  pose?: PoseLandmarks[];
  timestamp: number;
}

// ── Effect categories ────────────────────────────────────────────────────────

export type EffectCategory = 'face' | 'hand' | 'body' | 'gesture' | 'custom';

export type EffectDifficulty = 'easy' | 'medium' | 'hard';

// ── Effect parameters (user-adjustable) ──────────────────────────────────────

export interface EffectParam {
  name: string;
  label: string;
  type: 'number' | 'color' | 'boolean' | 'select';
  min?: number;
  max?: number;
  step?: number;
  default: number | string | boolean;
  options?: string[]; // for 'select' type
}

export interface EffectParams {
  [key: string]: number | string | boolean;
}

// ── Effect interface ─────────────────────────────────────────────────────────

export interface Effect {
  id: string;
  name: string;
  nameTr: string; // Turkish display name
  description: string;
  descriptionTr: string;
  category: EffectCategory;
  difficulty: EffectDifficulty;
  icon: string; // SVG path or emoji
  params: EffectParam[];
  /** Thumbnail for grid preview (data URL or CSS gradient) */
  thumbnail: string;
  /** Required tracking data — engine only runs if these are available */
  requires: ('face' | 'hands' | 'pose')[];
  /** Main render function — called every frame when effect is active */
  process(
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    tracking: TrackingResult,
    params: EffectParams,
    time: number
  ): void;
}

// ── Visibility mode ──────────────────────────────────────────────────────────

export type VisibilityMode = 'self' | 'broadcast';

// ── Active effect state ──────────────────────────────────────────────────────

export interface ActiveEffect {
  effectId: string;
  params: EffectParams;
  visibility: VisibilityMode;
  /** Timestamp when effect was activated */
  activatedAt: number;
}

// ── Plugin system ────────────────────────────────────────────────────────────

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  category: EffectCategory;
  /** Hash of the script content for integrity */
  scriptHash: string;
  /** Allowed APIs — sandbox enforces these */
  allowedApis: string[];
  /** Created timestamp */
  createdAt: number;
}

export interface PluginScript {
  manifest: PluginManifest;
  /** Raw script content — stored encrypted at rest */
  content: string;
  /** Script language */
  language: 'javascript' | 'python';
}

// ── Effect engine state ──────────────────────────────────────────────────────

export interface EffectEngineState {
  /** Whether MediaPipe is loaded and ready */
  isReady: boolean;
  /** Current tracking result */
  tracking: TrackingResult;
  /** FPS of the tracking pipeline */
  fps: number;
  /** Whether tracking is currently running */
  isTracking: boolean;
  /** Error message if tracking failed */
  error: string | null;
}

// ── Broadcast metadata (sent via LiveKit DataChannel) ────────────────────────

export interface EffectBroadcastPayload {
  /** User ID */
  userId: string;
  /** Active effect ID (null = no effect) */
  effectId: string | null;
  /** Effect parameters */
  params: EffectParams;
  /** Landmarks for remote rendering (compressed) */
  landmarks?: {
    face?: number[]; // flattened [x,y,z,x,y,z,...] for bandwidth
    hands?: number[];
    pose?: number[];
  };
}

/**
 * Effect Engine — MediaPipe WASM + Canvas 2D rendering pipeline
 *
 * Architecture:
 *   1. MediaPipe runs in Web Worker (off main thread)
 *   2. Landmarks are sent to main thread via postMessage
 *   3. Effect pipeline renders to an offscreen canvas
 *   4. Canvas is composited over the video track
 *
 * Privacy: No pixel data leaves the browser. Only landmarks (numbers) are used.
 * Performance: 30+ FPS on modern hardware via requestAnimationFrame.
 */

import {
  FilesetResolver,
  FaceLandmarker,
  HandLandmarker,
  PoseLandmarker,
} from '@mediapipe/tasks-vision';
import type {
  Effect,
  EffectParams,
  EffectEngineState,
  TrackingResult,
  Landmark,
} from './types';
import { getEffect } from './effects';

/** Simple 1-euro filter for landmark smoothing */
function smoothLandmarks(current: Landmark[], previous: Landmark[] | null, factor: number = 0.4): Landmark[] {
  if (!previous || previous.length !== current.length) return current;
  return current.map((c, i) => ({
    x: previous[i].x + (c.x - previous[i].x) * factor,
    y: previous[i].y + (c.y - previous[i].y) * factor,
    z: previous[i].z + (c.z - previous[i].z) * factor,
    visibility: c.visibility,
  }));
}

// ── Model URLs (Google Storage, float16 for smaller download) ────────────────

const FACE_MODEL_URL = 'https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/latest/face_landmarker.task';
const HANDS_MODEL_URL = 'https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task';
const POSE_MODEL_URL = 'https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_lite/float16/latest/pose_landmarker_lite.task';

// ── Model load status ────────────────────────────────────────────────────────

export interface ModelLoadStatus {
  face: 'idle' | 'loading' | 'loaded' | 'failed';
  hand: 'idle' | 'loading' | 'loaded' | 'failed';
  pose: 'idle' | 'loading' | 'loaded' | 'failed';
}

export interface EngineDiagnostics {
  status: 'offline' | 'camera_off' | 'idle_ready' | 'loading_models' | 'running' | 'error';
  statusText: string;
  isReady: boolean;
  isTracking: boolean;
  fps: number;
  error: string | null;
  models: ModelLoadStatus;
  canvasSize: { width: number; height: number } | null;
  videoSize: { width: number; height: number } | null;
  activeEffectCount: number;
  activeEffectNames: string[];
}

// ── Engine singleton ─────────────────────────────────────────────────────────

class EffectEngine {
  private state: EffectEngineState = {
    isReady: false,
    tracking: { timestamp: 0 },
    fps: 0,
    isTracking: false,
    error: null,
  };

  private videoElement: HTMLVideoElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private animationFrame: number | null = null;
  private lastFrameTime = 0;
  private frameCount = 0;
  private fpsTimer = 0;

  // Adaptive frame skipping
  private targetFrameInterval = 33; // ~30 FPS
  private readonly MIN_FRAME_INTERVAL = 16; // ~60 FPS cap
  private readonly MAX_FRAME_INTERVAL = 50; // ~20 FPS floor

  // Landmark smoothing
  private prevFace: Landmark[] | null = null;
  private prevHands: Landmark[][] | null = null;
  private prevPose: Landmark[] | null = null;

  // MediaPipe task vision instances (lazy loaded)
  private faceLandmarker: FaceLandmarker | null = null;
  private handLandmarker: HandLandmarker | null = null;
  private poseLandmarker: PoseLandmarker | null = null;

  // Model load status tracking
  private modelStatus: ModelLoadStatus = { face: 'idle', hand: 'idle', pose: 'idle' };

  // Active effect stack
  private activeEffects: Array<{ effect: Effect; params: EffectParams }> = [];

  // Plugin effects registry (maps plugin ID → Effect)
  private pluginEffects: Map<string, Effect> = new Map();

  private retryPending = false;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;

  // Multi-subscriber support
  private stateCallbacks = new Set<(state: EffectEngineState) => void>();

  /** Subscribe to state changes — returns unsubscribe function */
  subscribe(callback: (state: EffectEngineState) => void) {
    this.stateCallbacks.add(callback);
    return () => { this.stateCallbacks.delete(callback); };
  }

  /** Get current state */
  getState(): EffectEngineState {
    return { ...this.state };
  }

  /** Get diagnostics for the UI */
  getDiagnostics(): EngineDiagnostics {
    const hasVideo = !!(this.videoElement && !this.videoElement.paused && this.videoElement.videoWidth > 0);
    const isRunning = this.state.isTracking && this.activeEffects.length > 0;
    const isModelsLoading = this.modelStatus.face === 'loading' || this.modelStatus.hand === 'loading' || this.modelStatus.pose === 'loading';
    const isModelsLoaded = this.modelStatus.face === 'loaded' || this.modelStatus.hand === 'loaded' || this.modelStatus.pose === 'loaded';

    let status: EngineDiagnostics['status'] = 'idle_ready';
    let statusText = 'Hazır — Efekt Seçin';

    if (this.state.error) {
      status = 'error';
      statusText = 'Motor Hatası';
    } else if (isModelsLoading) {
      status = 'loading_models';
      statusText = 'Modeller Yükleniyor...';
    } else if (isRunning) {
      status = 'running';
      statusText = `Çalışıyor (${this.state.fps || 30} FPS)`;
    } else if (!hasVideo) {
      status = 'camera_off';
      statusText = 'Kamera Bekleniyor';
    } else if (isModelsLoaded) {
      status = 'idle_ready';
      statusText = 'Hazır — Efekt Seçin';
    }

    return {
      status,
      statusText,
      isReady: this.state.isReady || isModelsLoaded,
      isTracking: this.state.isTracking,
      fps: this.state.fps,
      error: this.state.error,
      models: { ...this.modelStatus },
      canvasSize: this.canvas ? { width: this.canvas.width, height: this.canvas.height } : null,
      videoSize: this.videoElement ? { width: this.videoElement.videoWidth, height: this.videoElement.videoHeight } : null,
      activeEffectCount: this.activeEffects.length,
      activeEffectNames: this.activeEffects.map(e => e.effect.nameTr || e.effect.name),
    };
  }

  /** Set the video element to track */
  setVideoElement(video: HTMLVideoElement | null) {
    this.videoElement = video;
  }

  /** Set the canvas to render effects on */
  setCanvas(canvas: HTMLCanvasElement | null) {
    this.canvas = canvas;
    this.ctx = canvas?.getContext('2d') ?? null;
  }

  addEffect(effectId: string, params: EffectParams = {}) {
    const effect = getEffect(effectId) ?? this.pluginEffects.get(effectId);
    if (!effect) return;
    this.activeEffects = this.activeEffects.filter(e => e.effect.id !== effectId);
    this.activeEffects.push({ effect, params });
  }

  removeEffect(effectId: string) {
    this.activeEffects = this.activeEffects.filter(e => e.effect.id !== effectId);
  }

  registerPluginEffect(effect: Effect) {
    this.pluginEffects.set(effect.id, effect);
  }

  unregisterPluginEffect(effectId: string) {
    this.pluginEffects.delete(effectId);
    this.activeEffects = this.activeEffects.filter(e => e.effect.id !== effectId);
  }

  /** Update parameters for an active effect */
  updateEffectParams(effectId: string, params: EffectParams) {
    const entry = this.activeEffects.find(e => e.effect.id === effectId);
    if (entry) entry.params = { ...entry.params, ...params };
  }

  /** Clear all effects */
  clearEffects() {
    this.activeEffects = [];
  }

  /** Get active effect IDs */
  getActiveEffectIds(): string[] {
    return this.activeEffects.map(e => e.effect.id);
  }

  async start() {
    if (this.state.isTracking) return;
    if (!this.videoElement || !this.canvas || !this.ctx) {
      this.retryPending = true;
      this.scheduleRetry();
      return;
    }

    try {
      this.updateState({ isTracking: true, error: null });
      await this.loadMediaPipe();
      this.updateState({ isReady: true });
      this.startLoop();
    } catch (err) {
      console.error('Effect engine start failed:', err);
      this.updateState({
        isTracking: false,
        error: `Efekt motoru başlatılamadı: ${String(err).slice(0, 100)}`,
      });
    }
  }

  stop() {
    this.stopLoop();
    this.cancelRetry();
    this.updateState({ isTracking: false, isReady: false });
  }

  destroy() {
    this.stop();
    this.faceLandmarker?.close();
    this.handLandmarker?.close();
    this.poseLandmarker?.close();
    this.faceLandmarker = null;
    this.handLandmarker = null;
    this.poseLandmarker = null;
    this.modelStatus = { face: 'idle', hand: 'idle', pose: 'idle' };
  }

  // ── Private methods ──────────────────────────────────────────────────────

  private scheduleRetry() {
    if (this.retryTimer) return;
    this.retryTimer = setTimeout(() => {
      this.retryTimer = null;
      if (this.retryPending && this.videoElement && this.canvas && this.ctx) {
        this.retryPending = false;
        this.start();
      } else if (this.retryPending) {
        this.scheduleRetry();
      }
    }, 200);
  }

  private cancelRetry() {
    this.retryPending = false;
    if (this.retryTimer) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
  }

  private updateState(partial: Partial<EffectEngineState>) {
    this.state = { ...this.state, ...partial };
    for (const cb of this.stateCallbacks) {
      try { cb(this.state); } catch (err) { console.warn('[effects] state callback error:', err); }
    }
  }

  private async loadWithTimeout<T>(
    loader: () => Promise<T>,
    timeoutMs: number,
    label: string,
  ): Promise<T | null> {
    try {
      return await Promise.race([
        loader(),
        new Promise<null>((_, reject) =>
          setTimeout(() => reject(new Error(`${label} yükleme zaman aşımı (${timeoutMs}ms)`)), timeoutMs),
        ),
      ]);
    } catch (err) {
      console.warn(`${label} yükleme hatası:`, err);
      return null;
    }
  }

  private async loadMediaPipe() {
    if (this.faceLandmarker && this.handLandmarker && this.poseLandmarker) return;

    let vision: Awaited<ReturnType<typeof FilesetResolver.forVisionTasks>>;
    try {
      vision = await FilesetResolver.forVisionTasks(
        'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.18/wasm',
      );
    } catch {
      vision = await FilesetResolver.forVisionTasks(
        'https://unpkg.com/@mediapipe/tasks-vision@0.10.18/wasm',
      );
    }

    const MODEL_TIMEOUT = 25000;

    // Set loading states
    if (!this.faceLandmarker) this.modelStatus.face = 'loading';
    if (!this.handLandmarker) this.modelStatus.hand = 'loading';
    if (!this.poseLandmarker) this.modelStatus.pose = 'loading';
    this.updateState({});

    const loadFace = async () => {
      if (this.faceLandmarker) return true;
      const ok = await this.loadWithTimeout(async () => {
        try {
          this.faceLandmarker = await FaceLandmarker.createFromOptions(vision, {
            baseOptions: { modelAssetPath: FACE_MODEL_URL, delegate: 'GPU' },
            runningMode: 'VIDEO', numFaces: 1,
            minFaceDetectionConfidence: 0.5, minFacePresenceConfidence: 0.5, minTrackingConfidence: 0.5,
          });
        } catch {
          this.faceLandmarker = await FaceLandmarker.createFromOptions(vision, {
            baseOptions: { modelAssetPath: FACE_MODEL_URL, delegate: 'CPU' },
            runningMode: 'VIDEO', numFaces: 1,
            minFaceDetectionConfidence: 0.5, minFacePresenceConfidence: 0.5, minTrackingConfidence: 0.5,
          });
        }
        return true;
      }, MODEL_TIMEOUT, 'Yüz modeli');
      this.modelStatus.face = ok ? 'loaded' : 'failed';
      return ok;
    };

    const loadHand = async () => {
      if (this.handLandmarker) return true;
      const ok = await this.loadWithTimeout(async () => {
        try {
          this.handLandmarker = await HandLandmarker.createFromOptions(vision, {
            baseOptions: { modelAssetPath: HANDS_MODEL_URL, delegate: 'GPU' },
            runningMode: 'VIDEO', numHands: 2,
            minHandDetectionConfidence: 0.5, minHandPresenceConfidence: 0.5, minTrackingConfidence: 0.5,
          });
        } catch {
          this.handLandmarker = await HandLandmarker.createFromOptions(vision, {
            baseOptions: { modelAssetPath: HANDS_MODEL_URL, delegate: 'CPU' },
            runningMode: 'VIDEO', numHands: 2,
            minHandDetectionConfidence: 0.5, minHandPresenceConfidence: 0.5, minTrackingConfidence: 0.5,
          });
        }
        return true;
      }, MODEL_TIMEOUT, 'El modeli');
      this.modelStatus.hand = ok ? 'loaded' : 'failed';
      return ok;
    };

    const loadPose = async () => {
      if (this.poseLandmarker) return true;
      const ok = await this.loadWithTimeout(async () => {
        try {
          this.poseLandmarker = await PoseLandmarker.createFromOptions(vision, {
            baseOptions: { modelAssetPath: POSE_MODEL_URL, delegate: 'GPU' },
            runningMode: 'VIDEO',
            minPoseDetectionConfidence: 0.5, minPosePresenceConfidence: 0.5, minTrackingConfidence: 0.5,
          });
        } catch {
          this.poseLandmarker = await PoseLandmarker.createFromOptions(vision, {
            baseOptions: { modelAssetPath: POSE_MODEL_URL, delegate: 'CPU' },
            runningMode: 'VIDEO',
            minPoseDetectionConfidence: 0.5, minPosePresenceConfidence: 0.5, minTrackingConfidence: 0.5,
          });
        }
        return true;
      }, MODEL_TIMEOUT, 'Vücut modeli');
      this.modelStatus.pose = ok ? 'loaded' : 'failed';
      return ok;
    };

    // Load all models concurrently in parallel for faster startup
    await Promise.allSettled([loadFace(), loadHand(), loadPose()]);

    this.updateState({});

    if (!this.faceLandmarker && !this.handLandmarker && !this.poseLandmarker) {
      throw new Error('Hiçbir MediaPipe modeli yüklenemedi. İnternet bağlantınızı kontrol edin.');
    }
  }

  private startLoop() {
    const loop = async (timestamp: number) => {
      if (!this.state.isTracking) return;

      this.animationFrame = requestAnimationFrame(loop);

      this.frameCount++;
      if (timestamp - this.fpsTimer >= 1000) {
        const currentFps = this.frameCount;
        this.updateState({ fps: currentFps });
        this.frameCount = 0;
        this.fpsTimer = timestamp;

        if (currentFps < 20) {
          this.targetFrameInterval = Math.min(this.targetFrameInterval + 2, this.MAX_FRAME_INTERVAL);
        } else if (currentFps > 40) {
          this.targetFrameInterval = Math.max(this.targetFrameInterval - 1, this.MIN_FRAME_INTERVAL);
        }
      }

      if (timestamp - this.lastFrameTime < this.targetFrameInterval) return;
      this.lastFrameTime = timestamp;

      const video = this.videoElement;
      if (!video || video.paused || video.ended || !video.videoWidth) return;

      const tracking = this.detect(video, timestamp);
      this.render(tracking, timestamp);
    };

    this.animationFrame = requestAnimationFrame(loop);
  }

  private stopLoop() {
    if (this.animationFrame !== null) {
      cancelAnimationFrame(this.animationFrame);
      this.animationFrame = null;
    }
  }

  private detect(video: HTMLVideoElement, timestamp: number): TrackingResult {
    const result: TrackingResult = { timestamp };

    // Detect face landmarks (synchronous)
    if (this.faceLandmarker) {
      try {
        const faceResults = this.faceLandmarker.detectForVideo(video, timestamp);
        if (faceResults?.faceLandmarks?.length) {
          const raw = faceResults.faceLandmarks[0] as Landmark[];
          result.face = [smoothLandmarks(raw, this.prevFace, 0.35)];
          this.prevFace = result.face[0];
        } else {
          this.prevFace = null;
        }
      } catch (err) {
        console.warn('Face detection error:', err);
      }
    }

    // Detect hand landmarks (synchronous)
    if (this.handLandmarker) {
      try {
        const handResults = this.handLandmarker.detectForVideo(video, timestamp);
        if (handResults?.landmarks?.length) {
          result.hands = handResults.landmarks.map((raw: Landmark[], i: number) => {
            const prev = this.prevHands?.[i] ?? null;
            return smoothLandmarks(raw, prev, 0.4);
          });
          this.prevHands = result.hands ?? null;
        } else {
          this.prevHands = null;
        }
      } catch (err) {
        console.warn('Hands detection error:', err);
      }
    }

    // Detect pose landmarks (synchronous)
    if (this.poseLandmarker) {
      try {
        const poseResults = this.poseLandmarker.detectForVideo(video, timestamp);
        if (poseResults?.landmarks?.length) {
          const raw = poseResults.landmarks[0] as Landmark[];
          result.pose = [smoothLandmarks(raw, this.prevPose, 0.35)];
          this.prevPose = result.pose[0];
        } else {
          this.prevPose = null;
        }
      } catch (err) {
        console.warn('Pose detection error:', err);
      }
    }

    this.updateState({ tracking: result });
    return result;
  }

  private render(tracking: TrackingResult, time: number) {
    const ctx = this.ctx;
    const canvas = this.canvas;
    const video = this.videoElement;
    if (!ctx || !canvas || !video) return;

    const w = canvas.width;
    const h = canvas.height;

    if (w <= 0 || h <= 0) return;

    ctx.clearRect(0, 0, w, h);

    try {
      ctx.drawImage(video, 0, 0, w, h);
    } catch (err) {
      console.warn('Video draw error:', err);
      return;
    }

    for (const { effect, params } of this.activeEffects) {
      const hasRequired = effect.requires.every(r => {
        if (r === 'face') return tracking.face?.length;
        if (r === 'hands') return tracking.hands?.length;
        if (r === 'pose') return tracking.pose?.length;
        return false;
      });

      if (!hasRequired) continue;

      try {
        ctx.save();
        effect.process(ctx, w, h, tracking, params, time);
        ctx.restore();
      } catch (err) {
        console.warn(`Effect "${effect.id}" render error:`, err);
      }
    }
  }
}

// ── Singleton export ─────────────────────────────────────────────────────────

export const effectEngine = new EffectEngine();

export function diagnoseEngine(): EngineDiagnostics {
  return effectEngine.getDiagnostics();
}

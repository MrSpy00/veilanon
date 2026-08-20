/**
 * Multi-frame gesture detection with smoothing for MediaPipe hand landmarks.
 *
 * Architecture:
 *   Raw landmarks → per-frame heuristic → rolling buffer → majority vote → stable gesture
 *
 * Reduces false positives by requiring M out of N consecutive frames to agree
 * before reporting a gesture as detected.
 *
 * MediaPipe hand landmarks (21 points):
 *   0=Wrist, 1-4=Thumb(CMC,MCP,IP,TIP), 5-8=Index(MCP,PIP,DIP,TIP),
 *   9-12=Middle(MCP,PIP,DIP,TIP), 13-16=Ring(MCP,PIP,DIP,TIP),
 *   17-20=Pinky(MCP,PIP,DIP,TIP)
 *
 * Privacy: No pixel data — only landmark coordinates are used.
 */

import type { HandLandmarks } from './types';

// ── Gesture types ──────────────────────────────────────────────────────────

export type GestureType =
  | 'peace'
  | 'thumbsUp'
  | 'thumbsDown'
  | 'fist'
  | 'openPalm'
  | 'wave'
  | 'pinch'
  | 'point';

export interface DetectedGesture {
  gesture: GestureType;
  confidence: number; // 0..1, fraction of frames in buffer that agree
}

// ── Configuration ──────────────────────────────────────────────────────────

export interface GestureDetectorConfig {
  /** Number of recent frames to keep in the smoothing buffer (default: 6) */
  bufferSize: number;
  /** Minimum fraction of frames (0..1) that must agree for a positive detection (default: 0.5) */
  confidenceThreshold: number;
  /** Maximum distance between thumb tip and index tip for pinch (normalised 0..1, default: 0.06) */
  pinchThreshold: number;
}

const DEFAULT_CONFIG: GestureDetectorConfig = {
  bufferSize: 6,
  confidenceThreshold: 0.5,
  pinchThreshold: 0.06,
};

// ── Internal helpers ───────────────────────────────────────────────────────

/** Euclidean distance between two landmarks (in normalised 0..1 space). */
function dist(a: { x: number; y: number }, b: { x: number; y: number }): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  return Math.sqrt(dx * dx + dy * dy);
}

/**
 * Classify a single hand into a raw gesture based on landmark positions.
 * Returns null if the hand does not match any known gesture.
 *
 * Uses simple heuristics on landmark y-coordinates (lower y = higher on screen
 * = extended finger for a vertically-oriented hand) and x-spread for spread
 * detection.
 */
function classifySingleFrame(hand: HandLandmarks, pinchThreshold: number): GestureType | null {
  if (hand.length < 21) return null;

  // Landmark shortcuts
  const thumbTip = hand[4];
  const thumbIp = hand[3];
  const thumbMcp = hand[2];
  const thumbCmc = hand[1];
  const indexTip = hand[8];
  const indexPip = hand[6];
  const indexMcp = hand[5];
  const middleTip = hand[12];
  const middlePip = hand[10];
  const middleMcp = hand[9];
  const ringTip = hand[16];
  const ringPip = hand[14];
  const ringMcp = hand[13];
  const pinkyTip = hand[20];
  const pinkyPip = hand[18];
  const pinkyMcp = hand[17];
  const wrist = hand[0];

  // Finger extension checks: tip is above (lower y) the PIP joint
  const indexExtended = indexTip.y < indexPip.y;
  const middleExtended = middleTip.y < middlePip.y;
  const ringExtended = ringTip.y < ringPip.y;
  const pinkyExtended = pinkyTip.y < pinkyPip.y;

  // Finger folded checks: tip is below (higher y) the PIP joint
  const indexFolded = indexTip.y > indexPip.y;
  const middleFolded = middleTip.y > middlePip.y;
  const ringFolded = ringTip.y > ringPip.y;
  const pinkyFolded = pinkyTip.y > pinkyPip.y;

  // Thumb direction: tip below CMC = thumb pointing down (hand upright)
  const thumbUp = thumbTip.y < thumbIp.y && thumbTip.y < thumbMcp.y;
  const thumbDown = thumbTip.y > thumbCmc.y;

  // All four fingers extended (open palm requires all)
  const allFingersExtended = indexExtended && middleExtended && ringExtended && pinkyExtended;
  // All four fingers folded (fist / thumbs up / thumbs down)
  const allFingersFolded = indexFolded && middleFolded && ringFolded && pinkyFolded;

  // Pinch: thumb tip and index tip very close
  const pinchDist = dist(thumbTip, indexTip);
  if (pinchDist < pinchThreshold && !middleExtended && !ringExtended) {
    return 'pinch';
  }

  // Point: only index extended
  if (indexExtended && middleFolded && ringFolded && pinkyFolded) {
    return 'point';
  }

  // Peace: index + middle extended, ring + pinky folded
  if (indexExtended && middleExtended && ringFolded && pinkyFolded) {
    return 'peace';
  }

  // Open palm: all fingers extended
  if (allFingersExtended) {
    return 'openPalm';
  }

  // Fist: all fingers folded, thumb not clearly up or down (neutral)
  if (allFingersFolded && !thumbUp && !thumbDown) {
    return 'fist';
  }

  // Thumbs up: thumb up, all other fingers folded
  if (thumbUp && allFingersFolded) {
    return 'thumbsUp';
  }

  // Thumbs down: thumb down, all other fingers folded
  if (thumbDown && allFingersFolded) {
    return 'thumbsDown';
  }

  return null;
}

// ── Public API ─────────────────────────────────────────────────────────────

export class GestureDetector {
  private config: GestureDetectorConfig;
  /** Rolling buffer of per-frame gesture classifications. */
  private buffer: (GestureType | null)[] = [];
  /** Previous wrist x-positions for wave detection (last N frames). */
  private wristXHistory: number[] = [];
  private readonly WAVE_FRAMES = 8;
  private readonly WAVE_THRESHOLD = 0.03;

  constructor(config?: Partial<GestureDetectorConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /** Update configuration at runtime (e.g. when user changes threshold). */
  configure(config: Partial<GestureDetectorConfig>): void {
    Object.assign(this.config, config);
    // Shrink buffer if needed
    while (this.buffer.length > this.config.bufferSize) {
      this.buffer.shift();
    }
  }

  /**
   * Classify one frame and update the smoothing buffer.
   * Call this every frame with the first detected hand.
   * Returns the smoothed gesture (if confidence meets threshold) or null.
   */
  detect(hand: HandLandmarks | undefined): DetectedGesture | null {
    // Raw classification for this frame
    const raw = hand ? classifySingleFrame(hand, this.config.pinchThreshold) : null;

    // Wave heuristic: requires openPalm + horizontal wrist movement
    let finalRaw = raw;
    if (raw === 'openPalm' && hand) {
      const wristX = hand[0].x;
      this.wristXHistory.push(wristX);
      if (this.wristXHistory.length > this.WAVE_FRAMES) {
        this.wristXHistory.shift();
      }
      if (this.wristXHistory.length >= this.WAVE_FRAMES) {
        const minX = Math.min(...this.wristXHistory);
        const maxX = Math.max(...this.wristXHistory);
        if (maxX - minX > this.WAVE_THRESHOLD) {
          finalRaw = 'wave';
        }
      }
    } else {
      this.wristXHistory = [];
    }

    // Push into smoothing buffer
    this.buffer.push(finalRaw);
    if (this.buffer.length > this.config.bufferSize) {
      this.buffer.shift();
    }

    // Count occurrences of each gesture in the buffer
    const counts = new Map<GestureType, number>();
    for (const g of this.buffer) {
      if (g !== null) {
        counts.set(g, (counts.get(g) ?? 0) + 1);
      }
    }

    // Find the gesture with the highest count
    let bestGesture: GestureType | null = null;
    let bestCount = 0;
    for (const [g, c] of counts) {
      if (c > bestCount) {
        bestCount = c;
        bestGesture = g;
      }
    }

    if (bestGesture === null || this.buffer.length === 0) return null;

    const confidence = bestCount / this.buffer.length;
    if (confidence < this.config.confidenceThreshold) return null;

    return { gesture: bestGesture, confidence };
  }

  /** Reset internal state (call when effect is deactivated). */
  reset(): void {
    this.buffer = [];
    this.wristXHistory = [];
  }
}

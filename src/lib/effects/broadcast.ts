/**
 * DataChannel broadcast — syncs effect metadata to remote participants
 *
 * Sends EffectBroadcastPayload via LiveKit DataChannel (topic: 'effects', lossy).
 * Receives broadcasts from remote participants and forwards to consumers.
 * Only landmark data (not pixel data) is sent — privacy invariant preserved.
 * Landmarks are compressed to flattened number[] for bandwidth efficiency.
 */

import { RoomEvent, type Room, type LocalParticipant, type RemoteParticipant } from 'livekit-client';
import type { DataPacket_Kind } from 'livekit-client';
import type { EffectBroadcastPayload, EffectParams, TrackingResult } from './types';
import { subscribeToEffectEvents, type EffectsEventType } from './store';
import { effectEngine } from './engine';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type DataReceivedHandler = (payload: any, participant?: RemoteParticipant, kind?: DataPacket_Kind, topic?: string) => void;

// ── Constants ────────────────────────────────────────────────────────────────

const TOPIC = 'effects';
const BROADCAST_THROTTLE_MS = 100; // ~10 Hz max broadcast rate
const MAX_LANDMARK_POINTS = 478; // face mesh max points to send

// ── Landmark compression ─────────────────────────────────────────────────────

/** Flatten Landmark[] to number[] for bandwidth-efficient transmission. */
function compressLandmarks(landmarks: { x: number; y: number; z: number }[]): number[] {
  const out: number[] = [];
  const limit = Math.min(landmarks.length, MAX_LANDMARK_POINTS);
  for (let i = 0; i < limit; i++) {
    const lm = landmarks[i];
    // Quantize to 4 decimal places (~0.1mm precision) — saves bytes vs full float
    out.push(Math.round(lm.x * 10000) / 10000);
    out.push(Math.round(lm.y * 10000) / 10000);
    out.push(Math.round(lm.z * 10000) / 10000);
  }
  return out;
}

/** Build compressed landmarks from a TrackingResult. */
function buildCompressedLandmarks(tracking: TrackingResult): EffectBroadcastPayload['landmarks'] {
  if (!tracking.face && !tracking.hands && !tracking.pose) return undefined;
  return {
    face: tracking.face?.[0] ? compressLandmarks(tracking.face[0]) : undefined,
    hands: tracking.hands?.[0] ? compressLandmarks(tracking.hands[0]) : undefined,
    pose: tracking.pose?.[0] ? compressLandmarks(tracking.pose[0]) : undefined,
  };
}

// ── Send ─────────────────────────────────────────────────────────────────────

let lastBroadcastTime = 0;
let currentRoomId: string | null = null;

export async function sendEffectBroadcast(
  room: Room,
  payload: EffectBroadcastPayload,
): Promise<void> {
  if (!room || !room.localParticipant) return;
  try {
    const data = new TextEncoder().encode(JSON.stringify(payload));
    await (room.localParticipant as LocalParticipant).publishData(data, {
      reliable: false,
      topic: TOPIC,
    });
  } catch (err) {
    console.warn('[broadcast] send failed:', err);
  }
}

// ── Receive ──────────────────────────────────────────────────────────────────

type BroadcastCallback = (payload: EffectBroadcastPayload) => void;

/**
 * Listen for incoming effect broadcasts on a room.
 * Returns an unsubscribe function.
 */
export function onEffectBroadcast(
  room: Room,
  callback: BroadcastCallback,
): () => void {
  if (!room || typeof room.on !== 'function') {
    return () => {};
  }

  function handler(
    payload: Uint8Array,
    participant?: RemoteParticipant,
    kind?: DataPacket_Kind,
    topic?: string,
  ) {
    if (topic !== TOPIC) return;
    try {
      const text = new TextDecoder().decode(payload);
      const data = JSON.parse(text) as EffectBroadcastPayload;
      if (data && typeof data.userId === 'string') {
        callback(data);
      }
    } catch {
      // Malformed broadcast — ignore
    }
  }

  room.on(RoomEvent.DataReceived, handler as DataReceivedHandler);
  return () => {
    if (room && typeof room.off === 'function') {
      room.off(RoomEvent.DataReceived, handler as DataReceivedHandler);
    }
  };
}

// ── Auto-broadcast loop ──────────────────────────────────────────────────────

let broadcastUnsubscribers: (() => void)[] = [];
let isLoopRunning = false;

/**
 * Start auto-broadcasting local effect changes via DataChannel.
 * Subscribes to store events and sends throttled broadcasts.
 * Also sends tracking landmarks on each engine tick.
 */
export function startBroadcastLoop(room: Room): void {
  stopBroadcastLoop();
  if (!room) return;

  const roomId = room.name ?? 'unknown';
  if (currentRoomId !== roomId) {
    lastBroadcastTime = 0;
    currentRoomId = roomId;
  }

  isLoopRunning = true;

  // Subscribe to effect state change events
  const events: EffectsEventType[] = ['effectActivated', 'effectDeactivated', 'paramsChanged'];
  for (const evt of events) {
    const unsub = subscribeToEffectEvents(evt, (payload) => {
      if (!room || !isLoopRunning) return;
      const now = Date.now();
      if (now - lastBroadcastTime < BROADCAST_THROTTLE_MS) return;
      lastBroadcastTime = now;

      // Send with current identity
      const identity = room.localParticipant?.identity ?? '';
      sendEffectBroadcast(room, { ...payload, userId: identity });
    });
    broadcastUnsubscribers.push(unsub);
  }

  // Periodic landmark broadcast on engine tick (~10 Hz)
  let tickInterval: ReturnType<typeof setInterval> | null = setInterval(() => {
    if (!isLoopRunning || room.state !== 'connected') return;
    const state = effectEngine.getState();
    if (!state.isTracking || !state.tracking.timestamp) return;

    const identity = room.localParticipant?.identity ?? '';
    const tracking = state.tracking;
    const landmarks = buildCompressedLandmarks(tracking);
    if (!landmarks) return;

    sendEffectBroadcast(room, {
      userId: identity,
      effectId: null, // landmarks-only tick, no effect change
      params: {},
      landmarks,
    });
  }, BROADCAST_THROTTLE_MS);

  broadcastUnsubscribers.push(() => {
    if (tickInterval) {
      clearInterval(tickInterval);
      tickInterval = null;
    }
  });
}

/**
 * Stop auto-broadcasting and clean up all listeners.
 */
export function stopBroadcastLoop(): void {
  isLoopRunning = false;
  for (const unsub of broadcastUnsubscribers) {
    try { unsub(); } catch { /* best effort */ }
  }
  broadcastUnsubscribers = [];
}

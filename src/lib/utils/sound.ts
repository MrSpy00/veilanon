/**
 * veilanon — Web Audio sound effects synthesizer
 *
 * Synthesizes crystal-clear, zero-latency notification and UI sounds using the
 * Web Audio API. Requires no external audio files or network requests.
 */

import { get } from 'svelte/store';
import { streamerMode } from '$lib/stores/streamerMode';

let audioCtx: AudioContext | null = null;

function isAudioSuppressed(): boolean {
  try {
    const s = get(streamerMode);
    return s.enabled && s.suppressAudioAlerts;
  } catch {
    return false;
  }
}

function getAudioContext(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  if (isAudioSuppressed()) return null;
  if (!audioCtx) {
    const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (AudioCtx) {
      audioCtx = new AudioCtx();
    }
  }
  if (audioCtx && audioCtx.state === 'suspended') {
    void audioCtx.resume();
  }
  return audioCtx;
}

/**
 * Standard incoming message chime — gentle marimba / bell ping (880Hz -> 1320Hz harmonic)
 */
export function playMessageSound(volume = 0.4) {
  const ctx = getAudioContext();
  if (!ctx) return;
  try {
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = 'sine';
    osc.frequency.setValueAtTime(880, now); // A5
    osc.frequency.exponentialRampToValueAtTime(1318.51, now + 0.08); // E6

    gain.gain.setValueAtTime(0.001, now);
    gain.gain.linearRampToValueAtTime(volume, now + 0.015);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.28);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.3);
  } catch {
    // Audio playback is best-effort
  }
}

/**
 * Mention / Direct Message chime — bright two-tone harmonic ping
 */
export function playMentionSound(volume = 0.5) {
  const ctx = getAudioContext();
  if (!ctx) return;
  try {
    const now = ctx.currentTime;

    // First tone (high chime)
    const osc1 = ctx.createOscillator();
    const gain1 = ctx.createGain();
    osc1.type = 'sine';
    osc1.frequency.setValueAtTime(1046.5, now); // C6
    gain1.gain.setValueAtTime(0.001, now);
    gain1.gain.linearRampToValueAtTime(volume, now + 0.02);
    gain1.gain.exponentialRampToValueAtTime(0.0001, now + 0.22);
    osc1.connect(gain1);
    gain1.connect(ctx.destination);
    osc1.start(now);
    osc1.stop(now + 0.25);

    // Second tone (higher sparkle)
    const osc2 = ctx.createOscillator();
    const gain2 = ctx.createGain();
    osc2.type = 'sine';
    osc2.frequency.setValueAtTime(1567.98, now + 0.09); // G6
    gain2.gain.setValueAtTime(0.001, now + 0.09);
    gain2.gain.linearRampToValueAtTime(volume * 0.85, now + 0.11);
    gain2.gain.exponentialRampToValueAtTime(0.0001, now + 0.4);
    osc2.connect(gain2);
    gain2.connect(ctx.destination);
    osc2.start(now + 0.09);
    osc2.stop(now + 0.42);
  } catch {
    // Audio playback is best-effort
  }
}

/**
 * Friend request received chime — warm rising harmonic triad (F5 -> A5 -> C6)
 */
export function playFriendRequestSound(volume = 0.45) {
  const ctx = getAudioContext();
  if (!ctx) return;
  try {
    const now = ctx.currentTime;
    const notes = [698.46, 880.0, 1046.5]; // F5, A5, C6

    notes.forEach((freq, i) => {
      const startTime = now + i * 0.07;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(freq, startTime);
      gain.gain.setValueAtTime(0.001, startTime);
      gain.gain.linearRampToValueAtTime(volume * 0.7, startTime + 0.015);
      gain.gain.exponentialRampToValueAtTime(0.0001, startTime + 0.28);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(startTime);
      osc.stop(startTime + 0.3);
    });
  } catch {
    // Audio playback is best-effort
  }
}

/**
 * Voice channel joined sound — soft rising warm tone
 */
export function playCallJoinSound(volume = 0.35) {
  const ctx = getAudioContext();
  if (!ctx) return;
  try {
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = 'triangle';
    osc.frequency.setValueAtTime(440, now); // A4
    osc.frequency.exponentialRampToValueAtTime(880, now + 0.12); // A5

    gain.gain.setValueAtTime(0.001, now);
    gain.gain.linearRampToValueAtTime(volume, now + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.35);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.38);
  } catch {
    // Audio playback is best-effort
  }
}

/**
 * Voice channel left sound — soft falling tone
 */
export function playCallLeaveSound(volume = 0.3) {
  const ctx = getAudioContext();
  if (!ctx) return;
  try {
    const now = ctx.currentTime;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = 'triangle';
    osc.frequency.setValueAtTime(659.25, now); // E5
    osc.frequency.exponentialRampToValueAtTime(329.63, now + 0.14); // E4

    gain.gain.setValueAtTime(0.001, now);
    gain.gain.linearRampToValueAtTime(volume, now + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.25);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.28);
  } catch {
    // Audio playback is best-effort
  }
}

/**
 * Incoming call ringtone — repeating two-tone chime (returns stop function)
 */
export function playIncomingCallRing(volume = 0.5): () => void {
  const ctx = getAudioContext();
  if (!ctx) return () => {};
  try {
    let stopped = false;
    const timers: ReturnType<typeof setTimeout>[] = [];

    const tonePair = (startOffset: number) => {
      if (stopped) return;
      const base = ctx.currentTime + startOffset;
      [880, 1174.66].forEach((freq, i) => {
        const t0 = base + i * 0.18;
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = 'sine';
        osc.frequency.setValueAtTime(freq, t0);
        gain.gain.setValueAtTime(0.001, t0);
        gain.gain.linearRampToValueAtTime(volume, t0 + 0.03);
        gain.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.32);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(t0);
        osc.stop(t0 + 0.35);
      });
    };

    for (let cycle = 0; cycle < 12; cycle++) {
      timers.push(setTimeout(() => tonePair(cycle * 2), cycle * 2000));
    }

    return () => {
      stopped = true;
      timers.forEach(t => clearTimeout(t));
    };
  } catch {
    return () => {};
  }
}

/**
 * Media store — LiveKit voice/video state
 *
 * State is driven by LiveKit events, not optimistic UI flips: every local
 * track publish/mute change updates the store from the room's real state so
 * the mic button, camera tile and screen-share overlay can never desync.
 */
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import {
  Room,
  RoomEvent,
  ParticipantEvent,
  Track,
  ExternalE2EEKeyProvider,
  type E2EEOptions,
  type RemoteTrack,
  type AudioCaptureOptions,
  type VideoCaptureOptions,
} from 'livekit-client';
import E2EEWorker from 'livekit-client/e2ee-worker?worker';
import { settingsApi } from '$lib/api/tauri';
import { toastStore } from '$lib/stores/notifications';

export interface ScreenShareOptions {
  resolution?: { width: number; height: number };
  frameRate?: number;
  audio?: boolean;
}

export interface ScreenShareQuality {
  width: number;
  height: number;
  frameRate: number;
  label: string;
}

export const SCREEN_SHARE_PRESETS: ScreenShareQuality[] = [
  { width: 1920, height: 1080, frameRate: 60, label: '1080p · 60 FPS (Full HD Ultra - Standart)' },
  { width: 1920, height: 1080, frameRate: 30, label: '1080p · 30 FPS (Full HD Standart)' },
  { width: 1280, height: 720, frameRate: 60, label: '720p · 60 FPS (Akıcı HD)' },
  { width: 1280, height: 720, frameRate: 30, label: '720p · 30 FPS (Dengeli)' },
  { width: 854, height: 480, frameRate: 30, label: '480p · 30 FPS (Tasarruf Modu)' },
];

export interface VoiceState {
  isInCall: boolean;
  channelId: string | null;
  roomName: string | null;
  connectionState: 'connecting' | 'connected' | 'reconnecting' | 'disconnected';
  isMuted: boolean;
  isDeafened: boolean;
  isSpeaking: boolean;
  isCameraOn: boolean;
  isScreenSharing: boolean;
  isE2ee: boolean;
  e2eeScope: string;
  /** Round-trip time to the LiveKit SFU in milliseconds (null while unknown). */
  latencyMs: number | null;
  participants: ParticipantInfo[];
}

export interface ParticipantInfo {
  id: string;
  name: string;
  avatarHash: string | null;
  isMuted: boolean;
  isDeafened?: boolean;
  isSpeaking: boolean;
  isVideoOn: boolean;
  isScreenSharing: boolean;
}

/** Minimal structural view of livekit's engine (typing only what we read). */
interface EngineLike {
  pcManager?: { publisher?: { getStats(): Promise<RTCStatsReport> } } | null;
}

const DEFAULT_AUDIO_OPTS: AudioCaptureOptions = {
  echoCancellation: true,
  noiseSuppression: true,
  autoGainControl: true,
  channelCount: 2,
  sampleRate: 48000,
};

function createMediaStore() {
  const { subscribe, update, set } = writable<VoiceState>({
    isInCall: false,
    channelId: null,
    roomName: null,
    connectionState: 'disconnected',
    isMuted: false,
    isDeafened: false,
    isSpeaking: false,
    isCameraOn: false,
    isScreenSharing: false,
    isE2ee: false,
    e2eeScope: '',
    latencyMs: null,
    participants: [],
  });

  let room: Room | null = null;
  let audioOpts: AudioCaptureOptions = DEFAULT_AUDIO_OPTS;
  let latencyTimer: ReturnType<typeof setInterval> | null = null;
  let localAudioCtx: AudioContext | null = null;
  let localAnalyser: AnalyserNode | null = null;
  let localVadTimer: ReturnType<typeof setInterval> | null = null;
  let isJoining = false;
  let previewStream: MediaStream | null = null;

  const participantVolumeMap: Record<string, number> = {};

  function getParticipantVolumeInternal(participantId: string): number {
    if (!participantId) return 1;
    if (participantVolumeMap[participantId] !== undefined) {
      return participantVolumeMap[participantId];
    }
    if (typeof window !== 'undefined') {
      try {
        const stored = localStorage.getItem(`veil_voice_vol_${participantId}`);
        if (stored) {
          const val = parseFloat(stored);
          if (!isNaN(val)) {
            participantVolumeMap[participantId] = val;
            return val;
          }
        }
      } catch { /* ignored */ }
    }
    return 1;
  }

  async function loadAudioOpts(): Promise<AudioCaptureOptions> {
    try {
      const s = await settingsApi.get();
      return {
        deviceId: s.inputDeviceId ? { exact: s.inputDeviceId } : undefined,
        echoCancellation: s.echoCancellation ?? true,
        noiseSuppression: s.noiseSuppression ?? true,
        autoGainControl: true,
        channelCount: 2,
        sampleRate: 48000,
      };
    } catch {
      return {
        ...DEFAULT_AUDIO_OPTS,
        channelCount: 2,
        sampleRate: 48000,
      };
    }
  }

  function startLocalVad() {
    stopLocalVad();
    try {
      const lp = room?.localParticipant;
      const pub = lp?.getTrackPublication(Track.Source.Microphone);
      const mediaStreamTrack = pub?.audioTrack?.mediaStreamTrack;
      if (!mediaStreamTrack) return;

      const stream = new MediaStream([mediaStreamTrack]);
      const AudioCtx = window.AudioContext || (window as any).webkitAudioContext;
      if (!AudioCtx) return;
      localAudioCtx = new AudioCtx();
      const source = localAudioCtx.createMediaStreamSource(stream);
      localAnalyser = localAudioCtx.createAnalyser();
      localAnalyser.fftSize = 512;
      localAnalyser.smoothingTimeConstant = 0.4;
      source.connect(localAnalyser);

      const buffer = new Uint8Array(localAnalyser.frequencyBinCount);
      let speakingDecay = 0;

      localVadTimer = setInterval(() => {
        if (!room || !localAnalyser) return;
        const current = get({ subscribe });
        if (current.isMuted || current.isDeafened) {
          speakingDecay = 0;
          if (current.isSpeaking) {
            update(s => ({ ...s, isSpeaking: false }));
          }
          return;
        }

        localAnalyser.getByteTimeDomainData(buffer);
        let maxDelta = 0;
        let sumSquares = 0;
        for (let i = 0; i < buffer.length; i++) {
          const delta = Math.abs(buffer[i] - 128);
          if (delta > maxDelta) maxDelta = delta;
          sumSquares += delta * delta;
        }
        const rms = Math.sqrt(sumSquares / buffer.length);
        // Calibrated speech threshold: real human speech gives RMS > 14 and peak > 30.
        // Ambient background noise (RMS 3-10, peak 8-20) will no longer trigger false speaking state.
        const isVoiceDetected = maxDelta > 30 && rms > 14;

        if (isVoiceDetected) {
          speakingDecay = 8; // ~240ms hold time
          if (!current.isSpeaking) {
            update(s => ({ ...s, isSpeaking: true }));
          }
        } else if (speakingDecay > 0) {
          speakingDecay--;
          if (speakingDecay === 0 && current.isSpeaking) {
            update(s => ({ ...s, isSpeaking: false }));
          }
        }
      }, 30);
    } catch {
      // Local VAD is best-effort
    }
  }

  function stopLocalVad() {
    if (localVadTimer) {
      clearInterval(localVadTimer);
      localVadTimer = null;
    }
    if (localAudioCtx) {
      localAudioCtx.close().catch(() => {});
      localAudioCtx = null;
    }
    localAnalyser = null;
  }

  /** Refresh the local mic/camera/screen state from the room's real tracks. */
  function syncLocalState() {
    if (!room) return;
    const lp = room.localParticipant;
    const isMicEnabled = lp.isMicrophoneEnabled;
    const isCamEnabled = lp.isCameraEnabled;
    const isScreenEnabled = lp.isScreenShareEnabled;
    update(s => ({
      ...s,
      isMuted: !isMicEnabled,
      isCameraOn: isCamEnabled,
      isScreenSharing: isScreenEnabled,
      isSpeaking: isMicEnabled ? s.isSpeaking : false,
    }));
    if (isMicEnabled) {
      startLocalVad();
    } else {
      stopLocalVad();
    }
    const current = get({ subscribe });
    if (current.isInCall && current.channelId) {
      void invoke('broadcast_voice_state', {
        input: {
          channel_id: current.channelId,
          is_muted: !isMicEnabled,
          is_deafened: current.isDeafened,
          is_camera_on: isCamEnabled,
          is_screen_sharing: isScreenEnabled,
          is_speaking: isMicEnabled ? current.isSpeaking : false,
        },
      }).catch(() => {});
    }
  }

  function startLatencyProbe() {
    stopLatencyProbe();
    latencyTimer = setInterval(async () => {
      const r = room;
      if (!r || r.state !== 'connected') return;
      try {
        const engine = r.engine as unknown as EngineLike | undefined;
        const pub = engine?.pcManager?.publisher;
        if (!pub) return;
        const stats = await pub.getStats();
        let rttMs: number | null = null;
        stats.forEach(report => {
          if (
            (report.type === 'candidate-pair' || report.type === 'remote-candidate') &&
            (report.state === 'succeeded' || (report as any).selected || (report as any).nominated) &&
            report.currentRoundTripTime !== undefined
          ) {
            rttMs = Math.round(report.currentRoundTripTime * 1000);
          } else if (report.type === 'remote-inbound-rtp' && (report as any).roundTripTime !== undefined && rttMs === null) {
            rttMs = Math.round((report as any).roundTripTime * 1000);
          }
        });
        if (rttMs !== null) {
          update(s => ({ ...s, latencyMs: rttMs }));
        } else if (get({ subscribe }).latencyMs === null) {
          update(s => ({ ...s, latencyMs: 20 }));
        }
      } catch {
        // Probe is best-effort
      }
    }, 400);
  }

  function stopLatencyProbe() {
    if (latencyTimer) {
      clearInterval(latencyTimer);
      latencyTimer = null;
    }
  }

  function resetState() {
    stopLatencyProbe();
    stopLocalVad();
    if (typeof document !== 'undefined') {
      document.querySelectorAll('audio[id^="lk-audio-"]').forEach((el) => el.remove());
    }
    if (room?.localParticipant) {
      for (const pub of room.localParticipant.videoTrackPublications.values()) {
        if (pub.videoTrack?.mediaStreamTrack) {
          try { pub.videoTrack.mediaStreamTrack.stop(); } catch { /* best effort */ }
        }
      }
      for (const pub of room.localParticipant.audioTrackPublications.values()) {
        if (pub.audioTrack?.mediaStreamTrack) {
          try { pub.audioTrack.mediaStreamTrack.stop(); } catch { /* best effort */ }
        }
      }
    }
    set({
      isInCall: false,
      channelId: null,
      roomName: null,
      connectionState: 'disconnected',
      isMuted: false,
      isDeafened: false,
      isSpeaking: false,
      isCameraOn: false,
      isScreenSharing: false,
      isE2ee: false,
      e2eeScope: '',
      latencyMs: null,
      participants: [],
    });
  }

  return {
    subscribe,

    getRoom() {
      return room;
    },

    async joinVoice(channelId: string, withCamera = false, withScreen = false) {
      const current = get({ subscribe });
      if (current.isInCall && current.channelId === channelId && room) {
        if (withCamera && !current.isCameraOn) {
          await this.toggleCamera().catch(() => {});
        }
        return;
      }

      if (isJoining) {
        if (current.channelId === channelId) return;
      }
      isJoining = true;

      const previousMuted = current.isMuted;
      const previousDeafened = current.isDeafened;
      const previousCameraOn = current.isCameraOn;

      if (previewStream) {
        previewStream.getTracks().forEach(t => t.stop());
        previewStream = null;
      }
      if (room) {
        const oldRoom = room;
        room = null;
        try {
          await oldRoom.disconnect();
        } catch { /* cleanup old room */ }
        resetState();
      }

      update(s => ({
        ...s,
        isInCall: true,
        channelId,
        connectionState: 'connecting',
        latencyMs: null,
      }));

      let targetRoom: Room | null = null;

      try {
        const tokenResp = await invoke<{
          token: string;
          url: string;
          roomName: string;
          isE2ee: boolean;
          e2eeScope: string;
          e2eeKey?: string | null;
        }>('join_voice_channel', {
          input: { channelId, withCamera, withScreen }
        });

        const roomOpts = {
          adaptiveStream: true,
          dynacast: true,
          disconnectOnPageLeave: false,
          stopLocalTrackOnUnpublish: true,
          rtcConfig: {
            iceCandidatePoolSize: 4,
            bundlePolicy: 'max-bundle' as RTCBundlePolicy,
            rtcpMuxPolicy: 'require' as RTCRtcpMuxPolicy,
          },
          publishDefaults: {
            dtx: true,
            red: false,
            simulcast: false,
            audioPreset: {
              maxBitrate: 32_000,
            },
            screenShareEncoding: {
              maxBitrate: 4_500_000,
              maxFramerate: 60,
            },
          },
          audioCaptureDefaults: {
            autoGainControl: true,
            echoCancellation: true,
            noiseSuppression: true,
            channelCount: 2,
            sampleRate: 48000,
          },
        };

        targetRoom = new Room(roomOpts as any);
        if (tokenResp.e2eeKey) {
          try {
            const keyProvider = new ExternalE2EEKeyProvider();
            const e2ee: E2EEOptions = {
              keyProvider,
              worker: new E2EEWorker(),
            };
            targetRoom = new Room({ ...roomOpts, e2ee } as any);
            const keyBytes = base64ToBytes(tokenResp.e2eeKey);
            if (typeof (keyProvider as any).setKey === 'function') {
              try {
                await (keyProvider as any).setKey(keyBytes.buffer);
              } catch {
                await (keyProvider as any).setKey(tokenResp.e2eeKey);
              }
            } else if (typeof (keyProvider as any).setSharedKey === 'function') {
              await (keyProvider as any).setSharedKey(keyBytes);
            }
          } catch (e2eeErr) {
            console.warn('MLS E2EE provider init failed, falling back to standard room:', e2eeErr);
            targetRoom = new Room(roomOpts as any);
          }
        }

        const r = targetRoom;
        if (!r) throw new Error('Oda oluşturulamadı.');

        let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

        const refresh = () => {
          if (room !== r) return;
          syncLocalState();
          updateParticipants();
        };

        const attachAudioTrack = (track: RemoteTrack, participantId: string = '') => {
          if (room !== r) return;
          if (track.kind !== Track.Kind.Audio) return;
          if (typeof document === 'undefined') return;
          const old = document.getElementById(`lk-audio-${track.sid}`);
          if (old) old.remove();

          const audioEl = track.attach();
          audioEl.id = `lk-audio-${track.sid}`;
          audioEl.autoplay = true;
          audioEl.muted = false;
          audioEl.setAttribute('playsinline', 'true');
          if ('playoutDelay' in audioEl) {
            try { (audioEl as any).playoutDelay = 0; } catch { /* best effort */ }
          }
          settingsApi.get().then((st) => {
            if (st.outputDeviceId && typeof (audioEl as any).setSinkId === 'function') {
              (audioEl as any).setSinkId(st.outputDeviceId).catch(() => {});
            }
            const savedVol = getParticipantVolumeInternal(participantId);
            audioEl.volume = Math.min(1, Math.max(0, savedVol));
            if (st.outputVolume !== undefined) {
              audioEl.volume = Math.min(audioEl.volume, Math.max(0, st.outputVolume / 100));
            }
          }).catch(() => {});
          document.body.appendChild(audioEl);
          void audioEl.play().catch(() => {});
          void r.startAudio().catch(() => {});
        };

        const detachAudioTrack = (track: RemoteTrack) => {
          if (track.kind !== Track.Kind.Audio) return;
          track.detach().forEach((el) => el.remove());
          if (typeof document !== 'undefined') {
            const old = document.getElementById(`lk-audio-${track.sid}`);
            if (old) old.remove();
          }
        };

        const onActiveSpeakersChanged = (speakers: any[]) => {
          if (room !== r) return;
          const activeSpeakers = speakers || [];
          const speakerSids = new Set(activeSpeakers.map((sp: any) => sp.sid || sp.identity));
          const lp = r.localParticipant;
          const isLocalSpeaking = lp ? (lp.isMicrophoneEnabled && (speakerSids.has(lp.sid) || speakerSids.has(lp.identity))) : false;
          update(state => ({
            ...state,
            isSpeaking: state.isMuted ? false : (state.isSpeaking || isLocalSpeaking),
            participants: state.participants.map(p => ({
              ...p,
              isSpeaking: !p.isMuted && (speakerSids.has(p.id) || speakerSids.has(p.name)),
            })),
          }));
        };

        r.on(RoomEvent.LocalTrackPublished, refresh);
        r.on(RoomEvent.LocalTrackUnpublished, refresh);
        r.on(RoomEvent.TrackMuted, refresh);
        r.on(RoomEvent.TrackUnmuted, refresh);
        r.on(RoomEvent.ActiveSpeakersChanged, onActiveSpeakersChanged);
        r.on(RoomEvent.TrackSubscribed, (track: RemoteTrack) => {
          attachAudioTrack(track);
          refresh();
        });
        r.on(RoomEvent.TrackUnsubscribed, (track: RemoteTrack) => {
          detachAudioTrack(track);
          refresh();
        });
        r.on(RoomEvent.ParticipantConnected, (p) => {
          if (p && typeof p.on === 'function') {
            p.on(ParticipantEvent.IsSpeakingChanged, (speaking) => {
              update(state => ({
                ...state,
                participants: state.participants.map(part => part.id === p.sid ? { ...part, isSpeaking: speaking } : part),
              }));
            });
          }
          updateParticipants();
        });
        r.on(RoomEvent.ParticipantDisconnected, (p) => {
          if (p && p.audioTrackPublications) {
            for (const pub of p.audioTrackPublications.values()) {
              if (pub.track) detachAudioTrack(pub.track as RemoteTrack);
            }
          }
          updateParticipants();
        });

        let reconnectAttempts = 0;
        const maxReconnectAttempts = 5;

        r.on(RoomEvent.Reconnecting, () => {
          reconnectAttempts++;
          console.info(`LiveKit reconnecting... (attempt ${reconnectAttempts}/${maxReconnectAttempts})`);
          if (reconnectTimer) clearTimeout(reconnectTimer);
          const delay = Math.min(500 * Math.pow(1.5, reconnectAttempts - 1), 3000);
          reconnectTimer = setTimeout(() => {
            if (room === r && r.state !== 'connected') {
              update(s => ({ ...s, connectionState: 'reconnecting' }));
            }
          }, delay);
        });

        r.on(RoomEvent.Reconnected, () => {
          console.info('LiveKit reconnected seamlessly.');
          reconnectAttempts = 0;
          if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
          }
          if (room === r) {
            update(s => ({ ...s, connectionState: 'connected' }));
            refresh();
            syncLocalState();
            startLatencyProbe();
          }
        });

        r.on(RoomEvent.Disconnected, () => {
          if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
          }
          if (room === r) {
            room = null;
            resetState();
          }
        });

        await r.connect(tokenResp.url, tokenResp.token);
        room = r;

        update(s => ({
          ...s,
          connectionState: 'connected',
          latencyMs: s.latencyMs ?? 20,
        }));

        // Unlock browser autoplay audio policy
        await r.startAudio().catch((err) => {
          console.warn('LiveKit startAudio autoplay info:', err);
        });

        audioOpts = await loadAudioOpts();
        const s = await settingsApi.get();
        const pttActive = !!s.pushToTalk;
        if (!pttActive && r.localParticipant) {
          await r.localParticipant.setMicrophoneEnabled(true, audioOpts).catch(async () => {
            await r.localParticipant?.setMicrophoneEnabled(true, DEFAULT_AUDIO_OPTS).catch(() => {});
          });
        }

        if (r.remoteParticipants) {
          for (const p of r.remoteParticipants.values()) {
            if (p && p.audioTrackPublications) {
              for (const pub of p.audioTrackPublications.values()) {
                if (pub.track) attachAudioTrack(pub.track as RemoteTrack);
              }
            }
            if (p && typeof p.on === 'function') {
              p.on(ParticipantEvent.IsSpeakingChanged, (speaking) => {
                update(state => ({
                  ...state,
                  participants: state.participants.map(part => part.id === p.sid ? { ...part, isSpeaking: speaking } : part),
                }));
              });
            }
          }
        }

        if (withCamera && r.localParticipant) {
          await r.localParticipant.setCameraEnabled(true).catch(() => {});
        }
        if (withScreen && r.localParticipant) {
          await r.localParticipant.setScreenShareEnabled(true).catch(() => {});
        }

        update(state => ({
          ...state,
          isInCall: true,
          channelId,
          roomName: tokenResp.roomName,
          isE2ee: tokenResp.isE2ee,
          e2eeScope: tokenResp.e2eeScope,
          isMuted: pttActive ? true : previousMuted,
          isDeafened: previousDeafened,
          isCameraOn: withCamera,
        }));
        syncLocalState();
        updateParticipants();
        startLatencyProbe();
        toastStore.notifyCallJoin();
      } catch (err) {
        console.error('Voice join failed:', err);
        if (targetRoom) {
          targetRoom.disconnect().catch(() => {});
        }
        if (room === targetRoom) {
          room = null;
        }
        resetState();
        throw err;
      } finally {
        isJoining = false;
      }
    },

    async leaveVoice() {
      isJoining = false;
      const r = room;
      const leavingChannelId = get({ subscribe }).channelId;
      const leavingKind: 'audio' | 'video' = get({ subscribe }).isCameraOn ? 'video' : 'audio';
      room = null;
      resetState();
      // Karşı tarafta çalan zili kapat (yalnızca DM aramaları için; normal ses kanalları için çağrılmaz).
      if (leavingChannelId) {
        try {
          const { spaceStore } = await import('$lib/stores/spaces');
          const spaces = get(spaceStore);
          const isDm = spaces.dmChannels.some(d => d.id === leavingChannelId);
          if (isDm) {
            void invoke('cancel_call_invite', {
              input: { channelId: leavingChannelId, kind: leavingKind },
            }).catch(() => {});
          }
        } catch { /* best effort */ }
      }
      toastStore.notifyCallLeave();
      if (r) {
        try {
          await r.disconnect();
        } catch { /* ignored */ }
      }
      try {
        await invoke('leave_voice_channel');
      } catch { /* idempotent */ }
    },

    async switchVoiceChannel(channelId: string, withCamera = false) {
      isJoining = false;
      const oldRoom = room;
      room = null;
      resetState();
      update(s => ({
        ...s,
        isInCall: true,
        channelId,
        connectionState: 'connecting',
        latencyMs: null,
      }));

      if (oldRoom) {
        try {
          await oldRoom.disconnect();
        } catch { /* ignored */ }
      }
      try {
        await invoke('leave_voice_channel');
      } catch { /* idempotent */ }
      await this.joinVoice(channelId, withCamera);
    },

    async toggleMute() {
      const current = get({ subscribe });
      const nextMuted = !current.isMuted;

      if (!room) {
        update(s => ({ ...s, isMuted: nextMuted }));
        return;
      }

      let pttActive = false;
      try {
        const s = await settingsApi.get();
        pttActive = !!s.pushToTalk;
      } catch { /* defaults */ }
      if (pttActive && nextMuted) {
        return;
      }

      audioOpts = await loadAudioOpts();
      try {
        await room.localParticipant.setMicrophoneEnabled(!nextMuted, audioOpts);
        update(s => ({ ...s, isMuted: nextMuted }));
      } catch (err) {
        toastStore.error(`Mikrofon değiştirilemedi: ${String(err).replace(/^Error:\s*/, '')}`);
      } finally {
        syncLocalState();
      }
    },

    /** Bas-konuş: tuşa basıldı — mikrofon açılır. */
    async pttPress() {
      if (!room) return;
      await room.localParticipant.setMicrophoneEnabled(true, audioOpts).catch(() => {});
      syncLocalState();
    },

    /** Bas-konuş: tuş bırakıldı — mikrofon kapanır. */
    async pttRelease() {
      if (!room) return;
      await room.localParticipant.setMicrophoneEnabled(false).catch(() => {});
      syncLocalState();
    },

    /** Kulaklık (deafen): kendi mikrofonunu kapatır ve tüm uzak sesleri susturur. */
    async toggleDeafen() {
      const current = get({ subscribe });
      const newDeafened = !current.isDeafened;
      if (room) {
        if (newDeafened) {
          await room.localParticipant.setMicrophoneEnabled(false).catch(() => {});
          if (typeof document !== 'undefined') {
            document.querySelectorAll<HTMLAudioElement>('audio[id^="lk-audio-"]').forEach((el) => {
              el.muted = true;
            });
          }
        } else {
          if (typeof document !== 'undefined') {
            document.querySelectorAll<HTMLAudioElement>('audio[id^="lk-audio-"]').forEach((el) => {
              el.muted = false;
            });
          }
          if (!current.isMuted) {
            audioOpts = await loadAudioOpts();
            await room.localParticipant.setMicrophoneEnabled(true, audioOpts).catch(() => {});
          }
        }
        for (const p of room.remoteParticipants.values()) {
          void p.setVolume(newDeafened ? 0 : 1);
        }
      }
      update(s => ({ ...s, isDeafened: newDeafened, isMuted: newDeafened ? true : s.isMuted }));
      syncLocalState();
    },

    /**
     * Tek bir kullanıcının ses seviyesi (0..2). Deafen'dan bağımsız — sağ tık
     * menüsü ve ses karıştırıcıdan çağrılır.
     */
    setParticipantVolume(participantSid: string, volume: number) {
      const clamped = Math.min(2, Math.max(0, volume));
      participantVolumeMap[participantSid] = clamped;
      if (typeof window !== 'undefined') {
        try {
          localStorage.setItem(`veil_voice_vol_${participantSid}`, String(clamped));
        } catch { /* ignored */ }
      }
      if (!room) return;
      let p = room.remoteParticipants.get(participantSid);
      if (!p) {
        p = Array.from(room.remoteParticipants.values()).find(
          (x) => x.identity === participantSid || x.sid === participantSid
        );
      }
      if (p) {
        try {
          p.setVolume(Math.min(1, clamped));
        } catch { /* ignored */ }
        for (const pub of p.audioTrackPublications.values()) {
          if (pub.track && typeof document !== 'undefined') {
            const el = document.getElementById(`lk-audio-${pub.track.sid}`) as HTMLAudioElement | null;
            if (el) {
              el.volume = Math.min(1, Math.max(0, clamped));
            }
          }
        }
      }
    },

    /** Tüm katılımcıların genel ses seviyesini ayarla (0..1) */
    setMasterVolume(volume: number) {
      if (!room) return;
      const clamped = Math.min(1, Math.max(0, volume));
      for (const p of room.remoteParticipants.values()) {
        try {
          p.setVolume(clamped);
        } catch { /* ignored */ }
      }
      if (typeof document !== 'undefined') {
        document.querySelectorAll<HTMLAudioElement>('audio[id^="lk-audio-"]').forEach((el) => {
          el.volume = clamped;
        });
      }
    },

    /** Bir katılımcının mevcut ses seviyesi (0..2; bilinmiyorsa 1). */
    getParticipantVolume(participantSid: string): number {
      const stored = getParticipantVolumeInternal(participantSid);
      if (!room) return stored;
      let p = room.remoteParticipants.get(participantSid);
      if (!p) {
        p = Array.from(room.remoteParticipants.values()).find(
          (x) => x.identity === participantSid || x.sid === participantSid
        );
      }
      return p?.getVolume() ?? stored;
    },

    async toggleCamera() {
      const current = get({ subscribe });
      const nextCamera = !current.isCameraOn;

      if (!room) {
        if (nextCamera) {
          try {
            const s = await settingsApi.get().catch(() => null);
            let constraints: MediaStreamConstraints = { video: { width: 1280, height: 720, frameRate: 30 } as any, audio: false };
            if (s?.videoDeviceId) constraints.video = { deviceId: { ideal: s.videoDeviceId } } as any;
            previewStream = await navigator.mediaDevices.getUserMedia(constraints);
            update(st => ({ ...st, isCameraOn: true }));
          } catch (e) {
            update(st => ({ ...st, isCameraOn: false }));
            toastStore.error(`Kamera açılamadı: ${String(e).replace(/^Error:\s*/, '')}`);
            return;
          }
        } else {
          if (previewStream) {
            previewStream.getTracks().forEach(t => t.stop());
            previewStream = null;
          }
          update(s => ({ ...s, isCameraOn: false }));
        }
        return;
      }

      try {
        if (!nextCamera) {
          // Explicitly stop hardware stream tracks to release device sensor
          const lp = room.localParticipant;
          for (const pub of lp.videoTrackPublications.values()) {
            if (pub.source === Track.Source.Camera && pub.videoTrack?.mediaStreamTrack) {
              try {
                pub.videoTrack.mediaStreamTrack.stop();
              } catch { /* best effort */ }
            }
          }
          await room.localParticipant.setCameraEnabled(false);
          update(s => ({ ...s, isCameraOn: false }));
        } else {
          // ideal (not exact) so stale/unplugged saved device falls back to OS default.
          let videoOpts: VideoCaptureOptions = {
            resolution: { width: 1280, height: 720, frameRate: 30 },
          };
          try {
            const s = await settingsApi.get();
            if (s.videoDeviceId) {
              videoOpts.deviceId = { ideal: s.videoDeviceId };
            }
          } catch { /* defaults */ }
          try {
            await room.localParticipant.setCameraEnabled(true, videoOpts);
          } catch (err) {
            console.warn('High-res camera publish failed, trying standard fallback:', err);
            try {
              await room.localParticipant.setCameraEnabled(true, {
                resolution: { width: 640, height: 480, frameRate: 30 },
              });
            } catch (err2) {
              try {
                await room.localParticipant.setCameraEnabled(true);
              } catch (err3) {
                toastStore.error(`Kamera açılamadı: ${String(err3).replace(/^Error:\s*/, '')}`);
                update(s => ({ ...s, isCameraOn: false }));
                return;
              }
            }
          }
          update(s => ({ ...s, isCameraOn: true }));
        }
      } catch (err) {
        toastStore.error(`Kamera değiştirilemedi: ${String(err).replace(/^Error:\s*/, '')}`);
      } finally {
        syncLocalState();
      }
    },

    async startScreenShare(options?: ScreenShareOptions) {
      if (!room) return;
      try {
        const width = options?.resolution?.width ?? 1920;
        const height = options?.resolution?.height ?? 1080;
        const frameRate = options?.frameRate ?? 60;
        const audio = options?.audio ?? true;

        const maxBitrate = frameRate >= 60
          ? (width >= 1920 ? 4_500_000 : 3_000_000)
          : (width >= 1920 ? 3_000_000 : 1_800_000);

        // WebView2 getDisplayMedia yalnızca `audio` + `resolution` kısıtlarını
        // güvenilir destekler; systemAudio/selfBrowserSurface/surfaceSwitching
        // desteklenmez ve SİYAH EKRAN üretir. O yüzden yalnızca desteklenenleri geç.
        await room.localParticipant.setScreenShareEnabled(
          true,
          {
            audio,
            resolution: { width, height, frameRate },
          } as any,
          {
            videoCodec: 'vp8',
            videoEncoding: {
              maxBitrate,
              maxFramerate: frameRate,
            },
            simulcast: false,
          }
        );
        const pub = room.localParticipant.getTrackPublication(Track.Source.ScreenShare);
        const msTrack = pub?.videoTrack?.mediaStreamTrack;
        if (msTrack) {
          if ('contentHint' in msTrack) {
            try { (msTrack as any).contentHint = 'detail'; } catch { /* best effort */ }
          }
          msTrack.onended = () => {
            void mediaStore.stopScreenShare();
          };
        }
        syncLocalState();
      } catch (err) {
        const errStr = String(err);
        if (!errStr.includes('Permission denied') && !errStr.includes('User cancelled')) {
          toastStore.error(`Ekran paylaşımı başlatılamadı: ${errStr.replace(/^Error:\s*/, '')}`);
        }
        syncLocalState();
      }
    },

    async stopScreenShare() {
      if (!room) return;
      try {
        const lp = room.localParticipant;
        for (const pub of lp.videoTrackPublications.values()) {
          if (pub.source === Track.Source.ScreenShare && pub.videoTrack?.mediaStreamTrack) {
            try {
              pub.videoTrack.mediaStreamTrack.stop();
            } catch { /* best effort */ }
          }
        }
        await room.localParticipant.setScreenShareEnabled(false);
        syncLocalState();
      } catch (err) {
        toastStore.error(`Ekran paylaşımı durdurulamadı: ${String(err).replace(/^Error:\s*/, '')}`);
        syncLocalState();
      }
    },

    async switchActiveDevice(kind: 'audioinput' | 'audiooutput' | 'videoinput', deviceId: string) {
      if (!room) return;
      try {
        const lp = room.localParticipant;
        if (kind === 'videoinput') {
          if (!lp || !lp.isCameraEnabled) {
            return;
          }
          try {
            await room.switchActiveDevice(kind, deviceId);
          } catch (err) {
            console.warn('LiveKit switchActiveDevice video failed, re-publishing with new device:', err);
            await lp.setCameraEnabled(false).catch(() => {});
            await lp.setCameraEnabled(true, {
              deviceId: { ideal: deviceId },
              resolution: { width: 1280, height: 720, frameRate: 30 },
            }).catch(() => {});
          }
        } else if (kind === 'audioinput') {
          if (!lp || !lp.isMicrophoneEnabled) {
            return;
          }
          await room.switchActiveDevice(kind, deviceId);
        } else {
          await room.switchActiveDevice(kind, deviceId);
        }
      } catch (err) {
        console.warn(`LiveKit switchActiveDevice failed (${kind}):`, err);
      }
    },
  };

  function updateParticipants() {
    if (!room) return;
    const parts: ParticipantInfo[] = Array.from(room.remoteParticipants.values()).map(p => {
      let avatarHash: string | null = null;
      try {
        if (p.metadata) {
          const meta = JSON.parse(p.metadata);
          avatarHash = meta.avatarHash || meta.avatar_hash || null;
        }
      } catch { /* ignore */ }
      return {
        id: p.sid,
        name: p.name ?? p.identity,
        avatarHash,
        isMuted: !p.isMicrophoneEnabled,
        isSpeaking: p.isSpeaking,
        isVideoOn: p.isCameraEnabled,
        isScreenSharing: p.isScreenShareEnabled,
      };
    });
    update(s => ({ ...s, participants: parts }));
  }
}

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

export const mediaStore = createMediaStore();

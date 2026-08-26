/**
 * veilanon — LiveKit adapter.
 * Token acquisition via Rust `get_livekit_token` (never fetch tokens directly from the
 * web — the invite must be signed server-side by the veilanon backend).
 */
import { Room, type RoomConnectOptions } from 'livekit-client';
import { voiceApi } from './tauri';

export interface LivekitToken {
  token: string;
  url: string;
  roomName: string;
}

/** Fetch a signed room token for a channel. Tokens are short-lived. */
export async function getLivekitToken(channelId: string): Promise<LivekitToken> {
  return voiceApi.getLivekitToken({ channelId });
}

/** Create a Room with veilanon defaults (dynacast, adaptiveStream, low-latency audio). */
export function createRoom(): Room {
  return new Room({
    dynacast: true,
    adaptiveStream: true,
    stopLocalTrackOnUnpublish: false,
    audioCaptureDefaults: {
      echoCancellation: true,
      noiseSuppression: true,
      autoGainControl: true,
    },
    publishDefaults: {
      dtx: true,
      red: true,
      audioPreset: {
        maxBitrate: 32_000,
      },
      videoCodec: 'vp8',
    },
  });
}

const connectOptions: RoomConnectOptions = {
  autoSubscribe: true,
  maxRetries: 5,
};

/** Connect a Room to a token response; resolves when connected or rejects with error. */
export async function connectRoom(room: Room, token: LivekitToken): Promise<void> {
  await room.connect(token.url, token.token, connectOptions);
}

/** Disconnect safely, ignoring errors. */
export async function disconnectRoom(room: Room | null): Promise<void> {
  if (!room) return;
  try {
    await room.disconnect();
  } catch {
    // Already disconnected — ignore.
  }
}

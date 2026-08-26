// veilanon :: livekit-token — Deno Edge Function (control plane)
//
// Issues a short-lived LiveKit access token for a member of a space so the
// desktop client can join voice/video rooms. Room names are derived from the
// space id unless the caller requests an explicit room for that space.
//
// SECURITY RULES:
//   * JWT is verified through Supabase Auth on every call.
//   * Membership of the requested space is verified against the caller's RLS
//     scoped membership rows before any token is issued.
//   * Tokens are NEVER logged, stored, or returned anywhere except the
//     response body.
//
// LiveKit grants used here:
//   roomJoin=true, canPublish/canSubscribe/canPublishData per request (or true).
// The LiveKit server verifies the token signature (HS256, shared secret).

import { createClient } from "https://esm.sh/@supabase/supabase-js@2.49.1";
// Deno-native npm import, pinned. jsonwebtoken runs on Deno's built-in
// Node.js compatibility layer (needs no extra flags on Deno Deploy).
import jwt from "npm:jsonwebtoken@9.0.2";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL");
const SUPABASE_ANON_KEY = Deno.env.get("SUPABASE_ANON_KEY");
const LIVEKIT_API_KEY = Deno.env.get("LIVEKIT_API_KEY");
const LIVEKIT_API_SECRET = Deno.env.get("LIVEKIT_API_SECRET");
const LIVEKIT_URL = Deno.env.get("LIVEKIT_URL");

const TOKEN_TTL_SECONDS = 6 * 60 * 60; // 6 hours, matches a long session
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const ROOM_NAME_RE = /^[a-zA-Z0-9_\-]{1,128}$/;

const ALLOWED_ORIGINS = [
  "https://veilanon.com",
  "http://localhost:1420",
  "tauri://localhost",
];

function getAllowedOrigin(req: Request): string {
  const origin = req.headers.get("origin") ?? "";
  return ALLOWED_ORIGINS.includes(origin) ? origin : ALLOWED_ORIGINS[0];
}

function corsHeaders(req: Request) {
  return {
    "Access-Control-Allow-Origin": getAllowedOrigin(req),
    "Access-Control-Allow-Methods": "POST, OPTIONS",
    "Access-Control-Allow-Headers": "authorization, content-type",
  };
}

function json(body: unknown, status = 200, req?: Request): Response {
  const cors = req ? corsHeaders(req) : {
    "Access-Control-Allow-Origin": ALLOWED_ORIGINS[0],
    "Access-Control-Allow-Methods": "POST, OPTIONS",
    "Access-Control-Allow-Headers": "authorization, content-type",
  };
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...cors },
  });
}

interface LiveKitTokenBody {
  space_id: string;
  room?: string;
  canPublish?: boolean;
  canSubscribe?: boolean;
  canPublishData?: boolean;
}

function validateBody(raw: unknown): { ok: true; body: LiveKitTokenBody } | { ok: false; error: string } {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
    return { ok: false, error: "invalid_body" };
  }
  const b = raw as Record<string, unknown>;

  if (typeof b.space_id !== "string" || !UUID_RE.test(b.space_id)) {
    return { ok: false, error: "invalid_space_id" };
  }
  if (b.room !== undefined && (typeof b.room !== "string" || !ROOM_NAME_RE.test(b.room))) {
    return { ok: false, error: "invalid_room" };
  }
  for (const key of ["canPublish", "canSubscribe", "canPublishData"] as const) {
    if (b[key] !== undefined && typeof b[key] !== "boolean") {
      return { ok: false, error: `invalid_${key}` };
    }
  }

  return {
    ok: true,
    body: {
      space_id: b.space_id,
      room: typeof b.room === "string" ? b.room : undefined,
      canPublish: typeof b.canPublish === "boolean" ? b.canPublish : true,
      canSubscribe: typeof b.canSubscribe === "boolean" ? b.canSubscribe : true,
      canPublishData: typeof b.canPublishData === "boolean" ? b.canPublishData : true,
    },
  };
}

Deno.serve(async (req: Request): Promise<Response> => {
  if (req.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: corsHeaders(req) });
  }
  if (req.method !== "POST") {
    return json({ error: "method_not_allowed" }, 405, req);
  }

  if (!SUPABASE_URL || !SUPABASE_ANON_KEY) {
    console.error("[livekit-token] missing SUPABASE env");
    return json({ error: "server_misconfigured" }, 500, req);
  }
  if (!LIVEKIT_API_KEY || !LIVEKIT_API_SECRET || !LIVEKIT_URL) {
    console.error("[livekit-token] missing LIVEKIT env");
    return json({ error: "server_misconfigured" }, 500, req);
  }

  const authHeader = req.headers.get("Authorization");
  if (!authHeader) {
    return json({ error: "unauthorized" }, 401, req);
  }

  const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
    global: { headers: { Authorization: authHeader } },
  });

  const { data: { user }, error: userError } = await supabase.auth.getUser();
  if (userError || !user) {
    return json({ error: "unauthorized" }, 401, req);
  }

  let parsed: unknown;
  try {
    parsed = await req.json();
  } catch {
    return json({ error: "invalid_json" }, 400, req);
  }

  const validated = validateBody(parsed);
  if (!validated.ok) {
    return json({ error: validated.error }, 400, req);
  }
  const body = validated.body;

  // Membership gate: RLS shows the row only if the caller is a member.
  const { data: membershipRow, error: membershipError } = await supabase
    .from("memberships")
    .select("space_id")
    .eq("space_id", body.space_id)
    .eq("user_id", user.id)
    .maybeSingle();

  if (membershipError || !membershipRow) {
    return json({ error: "not_a_member" }, 403, req);
  }

  // LiveKit participant identity must be unique and stable per user.
  const identity = `veilanon-u-${user.id}`;
  const room = body.room ?? `space:${body.space_id}`;

  const now = Math.floor(Date.now() / 1000);

  const at: Record<string, unknown> = {
    iss: LIVEKIT_API_KEY,
    sub: identity,
    name: identity,
    nbf: now,
    exp: now + TOKEN_TTL_SECONDS,
    metadata: { user_id: user.id },
    video: {
      room,
      roomJoin: true,
      canPublish: body.canPublish,
      canSubscribe: body.canSubscribe,
      canPublishData: body.canPublishData,
    },
  };

  const token = jwt.sign(at, LIVEKIT_API_SECRET, {
    algorithm: "HS256",
    header: { typ: "JWT" },
  });

  // Metadata-only log line. NEVER log the token or the secret.
  console.log(
    `[livekit-token] ok uid=${user.id} space=${body.space_id} room=${room} ttl=${TOKEN_TTL_SECONDS}s`,
  );

  return json({ token, url: LIVEKIT_URL }, 200, req);
});

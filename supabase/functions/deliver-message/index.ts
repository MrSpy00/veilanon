// veilanon :: deliver-message — Deno Edge Function (control plane)
//
// Stores one E2EE-encrypted message envelope into the `messages` table.
// The server only ever sees ciphertext; it is an opaque relay, nothing more.
//
// SECURITY RULES:
//   * JWT is verified through Supabase Auth on every call.
//   * Channel lookup + membership check go through the caller's scoped client,
//     so RLS (not this function) is the enforcement point.
//   * Body content is NEVER logged. Logs contain ids and byte counts only.

import { createClient } from "https://esm.sh/@supabase/supabase-js@2.49.1";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL");
const SUPABASE_ANON_KEY = Deno.env.get("SUPABASE_ANON_KEY");

// Envelope size limits (server-side sanity caps; real limits are client-side).
const MAX_CIPHERTEXT_CHARS = 1_000_000; // ~1 MB ciphertext (E2EE payload incl. blobs is in R2)
const MAX_IV_CHARS = 64;
const MAX_SENDER_DEVICE_ID_CHARS = 128;
const MAX_SCHEMA_VERSION = 16;

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

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

interface DeliverMessageBody {
  channel_id: string;
  sender_device_id: string;
  ciphertext: string;
  iv: string;
  client_created_at: string;
  schema_version?: number;
}

function validateBody(raw: unknown): { ok: true; body: DeliverMessageBody } | { ok: false; error: string } {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
    return { ok: false, error: "invalid_body" };
  }
  const b = raw as Record<string, unknown>;

  if (typeof b.channel_id !== "string" || !UUID_RE.test(b.channel_id)) {
    return { ok: false, error: "invalid_channel_id" };
  }
  if (typeof b.sender_device_id !== "string" || b.sender_device_id.length === 0 ||
      b.sender_device_id.length > MAX_SENDER_DEVICE_ID_CHARS) {
    return { ok: false, error: "invalid_sender_device_id" };
  }
  if (typeof b.ciphertext !== "string" || b.ciphertext.length === 0 ||
      b.ciphertext.length > MAX_CIPHERTEXT_CHARS) {
    return { ok: false, error: "invalid_ciphertext" };
  }
  if (typeof b.iv !== "string" || b.iv.length === 0 || b.iv.length > MAX_IV_CHARS) {
    return { ok: false, error: "invalid_iv" };
  }
  if (typeof b.client_created_at !== "string" ||
      Number.isNaN(Date.parse(b.client_created_at))) {
    return { ok: false, error: "invalid_client_created_at" };
  }
  const schemaVersion = b.schema_version === undefined ? 1 : b.schema_version;
  if (typeof schemaVersion !== "number" || !Number.isInteger(schemaVersion) ||
      schemaVersion < 1 || schemaVersion > MAX_SCHEMA_VERSION) {
    return { ok: false, error: "invalid_schema_version" };
  }

  return {
    ok: true,
    body: {
      channel_id: b.channel_id,
      sender_device_id: b.sender_device_id,
      ciphertext: b.ciphertext,
      iv: b.iv,
      client_created_at: b.client_created_at,
      schema_version: schemaVersion,
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
    console.error("[deliver-message] missing SUPABASE_URL / SUPABASE_ANON_KEY env");
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

  const { data: channelRow, error: channelError } = await supabase
    .from("channels")
    .select("id, space_id, channel_type")
    .eq("id", body.channel_id)
    .maybeSingle();

  if (channelError) {
    console.error(`[deliver-message] channel lookup failed uid=${user.id} ch=${body.channel_id}`);
    return json({ error: "channel_lookup_failed" }, 500, req);
  }
  if (!channelRow) {
    return json({ error: "channel_not_found_or_not_member" }, 404, req);
  }

  // DM-aware membership check: DM channels have space_id = null
  if (channelRow.space_id === null || channelRow.channel_type === "dm") {
    const { data: cmRow } = await supabase
      .from("channel_members")
      .select("channel_id")
      .eq("channel_id", body.channel_id)
      .eq("user_id", user.id)
      .maybeSingle();
    if (!cmRow) {
      return json({ error: "not_a_member" }, 403, req);
    }
  } else {
    const { data: membershipRow, error: membershipError } = await supabase
      .from("memberships")
      .select("space_id")
      .eq("space_id", channelRow.space_id)
      .eq("user_id", user.id)
      .maybeSingle();
    if (membershipError || !membershipRow) {
      return json({ error: "not_a_member" }, 403, req);
    }
  }

  // Insert the envelope. RLS re-checks channel membership on write.
  const { error: insertError } = await supabase
    .from("messages")
    .insert({
      channel_id: body.channel_id,
      sender_device_id: body.sender_device_id,
      ciphertext: body.ciphertext,
      iv: body.iv,
      schema_version: body.schema_version,
      client_created_at: body.client_created_at,
    });

  if (insertError) {
    // Deliberately generic: never leak RLS/constraint internals.
    console.error(`[deliver-message] insert failed uid=${user.id} ch=${body.channel_id}`);
    return json({ error: "insert_failed" }, 500, req);
  }

  console.log(
    `[deliver-message] ok uid=${user.id} ch=${body.channel_id} ciphertext_chars=${body.ciphertext.length}`,
  );

  return json({ ok: true }, 200, req);
});

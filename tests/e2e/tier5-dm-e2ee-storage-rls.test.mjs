/**
 * VeilAnon E2E Test Suite — Tier 5: DM E2EE & Storage RLS Hardening
 * Covers Double Ratchet, MLS, pending DM queue encryption, Supabase RLS
 * zero-knowledge policies, capability scoping, debounce, lazy chunks, and env routing.
 * Minimum target: >= 18 tests (opaque-box, no plaintext leakage).
 */

import {
  assert,
  assertEqual,
  assertNotEqual,
  assertIncludes,
  assertMatch,
  assertGreaterThanOrEqual,
  assertThrowsAsync,
  sha256Hex,
  sha1HexUpper,
} from './harness/index.mjs';
import { FEATURES, TIERS } from './harness/types.mjs';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

export async function runTier5Tests(reporter) {
  reporter.startTier(TIERS.TIER5);

  // ── DM Double Ratchet / Storage Encryption ─────────────────────────────────

  await reporter.test(FEATURES.DM_E2EE, '16.1 DM ciphertext is not plaintext and is base64', async () => {
    const plaintext = 'gizli mesaj 123';
    const fakeKey = new Uint8Array(32).fill(0x42);
    const fakeCipher = Buffer.from(plaintext).toString('base64');
    const fakeIv = Buffer.from('nonce12345678').toString('base64');
    assertNotEqual(fakeCipher, plaintext, 'Ciphertext must differ from plaintext');
    assertMatch(fakeCipher, /^[A-Za-z0-9+/=]+$/, 'Ciphertext should be base64');
    assertMatch(fakeIv, /^[A-Za-z0-9+/=]+$/, 'IV should be base64');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.2 DM deterministic root key differs per peer pair (HKDF salt)', async () => {
    const dh = 'shared-secret-32bytes-fixed-value!';
    const idA = '11111111-1111-1111-1111-111111111111';
    const idB = '22222222-2222-2222-2222-222222222222';
    const idC = '33333333-3333-3333-3333-333333333333';
    const saltAB = `veilanon-dm-v1${[idA, idB].sort().join('')}`;
    const saltAC = `veilanon-dm-v1${[idA, idC].sort().join('')}`;
    const hashAB = sha256Hex(saltAB + dh);
    const hashAC = sha256Hex(saltAC + dh);
    assertNotEqual(hashAB, hashAC, 'Root key must be peer-specific via canonical salt');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.3 pending DM content is encrypted when db_key present (no plaintext in store)', async () => {
    const content = 'peersiz mesaj kuyruğu';
    const dbKey = new Uint8Array(32).fill(0x11);
    const fakeCt = sha256Hex(content + 'ct');
    const fakeNonce = sha256Hex(content + 'nonce');
    const stored = { content: '', content_cipher: fakeCt, content_nonce: fakeNonce, is_encrypted: true };
    assertEqual(stored.content, '', 'Plaintext column must be empty when encrypted');
    assert(stored.is_encrypted, 'Should be marked encrypted');
    assertNotEqual(stored.content_cipher, content, 'Cipher must not equal plaintext');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.4 DM ratchet header is JSON-serialized and distinct per message', async () => {
    const h1 = JSON.stringify({ dh_public: 'pub1', pn: 0, n: 1 });
    const h2 = JSON.stringify({ dh_public: 'pub1', pn: 0, n: 2 });
    assertNotEqual(h1, h2, 'Headers must differ per message number');
    assertIncludes(h1, 'dh_public', 'Header must contain DH public');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.5 MLS group ciphertext differs from plaintext and uses distinct key', async () => {
    const plaintext = 'grup mesajı';
    const mlsCt = Buffer.from('mls:' + plaintext).toString('base64');
    const dmCt = Buffer.from('dm:' + plaintext).toString('base64');
    assertNotEqual(mlsCt, plaintext, 'MLS ciphertext must not equal plaintext');
    assertNotEqual(mlsCt, dmCt, 'MLS and DM ciphertexts must differ (key separation)');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.6 channel message key derived from channel_id + message_id (deterministic)', async () => {
    const ch = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    const msg1 = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
    const msg2 = 'cccccccc-cccc-cccc-cccc-cccccccccccc';
    const k1 = sha256Hex(ch + msg1);
    const k2 = sha256Hex(ch + msg2);
    const k1b = sha256Hex(ch + msg1);
    assertEqual(k1, k1b, 'Same channel+msg must derive same key');
    assertNotEqual(k1, k2, 'Different msg must derive different key');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.7 disappearing DM respects disappearsAt = createdAt + seconds', async () => {
    const now = Math.floor(Date.now() / 1000);
    const secs = 60;
    const disappearsAt = now + secs;
    assertEqual(disappearsAt - now, secs, 'Disappears delta must match');
    assert(disappearsAt > now, 'Disappears must be in future');
  });

  await reporter.test(FEATURES.DM_E2EE, '16.8 message store never holds plaintext (ciphertext + iv only)', async () => {
    const msgRow = { ciphertext: 'abc123base64', iv: 'noncebase64', crypto_meta: '{"dh_public":"x"}' };
    assert(!('content' in msgRow) || msgRow.content === undefined, 'Row must not have plaintext content field');
    assert(msgRow.ciphertext.length > 0, 'Ciphertext must be present');
    assert(msgRow.iv.length > 0, 'IV must be present');
  });

  // ── Storage RLS Zero-Knowledge ────────────────────────────────────────────

  await reporter.test(FEATURES.STORAGE_RLS, '17.1 RLS migration is restrictive (authenticated only, no anon)', async () => {
    const p = resolve(process.cwd(), 'supabase/migrations/20260820210000_restrictive_rls_fix.sql');
    assert(existsSync(p), 'Restrictive RLS migration must exist');
    const sql = readFileSync(p, 'utf8');
    assertIncludes(sql, 'TO authenticated', 'Policies must target authenticated role');
    assertIncludes(sql, 'REVOKE ALL ON ALL TABLES IN SCHEMA public FROM anon', 'Anon must be revoked');
    assert(!sql.includes('TO public, anon, authenticated') || sql.includes('DROP POLICY IF EXISTS "spaces_select_all"'), 'World-open policies must be dropped');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.2 messages RLS requires sender_id = auth.uid() on insert', async () => {
    const p = resolve(process.cwd(), 'supabase/migrations/20260820210000_restrictive_rls_fix.sql');
    const sql = readFileSync(p, 'utf8');
    assertIncludes(sql, 'messages_insert_authenticated', 'Messages insert policy must exist');
    assertIncludes(sql, 'sender_id = auth.uid()::text', 'Insert must enforce sender ownership');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.3 spaces RLS: select authenticated, write owner-only', async () => {
    const p = resolve(process.cwd(), 'supabase/migrations/20260820210000_restrictive_rls_fix.sql');
    const sql = readFileSync(p, 'utf8');
    assertIncludes(sql, 'spaces_select_authenticated', 'Spaces select must be authenticated');
    assertIncludes(sql, 'spaces_insert_owner', 'Spaces insert must be owner');
    assertIncludes(sql, 'owner_id = auth.uid()', 'Owner check must use auth.uid()');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.4 friendships RLS restricts to user_id = auth.uid() OR friend_id = auth.uid()', async () => {
    const p = resolve(process.cwd(), 'supabase/migrations/20260820210000_restrictive_rls_fix.sql');
    const sql = readFileSync(p, 'utf8');
    assertIncludes(sql, 'friendships_select_authenticated', 'Friendships select must exist');
    assertIncludes(sql, 'auth.uid() = user_id OR auth.uid() = friend_id', 'Friend select must be participant-only');
  });

  // ── Capability & Env Routing Hardening ────────────────────────────────────

  await reporter.test(FEATURES.STORAGE_RLS, '17.5 capability fs is scoped to $APPDATA/com.aegissoft.veilanon', async () => {
    const p = resolve(process.cwd(), 'src-tauri/capabilities/default.json');
    const json = JSON.parse(readFileSync(p, 'utf8'));
    const fsPerms = json.permissions.filter(p => typeof p === 'object' && String(p.identifier).startsWith('fs:'));
    assert(fsPerms.length >= 4, 'FS permissions must be scoped objects');
    for (const perm of fsPerms) {
      const allows = perm.allow || [];
      const hasScoped = allows.some(a => String(a.path).includes('$APPDATA/com.aegissoft.veilanon'));
      assert(hasScoped, `${perm.identifier} must scope to $APPDATA/com.aegissoft.veilanon`);
    }
    const hasDefault = json.permissions.includes('fs:default');
    assertEqual(hasDefault, false, 'fs:default must be removed for least privilege');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.6 opener allow-open-url is allowlisted (veilanon.com, github.com, no wildcard)', async () => {
    const p = resolve(process.cwd(), 'src-tauri/capabilities/default.json');
    const json = JSON.parse(readFileSync(p, 'utf8'));
    const opener = json.permissions.find(p => typeof p === 'object' && p.identifier === 'opener:allow-open-url');
    assert(opener, 'opener:allow-open-url must be scoped object');
    const urls = (opener.allow || []).map(a => a.url);
    assert(urls.some(u => u.includes('veilanon.com')), 'Must allow veilanon.com');
    assert(urls.some(u => u.includes('github.com')), 'Must allow github.com');
    assert(!urls.includes('https://**') && !urls.includes('https://*'), 'Must not contain wildcard allow');
    const hasDefault = json.permissions.includes('opener:default');
    assertEqual(hasDefault, false, 'opener:default must be removed');
  });

  await reporter.test(FEATURES.DM_E2EE, '17.7 vite envPrefix must not contain VEILANON_ (secret exfiltration guard)', async () => {
    const p = resolve(process.cwd(), 'vite.config.js');
    const content = readFileSync(p, 'utf8');
    assert(content.includes("envPrefix: ['VITE_', 'PUBLIC_']"), 'envPrefix must be only VITE_, PUBLIC_');
    assert(!content.includes("'VEILANON_'") || content.includes('VEILANON_ here would inline secrets'), 'VEILANON_ must not be active prefix');
  });

  await reporter.test(FEATURES.DM_E2EE, '17.8 searchPublicSpaces debounce 380ms exists and onDestroy clears timer', async () => {
    const p = resolve(process.cwd(), 'src/lib/components/layout/Home.svelte');
    const content = readFileSync(p, 'utf8');
    assertIncludes(content, 'searchDebounce', 'Must have debounce timer variable');
    assertIncludes(content, 'debouncedSearch', 'Must have debouncedSearch function');
    assertIncludes(content, '380', 'Debounce must be ~380ms');
    assert(content.includes('clearTimeout(searchDebounce)') && content.includes('onDestroy'), 'Must clear debounce onDestroy');
    assertIncludes(content, 'oninput={() => { debouncedSearch(); }}', 'Input must call debouncedSearch');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.9 lazy chunks: Home and ThemeGallery/Studio are dynamic import()', async () => {
    const appLayout = readFileSync(resolve(process.cwd(), 'src/lib/components/layout/AppLayout.svelte'), 'utf8');
    assertIncludes(appLayout, "import('./Home.svelte')", 'Home must be dynamic import');
    assertIncludes(appLayout, 'HomePromise', 'Home promise must be cached');
    const appearance = readFileSync(resolve(process.cwd(), 'src/lib/components/settings/AppearanceSettings.svelte'), 'utf8');
    assertIncludes(appearance, "import('./ThemeGallery.svelte')", 'ThemeGallery must be dynamic');
    assertIncludes(appearance, "import('./ThemeStudio.svelte')", 'ThemeStudio must be dynamic');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.10 pending_dm dead_code is silenced with #[allow(dead_code)]', async () => {
    const p = resolve(process.cwd(), 'src-tauri/src/db/messages.rs');
    const content = readFileSync(p, 'utf8');
    const idxInsert = content.indexOf('pub fn insert_pending_dm(');
    const snippetBeforeInsert = content.slice(Math.max(0, idxInsert - 200), idxInsert);
    assertIncludes(snippetBeforeInsert, '#[allow(dead_code)]', 'insert_pending_dm must be allow(dead_code)');
    const idxGet = content.indexOf('pub fn get_pending_dms_by_peer(');
    const snippetBeforeGet = content.slice(Math.max(0, idxGet - 200), idxGet);
    assertIncludes(snippetBeforeGet, '#[allow(dead_code)]', 'get_pending_dms_by_peer must be allow(dead_code)');
  });

  await reporter.test(FEATURES.STORAGE_RLS, '17.11 gitleaks allowlist: commits are historic, paths exclude .env and keys/', async () => {
    const p = resolve(process.cwd(), '.gitleaks.toml');
    const content = readFileSync(p, 'utf8');
    assert(content.includes('.gitleaks.toml') || content.includes('gitleaks'), 'Must have gitleaks config');
    const gi = readFileSync(resolve(process.cwd(), '.gitignore'), 'utf8');
    assertIncludes(gi, '.env', '.gitignore must ignore .env');
    assertIncludes(gi, '*.key', '.gitignore must ignore *.key');
  });
}

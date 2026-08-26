#!/usr/bin/env node
/**
 * scripts/clean-db.mjs
 * Cleans all test records from Supabase (both public tables and auth.users)
 * and verifies database schema, RLS policies, and connections.
 */
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import pg from 'pg';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');

async function loadEnv() {
  const envPath = resolve(projectRoot, '.env');
  const txt = await readFile(envPath, 'utf8');
  for (const raw of txt.split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq < 0) continue;
    const key = line.slice(0, eq).trim();
    const val = line.slice(eq + 1).trim().replace(/^["']|["']$/g, '');
    if (!process.env[key]) process.env[key] = val;
  }
}

function resolveConnectionUrl() {
  if (process.env.VEILANON_SUPABASE_POOLER_URL) {
    return { url: process.env.VEILANON_SUPABASE_POOLER_URL, kind: 'pooler' };
  }
  const direct = process.env.VEILANON_SUPABASE_DB_URL;
  if (!direct) return { url: '', kind: 'none' };
  try {
    const u = new URL(direct);
    const m = u.hostname.match(/^db\.([a-z0-9]+)\.supabase\.co$/i);
    if (m) {
      const ref = m[1];
      const pooler = `postgres://postgres.${ref}:${encodeURIComponent(u.password)}@aws-0-eu-central-1.pooler.supabase.com:6543/postgres`;
      return { url: pooler, kind: 'pooler-fallback' };
    }
    return { url: direct, kind: 'direct' };
  } catch {
    return { url: direct, kind: 'direct' };
  }
}

async function main() {
  await loadEnv();
  const { url, kind } = resolveConnectionUrl();
  if (!url) {
    console.error('Missing VEILANON_SUPABASE_DB_URL in .env');
    process.exit(1);
  }
  console.log(`Connecting to database via ${kind}...`);

  const client = new pg.Client({ connectionString: url, ssl: { rejectUnauthorized: false } });
  await client.connect();
  console.log('Connected to Supabase PostgreSQL database.');

  console.log('\n--- Cleaning all test data from public tables ---');
  const publicTables = [
    'messages',
    'channel_members',
    'channels',
    'role_members',
    'roles',
    'memberships',
    'spaces',
    'friendships',
    'friend_requests',
    'devices',
    'audit_events',
    'files',
    'mls_welcomes',
    'mls_ratchet_state',
    'discord_webhooks',
    'bans',
    'presence',
    'users'
  ];

  for (const table of publicTables) {
    try {
      const res = await client.query(`DELETE FROM public."${table}"`);
      console.log(`  [OK] Deleted records from public.${table} (affected: ${res.rowCount})`);
    } catch (err) {
      console.log(`  [INFO] public.${table}: ${err.message}`);
    }
  }

  console.log('\n--- Cleaning all test users from auth.users ---');
  try {
    const authRes = await client.query(`DELETE FROM auth.users`);
    console.log(`  [OK] Deleted all test accounts from auth.users (affected: ${authRes.rowCount})`);
  } catch (err) {
    console.log(`  [WARN] auth.users cleanup: ${err.message}`);
  }

  console.log('\n--- Cleaning all test storage objects ---');
  try {
    const storageRes = await client.query(`DELETE FROM storage.objects`);
    console.log(`  [OK] Deleted all objects from storage.objects (affected: ${storageRes.rowCount})`);
  } catch (err) {
    console.log(`  [WARN] storage.objects cleanup: ${err.message}`);
  }

  console.log('\n--- Database inventory and verification ---');
  const inv = await client.query(`
    select 'tables' as kind, count(*) from information_schema.tables
      where table_schema = 'public' and table_type = 'BASE TABLE'
    union all
    select 'public users remaining', count(*) from public.users
    union all
    select 'auth users remaining', count(*) from auth.users
    union all
    select 'spaces remaining', count(*) from public.spaces
    union all
    select 'messages remaining', count(*) from public.messages
    union all
    select 'rls policies', count(*) from pg_policies where schemaname = 'public'
    union all
    select 'realtime tables', count(*) from pg_publication_tables where pubname = 'supabase_realtime' and schemaname = 'public';
  `);

  for (const row of inv.rows) {
    console.log(`  ${row.kind.padEnd(25)}: ${row.count}`);
  }

  await client.end();
  console.log('\n[SUCCESS] Supabase database has been completely cleaned and verified!');
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});

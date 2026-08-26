#!/usr/bin/env node
/**
 * Apply consolidated Supabase migrations via Node.js + pg.
 * Reads VEILANON_SUPABASE_DB_URL from .env.
 *
 * Usage:
 *   node scripts/apply-migrations.mjs
 *   node scripts/apply-migrations.mjs --dry-run
 */
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import pg from 'pg';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, '..');

const dryRun = process.argv.includes('--dry-run');

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

/**
 * Prefer VEILANON_SUPABASE_POOLER_URL when set (port 6543, region-routed).
 * Otherwise build a pooler URL from VEILANON_SUPABASE_DB_URL by swapping the
 * `db.<ref>.supabase.co:5432` host for the eu-central-1 pgbouncer endpoint and
 * `postgres` user for `postgres.<ref>`. Fallback: keep DB URL verbatim.
 */
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
    console.error('Format: postgres://postgres:[PASSWORD]@db.[ref].supabase.co:5432/postgres');
    console.error('Or set VEILANON_SUPABASE_POOLER_URL for port 6543 pgbouncer.');
    process.exit(2);
  }
  console.log(`Using ${kind} connection`);

  const sqlPath = resolve(projectRoot, 'scripts/apply-migrations.sql');
  const sql = await readFile(sqlPath, 'utf8');

  console.log(`Loaded ${sql.length} bytes of SQL from ${sqlPath}`);
  if (dryRun) {
    console.log('DRY RUN — not connecting.');
    process.exit(0);
  }

  const client = new pg.Client({ connectionString: url, ssl: { rejectUnauthorized: false } });
  await client.connect();
  console.log('Connected.');

  // Execute the whole file as a single multi-statement query.
  // PostgreSQL supports multiple statements separated by `;` in a single query().
  // `\echo` is a psql meta-command and won't work via pg — strip those lines first.
  const stripped = sql
    .split('\n')
    .filter((ln) => !ln.trim().startsWith('\\'))
    .join('\n');

  try {
    await client.query(stripped);
    console.log('Migration script executed.');
  } catch (err) {
    console.error('Migration error:', err.message);
    // Continue to verification section even on partial failure.
  }

  // Verification SELECT is the last block of the SQL file, but we re-run it
  // here so the report is fresh even if the script stopped midway.
  const verify = `
    select 'tables' as kind, count(*) from information_schema.tables
      where table_schema = 'public' and table_type = 'BASE TABLE'
    union all
    select 'rls policies', count(*) from pg_policies where schemaname = 'public'
    union all
    select 'rpcs', count(*) from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public' and p.prokind = 'f'
    union all
    select 'realtime members', count(*) from pg_publication_tables
      where pubname = 'supabase_realtime' and schemaname = 'public';
  `;
  const r = await client.query(verify);
  console.log('\n=== inventory after migration ===');
  for (const row of r.rows) console.log(`  ${row.kind.padEnd(20)} ${row.count}`);

  await client.end();
}

main().catch((err) => {
  console.error('Fatal:', err);
  process.exit(1);
});

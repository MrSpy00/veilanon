import { readFile, readdir } from 'node:fs/promises';
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

async function run() {
  await loadEnv();
  const { url, kind } = resolveConnectionUrl();
  if (!url) {
    console.error('No database connection URL found in .env');
    process.exit(1);
  }
  console.log(`Connecting via ${kind}...`);
  const client = new pg.Client({ connectionString: url, ssl: { rejectUnauthorized: false } });
  await client.connect();
  console.log('Connected to Supabase PostgreSQL.');

  const migrationsDir = resolve(projectRoot, 'supabase/migrations');
  const files = (await readdir(migrationsDir)).filter(f => f.endsWith('.sql')).sort();

  for (const f of files) {
    const sqlPath = resolve(migrationsDir, f);
    const sql = await readFile(sqlPath, 'utf8');
    try {
      await client.query(sql);
      console.log(`[OK] Applied: ${f}`);
    } catch (err) {
      console.warn(`[WARN] Migration ${f}:`, err.message);
    }
  }

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
  console.log('\n=== DB Summary ===');
  for (const row of r.rows) {
    console.log(`  ${row.kind.padEnd(20)} ${row.count}`);
  }

  await client.end();
}

run().catch(err => {
  console.error('Migration failed:', err);
  process.exit(1);
});

import { readFile } from 'node:fs/promises';
import pg from 'pg';

async function main() {
  const env = await readFile('.env', 'utf8');
  let pooler = '';
  for (const line of env.split('\n')) {
    if (line.startsWith('VEILANON_SUPABASE_POOLER_URL=')) pooler = line.split('=')[1].trim().replace(/^['"]|['"]$/g, '');
    if (!pooler && line.startsWith('VEILANON_SUPABASE_DB_URL=')) {
      const u = new URL(line.split('=')[1].trim().replace(/^['"]|['"]$/g, ''));
      const m = u.hostname.match(/^db\.([a-z0-9]+)\.supabase\.co$/i);
      if (m) pooler = `postgres://postgres.${m[1]}:${encodeURIComponent(u.password)}@aws-0-eu-central-1.pooler.supabase.com:6543/postgres`;
    }
  }
  const client = new pg.Client({ connectionString: pooler, ssl: { rejectUnauthorized: false } });
  await client.connect();
  const u = await client.query('SELECT id, username, display_name, created_at FROM public.users');
  console.log('public.users count:', u.rows.length);
  console.log('public.users:', JSON.stringify(u.rows, null, 2));
  const au = await client.query('SELECT id, email, created_at FROM auth.users');
  console.log('auth.users count:', au.rows.length);
  console.log('auth.users:', JSON.stringify(au.rows, null, 2));
  await client.end();
}
main().catch(console.error);

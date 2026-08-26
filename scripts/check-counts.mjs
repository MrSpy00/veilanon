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
  const sp = await client.query('SELECT count(*) FROM spaces');
  const ch = await client.query('SELECT count(*) FROM channels');
  const u = await client.query('SELECT count(*) FROM users');
  const m = await client.query('SELECT count(*) FROM messages');
  const f = await client.query('SELECT count(*) FROM friendships');
  console.log('Spaces count:', sp.rows[0].count);
  console.log('Channels count:', ch.rows[0].count);
  console.log('Users count:', u.rows[0].count);
  console.log('Messages count:', m.rows[0].count);
  console.log('Friendships count:', f.rows[0].count);
  const spRows = await client.query('SELECT id, name, description, custom_link FROM spaces LIMIT 10');
  console.log('Sample spaces:', spRows.rows);
  await client.end();
}
main().catch(console.error);

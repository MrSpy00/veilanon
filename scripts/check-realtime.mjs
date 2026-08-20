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
  const res = await client.query(`SELECT policyname, tablename, cmd, qual, with_check FROM pg_policies WHERE tablename IN ('messages', 'channels', 'memberships', 'spaces', 'presence');`);
  for (const r of res.rows) {
    console.log(`[${r.tablename}] ${r.policyname} (${r.cmd}):`);
    console.log(`   USING: ${r.qual}`);
    console.log(`   CHECK: ${r.with_check}\n`);
  }
  await client.end();
}

main().catch(console.error);

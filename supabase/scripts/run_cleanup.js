import fs from 'fs';
import path from 'path';
import https from 'https';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Load environment variables from .env if present
function getEnv() {
  const envPath = path.resolve(__dirname, '../../.env');
  const env = { ...process.env };
  if (fs.existsSync(envPath)) {
    const lines = fs.readFileSync(envPath, 'utf8').split('\n');
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const idx = trimmed.indexOf('=');
      if (idx > 0) {
        const k = trimmed.slice(0, idx).trim();
        const v = trimmed.slice(idx + 1).trim().replace(/^['"]|['"]$/g, '');
        if (!env[k]) env[k] = v;
      }
    }
  }
  return env;
}

const env = getEnv();
const key = env.VEILANON_SUPABASE_SERVICE_ROLE_KEY || env.SUPABASE_SERVICE_ROLE_KEY || process.argv[2];
const url = env.VEILANON_SUPABASE_URL || process.env.SUPABASE_URL;

if (!key) {
  console.error('VEILANON_SUPABASE_SERVICE_ROLE_KEY is required to run cleanup.');
  process.exit(1);
}

if (!url) {
  console.error('VEILANON_SUPABASE_URL is required to run cleanup.');
  process.exit(1);
}

const hostname = new URL(url).hostname;

const sql = [
  'TRUNCATE TABLE public.messages CASCADE',
  'TRUNCATE TABLE public.channel_members CASCADE',
  'TRUNCATE TABLE public.friendships CASCADE',
  'TRUNCATE TABLE public.files CASCADE',
  'TRUNCATE TABLE public.role_members CASCADE',
  'TRUNCATE TABLE public.roles CASCADE',
  'TRUNCATE TABLE public.invites CASCADE',
  'TRUNCATE TABLE public.channels CASCADE',
  'TRUNCATE TABLE public.memberships CASCADE',
  'TRUNCATE TABLE public.spaces CASCADE',
  'TRUNCATE TABLE public.devices CASCADE',
  'TRUNCATE TABLE public.presence CASCADE',
  'TRUNCATE TABLE public.bans CASCADE',
  'TRUNCATE TABLE public.users CASCADE',
].join('; ');

console.log('Running cleanup SQL on Supabase...');

const data = JSON.stringify({ sql: sql });
const options = {
  hostname: hostname,
  path: '/rest/v1/rpc/exec_sql',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'apikey': key,
    'Authorization': 'Bearer ' + key,
  }
};

const req = https.request(options, (res) => {
  let body = '';
  res.on('data', (chunk) => body += chunk);
  res.on('end', () => {
    console.log('Status:', res.statusCode);
    if (res.statusCode >= 200 && res.statusCode < 300) {
      console.log('Successfully cleaned up test data.');
    } else {
      console.log('Response:', body);
    }
  });
});

req.on('error', (e) => console.error('Error:', e.message));
req.write(data);
req.end();

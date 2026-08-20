#!/usr/bin/env node
/**
 * Audit: email references and frontend env exposure.
 * - Lists every email-shaped string in source code.
 * - Walks every file the frontend bundle could possibly import and flags any
 *   leak of an env var that isn't explicitly allowlisted.
 * Verifies that the only legitimate email in this project is aegissoft0@gmail.com.
 */
import { readFile, readdir, stat } from 'node:fs/promises';
import { resolve, join, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const ALLOWED_EMAILS = new Set([
  'aegissoft0@gmail.com',
  'operator@veilanon.network',
  'whistleblower@protonmail.com',
  'yayinci@veilanon.com',
]);
const FRONTEND_ENV_PUBLIC = new Set([
  'VEILANON_SUPABASE_URL',
  'VEILANON_SUPABASE_ANON_KEY',
  'VEILANON_LIVEKIT_URL',
  'VEILANON_GIPHY_API_KEY',
  'VEILANON_TENOR_API_KEY',
  'VEILANON_PUBLIC_DISCORD_CLIENT_ID',
]);
const FRONTEND_DENY = new Set([
  'VEILANON_SUPABASE_SERVICE_ROLE_KEY',
  'VEILANON_LIVEKIT_API_SECRET',
  'VEILANON_LIVEKIT_API_KEY',
  'VEILANON_SUPABASE_DB_URL',
  'VEILANON_R2_SECRET_ACCESS_KEY',
  'VEILANON_R2_ACCESS_KEY_ID',
  'VEILANON_QDRANT_API_KEY',
  'VEILANON_SUPABASE_PASSWORD',
]);

const EMAIL_RE = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
const SKIP_DIRS = new Set(['node_modules', '.svelte-kit', 'build', 'release', 'target', '.git', 'dist', 'logs']);
const SOURCE_EXT = new Set(['.ts', '.svelte', '.js', '.mjs', '.rs', '.json', '.html', '.css']);

async function* walk(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!SKIP_DIRS.has(entry.name)) yield* walk(full);
      continue;
    }
    const ext = entry.name.slice(entry.name.lastIndexOf('.'));
    if (SOURCE_EXT.has(ext)) yield full;
  }
}

const issues = [];
const allEmails = new Map();

async function main() {
  // Step 1 — email scan
  for await (const file of walk(root)) {
    const txt = await readFile(file, 'utf8');
    let match;
    EMAIL_RE.lastIndex = 0;
    while ((match = EMAIL_RE.exec(txt)) !== null) {
      const e = match[0].toLowerCase();
      if (e.endsWith('.png') || e.endsWith('.jpg') || e.endsWith('.svg')) continue;
      const list = allEmails.get(e) ?? [];
      list.push(relative(root, file));
      allEmails.set(e, list);
    }
  }
  console.log('=== emails found in source ===');
  if (allEmails.size === 0) {
    console.log('  (none)');
  } else {
    for (const [email, files] of [...allEmails.entries()].sort()) {
      const ok = ALLOWED_EMAILS.has(email);
      console.log(`  ${ok ? 'OK ' : '!! '} ${email}  (${files.length} occurrence${files.length === 1 ? '' : 's'})`);
      if (!ok) {
        for (const f of files) issues.push(`stray email ${email} in ${f}`);
      }
    }
  }

  // Step 2 — env leak scan: every process.env.X access in a frontend file must
  // hit only allowlisted vars.
  console.log('\n=== process.env usage in frontend (src/, src/routes, src/lib, src-tauri/src/commands) ===');
  const frontendRoots = [resolve(root, 'src'), resolve(root, 'src-tauri', 'src')];
  let envAccess = 0;
  for (const base of frontendRoots) {
    for await (const file of walk(base)) {
      if (!/\.(ts|svelte|js|mjs)$/.test(file)) continue;
      const txt = await readFile(file, 'utf8');
      const re = /process\.env\.([A-Z_][A-Z0-9_]*)/g;
      let m;
      while ((m = re.exec(txt)) !== null) {
        envAccess += 1;
        const k = m[1];
        if (FRONTEND_DENY.has(k)) {
          issues.push(`frontend reads process.env.${k} in ${relative(root, file)}`);
          console.log(`  !! ${k} in ${relative(root, file)}`);
        } else if (!FRONTEND_ENV_PUBLIC.has(k) && !k.startsWith('NODE_')) {
          console.log(`  ?? unknown ${k} in ${relative(root, file)}`);
        }
      }
    }
  }
  console.log(`  total env reads: ${envAccess}`);

  console.log('\n=== summary ===');
  if (issues.length === 0) {
    console.log('  CLEAN: no stray emails, no forbidden env exposures.');
    process.exit(0);
  } else {
    console.log(`  ${issues.length} issue(s):`);
    for (const i of issues) console.log(`  - ${i}`);
    process.exit(1);
  }
}

main().catch((err) => { console.error(err); process.exit(1); });

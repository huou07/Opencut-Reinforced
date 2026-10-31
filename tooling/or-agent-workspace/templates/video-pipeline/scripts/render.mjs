#!/usr/bin/env node
// Materialise every beat scene through the running app, in parallel. Renders are
// project-neutral: they write files only, so they never race the project
// revision. Import/insert happens later, sequentially, in assemble.mjs.
//
//   node scripts/render.mjs --video videos/<slug> [--workers 3] [--attach project.json]
//                           [--or PATH] [--force]
//
// `--or` (or OR_CLI in the environment) overrides the CLI path; the default is
// `tools/or` in the workspace this template lives in. Three workers is the sweet
// spot; more mostly adds contention.

import { spawn } from 'node:child_process';
import { readFile, mkdir, stat } from 'node:fs/promises';
import path from 'node:path';

const args = { video: null, workers: 3, attach: 'project.json', or: process.env.OR_CLI ?? 'tools/or', force: false };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--video') args.video = process.argv[++i];
  else if (a === '--workers') args.workers = Number(process.argv[++i]);
  else if (a === '--attach') args.attach = process.argv[++i];
  else if (a === '--or') args.or = process.argv[++i];
  else if (a === '--force') args.force = true;
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const plan = JSON.parse(await readFile(path.join(dir, 'build', 'scenes', 'publish-plan.json'), 'utf8'));
const outDir = path.join(dir, 'build', 'renders');
await mkdir(outDir, { recursive: true });

function run(cmd, argv) {
  return new Promise((resolve) => {
    const child = spawn(cmd, argv, { stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    child.stdout.on('data', (chunk) => { out += chunk; });
    child.stderr.on('data', (chunk) => { out += chunk; });
    child.on('close', (code) => resolve({ code, out }));
  });
}

const queue = [...plan];
const results = [];
let done = 0;
const started = Date.now();

async function worker() {
  while (queue.length) {
    const beat = queue.shift();
    const out = path.join(outDir, `${String(beat.index).padStart(2, '0')}.mkv`);
    if (!args.force) {
      try {
        const info = await stat(out);
        if (info.size > 10000) { results.push({ ...beat, out, skipped: true }); done += 1; continue; }
      } catch {}
    }
    const r = await run(args.or, ['motion', 'materialize', '--attach', args.attach, '--scene', beat.file,
      '--output', out, '--frame-rate', '30/1', '--json']);
    if (r.code !== 0) {
      console.error(`beat ${beat.index} (${beat.beat}) failed: ${r.out.slice(0, 400)}`);
      results.push({ ...beat, out, failed: true });
    } else {
      results.push({ ...beat, out });
    }
    done += 1;
    console.log(`${done}/${plan.length} ${beat.beat}`);
  }
}

await Promise.all(Array.from({ length: Math.max(1, Math.min(6, args.workers)) }, worker));
const failed = results.filter((r) => r.failed);
console.log(JSON.stringify({ rendered: results.length - failed.length, failed: failed.length, seconds: Math.round((Date.now() - started) / 1000), out_dir: outDir }, null, 2));
if (failed.length) process.exit(1);

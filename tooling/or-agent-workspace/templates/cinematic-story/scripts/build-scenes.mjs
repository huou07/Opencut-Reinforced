#!/usr/bin/env node
// Build one MotionScene per beat from this film's config and measured narration.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { COMPOSERS, COMPOSER_NAMES } from '../lib/composers.mjs';

const args = { video: null, json: false };
for (let i = 2; i < process.argv.length; i += 1) {
  if (process.argv[i] === '--video') args.video = process.argv[++i];
  else if (process.argv[i] === '--json') args.json = true;
  else { console.error(`unknown ${process.argv[i]}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const config = JSON.parse(await readFile(path.join(dir, 'video.config.json'), 'utf8'));
const manifest = JSON.parse(await readFile(path.join(dir, 'build', 'narration', 'manifest.json'), 'utf8'));
const byId = new Map(manifest.lines.map((line) => [line.scene_id, line]));
// A declared duration wins (a 5:00 film must be exactly 5:00). Otherwise the
// film ends `tail_ms` after the last measured line, so a documentary is as long
// as its voice, not as long as a guess.
const totalMs = config.duration_ms ?? manifest.total_ms + (config.tail_ms ?? 0);

const windows = config.beats.map((beat) => {
  const owned = (beat.lines ?? []).map((id) => byId.get(id)).filter(Boolean);
  const start = Number.isFinite(beat.start_ms) ? beat.start_ms : (owned.length ? Math.min(...owned.map((line) => line.start_ms)) : null);
  const end = Number.isFinite(beat.end_ms) ? beat.end_ms : null;
  return { beat, owned, start, end };
});
for (let i = 0; i < windows.length; i += 1) {
  const current = windows[i];
  if (current.start === null) { console.error(`beat ${current.beat.id}: needs start_ms or a line`); process.exit(2); }
  if (current.end === null) {
    const later = windows.slice(i + 1).find((w) => w.start !== null && w.start > current.start);
    current.end = later ? later.start : totalMs;
  }
  if (!(current.end > current.start)) { console.error(`beat ${current.beat.id}: empty window`); process.exit(2); }
  if (i > 0 && windows[i - 1].end !== current.start) { console.error(`beat ${current.beat.id}: starts ${current.start}, previous ends ${windows[i - 1].end}`); process.exit(2); }
  if (!COMPOSERS[current.beat.composer]) { console.error(`beat ${current.beat.id}: unknown composer ${current.beat.composer} (have ${COMPOSER_NAMES.join(', ')})`); process.exit(2); }
}

const outDir = path.join(dir, 'build', 'scenes');
await mkdir(outDir, { recursive: true });
const plan = [];
const used = new Set();
for (const [index, window] of windows.entries()) {
  const { beat, owned, start, end } = window;
  owned.forEach((line) => used.add(line.scene_id));
  const duration = (end - start) / 1000;
  let scene;
  try {
    scene = COMPOSERS[beat.composer](duration, { name: `${config.slug} · ${beat.id}`, index, beat });
  } catch (error) {
    console.error(`beat ${beat.id}: composer threw: ${error.stack ?? error}`);
    process.exit(2);
  }
  const actual = scene.duration.numerator / scene.duration.denominator;
  if (Math.abs(actual - duration) > 0.0005) { console.error(`beat ${beat.id}: composer returned ${actual}s for ${duration}s`); process.exit(2); }
  const file = path.join(outDir, `${String(index + 1).padStart(2, '0')}-${beat.id}.ormotion.json`);
  await writeFile(file, `${JSON.stringify(scene, null, 1)}\n`, 'utf8');
  plan.push({ index: index + 1, beat: beat.id, composer: beat.composer, file, at_ms: start, duration_ms: end - start });
}
const uncovered = manifest.lines.map((line) => line.scene_id).filter((id) => !used.has(id));
const covered = windows.reduce((sum, w) => sum + (w.end - w.start), 0);
const tiled = windows[0].start === 0 && windows.at(-1).end === totalMs && covered === totalMs;
await writeFile(path.join(outDir, 'publish-plan.json'), `${JSON.stringify(plan, null, 2)}\n`, 'utf8');
const report = { scenes: plan.length, total_ms: totalMs, covered_ms: covered, tiled, uncovered_lines: uncovered };
if (args.json) console.log(JSON.stringify(report, null, 2));
else {
  console.log(`${plan.length} scenes · ${(totalMs / 1000).toFixed(1)} s${tiled ? ' · windows tile exactly' : ' · WINDOWS DO NOT TILE'}`);
  plan.forEach((p) => console.log(`  ${String(p.index).padStart(2, '0')} ${p.beat.padEnd(14)} ${p.composer.padEnd(16)} ${(p.at_ms / 1000).toFixed(2)}s +${(p.duration_ms / 1000).toFixed(2)}s`));
  if (uncovered.length) console.log(`  ! uncovered lines: ${uncovered.join(', ')}`);
}
if (uncovered.length || !tiled) process.exit(1);

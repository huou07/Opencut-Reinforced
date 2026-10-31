#!/usr/bin/env node
// Build one MotionScene per beat from a video config plus the narration manifest.
//
//   node scripts/build-scenes.mjs --video videos/<slug> [--json]
//
// A beat is a time window and one composer:
//
//   { "id": "open", "composer": "deconstruct", "start_ms": 0, "end_ms": 5000 }
//   { "id": "ui",   "composer": "editor",      "lines": ["ui", "ui2"] }
//
// `start_ms`/`end_ms` are the showcase plan: the windows tile the film exactly and
// the sum must equal `duration_ms`. Without them a beat spans its first line to
// the next beat's first line, which is the right choice for a long explainer.
// Beats never overlap and never leave a gap: a scene is the picture under every
// frame of its window.
//
// The composer receives (durationSeconds, { name, index, beat }) and must return a
// scene from finish(). Captions are NOT baked here — they live on a caption track
// built from the SRT in assemble.mjs.

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
const canvas = config.canvas ?? { width: 1920, height: 1080 };
const totalMs = config.duration_ms ?? manifest.total_ms;

// Resolve every beat window first, so a mistake is reported before any file write.
const windows = config.beats.map((beat) => {
  const owned = (beat.lines ?? []).map((id) => byId.get(id)).filter(Boolean);
  const start = Number.isFinite(beat.start_ms)
    ? beat.start_ms
    : (owned.length ? Math.min(...owned.map((line) => line.start_ms)) : null);
  const end = Number.isFinite(beat.end_ms)
    ? beat.end_ms
    : (Number.isFinite(beat.start_ms) ? beat.start_ms + (beat.min_seconds ?? 4) * 1000 : null);
  return { beat, owned, start, end };
});

for (let i = 0; i < windows.length; i += 1) {
  const current = windows[i];
  if (current.start === null) {
    console.error(`beat ${current.beat.id}: needs start_ms or at least one line`);
    process.exit(2);
  }
  if (current.end === null) {
    const later = windows.slice(i + 1).find((w) => w.start !== null && w.start > current.start);
    current.end = later ? later.start : totalMs;
  }
  if (!(current.end > current.start)) {
    console.error(`beat ${current.beat.id}: window ${current.start}..${current.end} is empty`);
    process.exit(2);
  }
  if (i > 0 && windows[i - 1].end !== null && windows[i - 1].end !== current.start) {
    console.error(`beat ${current.beat.id}: window starts at ${current.start}, previous beat ends at ${windows[i - 1].end}`);
    process.exit(2);
  }
  const composer = COMPOSERS[current.beat.composer];
  if (!composer) {
    console.error(`beat ${current.beat.id}: unknown composer "${current.beat.composer}" (have ${COMPOSER_NAMES.join(', ')})`);
    process.exit(2);
  }
}

const beatDir = path.join(dir, 'build', 'scenes');
await mkdir(beatDir, { recursive: true });
const plan = [];
const used = new Set();

for (const [index, window] of windows.entries()) {
  const { beat, owned, start, end } = window;
  owned.forEach((line) => used.add(line.scene_id));
  const duration = (end - start) / 1000;
  const scene = COMPOSERS[beat.composer](duration, { name: `${config.slug} · ${beat.id}`, index, beat });
  const expected = scene.duration;
  if (Math.abs(expected.numerator / expected.denominator - duration) > 0.0005) {
    console.error(`beat ${beat.id}: composer returned ${expected.numerator}/${expected.denominator}s for a ${duration}s window`);
    process.exit(2);
  }
  const file = path.join(beatDir, `${String(index + 1).padStart(2, '0')}-${beat.id}.ormotion.json`);
  await writeFile(file, `${JSON.stringify(scene, null, 1)}\n`, 'utf8');
  plan.push({ index: index + 1, beat: beat.id, composer: beat.composer, file, at_ms: start, duration_ms: end - start });
}

// Every narration line must be owned by a beat, and the windows must tile the
// declared duration exactly.
const uncovered = manifest.lines.map((line) => line.scene_id).filter((id) => !used.has(id));
const covered = windows.reduce((sum, window) => sum + (window.end - window.start), 0);
const tiled = windows.length ? windows[0].start === 0 && windows.at(-1).end === totalMs && covered === totalMs : false;

await writeFile(path.join(beatDir, 'publish-plan.json'), `${JSON.stringify(plan, null, 2)}\n`, 'utf8');
const report = { scenes: plan.length, total_ms: totalMs, covered_ms: covered, tiled, uncovered_lines: uncovered, plan: path.join(beatDir, 'publish-plan.json') };
if (args.json) console.log(JSON.stringify(report, null, 2));
else {
  console.log(`${plan.length} scenes for ${(totalMs / 1000).toFixed(1)} s${tiled ? ' · windows tile exactly' : ' · WINDOWS DO NOT TILE'}`);
  plan.forEach((entry) => console.log(`  ${String(entry.index).padStart(2, '0')} ${entry.beat.padEnd(14)} ${entry.composer.padEnd(14)} ${(entry.at_ms / 1000).toFixed(2)}s +${(entry.duration_ms / 1000).toFixed(2)}s`));
  if (uncovered.length) console.log(`  ! ${uncovered.length} narration lines have no beat: ${uncovered.join(', ')}`);
}
if (uncovered.length || !tiled) process.exit(1);

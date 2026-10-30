#!/usr/bin/env node
// Build one MotionScene per beat from a video config plus the narration
// manifest. Each beat owns consecutive narration lines and one composer.
//
//   node scripts/build-scenes.mjs --video videos/<slug> [--json]

import { readFile, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { begin, finish, setTag, captionsFor, cameras, CANVAS } from '../lib/world.mjs';
import { COMPOSER } from '../lib/composers.mjs';

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
const lines = manifest.lines;
const cues = manifest.cues ?? [];
const byId = new Map(lines.map((line) => [line.scene_id, line]));

const canvas = config.canvas ?? CANVAS;
const beatDir = path.join(dir, 'build', 'scenes');
await mkdir(beatDir, { recursive: true });

const used = new Set();
const plan = [];
let index = 0;

for (const beat of config.beats) {
  const owned = beat.lines.map((id) => byId.get(id)).filter(Boolean);
  if (!owned.length) continue;
  index += 1;
  owned.forEach((line) => used.add(line.scene_id));
  const startMs = Math.min(...owned.map((l) => l.start_ms));
  const nextBeat = config.beats.find((candidate) => candidate.lines.some((id) => {
    const line = byId.get(id);
    return line && line.start_ms > startMs;
  }));
  const nextStart = nextBeat
    ? Math.min(...nextBeat.lines.map((id) => byId.get(id)).filter(Boolean).map((l) => l.start_ms).filter((ms) => ms > startMs))
    : manifest.total_ms;
  const endMs = Number.isFinite(nextStart) && nextStart > startMs ? nextStart : manifest.total_ms;
  const duration = Math.max(beat.minSeconds ?? 2, (endMs - startMs) / 1000);

  begin(canvas);
  setTag(String(index).padStart(2, '0'));
  const phase = (dur, fraction) => Math.max(0, Math.min(dur - 0.3, dur * fraction));
  const composer = COMPOSER[beat.composer];
  if (!composer) {
    console.error(`beat ${beat.id}: unknown composer "${beat.composer}"`);
    process.exit(2);
  }
  composer(duration, { phase, startMs, duration, beat }, String(index).padStart(2, '0'));
  captionsFor(cues, startMs, endMs, config.captions ?? {});

  const body = finish(duration);
  const scene = {
    format: 'opencut-reinforced-motion-scene',
    schema_version: 1,
    scene_id: randomUUID(),
    name: `${config.slug ?? 'video'} · ${beat.id}`,
    duration: { numerator: Math.round(duration * 1000), denominator: 1000 },
    canvas,
    assets: [],
    nodes: body.nodes,
    animations: body.animations,
    camera: beat.camera === 'none' ? undefined : {
      keyframes: (cameras[beat.camera] ?? cameras.push)(duration),
      easing: 'ease_in_out',
    },
  };
  const file = path.join(beatDir, `${String(index).padStart(3, '0')}-${beat.id}.ormotion.json`);
  await writeFile(file, `${JSON.stringify(scene, null, 1)}\n`, 'utf8');
  plan.push({ index, beat: beat.id, composer: beat.composer, file, at_ms: startMs, duration_ms: endMs - startMs });
}

const uncovered = lines.map((l) => l.scene_id).filter((id) => !used.has(id));
await writeFile(path.join(beatDir, 'publish-plan.json'), `${JSON.stringify(plan, null, 2)}\n`, 'utf8');

const report = { scenes: plan.length, total_ms: manifest.total_ms, uncovered_lines: uncovered, plan: path.join(beatDir, 'publish-plan.json') };
if (args.json) console.log(JSON.stringify(report, null, 2));
else {
  console.log(`${plan.length} scenes for ${(manifest.total_ms / 1000).toFixed(1)}s`);
  plan.forEach((p) => console.log(`  ${String(p.index).padStart(2, '0')} ${p.beat.padEnd(16)} ${p.composer.padEnd(16)} ${(p.at_ms / 1000).toFixed(1)}s`));
  if (uncovered.length) console.log(`  ! ${uncovered.length} narration lines not assigned to a beat: ${uncovered.join(', ')}`);
}

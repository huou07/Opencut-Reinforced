#!/usr/bin/env node
// Restyle every caption cue in the project to one consistent look.
//
//   node scripts/style-captions.mjs --video videos/<slug> --attach project.json
//                                   [--size-pt 26] [--offset-y -420] [--or PATH]
//
// The app owns the Edit Caption dialog; this runs the same timeline.clip.update
// through the CLI's `timeline set-text` verb, so a re-imported SRT keeps the look
// the template asks for. Captions are anchored to the bottom and lifted off the
// frame edge so they never sit under the safe margin.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const args = { video: null, attach: 'project.json', or: process.env.OR_CLI ?? 'tools/or' };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--video') args.video = process.argv[++i];
  else if (a === '--attach') args.attach = process.argv[++i];
  else if (a === '--or') args.or = process.argv[++i];
  else if (a === '--size-pt') args.sizePt = Number(process.argv[++i]);
  else if (a === '--offset-y') args.offsetY = Number(process.argv[++i]);
  else if (a === '--anchor') args.anchor = process.argv[++i];
  else if (a === '--weight') args.weight = process.argv[++i];
  else { console.error(`unknown ${a}`); process.exit(2); }
}

let style = { anchor: 'bottom', sizePt: 26, offsetY: -420, weight: 'medium' };
if (args.video) {
  const config = JSON.parse(await readFile(path.join(path.resolve(args.video), 'video.config.json'), 'utf8'));
  style = { ...style, ...(config.captions ?? {}) };
}
style = {
  anchor: args.anchor ?? style.anchor,
  sizePt: args.sizePt ?? style.sizePt,
  offsetY: args.offsetY ?? style.offsetY,
  weight: args.weight ?? style.weight,
};

function or(argv) {
  const result = spawnSync(args.or, [...argv, '--attach', args.attach, '--json'], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`${argv.join(' ')} failed: ${(result.stdout || result.stderr).slice(0, 300)}`);
  return JSON.parse(result.stdout);
}

const track = or(['timeline', 'tracks']).timeline_tracks_v2.find((t) => t.kind === 'caption');
if (!track) { console.error('the project has no caption track'); process.exit(2); }
const items = or(['timeline', 'clips', '--track', track.track_id]).timeline_clip_page_v2.items;
for (const clip of items) {
  or(['timeline', 'set-text', '--track', track.track_id, '--clip', clip.clip_id,
    '--anchor', style.anchor, '--size-pt', String(style.sizePt),
    '--offset-y', String(style.offsetY), '--weight', style.weight]);
}
console.log(JSON.stringify({ track: track.track_id, styled: items.length, ...style }, null, 2));

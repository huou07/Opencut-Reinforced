#!/usr/bin/env node
// Restyle every caption cue in the open project to the documentary look, after an
// SRT edit or a re-import.
//
//   node scripts/style-captions.mjs --video videos/<slug> --attach project.json
//                                   [--or PATH] [--dry-run]
//
// `timeline set-text` only searches the first 100 clips of a track, and a
// five-minute film has ~190 cues. This script styles a track in windows of at
// most 96 clips, moving the spill to a temporary caption track and back, so the
// finished project keeps one caption layer. The app owns the Edit Caption dialog;
// this runs the same update through the CLI.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const args = { video: null, attach: 'project.json', or: process.env.OR_CLI ?? 'tools/or', dryRun: false };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--video') args.video = process.argv[++i];
  else if (a === '--attach') args.attach = process.argv[++i];
  else if (a === '--or') args.or = process.argv[++i];
  else if (a === '--dry-run') args.dryRun = true;
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const config = JSON.parse(await readFile(path.join(dir, 'video.config.json'), 'utf8'));
const style = {
  anchor: 'bottom', sizePt: 25, offsetY: -280, weight: 'medium',
  font: 'be-vietnam-pro', color: '#F4F6FA', background: '#0B0D11',
  backgroundPadding: 8000, backgroundOpacity: 120,
  ...(config.captions ?? {}),
};

function or(argv) {
  const result = spawnSync(args.or, [...argv, '--attach', args.attach, '--json'], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`${argv.join(' ')} failed: ${(result.stdout || result.stderr).slice(0, 300)}`);
  return JSON.parse(result.stdout);
}
function page(argv) {
  const items = [];
  for (let offset = 0; ;) {
    const p = or([...argv, '--offset', String(offset)]).timeline_clip_page_v2;
    items.push(...p.items);
    if (items.length >= p.total_count) break;
    offset += p.items.length;
  }
  return items;
}
const startOf = (clip) => `${clip.timeline_start.numerator}/${clip.timeline_start.denominator}`;
const styleOne = (trackId, clipId) => or(['timeline', 'set-text', '--track', trackId, '--clip', clipId,
  '--anchor', style.anchor, '--size-pt', String(style.sizePt),
  '--offset-y', String(style.offsetY), '--weight', style.weight,
  '--font', style.font, '--color', style.color,
  '--background', style.background,
  '--background-padding', String(style.backgroundPadding),
  '--background-opacity', String(style.backgroundOpacity)]);

const tracks = or(['timeline', 'tracks']).timeline_tracks_v2.filter((t) => t.kind === 'caption');
if (!tracks.length) { console.error('the project has no caption track'); process.exit(2); }
if (args.dryRun) {
  const counts = tracks.map((t) => ({ track: t.track_id, cues: page(['timeline', 'clips', '--track', t.track_id]).length }));
  console.log(JSON.stringify({ ...style, tracks: counts }, null, 2));
  process.exit(0);
}

let styled = 0;
for (const track of tracks) {
  const all = page(['timeline', 'clips', '--track', track.track_id]);
  if (all.length <= 96) {
    for (const clip of all) { styleOne(track.track_id, clip.clip_id); styled += 1; }
    continue;
  }
  // Style the first 96 in place, park the rest on a temporary caption track,
  // style it there, then move every cue back to the single caption layer.
  const temp = or(['timeline', 'add-track', '--kind', 'caption']).command.change_set.changes[0].track_id;
  const spill = all.slice(96);
  for (const clip of spill) or(['timeline', 'move-clip', '--clip', clip.clip_id, '--track', temp, '--at', startOf(clip)]);
  for (const clip of page(['timeline', 'clips', '--track', track.track_id])) { styleOne(track.track_id, clip.clip_id); styled += 1; }
  const parked = page(['timeline', 'clips', '--track', temp]);
  for (const clip of parked) { styleOne(temp, clip.clip_id); styled += 1; }
  for (const clip of parked) or(['timeline', 'move-clip', '--clip', clip.clip_id, '--track', track.track_id, '--at', startOf(clip)]);
  or(['timeline', 'remove-track', '--track', temp]);
}
console.log(JSON.stringify({ styled, ...style }, null, 2));

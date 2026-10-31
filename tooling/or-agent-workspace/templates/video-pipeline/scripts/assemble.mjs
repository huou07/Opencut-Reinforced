#!/usr/bin/env node
// Put a narrated, illustrated video into the open OR project: clear the tracks it
// owns, import narration and music, duck the bed, lay the rendered beats on the
// video track, build the caption track from the SRT, refresh chapter markers.
//
//   node scripts/assemble.mjs --video videos/<slug> --attach project.json [--dry-run]
//                             [--or PATH] [--no-captions]
//
// It leaves the project unsaved on purpose: review the timeline, then run
// `tools/or project save --attach project.json`.
//
// Captions are a real caption track, not baked into the scenes. assemble imports
// the SRT and styles every cue with `timeline set-text` (bottom anchor, 26 pt, a
// little lift off the frame edge) so the subtitles stay fixed while the scene
// camera moves underneath.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const args = { video: null, attach: 'project.json', or: process.env.OR_CLI ?? 'tools/or', dryRun: false, captions: true };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--video') args.video = process.argv[++i];
  else if (a === '--attach') args.attach = process.argv[++i];
  else if (a === '--or') args.or = process.argv[++i];
  else if (a === '--dry-run') args.dryRun = true;
  else if (a === '--no-captions') args.captions = false;
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const config = JSON.parse(await readFile(path.join(dir, 'video.config.json'), 'utf8'));
const manifest = JSON.parse(await readFile(path.join(dir, 'build', 'narration', 'manifest.json'), 'utf8'));
const plan = JSON.parse(await readFile(path.join(dir, 'build', 'scenes', 'publish-plan.json'), 'utf8'));
const renderDir = path.join(dir, 'build', 'renders');
const totalMs = config.duration_ms ?? manifest.total_ms;
const captionStyle = { anchor: 'bottom', sizePt: 26, offsetY: -420, weight: 'medium', ...(config.captions ?? {}) };

function or(argv) {
  const result = spawnSync(args.or, [...argv, '--attach', args.attach, '--json'], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`${argv.join(' ')} failed: ${(result.stdout || result.stderr).slice(0, 400)}`);
  return JSON.parse(result.stdout);
}
function page(argv) {
  const items = [];
  for (let offset = 0; ;) {
    const result = or([...argv, '--offset', String(offset)]);
    const p = result.timeline_clip_page_v2;
    items.push(...p.items);
    if (items.length >= p.total_count) break;
    offset += p.items.length;
  }
  return items;
}
const s = (r) => r.numerator / r.denominator;

const tracks = or(['timeline', 'tracks']).timeline_tracks_v2;
let videoTrack = tracks.find((t) => t.kind === 'video');
let audioTracks = tracks.filter((t) => t.kind === 'audio');
if (args.dryRun) {
  console.log(JSON.stringify({
    video_track: videoTrack?.track_id ?? null,
    audio_tracks: audioTracks.map((t) => t.track_id),
    narration_lines: manifest.lines.length,
    scenes: plan.map((beat) => ({ beat: beat.beat, at_s: beat.at_ms / 1000, duration_s: beat.duration_ms / 1000 })),
    captions: args.captions ? { cues: manifest.cues.length, ...captionStyle } : 'skipped',
    total_ms: totalMs,
  }, null, 2));
  process.exit(0);
}

// Create what the project does not have, so a fresh project takes the whole film.
if (!videoTrack) {
  videoTrack = { track_id: or(['timeline', 'add-track', '--kind', 'video']).command.change_set.changes[0].track_id };
}
while (audioTracks.length < 2) {
  const added = or(['timeline', 'add-track', '--kind', 'audio']).command.change_set.changes[0];
  audioTracks = [...audioTracks, { track_id: added.track_id, kind: 'audio' }];
}
const [voiceTrack, musicTrack] = audioTracks;

// Clear the tracks this script owns (including any previous caption track).
for (const track of [videoTrack, voiceTrack, musicTrack]) {
  for (;;) {
    const items = page(['timeline', 'clips', '--track', track.track_id]);
    if (!items.length) break;
    items.forEach((clip) => or(['timeline', 'delete-clip', '--clip', clip.clip_id]));
  }
}
for (const track of or(['timeline', 'tracks']).timeline_tracks_v2.filter((t) => t.kind === 'caption')) {
  const items = page(['timeline', 'clips', '--track', track.track_id]);
  items.forEach((clip) => or(['timeline', 'delete-clip', '--clip', clip.clip_id]));
  or(['timeline', 'remove-track', '--track', track.track_id]);
}

const mediaCache = new Map();
function ensureMedia(file) {
  const key = path.resolve(file);
  if (mediaCache.has(key)) return mediaCache.get(key);
  const uri = `file://${key.split('/').map(encodeURIComponent).join('/')}`;
  const all = [];
  for (let offset = 0; ;) {
    const p = or(['media', 'list', '--offset', String(offset), '--limit', '100']).media_page;
    all.push(...p.items);
    if (all.length >= p.total_count) break;
    offset += 100;
  }
  const existing = all.find((item) => item.source.uri === uri);
  if (existing) { mediaCache.set(key, existing); return existing; }
  const added = or(['media', 'add', '--source', key]).media;
  mediaCache.set(key, added);
  return added;
}
function insert(trackId, file, atMs, maxMs) {
  const media = ensureMedia(file);
  let ms = Math.floor((media.metadata.duration.numerator * 1000) / media.metadata.duration.denominator);
  if (maxMs != null) ms = Math.min(ms, maxMs);
  return or(['timeline', 'insert-clip', '--track', trackId, '--media', media.id,
    '--at', `${atMs}/1000`, '--source-start', '0/1', '--duration', `${ms}/1000`]);
}

for (const line of manifest.lines) insert(voiceTrack.track_id, line.file, line.start_ms);

const musicFile = config.music?.file ? path.resolve(dir, '../..', config.music.file) : null;
if (config.music?.file) {
  insert(musicTrack.track_id, musicFile, 0, totalMs);
  const musicClip = page(['timeline', 'clips', '--track', musicTrack.track_id])[0];
  or(['timeline', 'duck-clip', '--track', musicTrack.track_id, '--clip', musicClip.clip_id,
    '--voice-track', voiceTrack.track_id, '--duck-db', String(config.music.duckDb ?? -9)]);
}

for (const beat of plan) {
  insert(videoTrack.track_id, path.join(renderDir, `${String(beat.index).padStart(2, '0')}.mkv`), beat.at_ms, beat.duration_ms);
}

if (args.captions) {
  const result = spawnSync(args.or, ['timeline', 'import-captions', '--attach', args.attach,
    '--file', manifest.captions_srt, '--json'], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`import-captions failed: ${(result.stdout || result.stderr).slice(0, 300)}`);
  const captionTrack = or(['timeline', 'tracks']).timeline_tracks_v2.find((t) => t.kind === 'caption');
  if (captionTrack) {
    for (const clip of page(['timeline', 'clips', '--track', captionTrack.track_id])) {
      or(['timeline', 'set-text', '--track', captionTrack.track_id, '--clip', clip.clip_id,
        '--anchor', captionStyle.anchor, '--size-pt', String(captionStyle.sizePt),
        '--offset-y', String(captionStyle.offsetY), '--weight', captionStyle.weight]);
    }
  }
}

for (const marker of or(['timeline', 'markers']).timeline_marker_page.items) {
  or(['timeline', 'delete-marker', '--id', marker.marker_id]);
}
for (const marker of manifest.markers) {
  or(['timeline', 'add-marker', '--at', `${marker.at_ms}/1000`, '--label', marker.label]);
}

const summary = or(['project', 'summary']);
console.log(JSON.stringify({
  revision: summary.project_revision,
  video_track: videoTrack.track_id,
  voice_track: voiceTrack.track_id,
  music_track: musicTrack.track_id,
  narration_lines: manifest.lines.length,
  scenes: plan.length,
  captions: args.captions ? manifest.cues.length : 0,
  total_ms: totalMs,
  note: 'review the timeline, then `tools/or project save --attach project.json`',
}, null, 2));

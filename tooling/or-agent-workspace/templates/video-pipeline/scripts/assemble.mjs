#!/usr/bin/env node
// Put a narrated, illustrated video into the open OR project: clear the tracks,
// import narration and music, duck the bed, insert the rendered beats, refresh
// chapter markers, and save.
//
//   node scripts/assemble.mjs --video videos/<slug> --attach project.json [--dry-run]
//
// Tracks are discovered by kind (video / audio), so the template does not depend
// on ids from another project.

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const args = { video: null, attach: 'project.json', dryRun: false };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--video') args.video = process.argv[++i];
  else if (a === '--attach') args.attach = process.argv[++i];
  else if (a === '--dry-run') args.dryRun = true;
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const config = JSON.parse(await readFile(path.join(dir, 'video.config.json'), 'utf8'));
const manifest = JSON.parse(await readFile(path.join(dir, 'build', 'narration', 'manifest.json'), 'utf8'));
const plan = JSON.parse(await readFile(path.join(dir, 'build', 'scenes', 'publish-plan.json'), 'utf8'));
const renderDir = path.join(dir, 'build', 'renders');

function or(argv) {
  const r = spawnSync(path.resolve('tools/or'), [...argv, '--attach', args.attach, '--json'], { encoding: 'utf8' });
  if (r.status !== 0) throw new Error(`${argv.join(' ')} failed: ${(r.stdout || r.stderr).slice(0, 300)}`);
  return JSON.parse(r.stdout);
}
function page(argv) {
  const items = [];
  for (let offset = 0; ; ) {
    const res = or([...argv, '--offset', String(offset)]);
    const p = res.timeline_clip_page_v2;
    items.push(...p.items);
    if (items.length >= p.total_count) break;
    offset += p.items.length;
  }
  return items;
}

const tracks = or(['timeline', 'tracks']).timeline_tracks_v2;
const video = tracks.find((t) => t.kind === 'video');
const audioTracks = tracks.filter((t) => t.kind === 'audio');
if (!video) { console.error('the project has no video track; add one in the app first'); process.exit(2); }
if (!audioTracks.length) { console.error('the project has no audio track; add one in the app first'); process.exit(2); }
const [voiceTrack, musicTrack] = audioTracks;

const narrationSeconds = manifest.total_ms / 1000;
const mediaCache = new Map();
function ensureMedia(file) {
  const key = path.resolve(file);
  if (mediaCache.has(key)) return mediaCache.get(key);
  const all = [];
  for (let offset = 0; ; ) {
    const p = or(['media', 'list', '--offset', String(offset), '--limit', '100']).media_page;
    all.push(...p.items);
    if (all.length >= p.total_count) break;
    offset += 100;
  }
  const uri = `file://${key.split('/').map(encodeURIComponent).join('/')}`;
  const existing = all.find((item) => item.source.uri === uri);
  if (existing) { mediaCache.set(key, existing); return existing; }
  const added = or(['media', 'add', '--source', key]).media;
  mediaCache.set(key, added);
  return added;
}
function insert(trackId, file, atMs, maxMs) {
  const media = ensureMedia(file);
  const d = media.metadata.duration;
  let ms = Math.floor((d.numerator * 1000) / d.denominator);
  if (maxMs != null) ms = Math.min(ms, maxMs);
  return or(['timeline', 'insert-clip', '--track', trackId, '--media', media.id,
    '--at', `${atMs}/1000`, '--source-start', '0/1', '--duration', `${ms}/1000`]);
}

if (args.dryRun) {
  console.log(JSON.stringify({
    voice_track: voiceTrack.track_id, music_track: musicTrack.track_id, video_track: video.track_id,
    narration_lines: manifest.lines.length, builds: plan.length,
    beats: plan.map((b) => ({ beat: b.beat, at_s: b.at_ms / 1000, duration_s: b.duration_ms / 1000 })),
    music: config.music ?? null, markers: manifest.markers.length, seconds: narrationSeconds,
  }, null, 2));
  process.exit(0);
}

// clear the three tracks we own
for (const track of [voiceTrack, musicTrack, video]) {
  for (;;) {
    const items = page(['timeline', 'clips', '--track', track.track_id]);
    if (!items.length) break;
    items.forEach((clip) => or(['timeline', 'delete-clip', '--clip', clip.clip_id]));
  }
}
// caption track is optional: captions are baked into the scenes by default
if (config.captions?.baked !== false) {
  for (const track of or(['timeline', 'tracks']).timeline_tracks_v2.filter((t) => t.kind === 'caption')) {
    const items = page(['timeline', 'clips', '--track', track.track_id]);
    if (items.length) items.forEach((clip) => or(['timeline', 'delete-clip', '--clip', clip.clip_id]));
    or(['timeline', 'remove-track', '--track', track.track_id]);
  }
}

for (const line of manifest.lines) insert(voiceTrack.track_id, line.file, line.start_ms);

const musicFile = config.music?.file ? path.resolve(config.music.file) : null;
if (musicFile) {
  insert(musicTrack.track_id, musicFile, 0, manifest.total_ms);
  const musicClip = page(['timeline', 'clips', '--track', musicTrack.track_id])[0];
  if (musicClip) {
    or(['timeline', 'duck-clip', '--track', musicTrack.track_id, '--clip', musicClip.clip_id,
      '--voice-track', voiceTrack.track_id, '--duck-db', String(config.music.duckDb ?? -12)]);
  }
}

for (const beat of plan) {
  insert(video.track_id, path.join(renderDir, `${String(beat.index).padStart(2, '0')}.mkv`), beat.at_ms, beat.duration_ms);
}

for (const marker of or(['timeline', 'markers']).timeline_marker_page.items) {
  or(['timeline', 'delete-marker', '--id', marker.marker_id]);
}
for (const marker of manifest.markers) {
  or(['timeline', 'add-marker', '--at', `${marker.at_ms}/1000`, '--label', marker.label]);
}

const summary = or(['project', 'summary']);
console.log(JSON.stringify({
  saved_revision_before_save: summary.project_revision,
  narration_lines: manifest.lines.length, beats: plan.length,
  voice_track: voiceTrack.track_id, music_track: musicTrack.track_id, video_track: video.track_id,
  note: 'run `tools/or project save --attach project.json` to persist',
}, null, 2));

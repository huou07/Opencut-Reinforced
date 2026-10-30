#!/usr/bin/env node
// Speak script.json with the app's configured local TTS server, time-stretch
// each line with ffmpeg atempo, verify no line came back suspiciously short,
// then derive cue timing from the measured audio.
//
//   node scripts/narration.mjs --video videos/<slug> [--tempo 1.6] [--attach project.json]
//
// Voice settings are read from the app's narration-settings.json through the
// workspace pointer — never hardcode a voice name.

import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const args = { video: null, tempo: null, attach: 'project.json', voice: null };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--video') args.video = process.argv[++i];
  else if (a === '--tempo') args.tempo = Number(process.argv[++i]);
  else if (a === '--attach') args.attach = process.argv[++i];
  else if (a === '--voice') args.voice = process.argv[++i];
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const config = JSON.parse(await readFile(path.join(dir, 'video.config.json'), 'utf8'));
const script = JSON.parse(await readFile(path.join(dir, 'script.json'), 'utf8'));
const tempo = args.tempo ?? config.tempo ?? 1.6;
const gapMs = config.gap_ms ?? 120;

const pointer = JSON.parse(await readFile(path.resolve(args.attach), 'utf8'));
const settings = JSON.parse(await readFile(path.join(pointer.app_support_path, 'narration-settings.json'), 'utf8'));
const baseUrl = (settings.local_base_url || 'http://127.0.0.1:8000').replace(/\/$/, '');
const voice = (args.voice || settings.local_voice_name || settings.local_voice_id || '').trim();
if (!voice) { console.error('no local narration voice configured (Settings → Narration)'); process.exit(2); }

const outDir = path.join(dir, 'build', 'narration');
const rawDir = path.join(outDir, 'raw');
await mkdir(rawDir, { recursive: true });

async function tts(text) {
  const res = await fetch(`${baseUrl}/v1/audio/speech`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ input: text, voice, response_format: 'wav', sample_rate: 48000 }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return Buffer.from(await res.arrayBuffer());
}
function wavDuration(buffer) {
  let offset = 12, channels = 1, sampleRate = 48000, bits = 16;
  while (offset + 8 <= buffer.length) {
    const id = buffer.toString('ascii', offset, offset + 4);
    const size = buffer.readUInt32LE(offset + 4);
    const body = offset + 8;
    if (id === 'fmt ') { channels = buffer.readUInt16LE(body + 2); sampleRate = buffer.readUInt32LE(body + 4); bits = buffer.readUInt16LE(body + 14); }
    else if (id === 'data') return (Math.min(size, buffer.length - body) / (channels * bits / 8)) / sampleRate;
    offset = body + size + (size & 1);
  }
  throw new Error('bad wav');
}
function stretch(input, output) {
  const r = spawnSync('ffmpeg', ['-y', '-v', 'error', '-i', input, '-filter:a', `atempo=${tempo}`, '-ac', '1', '-ar', '48000', '-c:a', 'pcm_s16le', output], { encoding: 'utf8' });
  if (r.status !== 0) throw new Error((r.stderr || '').slice(0, 200));
}
function concat(paths, output) {
  const list = `${output}.txt`;
  spawnSync('sh', ['-c', `cat > "${list}" <<'EOF'\n${paths.map((p) => `file '${p.replace(/'/g, "'\\''")}'`).join('\n')}\nEOF`]);
  const r = spawnSync('ffmpeg', ['-y', '-v', 'error', '-f', 'concat', '-safe', '0', '-i', list, '-ac', '1', '-ar', '48000', '-c:a', 'pcm_s16le', output], { encoding: 'utf8' });
  if (r.status !== 0) throw new Error((r.stderr || '').slice(0, 200));
}

const lines = [];
let cursor = 0, index = 0, resplit = 0;
for (const scene of script.scenes) {
  index += 1;
  const expected = scene.narration.length / 16.5; // measured chars/second of this voice
  let raw = await tts(scene.narration);
  if (wavDuration(raw) < expected * 0.8) {
    const sentences = scene.narration.split(/(?<=[.!?])\s+/).filter(Boolean);
    if (sentences.length > 1) {
      resplit += 1;
      const parts = [];
      for (let i = 0; i < sentences.length; i += 1) {
        const partPath = path.join(rawDir, `${scene.id}-part${i}.wav`);
        await writeFile(partPath, await tts(sentences[i]));
        parts.push(partPath);
      }
      const joined = path.join(rawDir, `${scene.id}.wav`);
      concat(parts, joined);
      raw = await readFile(joined);
    }
  }
  const rawPath = path.join(rawDir, `${scene.id}.wav`);
  await writeFile(rawPath, raw);
  const finalPath = path.join(outDir, `${scene.id}.wav`);
  stretch(rawPath, finalPath);
  const durationMs = Math.round(wavDuration(await readFile(finalPath)) * 1000);
  lines.push({
    scene_id: scene.id, marker: scene.marker ?? null, narration: scene.narration,
    caption: scene.caption ?? scene.narration, file: finalPath,
    start_ms: cursor, end_ms: cursor + durationMs, duration_ms: durationMs,
  });
  cursor += durationMs + gapMs;
  console.log(`[${index}/${script.scenes.length}] ${scene.id} ${durationMs}ms`);
}

// --- caption cues from the script text -------------------------------------
const MAXW = 7, MAXC = 46;
const balanced = (words, maxw) => {
  const size = Math.ceil(words.length / Math.ceil(words.length / maxw));
  const out = [];
  for (let i = 0; i < words.length; i += size) out.push(words.slice(i, i + size).join(' '));
  return out;
};
const units = (text) => text.trim().split(/(?<=[,;:.!?])\s+/).filter(Boolean).flatMap((clause) => {
  const words = clause.split(/\s+/);
  return words.length > MAXW ? balanced(words, MAXW) : [clause];
});
const pack = (list) => {
  const out = [];
  let current = '';
  for (const unit of list) {
    const candidate = `${current} ${unit}`.trim();
    if (current && (candidate.split(/\s+/).length > MAXW || candidate.length > MAXC)) { out.push(current); current = unit; }
    else current = candidate;
  }
  if (current) out.push(current);
  const merged = [];
  for (const chunk of out) {
    if (merged.length && chunk.split(/\s+/).length <= 2 && merged.at(-1).length + 1 + chunk.length <= MAXC) merged[merged.length - 1] += ` ${chunk}`;
    else merged.push(chunk);
  }
  return merged;
};
const rawCues = [];
for (const line of lines) {
  const chunks = pack(units(line.caption));
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0) || 1;
  let consumed = 0;
  chunks.forEach((text, i) => {
    const start = line.start_ms + Math.round((line.duration_ms * consumed) / total) + (i ? 130 : 0);
    consumed += text.length;
    const end = i === chunks.length - 1 ? line.end_ms : line.start_ms + Math.round((line.duration_ms * consumed) / total);
    rawCues.push({ start_ms: start, end_ms: end, text });
  });
}
const cues = [];
rawCues.forEach((cue, i) => {
  const limit = rawCues[i + 1]?.start_ms ?? 1e9;
  const end = Math.min(cue.end_ms, limit);
  if (end - cue.start_ms < 280) {
    if (cues.length) { cues.at(-1).end_ms = Math.max(cues.at(-1).end_ms, end); cues.at(-1).text += ` ${cue.text}`; }
  } else cues.push({ ...cue, end_ms: end });
});
const srt = (ms) => {
  const h = Math.floor(ms / 3_600_000), m = Math.floor((ms % 3_600_000) / 60_000), s = Math.floor((ms % 60_000) / 1000), f = ms % 1000;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')},${String(f).padStart(3, '0')}`;
};
await writeFile(path.join(outDir, 'captions.srt'), `${cues.map((c, i) => `${i + 1}\n${srt(c.start_ms)} --> ${srt(c.end_ms)}\n${c.text}\n`).join('\n')}\n`, 'utf8');

const manifest = {
  format: 'or-voiceover-manifest', schema_version: 1, source: 'local-tts-server+atempo',
  voice, tempo, gap_ms: gapMs, script: path.join(dir, 'script.json'), title: config.title ?? null,
  captions_srt: path.join(outDir, 'captions.srt'), total_ms: lines.at(-1)?.end_ms ?? 0,
  cues, markers: lines.filter((l) => l.marker).map((l) => ({ label: l.marker, at_ms: l.start_ms })), lines,
};
await writeFile(path.join(outDir, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
console.log(JSON.stringify({ total_ms: manifest.total_ms, minutes: +(manifest.total_ms / 60000).toFixed(2), lines: lines.length, cues: cues.length, resplit, tempo }, null, 2));

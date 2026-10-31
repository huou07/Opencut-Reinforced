#!/usr/bin/env node
// Re-chunk the measured narration into readable caption cues. The audio timing is
// already locked in manifest.json; this only changes where a cue breaks, never
// when the voice speaks. No model, no transcription, no TTS call.
//
// The documentary rhythm lives in video.config.json → captions.chunk:
// at most 12 words / 72 characters per cue, at least 900 ms on screen, later
// chunks start 110 ms after their character position. That is why a five-minute
// film carries ~190 cues and not 65: one spoken line is two or three readable
// cards, and a card never leads the voice.
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const args = { video: null };
for (let i = 2; i < process.argv.length; i += 1) {
  if (process.argv[i] === '--video') args.video = process.argv[++i];
  else { console.error(`unknown ${process.argv[i]}`); process.exit(2); }
}
if (!args.video) { console.error('--video videos/<slug> is required'); process.exit(2); }

const dir = path.resolve(args.video);
const config = JSON.parse(await readFile(path.join(dir, 'video.config.json'), 'utf8'));
const rules = { max_words: 12, max_chars: 72, min_cue_ms: 900, lead_ms: 110, ...(config.captions?.chunk ?? {}) };
const manifestPath = path.join(dir, 'build', 'narration', 'manifest.json');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));

const MAX_WORDS = rules.max_words;
const MAX_CHARS = rules.max_chars;
const MIN_CUE_MS = rules.min_cue_ms;

const balanced = (words, maxWords) => {
  const size = Math.ceil(words.length / Math.ceil(words.length / maxWords));
  const out = [];
  for (let i = 0; i < words.length; i += size) out.push(words.slice(i, i + size).join(' '));
  return out;
};
const units = (text) => text.trim().split(/(?<=[,;:.!?…])\s+/).filter(Boolean).flatMap((clause) => {
  const words = clause.split(/\s+/);
  return words.length > MAX_WORDS ? balanced(words, MAX_WORDS) : [clause];
});
const pack = (list) => {
  const out = [];
  let current = '';
  for (const unit of list) {
    const candidate = `${current} ${unit}`.trim();
    if (current && (candidate.split(/\s+/).length > MAX_WORDS || candidate.length > MAX_CHARS)) {
      out.push(current);
      current = unit;
    } else current = candidate;
  }
  if (current) out.push(current);
  // fold a two-word orphan into the previous cue when it still fits
  const merged = [];
  for (const chunk of out) {
    if (merged.length && chunk.split(/\s+/).length <= 2
      && merged.at(-1).length + 1 + chunk.length <= MAX_CHARS + 8) {
      merged[merged.length - 1] += ` ${chunk}`;
    } else merged.push(chunk);
  }
  return merged;
};

const cues = [];
for (const line of manifest.lines) {
  const chunks = pack(units(line.caption || line.narration));
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0) || 1;
  let consumed = 0;
  chunks.forEach((text, i) => {
    const rawStart = line.start_ms + Math.round((line.duration_ms * consumed) / total) + (i ? rules.lead_ms : 0);
    consumed += text.length;
    const rawEnd = i === chunks.length - 1
      ? line.end_ms
      : line.start_ms + Math.round((line.duration_ms * consumed) / total);
    const start = Math.min(rawStart, line.end_ms - 300);
    const end = Math.min(Math.max(rawEnd, start + 400), line.end_ms);
    cues.push({ start_ms: start, end_ms: end, text, scene_id: line.scene_id });
  });
}

// Never overlap the next cue; a very short cue folds into its predecessor.
const clean = [];
cues.forEach((cue, i) => {
  const limit = cues[i + 1]?.start_ms ?? 1e9;
  const end = Math.min(cue.end_ms, limit);
  if (end - cue.start_ms < MIN_CUE_MS && clean.length
    && cue.text.split(/\s+/).length <= 2
    && clean.at(-1).text.length + 1 + cue.text.length <= MAX_CHARS + 12) {
    clean.at(-1).end_ms = Math.max(clean.at(-1).end_ms, end);
    clean.at(-1).text += ` ${cue.text}`;
  } else {
    clean.push({ ...cue, end_ms: end });
  }
});
for (let i = 0; i < clean.length - 1; i += 1) {
  if (clean[i].end_ms > clean[i + 1].start_ms) clean[i].end_ms = clean[i + 1].start_ms;
}

const srt = (ms) => {
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor((ms % 3_600_000) / 60_000);
  const s = Math.floor((ms % 60_000) / 1000);
  const f = ms % 1000;
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')},${String(f).padStart(3, '0')}`;
};
const srtPath = path.join(dir, 'build', 'narration', 'captions.srt');
await writeFile(srtPath, `${clean.map((cue, i) => `${i + 1}\n${srt(cue.start_ms)} --> ${srt(cue.end_ms)}\n${cue.text}\n`).join('\n')}\n`, 'utf8');
manifest.cues = clean;
manifest.captions_srt = srtPath;
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

const lengths = clean.map((c) => c.end_ms - c.start_ms);
console.log(JSON.stringify({
  cues: clean.length,
  avg_ms: Math.round(lengths.reduce((a, b) => a + b, 0) / lengths.length),
  min_ms: Math.min(...lengths),
  max_ms: Math.max(...lengths),
  words_avg: (clean.reduce((a, c) => a + c.text.split(/\s+/).length, 0) / clean.length).toFixed(1),
  srt: srtPath,
}, null, 2));

#!/usr/bin/env node
// Scaffold a new video folder from the template.
//
//   node scripts/new-video.mjs --slug my-video [--title "My video"] [--seconds 59]
//
// Writes videos/<slug>/script.json and video.config.json with a showcase-shaped
// beat map: seven windows that tile the declared duration exactly. Edit the words
// first, then follow the five steps in README.md.
//
// The starter is deliberately short. A longer piece is the same shape with more
// beats: every composer takes 5–13 s, so a five-minute film is 25–35 windows.

import { access, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const args = { slug: null, title: null, seconds: 20 };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--slug') args.slug = process.argv[++i];
  else if (a === '--title') args.title = process.argv[++i];
  else if (a === '--seconds') args.seconds = Number(process.argv[++i]);
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.slug || !/^[a-z0-9][a-z0-9-]*$/.test(args.slug)) {
  console.error('--slug must be kebab-case, e.g. --slug product-tour');
  process.exit(2);
}

const dir = path.resolve('videos', args.slug);
try { await access(dir); console.error(`${dir} already exists`); process.exit(2); } catch {}
await mkdir(dir, { recursive: true });

const script = {
  format: 'or-assistant-script',
  schema_version: 1,
  title: args.title ?? args.slug,
  language: 'vi',
  tempo: 1.6,
  voice: '',
  scenes: [
    { id: 'mo', beat: 'open', marker: '01 · Mở', start_ms: 700, narration: 'Mở bằng một hình ảnh hoàn chỉnh, rồi cho nó tách ra thành các lớp.' },
    { id: 'than', beat: 'body', start_ms: 5600, narration: 'Một hành động cụ thể trên giao diện, kèm kết quả nhìn thấy được.' },
    { id: 'diem', beat: 'body', start_ms: 10000, narration: 'Một câu nêu đúng giá trị, không đọc danh sách tính năng.' },
    { id: 'ket', beat: 'close', marker: 'Kết', start_ms: 15000, narration: 'Kết bằng tên sản phẩm và một việc người xem có thể thử ngay.' },
  ],
};
const total = Math.round(args.seconds * 1000);
const config = {
  slug: args.slug,
  title: script.title,
  language: 'vi',
  tempo: 1.6,
  duration_ms: total,
  canvas: { width: 1920, height: 1080 },
  captions: { anchor: 'bottom', sizePt: 26, offsetY: -420, weight: 'medium' },
  music: { file: 'work/music/bed.wav', duckDb: -9, prompt: 'uplifting, 120 bpm, strong drums' },
  beats: [
    { id: 'open', composer: 'deconstruct', start_ms: 0, end_ms: 5000, lines: ['mo'] },
    { id: 'editor', composer: 'editor', start_ms: 5000, end_ms: 9000, lines: ['than'] },
    { id: 'scene', composer: 'motionScene', start_ms: 9000, end_ms: 14000, lines: ['diem'] },
    { id: 'close', composer: 'close', start_ms: 14000, end_ms: total, lines: ['ket'] },
  ],
};
// The last beat runs to the declared end, so the windows always tile duration_ms.

await writeFile(path.join(dir, 'script.json'), `${JSON.stringify(script, null, 2)}\n`, 'utf8');
await writeFile(path.join(dir, 'video.config.json'), `${JSON.stringify(config, null, 2)}\n`, 'utf8');
console.log(`created ${dir}`);
console.log('next: write the lines, then follow README.md steps 2–5');

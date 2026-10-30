#!/usr/bin/env node
// Scaffold a new video folder from the template.
//
//   node scripts/new-video.mjs --slug my-video [--title "My video"] [--minutes 5]
//
// Writes videos/<slug>/script.json and video.config.json with a starter beat
// map. Edit the script first, then run the pipeline in README order.

import { mkdir, writeFile, access } from 'node:fs/promises';
import path from 'node:path';

const args = { slug: null, title: null, minutes: 5 };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--slug') args.slug = process.argv[++i];
  else if (a === '--title') args.title = process.argv[++i];
  else if (a === '--minutes') args.minutes = Number(process.argv[++i]);
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
  gap_ms: 120,
  scenes: [
    { id: 'mo-dau', marker: 'Mở đầu', narration: 'Mở bằng một kết quả hoặc một câu hỏi khiến người xem muốn biết cơ chế phía sau.' },
    { id: 'y-tuong', narration: 'Một cảnh giải thích ý tưởng chính, kèm một hành động cụ thể trên giao diện.' },
    { id: 'ket', narration: 'Kết bằng giá trị thực và một việc người xem có thể thử ngay.' },
  ],
};
const config = {
  slug: args.slug,
  title: script.title,
  language: 'vi',
  tempo: 1.6,
  gap_ms: 120,
  target_minutes: args.minutes,
  canvas: { width: 1920, height: 1080 },
  captions: { band: 952, font: 28, baked: true },
  music: { file: 'work/music/bed.wav', duckDb: -12, prompt: 'nhạc nền kịch tính dồn dập, 150 bpm, trống mạnh' },
  beats: [
    { id: 'open', composer: 'open', camera: 'push', lines: ['mo-dau'] },
    { id: 'assembly', composer: 'assembly', camera: 'pull', lines: ['y-tuong'] },
    { id: 'close', composer: 'close', camera: 'push', lines: ['ket'] },
  ],
};
await writeFile(path.join(dir, 'script.json'), `${JSON.stringify(script, null, 2)}\n`, 'utf8');
await writeFile(path.join(dir, 'video.config.json'), `${JSON.stringify(config, null, 2)}\n`, 'utf8');
console.log(`created ${dir}`);
console.log('next: edit script.json and video.config.json, then follow ../README.md');

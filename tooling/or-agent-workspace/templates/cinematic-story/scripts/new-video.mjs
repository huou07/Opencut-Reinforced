#!/usr/bin/env node
// Scaffold a new long-form video folder from the cinematic-story template.
//
//   node scripts/new-video.mjs --slug my-film [--title "My film"] [--seconds 300]
//
// `--seconds` pins an exact runtime (a 5:00 film). Omit it and the film ends
// `tail_ms` after the last measured line.
//
// Writes videos/<slug>/script.json and video.config.json with the house defaults
// (voice read from Settings → Narration, tempo 1.6, 190 ms gaps, the documentary
// caption look, a constant −6 dB bed) and a chapter-shaped beat map.
//
// The starter is a 60 s sketch: eight beats, two chapter markers. A five-minute
// film is the same shape with 25–40 beats and 10–14 markers. Write the words
// first; the narration step measures them and the beat windows are derived from
// the measured voice, so `duration_ms` is optional (omit it to end 1.4 s after
// the last line; set it to hit an exact runtime).

import { access, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const args = { slug: null, title: null, seconds: null };
for (let i = 2; i < process.argv.length; i += 1) {
  const a = process.argv[i];
  if (a === '--slug') args.slug = process.argv[++i];
  else if (a === '--title') args.title = process.argv[++i];
  else if (a === '--seconds') args.seconds = Number(process.argv[++i]);
  else { console.error(`unknown ${a}`); process.exit(2); }
}
if (!args.slug || !/^[a-z0-9][a-z0-9-]*$/.test(args.slug)) {
  console.error('--slug must be kebab-case, e.g. --slug product-story');
  process.exit(2);
}

const dir = path.resolve('videos', args.slug);
try { await access(dir); console.error(`${dir} already exists`); process.exit(2); } catch {}
await mkdir(dir, { recursive: true });

// One line per idea, 25–45 words. The opener names the frame; the body says what
// happens and why it matters; the close names the product and one next action.
const script = {
  format: 'or-assistant-script',
  schema_version: 1,
  title: args.title ?? args.slug,
  language: 'vi',
  tempo: 1.6,
  voice: '',
  gap_ms: 190,
  scenes: [
    { id: 'mo1', beat: 'open', marker: '01 · Mở', start_ms: 500, narration: 'Mở bằng một hình ảnh đã hoàn chỉnh, rồi cho nó tách ra thành các lớp: hình, tiếng, giọng và chữ.' },
    { id: 'mo2', beat: 'open', narration: 'Bốn lớp ấy phải khớp với nhau đến từng khung hình. Khi lệch đi, người xem cảm nhận được ngay, dù không gọi tên được.' },
    { id: 'ten1', beat: 'reveal', marker: '02 · Sản phẩm', narration: 'Sản phẩm này làm đúng việc đó: một môi trường dựng phim thật, nơi con người và trợ lý cùng làm việc trên một timeline.' },
    { id: 'ui1', beat: 'editor', narration: 'Đây là giao diện thật. Clip được đặt xuống, cắt đúng khung, và mọi thay đổi đều có thể hoàn tác.' },
    { id: 'ai1', beat: 'assistant', narration: 'Trợ lý không đứng ngoài cửa sổ chat. Nó đọc được dự án đang mở, dựng cảnh bằng mã, rồi trả kết quả về đúng track.' },
    { id: 'ms1', beat: 'motion', narration: 'Một cảnh có thể bắt đầu từ vài dòng mã: hình học dựng lên, camera đi vào, và khung hình cuối trở thành một clip bình thường.' },
    { id: 'ket1', beat: 'close', marker: '03 · Kết', narration: 'Đó là toàn bộ vòng làm việc, gói trong một ứng dụng chạy trên máy của bạn.' },
    { id: 'ket2', beat: 'close', narration: 'Nếu thấy hữu ích, hãy mở thử dự án mẫu và tự dựng một cảnh.' },
  ],
};
const config = {
  slug: args.slug,
  title: script.title,
  language: 'vi',
  tempo: 1.6,
  gap_ms: 190,
  tail_ms: 1400,
  ...(args.seconds ? { duration_ms: Math.round(args.seconds * 1000) } : {}),
  canvas: { width: 1920, height: 1080 },
  narration: { chars_per_second: 28 },
  captions: {
    anchor: 'bottom',
    sizePt: 25,
    offsetY: -280,
    weight: 'medium',
    font: 'be-vietnam-pro',
    color: '#F4F6FA',
    background: '#0B0D11',
    backgroundPadding: 8000,
    backgroundOpacity: 120,
    chunk: { max_words: 12, max_chars: 72, min_cue_ms: 900, lead_ms: 110 },
  },
  music: {
    file: 'work/music/bed.wav',
    duckDb: null,
    gainDb: -6,
    prompt: 'cinematic documentary, warm, 120 bpm, light drums',
  },
  beats: [
    { id: 'open', composer: 'coldOpen', start_ms: 0, lines: ['mo1', 'mo2'] },
    { id: 'reveal', composer: 'reveal', lines: ['ten1'] },
    { id: 'editor', composer: 'editorBuild', lines: ['ui1'] },
    { id: 'assistant', composer: 'chatAsk', lines: ['ai1'] },
    { id: 'motion', composer: 'motionBuild', lines: ['ms1'] },
    { id: 'close', composer: 'close', lines: ['ket1', 'ket2'] },
  ],
};
// The windows are derived: the last beat ends at duration_ms, or `tail_ms` after
// the last measured line. Do not hand-write end_ms unless you need a hard cut.

await writeFile(path.join(dir, 'script.json'), `${JSON.stringify(script, null, 2)}\n`, 'utf8');
await writeFile(path.join(dir, 'video.config.json'), `${JSON.stringify(config, null, 2)}\n`, 'utf8');
console.log(`created ${dir}`);
console.log('next: write the lines, then follow README.md steps 2–6');

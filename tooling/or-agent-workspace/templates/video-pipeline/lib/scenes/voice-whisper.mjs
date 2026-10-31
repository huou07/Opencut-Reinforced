// S5 — 31–39 s. VieNeu-TTS speaks the script locally; Whisper listens and the
// captions lock to the voice.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';

const SCRIPT = ['VieNeu-TTS đọc kịch bản', 'bằng giọng local.', 'Rồi Whisper nghe lại', 'và tạo phụ đề.'];

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 700, 200, 1300, 1000, A(C.cyan, 12));
  ell('glowB', 200, 560, 900, 800, A(C.violet, 10));
  for (let i = 0; i < 12; i += 1) rect(`gx${i}`, 120 + i * 150, 0, 1, 1080, A(C.edge, 16));

  // ---------------------------------------------------------------- script card
  const P = { x: 150, y: 236, w: 700, h: 430 };
  rect('scriptShadow', P.x + 14, P.y + 16, P.w, P.h, { red: 0, green: 0, blue: 0, alpha: 140 });
  rect('script', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('scriptBar', P.x, P.y, P.w, 42, C.panel2);
  ell('scriptDot', P.x + 18, P.y + 17, 8, 8, A(C.cyan, 235));
  txt('scriptTitle', 'KỊCH BẢN', P.x + 34, P.y + 21, 14, A(C.ink, 230));
  txt('scriptFile', 'script.json', P.x + P.w - 20 - textW('script.json', 12), P.y + 22, 12, A(C.dim, 210));
  rect('scriptRule', P.x, P.y + 42, P.w, 1, A(C.edge2, 180));
  let karaoke = 0;
  SCRIPT.forEach((line, i) => {
    const y = P.y + 90 + i * 62;
    const w = textW(line, 30);
    if (i < 2) karaoke = Math.max(karaoke, w);
    txt(`scL${i}`, line, P.x + 40, y, 30, A(i < 2 ? C.ink : C.mute, 230));
    anim(`scL${i}`, 'opacity', [[0.1 + i * 0.1, 0], [0.4 + i * 0.1, 1]], 'ease_out');
  });
  // karaoke sweep behind the spoken lines
  rect('kara', P.x + 30, P.y + 78, 0, 46, A(C.cyan, 34));
  anim('kara', 'width', [[0.7, 0], [1.4, karaoke + 20], [2.2, 0.6], [3.0, karaoke + 20], [3.5, karaoke + 20]], 'ease_in_out');
  anim('kara', 'opacity', [[0.7, 0], [0.9, 1], [3.5, 1], [3.8, 0]], 'ease_in_out');
  rect('karaCursor', P.x + 30, P.y + 74, 3, 44, A(C.cyan, 200));
  anim('karaCursor', 'translation_x', [[0.7, 0], [3.2, 420]], 'ease_in_out');
  anim('karaCursor', 'opacity', [[0.7, 0], [0.9, 1], [3.4, 1], [3.6, 0]], 'ease_in_out');

  // synthesis status under the script
  rect('statBg', 260, 700, 1400, 34, A(C.panel, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
  rect('statFill', 262, 731, 4, 2, A(C.cyan, 230));
  anim('statFill', 'width', [[0.7, 4], [2.9, 1380]], 'linear');
  txt('statLabel', 'ĐANG TỔNG HỢP GIỌNG ĐỌC', 280, 717, 12, A(C.dim, 220));
  txt('statPct', '100%', 1640 - textW('100%', 12), 717, 12, A(C.mute, 210));
  anim('statPct', 'opacity', [[2.94, 0], [3.04, 1]], 'linear');
  ['statBg', 'statFill', 'statLabel', 'statPct'].forEach((id) => {
    anim(id, 'opacity', [[3.3, 1], [3.55, 0]], 'ease_in');
  });

  // ------------------------------------------------------------------ waveform
  const A1 = { x: 960, y: 300, w: 800, h: 200 };
  rect('audPanel', A1.x - 30, A1.y - 90, A1.w + 60, A1.h + 180, A(C.panel, 255), { stroke: A(C.edge2, 200), stroke_width: 1 });
  rect('audBar', A1.x - 30, A1.y - 90, A1.w + 60, 38, C.panel2);
  ell('audDot', A1.x - 12, A1.y - 73, 8, 8, A(C.cyan, 235));
  txt('audTitle', 'VIENEU-TTS', A1.x + 4, A1.y - 70, 13, A(C.ink, 230));
  txt('audLocal', 'LOCAL · 48 kHz', A1.x + A1.w - 50 - textW('LOCAL · 48 kHz', 12), A1.y - 70, 12, A(C.green, 220));
  const BARS = 54;
  for (let i = 0; i < BARS; i += 1) {
    const base = 16 + ((i * 71 + 23) % 92);
    const id = `w${i}`;
    rect(id, A1.x + i * 14.5, A1.y + 96, 5, base, A(cyanBar(i), 215));
    const keys = [];
    for (let k = 0; k <= 6; k += 1) {
      const t = (3.4 * k) / 6;
      const wave = 0.2 + 0.8 * Math.abs(Math.sin((k + 1) * (0.7 + ((i * 7919) % 89) / 55) + i * 0.4));
      keys.push([t, Math.max(5, base * wave)]);
    }
    anim(id, 'height', keys, 'ease_in_out');
    anim(id, 'translation_y', keys.map(([t, v]) => [t, (base - v) / 2]), 'ease_in_out');
  }
  function cyanBar(i) { return i % 7 === 0 ? C.blue : C.cyan; }
  rect('audBase', A1.x, A1.y + 190, A1.w, 1, A(C.edge2, 190));
  for (let i = 0; i < 5; i += 1) {
    rect(`audTick${i}`, A1.x + i * 200, A1.y + 178, 1, 12, A(C.edge2, 180));
    txt(`audTickT${i}`, `${i * 2}s`, A1.x + i * 200 + 4, A1.y + 224, 11, A(C.dim, 200));
  }
  // mic + rings
  const M = { cx: A1.x + 60, cy: A1.y + 56 };
  ell('mic', M.cx - 16, M.cy - 16, 32, 32, A(C.cyan, 235));
  rect('micStem', M.cx - 3, M.cy + 16, 6, 12, A(C.cyan, 200));
  rect('micBase', M.cx - 12, M.cy + 28, 24, 3, A(C.cyan, 200));
  for (let i = 0; i < 3; i += 1) {
    const r = 30 + i * 26;
    ell(`micR${i}`, M.cx - r, M.cy - r, r * 2, r * 2, A(C.cyan, 0), { stroke: A(C.cyan, 120 - i * 30), stroke_width: 1.4 });
    anim(`micR${i}`, 'scale', [[0.4 + i * 0.3, 0.6], [1.4 + i * 0.3, 1.6]], 'ease_out');
    anim(`micR${i}`, 'opacity', [[0.4 + i * 0.3, 0], [0.7 + i * 0.3, 1], [1.4 + i * 0.3, 0]], 'ease_out');
  }

  // ------------------------------------------------------- whisper + captions
  const T_W = 3.4;
  rect('capBand', 260, 760, 1400, 132, A(C.bg2, 235), { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('capBandFill', 260, 760, 3, 132, A(C.ciel ?? C.cyan, 200));
  txt('capTag', 'WHISPER · LOCAL', 292, 792, 13, A(C.mute, 220));
  ell('capTagDot', 282, 800, 7, 7, A(C.green, 230));
  const words = ['Hình ảnh', 'tách', 'ra', 'từng', 'lớp,', 'rồi', 'khớp', 'lại.'];
  let cx = 330;
  words.forEach((word, i) => {
    const w = textW(word, 32) + 16;
    rect(`capW${i}`, cx, 858, w - 12, 3, A(C.cyan, 190));
    txt(`capWt${i}`, word, cx, 842, 32, A(C.ink, 240));
    const t0 = T_W + 0.3 + i * 0.16;
    anim(`capW${i}`, 'opacity', [[t0, 0], [t0 + 0.2, 1]], 'ease_out');
    anim(`capWt${i}`, 'opacity', [[t0, 0], [t0 + 0.2, 1]], 'ease_out');
    anim(`capWt${i}`, 'translation_y', [[t0, 8], [t0 + 0.24, 0]], 'ease_out');
    cx += w;
  });
  ['capBand', 'capBandFill', 'capTag', 'capTagDot'].forEach((id) => anim(id, 'opacity', [[T_W, 0], [T_W + 0.4, 1]], 'ease_out'));

  // whisper result card
  const Wc = { x: 960, y: 300, w: 800, h: 250 };
  rect('wcShadow', Wc.x + 12, Wc.y + 14, Wc.w, Wc.h, { red: 0, green: 0, blue: 0, alpha: 120 });
  rect('wc', Wc.x, Wc.y, Wc.w, Wc.h, C.panel, { stroke: A(C.edge2, 200), stroke_width: 1 });
  ell('wcDot', Wc.x + 20, Wc.y + 18, 8, 8, A(C.violet, 235));
  txt('wcTitle', 'WHISPER · LOCAL', Wc.x + 36, Wc.y + 22, 13, A(C.ink, 230));
  txt('wcLang', 'TIẾNG VIỆT · 12 CUE', Wc.x + Wc.w - 22 - textW('TIẾNG VIỆT · 12 CUE', 12), Wc.y + 23, 12, A(C.dim, 210));
  const tr = ['Hình ảnh tách ra từng lớp,', 'rồi khớp lại.', 'Một video hoàn chỉnh.'];
  tr.forEach((line, i) => {
    txt(`wcL${i}`, line, Wc.x + 30, Wc.y + 84 + i * 46, 24, A(i === 1 ? C.ink : C.mute, 230));
    rect(`wcT${i}`, Wc.x + 20, Wc.y + 72 + i * 46, 2, 26, A(C.violet, 160));
  });
  rect('wcRule', Wc.x + 20, Wc.y + Wc.h - 34, Wc.w - 40, 1, A(C.edge, 180));
  txt('wcNote', 'ĐỒNG BỘ TỰ ĐỘNG VỚI GIỌNG ĐỌC', Wc.x + 20, Wc.y + Wc.h - 18, 11, A(C.dim, 200));
  ['wcShadow', 'wc', 'wcDot', 'wcTitle', 'wcLang', 'wcRule', 'wcNote', ...tr.map((_, i) => `wcL${i}`), ...tr.map((_, i) => `wcT${i}`)].forEach((id) => {
    anim(id, 'opacity', [[T_W + 0.45, 0], [T_W + 0.75, 1]], 'ease_out');
  });
  anim('wc', 'translation_y', [[T_W + 0.45, 20], [T_W + 0.8, 0]], 'ease_out');
  anim('wcShadow', 'translation_y', [[T_W + 0.45, 20], [T_W + 0.8, 0]], 'ease_out');

  // whisper listening arc
  const Wp = { cx: 700, cy: 640 };
  for (let i = 0; i < 3; i += 1) {
    ell(`whisR${i}`, Wp.cx - 40, Wp.cy - 40, 80, 80, A(C.violet, 0), { stroke: A(C.violet, 150 - i * 40), stroke_width: 1.6 });
    anim(`whisR${i}`, 'scale', [[T_W + i * 0.25, 0.5], [T_W + 0.9 + i * 0.25, 2.4]], 'ease_out');
    anim(`whisR${i}`, 'opacity', [[T_W + i * 0.25, 0], [T_W + 0.4 + i * 0.25, 1], [T_W + 0.9 + i * 0.25, 0]], 'ease_out');
  }
  txt('whisLabel', 'WHISPER', 1090, 660, 14, A(C.violet, 235));
  anim('whisLabel', 'opacity', [[T_W, 0], [T_W + 0.4, 1]], 'ease_out');
  anim('whisLabel', 'opacity', [[T_W + 3.6, 1], [T_W + 3.9, 0]], 'ease_out');
  anim('audTitle', 'opacity', [[T_W, 1], [T_W + 0.3, 0]], 'ease_in');
  anim('audLocal', 'opacity', [[T_W, 1], [T_W + 0.3, 0]], 'ease_in');
  // the voice card hands over to the Whisper card
  ['audPanel', 'audBar', 'audDot'].forEach((id) => anim(id, 'opacity', [[T_W + 0.3, 1], [T_W + 0.6, 0]], 'ease_in'));
  for (let i = 0; i < BARS; i += 1) anim(`w${i}`, 'opacity', [[T_W + 0.3, 1], [T_W + 0.6, 0]], 'ease_in');
  for (let i = 0; i < 5; i += 1) {
    anim(`audTick${i}`, 'opacity', [[T_W, 1], [T_W + 0.3, 0]], 'ease_in');
    anim(`audTickT${i}`, 'opacity', [[T_W, 1], [T_W + 0.3, 0]], 'ease_in');
  }
  ['mic', 'micStem', 'micBase'].forEach((id) => anim(id, 'opacity', [[T_W, 1], [T_W + 0.3, 0]], 'ease_in'));

  // end label
  txt('endTag', 'PHỤ ĐỀ TỰ ĐỘNG · KHỚP TỪNG TỪ', 700, 700, 14, A(C.dim, 230));
  anim('endTag', 'opacity', [[T_W + 2.4, 0], [T_W + 2.8, 1]], 'ease_out');

  particles('dust', 1300, 400, 20, 320, C.cyan, d, { size: 1.6, speed: 16 });

  return finish({
    name: opts.name ?? 'Voice + Whisper',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_y: 10 },
      { t: 1.0, zoom: 1.0 },
      { t: 2.8, zoom: 1.04, pan_x: 10 },
      { t: 4.2, zoom: 1.02, pan_y: 20 },
      { t: 6.2, zoom: 1.06, pan_y: 10 },
      { t: 8, zoom: 1.02 },
    ],
  });
}

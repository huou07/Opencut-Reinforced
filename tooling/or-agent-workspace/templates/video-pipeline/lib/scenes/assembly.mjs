// S6 — 39–52 s. Everything converges on one timeline, then the camera dives into
// the programme monitor and the finished film plays.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 200, 100, 1200, 900, A(C.blue, 10));
  ell('glowB', 1200, 400, 1100, 900, A(C.violet, 10));

  // ------------------------------------------------------------------ monitor
  const V = { x: 1184, y: 116, w: 560, h: 315 };
  rect('monBg', V.x - 10, V.y - 10, V.w + 20, V.h + 20, C.panel);
  rect('mon', V.x, V.y, V.w, V.h, C.bg);
  // shot A: sunset field
  rect('mA_sky', V.x, V.y, V.w, 210, { red: 17, green: 21, blue: 28, alpha: 255 });
  ell('mA_sun', V.x + 250, V.y + 40, 130, 130, A(C.amber, 235));
  ell('mA_sunC', V.x + 268, V.y + 58, 94, 94, A({ red: 251, green: 208, blue: 138, alpha: 255 }, 235));
  rect('mA_ridge', V.x + 4, V.y + 188, 300, 110, { red: 9, green: 11, blue: 14, alpha: 255 }, { rotation_degrees: -4 });
  rect('mA_ridgeL', V.x + 4, V.y + 188, 300, 2, A(C.amber, 80), { rotation_degrees: -4 });
  rect('mA_ground', V.x, V.y + 236, V.w, 79, { red: 7, green: 8, blue: 10, alpha: 255 });
  ell('mA_head', V.x + 96, V.y + 168, 12, 12, { red: 4, green: 5, blue: 7, alpha: 255 });
  rect('mA_body', V.x + 92, V.y + 178, 20, 30, { red: 4, green: 5, blue: 7, alpha: 255 });
  rect('mA_rim', V.x + 110, V.y + 178, 2, 28, A(C.amber, 170));
  for (let i = 0; i < 3; i += 1) {
    rect(`mA_b${i}`, V.x + 130 + i * 22, V.y + 52 + i * 10, 12, 1.5, A(C.ink, 100), { rotation_degrees: -16 });
    rect(`mA_b${i}b`, V.x + 136 + i * 22, V.y + 53 + i * 10, 12, 1.5, A(C.ink, 100), { rotation_degrees: 16 });
  }
  // shot B: city
  rect('mB', V.x, V.y, V.w, V.h, { red: 10, green: 15, blue: 21, alpha: 255 });
  rect('mB_sky', V.x, V.y, V.w, 180, { red: 12, green: 18, blue: 26, alpha: 255 });
  ell('mB_glow', V.x + 60, V.y + 150, 460, 220, A(C.cyan, 10));
  for (let i = 0; i < 9; i += 1) {
    const h = 90 + ((i * 53) % 150);
    rect(`mB_t${i}`, V.x + 10 + i * 62, V.y + V.h - h - 24, 48, h, { red: 14, green: 20, blue: 28, alpha: 255 });
    rect(`mB_tc${i}`, V.x + 10 + i * 62, V.y + V.h - h - 24, 48, 2, A(C.cyan, 70));
    for (let c = 0; c < 3; c += 1) {
      for (let k = 0; k < 6; k += 1) {
        rect(`mB_w${i}_${c}_${k}`, V.x + 20 + i * 62 + c * 14, V.y + V.h - h - 14 + k * 22, 7, 6, A((i + c + k) % 3 === 0 ? C.cyan : C.amber, 150));
      }
    }
  }
  rect('mB_ground', V.x, V.y + V.h - 24, V.w, 24, { red: 7, green: 9, blue: 12, alpha: 255 });
  rect('mB_groundL', V.x, V.y + V.h - 24, V.w, 1, A(C.cyan, 70));
  for (let i = 0; i < 4; i += 1) {
    rect(`mB_trail${i}`, V.x - 120, V.y + 208 + i * 26, 120, 2, A(C.cyan, 160));
    anim(`mB_trail${i}`, 'translation_x', [[6.9 - i * 0.1, 0], [7.9 - i * 0.1, 700]], 'linear');
  }
  // shot C: portrait
  rect('mC', V.x, V.y, V.w, V.h, { red: 24, green: 18, blue: 34, alpha: 255 });
  rect('mC_sky', V.x, V.y, V.w, 70, { red: 14, green: 11, blue: 21, alpha: 255 });
  ell('mC_halo', V.x + 120, V.y + 10, 300, 300, A(C.amber, 14));
  ell('mC_halo2', V.x + 146, V.y + 36, 248, 248, A(C.amber, 12));
  ell('mC_shoulder', V.x + 80, V.y + 250, 400, 220, { red: 40, green: 30, blue: 54, alpha: 255 });
  rect('mC_neck', V.x + 236, V.y + 222, 44, 50, { red: 34, green: 25, blue: 46, alpha: 255 });
  ell('mC_head', V.x + 180, V.y + 74, 152, 168, { red: 48, green: 36, blue: 63, alpha: 255 });
  ell('mC_hair', V.x + 174, V.y + 62, 164, 118, { red: 30, green: 22, blue: 41, alpha: 255 });
  rect('mC_rim1', V.x + 326, V.y + 100, 3, 56, A(C.violet, 200));
  rect('mC_rim2', V.x + 332, V.y + 158, 3, 44, A(C.violet, 150));
  rect('mC_rim3', V.x + 372, V.y + 262, 3, 74, A(C.violet, 110));
  ell('mC_chin', V.x + 232, V.y + 206, 48, 40, { red: 44, green: 33, blue: 58, alpha: 255 });
  particles('mC_dust', V.x + 430, V.y + 90, 14, 140, C.violet, d, { size: 1.5, speed: 10 });
  // shot visibility windows (local seconds)
  const showA = [[0, 1], [6.72, 1], [6.86, 0]];
  const showB = [[0, 0], [6.8, 0], [6.9, 1], [9.18, 1], [9.32, 0]];
  const showC = [[0, 0], [9.18, 0], [9.32, 1], [13, 1]];
  const setShot = (ids, frames) => ids.forEach((id) => anim(id, 'opacity', frames, 'linear'));
  setShot(['mA_sky', 'mA_sun', 'mA_sunC', 'mA_ridge', 'mA_ridgeL', 'mA_ground', 'mA_head', 'mA_body', 'mA_rim',
    ...Array.from({ length: 3 }, (_, i) => `mA_b${i}`), ...Array.from({ length: 3 }, (_, i) => `mA_b${i}b`)], showA);
  setShot(['mB', 'mB_sky', 'mB_glow', 'mB_ground', 'mB_groundL',
    ...Array.from({ length: 9 }, (_, i) => `mB_t${i}`), ...Array.from({ length: 9 }, (_, i) => `mB_tc${i}`),
    ...Array.from({ length: 9 }, (_, i) => Array.from({ length: 3 }, (_, c) => Array.from({ length: 6 }, (_, k) => `mB_w${i}_${c}_${k}`))).flat(2),
    ...Array.from({ length: 4 }, (_, i) => `mB_trail${i}`)], showB);
  setShot(['mC', 'mC_sky', 'mC_halo', 'mC_halo2', 'mC_shoulder', 'mC_neck', 'mC_head', 'mC_hair', 'mC_rim1', 'mC_rim2', 'mC_rim3', 'mC_chin',
    ...Array.from({ length: 14 }, (_, i) => `mC_dust${i}`)], showC);
  anim('mA_sun', 'scale', [[0, 1], [6.7, 1.1]], 'ease_in_out');
  anim('mA_sunC', 'scale', [[0, 1], [6.7, 1.06]], 'ease_in_out');
  anim('mA_head', 'translation_x', [[0, 0], [6.7, -16]], 'ease_in_out');
  anim('mA_body', 'translation_x', [[0, 0], [6.7, -16]], 'ease_in_out');
  anim('mC_head', 'scale', [[9.3, 1], [12.8, 1.07]], 'ease_in_out');
  anim('mC_shoulder', 'scale', [[9.3, 1], [12.8, 1.04]], 'ease_in_out');
  rect('cutFlash', V.x, V.y, V.w, V.h, A(C.ink, 0));
  anim('cutFlash', 'opacity', [[6.74, 0], [6.78, 0.55], [6.94, 0]], 'ease_out');
  anim('cutFlash', 'opacity', [[9.2, 0], [9.24, 0.45], [9.4, 0]], 'ease_out');
  // monitor furniture
  txt('monSlug', 'PROGRAM', V.x, V.y + V.h + 22, 11, A(C.dim, 210));
  txt('monTc', '00:00:44:12', V.x + V.w - textW('00:00:44:12', 11), V.y + V.h + 22, 11, A(C.mute, 200));

  // --------------------------------------------------------------- the assembly
  const lanes = [
    ['V', 472, 54, C.amber, 0, 'layer 01'],
    ['VOICE', 548, 44, C.cyan, 1, 'layer 02'],
    ['MUSIC', 610, 36, C.blue, 2, 'layer 03'],
    ['CAPTIONS', 666, 34, C.ink, 3, 'layer 04'],
  ];
  const L = { x: 250, w: 1560 };
  lanes.forEach(([name, y, h, color, i], idx) => {
    const t0 = 0.12 + idx * 0.14;
    const dy = idx % 2 === 0 ? -190 : 190;
    rect(`ln${i}`, L.x + 64, y, L.w - 64, h, A(C.panel2, 255));
    rect(`lnTag${i}`, L.x, y, 64, h, A(C.bg2, 255));
    txt(`lnTagT${i}`, name, L.x + 10, y + h / 2 + 2, 11, A(color, 230));
    anim(`ln${i}`, 'translation_y', [[t0, dy], [t0 + 0.6, 0]], 'ease_out');
    anim(`lnTag${i}`, 'translation_y', [[t0, dy], [t0 + 0.6, 0]], 'ease_out');
    anim(`lnTagT${i}`, 'translation_y', [[t0, dy], [t0 + 0.6, 0]], 'ease_out');
    anim(`ln${i}`, 'opacity', [[5.4, 1], [5.8, 0]], 'ease_in');
    anim(`lnTag${i}`, 'opacity', [[5.4, 1], [5.8, 0]], 'ease_in');
    anim(`lnTagT${i}`, 'opacity', [[5.4, 1], [5.8, 0]], 'ease_in');
    [0, 1, 2].forEach(() => {});
    anim(`ln${i}`, 'opacity', [[11.45, 0], [11.7, 1]], 'ease_out');
    anim(`lnTag${i}`, 'opacity', [[11.45, 0], [11.7, 1]], 'ease_out');
    anim(`lnTagT${i}`, 'opacity', [[11.45, 0], [11.7, 1]], 'ease_out');
    anim(`ln${i}`, 'translation_y', [[11.45, 0], [11.7, 0]], 'ease_out');
  });
  // clips on the lanes
  const clips = [
    [0, 40, 300, C.amber], [0, 350, 220, C.teal], [0, 580, 420, C.blue], [0, 1010, 300, C.pink], [0, 1320, 170, C.green],
    [1, 40, 520, C.cyan], [1, 570, 460, C.teal], [1, 1040, 440, C.cyan],
    [2, 40, 1450, C.blue],
    [3, 180, 300, C.ink], [3, 490, 260, C.ink], [3, 760, 340, C.ink], [3, 1110, 300, C.ink],
  ];
  clips.forEach(([lane, x, w, color], i) => {
    const y = lanes[lane][1] + 4;
    const h = lanes[lane][2] - 8;
    rect(`c${i}`, L.x + 64 + x, y, w, h, A(color, lane === 3 ? 120 : 175));
    rect(`c${i}capY`, L.x + 64 + x, y, w, 2, A(color, 235));
    const t0 = 0.9 + (i % 6) * 0.08;
    anim(`c${i}`, 'opacity', [[t0, 0], [t0 + 0.25, 1]], 'ease_out');
    anim(`c${i}capY`, 'opacity', [[t0, 0], [t0 + 0.25, 1]], 'ease_out');
    anim(`c${i}`, 'translation_x', [[5.4, 0], [5.8, 0]], 'linear');
    anim(`c${i}capY`, 'translation_x', [[5.4, 0], [5.8, 0]], 'linear');
  });
  // gaps close as the edit is arranged
  const shifts = { c1: -30, c2: -70, c3: -110, c4: -150, c5: -20, c6: -60, c7: -100, c8: 0, c11: -40 };
  Object.entries(shifts).forEach(([id, dx]) => {
    anim(id, 'translation_x', [[2.4, 0], [3.15, dx]], 'ease_in_out');
    anim(`${id}capY`, 'translation_x', [[2.4, 0], [3.15, dx]], 'ease_in_out');
  });
  // the playhead sweeps the whole edit
  rect('play', L.x + 64, 460, 2, 244, A(C.red, 230));
  rect('playHead', L.x + 56, 452, 18, 7, A(C.red, 230));
  anim('play', 'translation_x', [[2.2, 0], [5.6, L.w - 80]], 'linear');
  anim('playHead', 'translation_x', [[2.2, 0], [5.6, L.w - 80]], 'linear');
  ['play', 'playHead'].forEach((id) => {
    anim(id, 'opacity', [[5.3, 1], [5.7, 0]], 'ease_in');
    anim(id, 'opacity', [[11.45, 0], [11.7, 1]], 'ease_out');
  });

  // timecode counter
  const tc = ['00:00:00.000', '00:00:08.000', '00:00:21.000', '00:00:34.000', '00:00:47.000', '00:00:59.000'];
  tc.forEach((value, i) => {
    const id = `tc${i}`;
    txt(id, value, 250, 190, 34, A(i === tc.length - 1 ? C.ink : C.mute, 230));
    const t0 = 0.9 + i * 0.8;
    const last = i === tc.length - 1;
    anim(id, 'opacity', last
      ? [[0, 0], [t0, 0], [t0 + 0.14, 1], [5.4, 1], [5.8, 0]]
      : [[0, 0], [t0, 0], [t0 + 0.14, 1], [t0 + 0.66, 1], [t0 + 0.72, 0]], 'linear');
  });
  txt('tcLabel', 'DURATION', 250, 158, 11, A(C.dim, 210));
  txt('tcOk', '59.000 s', 250 + 260, 190, 14, A(C.green, 230));
  anim('tcOk', 'opacity', [[4.6, 0], [4.9, 1], [5.4, 1], [5.8, 0]], 'ease_out');
  txt('asLabel', 'DỰNG TIMELINE · 4 LỚP', 250, 232, 12, A(C.dim, 210));
  ['tcLabel', 'asLabel'].forEach((id) => anim(id, 'opacity', [[5.4, 1], [5.8, 0]], 'ease_in'));

  // ------------------------------------------------------------------ end slate
  rect('doneBg', V.x, V.y, V.w, V.h, A(C.bg, 255));
  txt('doneTitle', 'SEQUENCE', V.x + 40, V.y + 132, 22, A(C.mute, 220));
  txt('doneTc', '00:00:59.000', V.x + 40, V.y + 186, 40, A(C.ink, 240));
  txt('doneNote', 'SẴN SÀNG XUẤT', V.x + 40, V.y + 232, 13, A(C.green, 225));
  ['doneBg', 'doneTitle', 'doneTc', 'doneNote'].forEach((id) => anim(id, 'opacity', [[11.35, 0], [11.65, 1]], 'ease_out'));
  ['mC', 'mC_sky', 'mC_halo', 'mC_halo2', 'mC_shoulder', 'mC_neck', 'mC_head', 'mC_hair', 'mC_rim1', 'mC_rim2', 'mC_rim3', 'mC_chin',
    ...Array.from({ length: 14 }, (_, i) => `mC_dust${i}`)].forEach((id) => anim(id, 'opacity', [[11.2, 1], [11.4, 0]]));

  return finish({
    name: opts.name ?? 'Assembly',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_y: 16 },
      { t: 1.4, zoom: 1.0 },
      { t: 2.6, zoom: 1.05, pan_x: 20 },
      { t: 4.6, zoom: 1.02 },
      { t: 5.4, zoom: 1.08 },
      { t: 6.4, zoom: 3.37, pan_x: -1688, pan_y: 894 },
      { t: 11.2, zoom: 3.42, pan_x: -1704, pan_y: 900 },
      { t: 11.9, zoom: 1.04, pan_x: 0, pan_y: 0 },
      { t: 13, zoom: 1.0 },
    ],
  });
}

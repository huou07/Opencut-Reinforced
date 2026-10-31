// S2 — 5–13 s. The editor itself: window, viewer, bin and the timeline where
// clips slide in, snap and get arranged.
import { begin, finish, rect, ell, txt, centre, A, C, anim, fade, particles, textW, captions, mark, since } from '../kit.mjs';

const W = 1920;
const WIN = { x: 160, y: 96, w: 1600, h: 824 };

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', -400, -500, 1500, 1200, A(C.blue, 10));
  ell('glowB', 1300, 480, 1200, 900, A(C.violet, 8));

  // ------------------------------------------------------------------ window
  const m0 = mark();
  rect('winShadow', WIN.x + 18, WIN.y + 22, WIN.w, WIN.h, { red: 0, green: 0, blue: 0, alpha: 160 });
  rect('win', WIN.x, WIN.y, WIN.w, WIN.h, C.bg2, { stroke: A(C.edge2, 220), stroke_width: 1 });
  rect('winBar', WIN.x, WIN.y, WIN.w, 44, C.panel2);
  rect('winLine', WIN.x, WIN.y + 44, WIN.w, 1, A(C.edge2, 200));
  for (let i = 0; i < 3; i += 1) ell(`winDot${i}`, WIN.x + 18 + i * 18, WIN.y + 18, 8, 8, A(C.dim, 200));
  txt('winName', 'OR', WIN.x + 86, WIN.y + 21, 17, A(C.ink, 235));
  txt('winFile', 'gioi-thieu-opencut.orc', WIN.x + 122, WIN.y + 22, 15, A(C.mute, 190));
  const ids = since(m0);
  ids.forEach((id) => anim(id, 'opacity', [[0, 0], [0.3, 1]], 'ease_out'));
  // window opens from the timeline line
  anim('win', 'height', [[0.0, 3], [0.5, WIN.h]], 'ease_out');
  anim('win', 'translation_y', [[0.0, WIN.h / 2 - 2], [0.5, 0]], 'ease_out');
  anim('winBar', 'width', [[0.0, 40], [0.5, WIN.w]], 'ease_out');
  anim('winBar', 'translation_x', [[0.0, (WIN.w - 40) / 2], [0.5, 0]], 'ease_out');
  anim('winLine', 'width', [[0.0, 40], [0.5, WIN.w]], 'ease_out');
  anim('winLine', 'translation_x', [[0.0, (WIN.w - 40) / 2], [0.5, 0]], 'ease_out');
  anim('winShadow', 'opacity', [[0, 0], [0.5, 1]], 'ease_out');

  // ------------------------------------------------------------------ viewer
  const V = { x: 492, y: 164, w: 748, h: 420 };
  rect('viewerBg', V.x - 8, V.y - 8, V.w + 16, V.h + 16, C.panel);
  rect('viewer', V.x, V.y, V.w, V.h, C.bg);
  rect('viewerSky', V.x, V.y, V.w, 260, { red: 16, green: 20, blue: 27, alpha: 255 });
  ell('viewSun', V.x + 470, V.y + 60, 130, 130, A(C.amber, 232));
  ell('viewSunC', V.x + 492, V.y + 82, 86, 86, A({ red: 251, green: 208, blue: 136, alpha: 255 }, 235));
  rect('viewWater', V.x, V.y + 260, V.w, 160, { red: 8, green: 10, blue: 13, alpha: 255 });
  rect('viewRidge1', V.x - 80, V.y + 214, 560, 160, { red: 9, green: 11, blue: 14, alpha: 255 }, { rotation_degrees: -4 });
  rect('viewRidge1Lit', V.x - 80, V.y + 214, 560, 2, A(C.amber, 90), { rotation_degrees: -4 });
  rect('viewRidge2', V.x + 420, V.y + 232, 420, 150, { red: 10, green: 12, blue: 16, alpha: 255 }, { rotation_degrees: 3 });
  rect('viewRidge2Lit', V.x + 420, V.y + 232, 420, 2, A(C.amber, 70), { rotation_degrees: 3 });
  rect('viewHorizon', V.x, V.y + 258, V.w, 2, A(C.amber, 100));
  for (let i = 0; i < 6; i += 1) rect(`viewRefl${i}`, V.x + 486 - i * 8, V.y + 272 + i * 22, 96 - i * 8, 3, A(C.amber, 55 - i * 8));
  ell('viewHead', V.x + 186, V.y + 196, 18, 18, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('viewBody', V.x + 180, V.y + 212, 30, 44, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('viewRim', V.x + 209, V.y + 212, 3, 42, A(C.amber, 170));
  for (let i = 0; i < 2; i += 1) {
    rect(`viewBird${i}`, V.x + 300 + i * 30, V.y + 92 + i * 16, 18, 2, A(C.ink, 90), { rotation_degrees: -16 });
    rect(`viewBird${i}b`, V.x + 306 + i * 30, V.y + 94 + i * 16, 18, 2, A(C.ink, 90), { rotation_degrees: 16 });
  }
  particles('viewDust', V.x + 500, V.y + 160, 18, 220, C.amber, d, { size: 1.6, speed: 16 });
  // second and third shots, on top, swapped by the playhead
  rect('shot2Sky', V.x, V.y, V.w, V.h, { red: 10, green: 18, blue: 24, alpha: 255 });
  for (let i = 0; i < 7; i += 1) {
    const h = 60 + ((i * 47) % 190);
    rect(`shot2B${i}`, V.x + 40 + i * 104, V.y + 300 - h, 72, h, { red: 14, green: 26, blue: 34, alpha: 255 });
    for (let k = 0; k < 4; k += 1) rect(`shot2W${i}_${k}`, V.x + 50 + i * 104, V.y + 250 - h + k * 24, 10, 4, A(C.cyan, 150));
  }
  rect('shot3', V.x, V.y, V.w, V.h, { red: 18, green: 14, blue: 26, alpha: 255 });
  ell('shot3Head', V.x + 300, V.y + 120, 190, 190, { red: 26, green: 20, blue: 36, alpha: 255 });
  rect('shot3Body', V.x + 230, V.y + 300, 340, 160, { red: 22, green: 17, blue: 31, alpha: 255 });
  ell('shot3Rim', V.x + 300, V.y + 120, 190, 190, A(C.violet, 0), { stroke: A(C.violet, 120), stroke_width: 3 });
  anim('shot2Sky', 'opacity', [[0, 0], [4.32, 0], [4.44, 1], [5.24, 1], [5.34, 0]], 'linear');
  ['shot2Sky', ...Array.from({ length: 7 }, (_, i) => `shot2B${i}`), ...Array.from({ length: 7 }, (_, i) => Array.from({ length: 4 }, (_, k) => `shot2W${i}_${k}`)).flat()].forEach((id) => {
    anim(id, 'opacity', [[4.32, 0], [4.44, 1], [5.24, 1], [5.34, 0]], 'linear');
  });
  anim('shot3', 'opacity', [[0, 0], [5.28, 0], [5.4, 1], [6.3, 1], [6.4, 0]], 'linear');
  ['shot3Head', 'shot3Body', 'shot3Rim'].forEach((id) => {
    anim(id, 'opacity', [[5.28, 0], [5.4, 1], [6.3, 1], [6.4, 0]], 'linear');
  });
  anim('shot3Head', 'translation_x', [[5.4, 20], [6.4, -10]], 'ease_in_out');
  anim('shot3Body', 'translation_x', [[5.4, 20], [6.4, -10]], 'ease_in_out');
  anim('shot3Rim', 'translation_x', [[5.4, 20], [6.4, -10]], 'ease_in_out');
  anim('viewSun', 'scale', [[0, 1], [4.3, 1.06]], 'ease_in_out');
  anim('viewSunC', 'scale', [[0, 1], [4.3, 1.04]], 'ease_in_out');
  // viewer furniture
  rect('viewerRule', V.x, V.y + V.h + 20, V.w, 1, A(C.edge2, 160));
  txt('viewTc', '00:00:07:24', V.x, V.y + V.h + 34, 14, A(C.mute, 210));
  txt('viewRes', '1920 × 1080 · 30p', V.x + V.w - textW('1920 × 1080 · 30p', 14), V.y + V.h + 34, 14, A(C.mute, 170));
  anim('viewSun', 'translation_x', [[0, 0], [4, -30], [6.4, 0]], 'ease_in_out');

  // --------------------------------------------------------------- media bin
  const B = { x: 180, y: 170, w: 280, h: 416 };
  rect('binBg', B.x, B.y, B.w, B.h, C.panel);
  txt('binTitle', 'MEDIA', B.x + 18, B.y + 24, 14, A(C.dim, 230));
  rect('binRule', B.x + 18, B.y + 38, B.w - 36, 1, A(C.edge, 200));
  const thumbs = [
    ['sun', C.amber], ['city', C.cyan], ['face', C.violet], ['wave', C.blue],
  ];
  thumbs.forEach(([name, accent], i) => {
    const y = B.y + 56 + i * 80;
    rect(`binT${i}`, B.x + 16, y, 84, 56, A(accent, 26), { stroke: A(accent, 120), stroke_width: 1 });
    rect(`binT${i}bar`, B.x + 16, y + 52, 84, 3, A(accent, 200));
    txt(`binN${i}`, name.toUpperCase(), B.x + 112, y + 18, 14, A(C.ink, 220));
    txt(`binD${i}`, ['00:04', '00:03', '00:02', '00:05'][i], B.x + 112, y + 40, 12, A(C.dim, 200));
  });
  rect('binImport', B.x + 16, B.y + B.h - 44, B.w - 32, 28, A(C.panel3, 255), { stroke: A(C.edge, 200), stroke_width: 1 });
  txt('binImportT', '+ Import', B.x + 30, B.y + B.h - 36, 13, A(C.mute, 220));

  // --------------------------------------------------------------- inspector
  const I = { x: 1264, y: 170, w: 476, h: 552 };
  rect('insBg', I.x, I.y, I.w, I.h, C.panel);
  txt('insTitle', 'INSPECTOR', I.x + 20, I.y + 24, 14, A(C.dim, 230));
  rect('insRule', I.x + 20, I.y + 38, I.w - 40, 1, A(C.edge, 200));
  for (let i = 0; i < 4; i += 1) {
    const y = I.y + 62 + i * 62;
    txt(`insL${i}`, ['POSITION', 'SCALE', 'OPACITY', 'VOLUME'][i], I.x + 20, y, 12, A(C.dim, 220));
    rect(`insTrack${i}`, I.x + 20, y + 22, I.w - 60, 3, A(C.edge2, 200));
    rect(`insFill${i}`, I.x + 20, y + 21, 110 + i * 52, 5, A(i === 3 ? C.cyan : C.ink, 200));
    ell(`insKnob${i}`, I.x + 20 + 110 + i * 52, y + 16, 13, 13, C.ink);
  }
  txt('insColor', 'COLOUR', I.x + 20, I.y + 330, 12, A(C.dim, 220));
  [C.amber, C.cyan, C.violet, C.blue, C.green].forEach((color, i) => {
    rect(`sw${i}`, I.x + 20 + i * 40, I.y + 346, 28, 28, A(color, 230));
  });
  txt('insEffect', 'MOTION SCENE', I.x + 20, I.y + 410, 12, A(C.dim, 220));
  rect('insEffectBox', I.x + 20, I.y + 426, I.w - 40, 34, A(C.panel2, 255), { stroke: A(C.edge, 200), stroke_width: 1 });
  txt('insEffectT', 'fade · 0.4 s', I.x + 34, I.y + 438, 13, A(C.mute, 220));

  // ---------------------------------------------------------------- timeline
  const T = { x: 180, y: 620, w: 1560, h: 278 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel);
  rect('tlHead', T.x, T.y, T.w, 30, C.panel2);
  txt('tlTitle', 'TIMELINE', T.x + 18, T.y + 16, 13, A(C.dim, 230));
  txt('tlTc', '00:00:08:00', T.x + T.w - 96, T.y + 16, 13, A(C.mute, 210));
  for (let i = 0; i < 34; i += 1) {
    rect(`tick${i}`, T.x + 20 + i * 44, T.y + 30, 1, i % 4 === 0 ? 12 : 6, A(C.edge2, 160));
    if (i % 4 === 0) txt(`tickT${i}`, `00:${String(i / 4).padStart(2, '0')}`, T.x + 20 + i * 44 + 4, T.y + 56, 10, A(C.dim, 170));
  }
  const lanes = [
    ['V2', T.y + 74, 28, C.violet, 'overlay'],
    ['V1', T.y + 106, 40, C.amber, 'video'],
    ['A1', T.y + 152, 36, C.cyan, 'voice'],
    ['A2', T.y + 194, 36, C.blue, 'music'],
  ];
  lanes.forEach(([name, y, h, color], i) => {
    rect(`lane${i}`, T.x + 20, y, T.w - 40, h, A(C.panel2, 255));
    rect(`laneTag${i}`, T.x + 20, y, 44, h, A(C.bg2, 255));
    txt(`laneTagT${i}`, name, T.x + 30, y + h / 2 + 2, 11, A(color, 230));
    rect(`laneHead${i}`, T.x + 20, y, T.w - 40, 1, A(C.edge, 150));
  });
  // clips already on the timeline
  const clips = [
    [0, T.x + 64, 300, C.amber],
    [0, T.x + 372, 190, C.teal],
    [0, T.x + 570, 380, C.blue],
    [1, T.x + 64, 404, C.amber],
    [1, T.x + 476, 300, C.teal],
    [1, T.x + 784, 460, C.blue],
    [2, T.x + 96, 420, C.cyan],
    [2, T.x + 524, 470, C.cyan],
    [3, T.x + 64, 1140, C.blue],
  ];
  clips.forEach(([lane, x, w, color], i) => {
    const y = lanes[lane][1] + 3;
    const h = lanes[lane][2] - 6;
    rect(`clip${i}`, x, y, w, h, A(color, 170));
    rect(`clip${i}cap`, x, y, w, 2, A(color, 240));
    if (lane === 1) for (let k = 0; k < Math.floor(w / 26); k += 1) rect(`clip${i}w${k}`, x + 8 + k * 26, y + h / 2 - 8, 3, 16, A(C.bg, 90));
  });
  // playhead
  rect('play', T.x + 20, T.y + 30, 2, T.h - 30, A(C.red, 220));
  rect('playHead', T.x + 12, T.y + 30, 18, 7, A(C.red, 230));
  anim('play', 'translation_x', [[0.4, 0], [7.2, 980]], 'linear');
  anim('playHead', 'translation_x', [[0.4, 0], [7.2, 980]], 'linear');

  // clips that drop from the bin into the timeline
  const drops = [
    [10, T.x + 900, C.pink, 260, 1],
    [11, T.x + 1170, C.green, 220, 1],
    [12, T.x + 1220, C.amber, 180, 2],
  ];
  drops.forEach(([i, x, color, w, lane], k) => {
    const y = lanes[lane][1] + 3;
    const h = lanes[lane][2] - 6;
    const t0 = 3.1 + k * 0.24;
    rect(`drop${k}`, x, y, w, h, A(color, 150));
    rect(`drop${k}cap`, x, y, w, 2, A(color, 240));
    anim(`drop${k}`, 'translation_y', [[t0, -170], [t0 + 0.42, 0]], 'ease_out');
    anim(`drop${k}cap`, 'translation_y', [[t0, -170], [t0 + 0.42, 0]], 'ease_out');
    anim(`drop${k}`, 'opacity', [[t0, 0], [t0 + 0.1, 1], [t0 + 0.6, 1], [t0 + 0.7, 1]], 'ease_out');
    anim(`drop${k}cap`, 'opacity', [[t0, 0], [t0 + 0.1, 1]], 'ease_out');
  });
  // selection marquee on the last clip
  rect('sel', T.x + 1170, lanes[1][1] + 1, 220, lanes[1][2] - 2, A(C.ink, 0), { stroke: A(C.ink, 200), stroke_width: 2 });
  anim('sel', 'opacity', [[4.4, 0], [4.6, 1], [6.4, 1], [6.6, 0]], 'ease_in_out');

  // subtle life: empty-band shimmer and guide lines
  for (let i = 0; i < 4; i += 1) {
    rect(`scan${i}`, WIN.x, WIN.y + 120 + i * 180, WIN.w, 1, A(C.ink, 8));
  }
  particles('dust', 1600, 300, 16, 300, C.ink, d, { size: 1.6, speed: 12 });

  return finish({
    name: opts.name ?? 'Editor',
    duration: d,
    camera: [
      { t: 0, zoom: 1.06, pan_y: 10 },
      { t: 0.9, zoom: 1.0 },
      { t: 2.6, zoom: 1.03, pan_y: 20 },
      { t: 3.9, zoom: 1.1, pan_x: 30, pan_y: 70 },
      { t: 5.0, zoom: 1.16, pan_x: 40, pan_y: 90 },
      { t: 6.4, zoom: 1.2, pan_x: 78, pan_y: 168 },
      { t: 7.6, zoom: 1.22, pan_x: 84, pan_y: 172 },
      { t: 8, zoom: 1.06, pan_x: 0, pan_y: 0 },
    ],
  });
}

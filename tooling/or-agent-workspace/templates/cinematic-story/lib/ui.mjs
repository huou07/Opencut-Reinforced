// Shared UI and world primitives for this film. Every id is prefixed so a scene
// can use the same helper twice without a duplicate-node failure.
import { W, H, CAPTION_Y, SAFE_BOTTOM, C, A, rect, ell, txt, centre, anim, textW, particles, waveform } from './kit.mjs';

export const UI = { x: 120, y: 62, w: 1680, h: 808 };

export function windowFrame(prefix, x = UI.x, y = UI.y, w = UI.w, h = UI.h, title = 'OpenCut Reinforced', file = 'gioi-thieu-or.orproj') {
  rect(`${prefix}Shadow`, x + 16, y + 20, w, h, { red: 0, green: 0, blue: 0, alpha: 165 });
  rect(`${prefix}Bg`, x, y, w, h, C.bg2, { stroke: A(C.edge2, 225), stroke_width: 1 });
  rect(`${prefix}Bar`, x, y, w, 46, C.panel2);
  rect(`${prefix}BarLine`, x, y + 46, w, 1, A(C.edge2, 200));
  for (let i = 0; i < 3; i += 1) ell(`${prefix}Dot${i}`, x + 18 + i * 18, y + 19, 8, 8, A(C.dim, 210));
  txt(`${prefix}Title`, title, x + 86, y + 22, 17, A(C.ink, 238));
  txt(`${prefix}File`, file, x + 86 + textW(title, 17) + 18, y + 23, 13, A(C.mute, 185));
  txt(`${prefix}Mode`, 'EDIT', x + w - 64, y + 22, 12, A(C.green, 225));
  ell(`${prefix}ModeDot`, x + w - 78, y + 24, 7, 7, A(C.green, 235));
  return { x, y, w, h, bodyY: y + 47 };
}

export function viewer(prefix, x, y, w, h, opts = {}) {
  const accent = opts.accent ?? C.amber;
  rect(`${prefix}Case`, x - 8, y - 8, w + 16, h + 16, C.panel);
  rect(`${prefix}Bg`, x, y, w, h, { red: 10, green: 12, blue: 16, alpha: 255 });
  // sky, sun, ridges, water, figure — all inside the frame
  rect(`${prefix}Sky`, x, y, w, h * 0.62, { red: 15, green: 19, blue: 26, alpha: 255 });
  ell(`${prefix}Halo`, x + w * 0.48, y + h * 0.04, w * 0.32, h * 0.42, A(accent, 14));
  ell(`${prefix}Sun`, x + w * 0.64, y + h * 0.12, Math.min(w, h) * 0.22, Math.min(w, h) * 0.22, A(accent, 238));
  ell(`${prefix}SunCore`, x + w * 0.665, y + h * 0.145, Math.min(w, h) * 0.155, Math.min(w, h) * 0.155, A({ red: 251, green: 208, blue: 134, alpha: 255 }, 238));
  ell(`${prefix}RidgeA`, x - w * 0.08, y + h * 0.48, w * 0.72, h * 0.34, { red: 9, green: 11, blue: 15, alpha: 255 });
  ell(`${prefix}RidgeB`, x + w * 0.42, y + h * 0.52, w * 0.62, h * 0.30, { red: 12, green: 15, blue: 20, alpha: 255 });
  rect(`${prefix}Water`, x, y + h * 0.62, w, h * 0.38, { red: 8, green: 10, blue: 13, alpha: 255 });
  rect(`${prefix}Horizon`, x, y + h * 0.615, w, 2, A(accent, 95));
  for (let i = 0; i < 5; i += 1) rect(`${prefix}Refl${i}`, x + w * 0.68 - i * w * 0.012, y + h * 0.645 + i * h * 0.035, w * 0.09 - i * 4, 3, A(accent, 55 - i * 8));
  ell(`${prefix}Head`, x + w * 0.245, y + h * 0.48, 15, 15, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect(`${prefix}Body`, x + w * 0.238, y + h * 0.495, 26, 42, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect(`${prefix}Rim`, x + w * 0.263, y + h * 0.498, 3, 38, A(accent, 180));
  for (let i = 0; i < 2; i += 1) {
    rect(`${prefix}Bird${i}`, x + w * 0.53 + i * 26, y + h * 0.22 + i * 13, 18, 2, A(C.ink, 100), { rotation_degrees: -15 });
    rect(`${prefix}Bird${i}b`, x + w * 0.537 + i * 26, y + h * 0.224 + i * 13, 18, 2, A(C.ink, 100), { rotation_degrees: 15 });
  }
  rect(`${prefix}Rule`, x, y + h + 18, w, 1, A(C.edge2, 150));
  txt(`${prefix}Tc`, opts.tc ?? '00:00:12:08', x, y + h + 30, 12, A(C.mute, 205));
  const label = opts.res ?? '1920 × 1080 · 30p';
  txt(`${prefix}Res`, label, x + w - textW(label, 12), y + h + 30, 12, A(C.mute, 165));
  return { x, y, w, h };
}

export function mediaBin(prefix, x, y, w, h, opts = {}) {
  rect(`${prefix}Bg`, x, y, w, h, C.panel);
  txt(`${prefix}Title`, 'MEDIA', x + 16, y + 22, 12, A(C.dim, 235));
  rect(`${prefix}Rule`, x + 16, y + 34, w - 32, 1, A(C.edge, 200));
  const items = opts.items ?? [
    ['shot-01', C.amber, '00:08'],
    ['shot-02', C.cyan, '00:06'],
    ['voice', C.violet, '00:12'],
    ['music', C.blue, '02:40'],
    ['scene-01', C.teal, '00:06'],
  ];
  items.forEach(([name, accent, dur], i) => {
    const iy = y + 52 + i * 68;
    rect(`${prefix}Thumb${i}`, x + 14, iy, 70, 46, A(accent, 28), { stroke: A(accent, 120), stroke_width: 1 });
    rect(`${prefix}ThumbBar${i}`, x + 14, iy + 43, 70, 3, A(accent, 210));
    txt(`${prefix}Name${i}`, name, x + 96, iy + 14, 13, A(C.ink, 225));
    txt(`${prefix}Dur${i}`, dur, x + 96, iy + 32, 11, A(C.dim, 200));
  });
  rect(`${prefix}Import`, x + 14, y + h - 40, w - 28, 26, A(C.panel3, 255), { stroke: A(C.edge, 200), stroke_width: 1 });
  txt(`${prefix}ImportT`, '+ Import media', x + 26, y + h - 33, 12, A(C.mute, 220));
  return { x, y, w, h };
}

export function inspector(prefix, x, y, w, h, opts = {}) {
  rect(`${prefix}Bg`, x, y, w, h, C.panel);
  txt(`${prefix}Title`, opts.title ?? 'INSPECTOR', x + 18, y + 22, 12, A(C.dim, 235));
  rect(`${prefix}Rule`, x + 18, y + 34, w - 36, 1, A(C.edge, 200));
  const rows = opts.rows ?? ['POSITION', 'SCALE', 'OPACITY', 'VOLUME'];
  rows.forEach((name, i) => {
    const ry = y + 58 + i * 52;
    txt(`${prefix}L${i}`, name, x + 18, ry, 11, A(C.dim, 220));
    rect(`${prefix}Track${i}`, x + 18, ry + 18, w - 54, 3, A(C.edge2, 190));
    rect(`${prefix}Fill${i}`, x + 18, ry + 17, 70 + i * 34, 5, A(i === 3 ? C.cyan : C.ink, 200));
    ell(`${prefix}Knob${i}`, x + 18 + 70 + i * 34, ry + 13, 11, 11, C.ink);
  });
  txt(`${prefix}Colour`, 'COLOUR', x + 18, y + 290, 11, A(C.dim, 220));
  [C.amber, C.cyan, C.violet, C.blue, C.green].forEach((color, i) => {
    rect(`${prefix}Sw${i}`, x + 18 + i * 34, y + 304, 24, 24, A(color, 230));
  });
  if (opts.extra) opts.extra(prefix, x, y, w, h);
  return { x, y, w, h };
}

export function timeline(prefix, x, y, w, h, opts = {}) {
  const lanes = opts.lanes ?? [
    ['C1', C.blue, 'caption'],
    ['V2', C.violet, 'overlay'],
    ['V1', C.amber, 'video'],
    ['A1', C.cyan, 'voice'],
    ['A2', C.blue, 'music'],
  ];
  rect(`${prefix}Bg`, x, y, w, h, C.panel);
  rect(`${prefix}Head`, x, y, w, 30, C.panel2);
  txt(`${prefix}Title`, opts.title ?? 'TIMELINE', x + 16, y + 16, 11, A(C.dim, 235));
  const tc = opts.tc ?? '00:00:12:08';
  txt(`${prefix}Tc`, tc, x + w - textW(tc, 11) - 16, y + 16, 11, A(C.mute, 205));
  for (let i = 0; i < Math.min(26, Math.floor(w / 54)); i += 1) {
    rect(`${prefix}Tick${i}`, x + 18 + i * 54, y + 30, 1, i % 4 === 0 ? 10 : 5, A(C.edge2, 150));
  }
  const laneH = (h - 44) / lanes.length;
  lanes.forEach(([name, color], i) => {
    const ly = y + 34 + i * laneH;
    rect(`${prefix}Lane${i}`, x + 12, ly, w - 24, laneH - 3, A(C.panel2, 255));
    rect(`${prefix}LaneTag${i}`, x + 12, ly, 38, laneH - 3, A(C.bg2, 255));
    txt(`${prefix}LaneT${i}`, name, x + 21, ly + laneH / 2 - 2, 10, A(color, 235));
  });
  (opts.clips ?? []).forEach((clip, i) => {
    const [lane, cx, cw, color, mark] = clip;
    const ly = y + 34 + lane * laneH;
    rect(`${prefix}Clip${i}`, cx, ly + 2, cw, laneH - 7, A(color, 185));
    rect(`${prefix}ClipCap${i}`, cx, ly + 2, cw, 2, A(color, 245));
    if (lane >= 1 && lane <= 2) {
      for (let k = 0; k < Math.floor(cw / 24); k += 1) rect(`${prefix}ClipW${i}_${k}`, cx + 7 + k * 24, ly + laneH / 2 - 6, 2, 12, A(C.bg, 100));
    }
    if (mark) txt(`${prefix}ClipT${i}`, mark, cx + 8, ly + laneH / 2 + 3, 9, A(C.bg, 175));
  });
  return { x, y, w, h, laneH, lanes };
}

export function playhead(prefix, x, y, h) {
  rect(`${prefix}Play`, x, y, 2, h, A(C.red, 225));
  rect(`${prefix}PlayHead`, x - 8, y, 18, 6, A(C.red, 235));
}

export function waveformBlock(prefix, x, y, w, bars, color, dur, opts = {}) {
  waveform(prefix, x, y, w, bars, color, dur, opts);
}

export function codePane(prefix, x, y, w, h, lines, opts = {}) {
  rect(`${prefix}Shadow`, x + 10, y + 12, w, h, { red: 0, green: 0, blue: 0, alpha: 145 });
  rect(`${prefix}Bg`, x, y, w, h, C.panel, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect(`${prefix}Bar`, x, y, w, 38, C.panel2);
  ell(`${prefix}Dot`, x + 16, y + 15, 8, 8, A(opts.accent ?? C.cyan, 235));
  txt(`${prefix}Title`, opts.title ?? 'MOTION SCENE', x + 32, y + 18, 12, A(C.ink, 232));
  txt(`${prefix}File`, opts.file ?? 'scene.ormotion.json', x + w - 16 - textW(opts.file ?? 'scene.ormotion.json', 11), y + 19, 11, A(C.dim, 210));
  rect(`${prefix}Rule`, x, y + 38, w, 1, A(C.edge2, 180));
  lines.forEach(([text, color, start], i) => {
    const ly = y + 64 + i * 27;
    txt(`${prefix}L${i}`, text, x + 48, ly, 16, A(color ?? C.mute, 228));
    rect(`${prefix}N${i}`, x + 16, ly - 6, 20, 1, A(C.dim, 140));
    if (start != null) {
      rect(`${prefix}Hl${i}`, x + 28, ly - 11, w - 48, 20, A(opts.accent ?? C.cyan, 0));
      anim(`${prefix}Hl${i}`, 'opacity', [[start, 0], [start + 0.2, 0.09], [start + 0.9, 0.05], [start + 1.1, 0]], 'linear');
    }
  });
  return { x, y, w, h };
}

export function tagChip(prefix, value, x, y, color, size = 14) {
  const w = textW(value, size) + 26;
  rect(`${prefix}Bg`, x, y - size * 1.05, w, size * 2.0, A(color, 24), { stroke: A(color, 145), stroke_width: 1 });
  txt(`${prefix}T`, value, x + 13, y, size, A(color, 238));
  return { w, x, y };
}

export function miniShot(prefix, x, y, w, h, accent = C.amber) {
  rect(`${prefix}Sky`, x, y, w, h * 0.66, { red: 15, green: 19, blue: 26, alpha: 255 });
  ell(`${prefix}Sun`, x + w * 0.62, y + h * 0.1, h * 0.24, h * 0.24, A(accent, 236));
  ell(`${prefix}RidgeA`, x, y + h * 0.48, w * 0.64, h * 0.4, { red: 9, green: 11, blue: 15, alpha: 255 });
  ell(`${prefix}RidgeB`, x + w * 0.38, y + h * 0.54, w * 0.62, h * 0.34, { red: 12, green: 15, blue: 20, alpha: 255 });
  rect(`${prefix}Ground`, x, y + h * 0.64, w, h * 0.36, { red: 7, green: 9, blue: 12, alpha: 255 });
  ell(`${prefix}Head`, x + w * 0.24, y + h * 0.5, 10, 10, { red: 4, green: 5, blue: 7, alpha: 255 });
  rect(`${prefix}Body`, x + w * 0.235, y + h * 0.51, 18, 28, { red: 4, green: 5, blue: 7, alpha: 255 });
  rect(`${prefix}Rim`, x + w * 0.253, y + h * 0.515, 2, 24, A(accent, 180));
  return { x, y, w, h };
}

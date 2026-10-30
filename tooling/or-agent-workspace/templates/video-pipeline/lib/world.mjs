// Shared drawing kit for the OR video template: a tiny scene builder, the
// graphic "world" backdrop, reusable pieces and camera presets.
//
// Every composer draws through the module-level builder; call begin() before a
// scene and finish(duration) after it to get validated-ready node/animation
// arrays.


export const CANVAS = { width: 1920, height: 1080 };
const W = 1920, H = 1080;
let BAND = 952;

export const C = {
  bg: { red: 9, green: 10, blue: 12, alpha: 255 },
  bg2: { red: 14, green: 16, blue: 19, alpha: 255 },
  band: { red: 6, green: 7, blue: 8, alpha: 255 },
  card: { red: 22, green: 24, blue: 28, alpha: 255 },
  card2: { red: 17, green: 19, blue: 22, alpha: 255 },
  edge: { red: 44, green: 48, blue: 55, alpha: 255 },
  ink: { red: 234, green: 236, blue: 240, alpha: 255 },
  mute: { red: 150, green: 158, blue: 168, alpha: 255 },
  dim: { red: 96, green: 103, blue: 112, alpha: 255 },
  gold: { red: 242, green: 181, blue: 77, alpha: 255 },
  teal: { red: 84, green: 202, blue: 184, alpha: 255 },
  blue: { red: 92, green: 143, blue: 212, alpha: 255 },
  purple: { red: 156, green: 136, blue: 205, alpha: 255 },
  red: { red: 219, green: 104, blue: 88, alpha: 255 },
  green: { red: 128, green: 178, blue: 104, alpha: 255 },
  brown: { red: 172, green: 118, blue: 80, alpha: 255 },
};
export const A = (c, a) => ({ ...c, alpha: a });
export const at = (s) => ({ numerator: Math.round(Math.max(0, s) * 1000), denominator: 1000 });


let nodes = [], anims = [], uid = 0, currentTag = '01';
export const nid = (p) => `${p}${uid++}`;
export const r = (id, x, y, w, h, fill, extra = {}) => { nodes.push({ node_id: id, kind: 'rectangle', position: { x: Math.round(x), y: Math.round(y) }, size: { width: Math.max(1, Math.round(w)), height: Math.max(1, Math.round(h)) }, fill, ...extra }); return id; };
export const e = (id, x, y, w, h, fill, extra = {}) => { nodes.push({ node_id: id, kind: 'ellipse', position: { x: Math.round(x), y: Math.round(y) }, size: { width: Math.max(1, Math.round(w)), height: Math.max(1, Math.round(h)) }, fill, ...extra }); return id; };
export const t = (id, value, x, y, size, fill) => { nodes.push({ node_id: id, kind: 'text', text: value, position: { x: Math.round(((x - 99) / 0.95) * 100) / 100, y: Math.round(y) }, font_size: size, fill }); return id; };
export const an = (id, prop, keys, easing = 'ease_in_out') => { anims.push({ node_id: id, property: prop, keyframes: keys.map(([time, value]) => ({ at: at(time), value })), easing }); };
export const fadeIn = (id, t0, d = 0.4) => an(id, 'opacity', [[Math.max(0, t0), 0], [Math.max(0, t0) + d, 1]], 'ease_out');
export const fadeOut = (id, t0, d = 0.4) => an(id, 'opacity', [[t0, 1], [t0 + d, 0]], 'ease_in');
export const pop = (id, t0, dist = 28, d = 0.45) => { an(id, 'translation_y', [[Math.max(0, t0), dist], [Math.max(0, t0) + d, 0]], 'ease_out'); fadeIn(id, t0, d); };
export const textW = (value, size) => value.length * size * 0.53;
export const centred = (id, value, cx, y, size, fill) => t(id, value, cx - textW(value, size) / 2, y, size, fill);
export const moveAll = (ids, prop, keys, easing = 'ease_in_out') => ids.forEach((id) => an(id, prop, keys, easing));
export const pulse = (id, t0, count, period = 0.9) => {
  const keys = [];
  for (let i = 0; i < count; i += 1) keys.push([t0 + i * period, 0.35], [t0 + i * period + period / 2, 1]);
  keys.push([t0 + count * period, 0.35]);
  an(id, 'opacity', keys);
};

// --- world -----------------------------------------------------------------
export function world(kind, dur, tag = currentTag) {
  r('bg', 0, 0, W, H, C.bg);
  r('band', 0, BAND, W, H - BAND, C.band);
  r('bandRule', 0, BAND, W, 1, A(C.edge, 200));
  // cinematic frame furniture: corner brackets, a hairline, a chapter mark
  const corners = [[54, 54, 1, 1], [1866, 54, -1, 1], [54, 906, 1, -1], [1866, 906, -1, -1]];
  corners.forEach(([x, y, dx, dy], i) => {
    r(`cn${i}a`, dx > 0 ? x : x - 40, y, 40, 2, A(C.edge, 170));
    r(`cn${i}b`, x, dy > 0 ? y : y - 40, 2, 40, A(C.edge, 170));
  });
  t('mark', tag, 56, 918, 12, A(C.dim, 210));
  e('glowA', -400, -380, 1300, 1100, A(C.blue, 16));
  an('glowA', 'translation_x', [[0, 0], [dur, 90]]);
  e('glowB', 1200, 420, 1100, 900, A(C.purple, 14));
  an('glowB', 'translation_x', [[0, 0], [dur, -70]]);
  if (kind === 'grid') {
    for (let i = 0; i < 9; i += 1) {
      const y = 180 + i * 78;
      r(`g${i}`, 0, y, W, 1, A(C.edge, 40 + i * 4));
      an(`g${i}`, 'translation_y', [[0, 0], [dur, 12]], 'linear');
    }
    for (let i = 0; i < 12; i += 1) r(`v${i}`, 160 + i * 140, 120, 1, 800, A(C.edge, 26));
  } else if (kind === 'dots') {
    for (let i = 0; i < 60; i += 1) {
      const x = 80 + (i % 12) * 150, y = 140 + Math.floor(i / 12) * 130;
      e(`d${i}`, x, y, 4, 4, A(C.edge, 90));
      an(`d${i}`, 'opacity', [[0, 0.3], [dur * 0.5, 0.9], [dur, 0.3]]);
    }
  } else if (kind === 'rings') {
    for (let i = 0; i < 5; i += 1) {
      e(`rg${i}`, 660 - i * 90, 300 - i * 70, 320 + i * 180, 320 + i * 180, A(C.bg, 0), { stroke: A(C.edge, 90 - i * 12), stroke_width: 2 });
      an(`rg${i}`, 'rotation', [[0, 0], [dur, 12]], 'linear');
    }
  }
}

// --- pieces ----------------------------------------------------------------
export function card3d(id, x, y, w, h, opts = {}) {
  r(`${id}Back`, x + 10, y + 12, w, h, { red: 0, green: 0, blue: 0, alpha: 90 });
  r(id, x, y, w, h, opts.fill ?? C.card, { stroke: opts.stroke ?? A(C.edge, 220), stroke_width: opts.sw ?? 1 });
  if (opts.head !== false) r(`${id}Head`, x, y, w, Math.min(34, h * 0.16), C.card2);
}

export function waveform(id, x, y, w, count, color, amp = 60, base = 0) {
  const step = w / count;
  for (let i = 0; i < count; i += 1) {
    const hh = 10 + ((i * 37 + base) % amp);
    r(`${id}${i}`, x + i * step, y - hh / 2, Math.max(2, step * 0.45), hh, A(color, 200));
    an(`${id}${i}`, 'opacity', [[0, 0.45], [0.2 + (i % 6) * 0.08, 1], [0.6 + (i % 4) * 0.1, 0.5]]);
  }
}

export function laneStrip(id, x, y, w, items, color) {
  r(`${id}Track`, x, y, w, 34, A(C.card2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
  items.forEach(([dx, cw, label], i) => {
    r(`${id}${i}`, x + dx, y + 3, cw, 28, A(color[i % color.length], 205));
    if (label) t(`${id}T${i}`, label, x + dx + 8, y + 9, 11, { red: 12, green: 13, blue: 15, alpha: 240 });
  });
}

// --- composers -------------------------------------------------------------
const CAMERA = {
  push: (dur) => [{ at: at(0), zoom: 1.0 }, { at: at(dur), zoom: 1.09, pan_x: -14 }],
  pull: (dur) => [{ at: at(0), zoom: 1.12, pan_x: 10 }, { at: at(dur), zoom: 1.0 }],
  pan: (dur) => [{ at: at(0), zoom: 1.06, pan_x: 50 }, { at: at(dur), zoom: 1.06, pan_x: -50 }],
  fly: (dur) => [{ at: at(0), zoom: 0.94 }, { at: at(dur * 0.55), zoom: 1.16, pan_x: -20 }, { at: at(dur), zoom: 1.02, pan_x: 18 }],
};

// --- captions ---------------------------------------------------------------
export function captionsFor(cueList, startMs, endMs, opts = {}) {
  const size = opts.font ?? 28;
  const bandY = opts.band ?? BAND;
  const own = cueList.filter((c) => c.start_ms >= startMs && c.start_ms < endMs);
  own.forEach((cue, i) => {
    const id = `cap${i}`;
    if (textW(cue.text, size) > (opts.maxWidth ?? 1700)) return;
    const localStart = Math.max(0, (cue.start_ms - startMs) / 1000 - 0.02);
    const localEnd = Math.max(0.1, (cue.end_ms - startMs) / 1000);
    centred(id, cue.text, 960, bandY + 42, size, C.ink);
    an(id, 'opacity', [[localStart, 0], [localStart + 0.06, 1], [Math.max(localStart + 0.1, localEnd - 0.06), 1], [localEnd, 0]], 'linear');
  });
}

// --- scene lifecycle --------------------------------------------------------
export function begin(canvas = CANVAS) {
  nodes = []; anims = []; uid = 0; currentTag = '01';
  canvasSize = { ...canvas };
}

let canvasSize = { ...CANVAS };
export function finish(duration) {
  const sceneNodes = nodes.slice();
  const byKey = new Map();
  for (const animation of anims) byKey.set(`${animation.node_id}|${animation.property}`, animation);
  const sceneAnims = [...byKey.values()];
  for (const animation of sceneAnims) {
    const clamped = [];
    for (const k of animation.keyframes) {
      const time = Math.min(Math.max(k.at.numerator / k.at.denominator, 0), duration);
      const prev = clamped.at(-1);
      if (prev && time <= prev.at.numerator / prev.at.denominator) { prev.value = k.value; continue; }
      clamped.push({ at: at(time), value: k.value });
    }
    if (clamped.length === 1) clamped.push({ at: at(duration), value: clamped[0].value });
    animation.keyframes = clamped;
  }
  return { nodes: sceneNodes, animations: sceneAnims };
}
export function setTag(tag) { currentTag = tag; }
export function setBandY(y) { BAND = y; }
export const cameras = CAMERA;

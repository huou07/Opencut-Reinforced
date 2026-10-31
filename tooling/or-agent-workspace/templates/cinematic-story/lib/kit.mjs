// Shared drawing kit for the OR video template.
//
// This is the kit behind the 59 s showcase: flat MotionScene nodes, a dark
// cinematic palette, camera presets, particles, waveforms, a lower caption band
// and helpers that keep text ink inside the frame. A composer draws through the
// module-level builder: call begin() before a scene and finish() after it.
import { randomUUID } from 'node:crypto';

export const W = 1920, H = 1080;
export const CAPTION_Y = 982;
// Everything a composer draws that must stay readable lives above this line; the
// caption track is laid out under it.
export const SAFE_BOTTOM = 900;

export const C = {
  bg: { red: 8, green: 9, blue: 11, alpha: 255 },
  bg2: { red: 12, green: 14, blue: 17, alpha: 255 },
  panel: { red: 17, green: 19, blue: 23, alpha: 255 },
  panel2: { red: 22, green: 25, blue: 30, alpha: 255 },
  panel3: { red: 28, green: 32, blue: 38, alpha: 255 },
  edge: { red: 42, green: 47, blue: 55, alpha: 255 },
  edge2: { red: 58, green: 65, blue: 75, alpha: 255 },
  ink: { red: 237, green: 240, blue: 245, alpha: 255 },
  mute: { red: 156, green: 165, blue: 176, alpha: 255 },
  dim: { red: 104, green: 113, blue: 125, alpha: 255 },
  faint: { red: 70, green: 78, blue: 89, alpha: 255 },
  amber: { red: 244, green: 182, blue: 78, alpha: 255 },
  cyan: { red: 79, green: 209, blue: 197, alpha: 255 },
  teal: { red: 84, green: 202, blue: 184, alpha: 255 },
  blue: { red: 96, green: 148, blue: 232, alpha: 255 },
  violet: { red: 167, green: 139, blue: 250, alpha: 255 },
  pink: { red: 244, green: 114, blue: 182, alpha: 255 },
  green: { red: 118, green: 204, blue: 130, alpha: 255 },
  red: { red: 226, green: 106, blue: 92, alpha: 255 },
};
export const A = (c, a) => ({ ...c, alpha: a });

const at = (s) => ({ numerator: Math.round(Math.max(0, s) * 1000), denominator: 1000 });

const CHAR = (ch) => {
  if (ch === ' ') return 0.28;
  if (ch === '.') return 0.26;
  if (ch === ',') return 0.26;
  if (ch === ':') return 0.3;
  if (ch === '·') return 0.4;
  if (ch === '—') return 0.9;
  if (ch === '-') return 0.33;
  if (ch === '/') return 0.4;
  if (ch === '|') return 0.26;
  if (ch === '…') return 0.9;
  if (ch >= 'A' && ch <= 'Z') return 0.63;
  if (ch >= '0' && ch <= '9') return 0.56;
  if ('ÁÀẢÃẠĂẮẰẲẴẶÂẤẦẨẪẬÉÈẺẼẸÊẾỀỂỄỆÍÌỈĨỊÓÒỎÕỌÔỐỒỔỖỘƠỚỜỞỠỢÚÙỦŨỤƯỨỪỬỮỰÝỲỶỸỴĐ'.includes(ch)) return 0.66;
  if (ch >= 'a' && ch <= 'z') return 0.52;
  return 0.5;
};
export const textW = (str, size) => [...str].reduce((sum, ch) => sum + CHAR(ch) * size, 0);

// Text ink starts at position.x + 5% of the remaining box and is baseline-shifted
// inside the measured line box; invert both so a caller can place ink.
const inkXToX = (inkX) => (inkX - 95.2) / 0.95;
const capYToY = (cy, size) => cy - 0.646 * size;

let nodes = [], anims = [], uid = 0;
const nid = (p) => `${p}${uid++}`;

export function rect(id, x, y, w, h, fill, extra = {}) {
  nodes.push({ node_id: id, kind: 'rectangle', position: { x: Math.round(x * 100) / 100, y: Math.round(y * 100) / 100 }, size: { width: Math.max(1, Math.round(w * 100) / 100), height: Math.max(1, Math.round(h * 100) / 100) }, fill, ...extra });
  return id;
}
export function ell(id, x, y, w, h, fill, extra = {}) {
  nodes.push({ node_id: id, kind: 'ellipse', position: { x: Math.round(x * 100) / 100, y: Math.round(y * 100) / 100 }, size: { width: Math.max(1, Math.round(w * 100) / 100), height: Math.max(1, Math.round(h * 100) / 100) }, fill, ...extra });
  return id;
}
export function txt(id, value, inkX, capY, size, fill, extra = {}) {
  nodes.push({ node_id: id, kind: 'text', text: value, position: { x: Math.round(inkXToX(inkX) * 100) / 100, y: Math.round(capYToY(capY, size) * 100) / 100 }, font_size: size, fill, ...extra });
  return id;
}
export function arrow(id, x, y, ex, ey, width, fill) {
  nodes.push({ node_id: id, kind: 'arrow', position: { x: Math.round(x), y: Math.round(y) }, end: { x: Math.round(ex), y: Math.round(ey) }, stroke_width: width, fill });
  return id;
}
export const centre = (id, value, cx, capY, size, fill, extra = {}) => txt(id, value, cx - textW(value, size) / 2, capY, size, fill, extra);

export function anim(id, property, keys, easing = 'ease_in_out') {
  const frames = keys.map(([t, v]) => ({ at: at(t), value: v, t }));
  let existing = anims.find((a) => a.node_id === id && a.property === property);
  if (!existing) {
    existing = { node_id: id, property, keyframes: [], easing };
    anims.push(existing);
  }
  frames.forEach((frame) => {
    const same = existing.keyframes.find((k) => k.t === frame.t);
    if (same) same.value = frame.value;
    else existing.keyframes.push(frame);
  });
  existing.keyframes.sort((a, b) => a.t - b.t);
  return id;
}
export const fade = (id, t0, d = 0.4) => anim(id, 'opacity', [[Math.max(0, t0), 0], [Math.max(0, t0) + d, 1]], 'ease_out');
export const fadeOut = (id, t0, d = 0.4) => anim(id, 'opacity', [[t0, 1], [t0 + d, 0]], 'ease_in');
export function pop(id, t0, dist = 24, d = 0.5) {
  anim(id, 'translation_y', [[Math.max(0, t0), dist], [Math.max(0, t0) + d, 0]], 'ease_out');
  return fade(id, t0, d);
}
export function wipeX(id, t0, full, d = 0.5, from = 0) {
  return anim(id, 'width', [[Math.max(0, t0), from], [Math.max(0, t0) + d, full]], 'ease_out');
}
export function slideTo(ids, property, keys, easing = 'ease_in_out') {
  ids.forEach((id) => anim(id, property, keys, easing));
  return ids;
}
export function pulse(id, t0, count, period = 0.8, low = 0.25, high = 1) {
  const keys = [];
  for (let i = 0; i < count; i += 1) keys.push([t0 + i * period, low], [t0 + i * period + period / 2, high]);
  if (keys.length) keys.push([keys.at(-1)[0] + period / 2, low]);
  return anim(id, 'opacity', keys);
}
// Scale a rectangle about its centre by pairing scale with a compensating move.
export function scaleAbout(id, x, y, w, h, t0, s0, s1, d, easing = 'ease_out') {
  anim(id, 'scale', [[Math.max(0, t0), s0], [Math.max(0, t0) + d, s1]], easing);
  const keys = [[Math.max(0, t0), -(w * (s0 - 1)) / 2], [Math.max(0, t0) + d, -(w * (s1 - 1)) / 2]];
  if (x !== null) anim(id, 'translation_x', keys.map(([t, v]) => [t, v]), easing);
  if (y !== null) anim(id, 'translation_y', keys.map(([t, v]) => [t, -(h * (s1 - 1)) / 2 + (h * (s0 - 1)) / 2 + v - v]), easing);
  return id;
}

// --- world ------------------------------------------------------------------
export function backdrop({ grid = 0, glows = true, vignette = true } = {}) {
  rect('bg', -20, -20, W + 40, H + 40, C.bg);
  if (glows) {
    ell('glowA', -520, -420, 1400, 1200, A(C.blue, 18));
    ell('glowB', 1080, 420, 1200, 1000, A(C.violet, 14));
    ell('glowC', 620, -260, 900, 700, A(C.cyan, 8));
  }
  if (grid) {
    for (let i = 0; i < 16; i += 1) rect(`gx${i}`, 120 + i * 112, 0, 1, H, A(C.edge, grid));
    for (let i = 0; i < 10; i += 1) rect(`gy${i}`, 0, 100 + i * 98, W, 1, A(C.edge, grid));
  }
  if (vignette) {
    // a flat, dark frame edge: four bars, never a gradient
    rect('vT', 0, 0, W, 46, A(C.bg, 120));
    rect('vB', 0, H - 46, W, 46, A(C.bg, 120));
  }
}

export function cornerMarks(color = A(C.edge2, 200), inset = 46, len = 38) {
  rect('cm0a', inset, inset, len, 2, color);
  rect('cm0b', inset, inset, 2, len, color);
  rect('cm1a', W - inset - len, inset, len, 2, color);
  rect('cm1b', W - inset - 2, inset, 2, len, color);
  rect('cm2a', inset, H - inset - 2, len, 2, color);
  rect('cm2b', inset, H - inset - len, 2, len, color);
  rect('cm3a', W - inset - len, H - inset - 2, len, 2, color);
  rect('cm3b', W - inset - 2, H - inset - len, 2, len, color);
}

export function label(id, value, x, y, color = A(C.dim, 255), size = 17) {
  txt(id, value, x, y, size, color);
  return id;
}
export function tag(id, value, x, y, color, size = 15) {
  const w = textW(value, size) + 26;
  rect(`${id}bg`, x - 13, y - size * 1.05, w, size * 2.0, A(color, 26), { stroke: A(color, 150), stroke_width: 1 });
  txt(`${id}t`, value, x, y, size, A(color, 235));
  return id;
}

// Small particles: deterministic scatter around a centre.
export function particles(prefix, cx, cy, count, radius, color, dur, opts = {}) {
  const speed = opts.speed ?? 22;
  for (let i = 0; i < count; i += 1) {
    const angle = (i / count) * Math.PI * 2 + (i % 5) * 0.31;
    const rad = radius * (0.28 + ((i * 37) % 100) / 100);
    const x = cx + Math.cos(angle) * rad;
    const y = cy + Math.sin(angle) * rad * (opts.squash ?? 0.7);
    const s = (opts.size ?? 3) + ((i * 13) % 4);
    ell(`${prefix}${i}`, x, y, s, s, A(color, 120 + ((i * 29) % 110)));
    anim(`${prefix}${i}`, 'translation_x', [[0, 0], [dur, Math.cos(angle + 1.1) * speed * (0.4 + ((i * 7) % 10) / 10)]], 'linear');
    anim(`${prefix}${i}`, 'translation_y', [[0, 0], [dur, Math.sin(angle + 1.1) * speed * (0.4 + ((i * 11) % 10) / 10)]], 'linear');
    anim(`${prefix}${i}`, 'opacity', [[0, 0.15 + ((i * 17) % 70) / 100], [dur * 0.5, 0.9], [dur, 0.2]], 'ease_in_out');
  }
}

export function waveform(prefix, x, y, w, bars, color, dur, { amp = 70, barW = 3, gap = 5, phase = 0 } = {}) {
  const step = w / bars;
  for (let i = 0; i < bars; i += 1) {
    const base = 8 + ((i * 53 + 17) % amp);
    const id = `${prefix}${i}`;
    rect(id, x + i * step, y - base / 2, barW, base, A(color, 190));
    const keys = [];
    const seed = ((i * 7919 + Math.round(phase * 1000)) % 97) / 97;
    for (let k = 0; k <= 6; k += 1) {
      const t = (dur * k) / 6;
      const wave = 0.35 + 0.65 * Math.abs(Math.sin((k + 1) * (0.8 + seed * 1.7) + i * 0.55));
      keys.push([t, Math.max(4, base * wave)]);
    }
    anim(id, 'height', keys, 'ease_in_out');
    anim(id, 'translation_y', keys.map(([t, v]) => [t, (base - v) / 2]), 'ease_in_out');
  }
}

// Captions: exact cues, drawn in the reserved lower band.
export function captions(cues, startMs, endMs, opts = {}) {
  const size = opts.size ?? 30;
  const cy = opts.y ?? CAPTION_Y;
  const color = opts.color ?? C.ink;
  const own = cues.filter((c) => c.start_ms < endMs && c.end_ms > startMs);
  own.forEach((cue, i) => {
    const localStart = Math.max(0, (cue.start_ms - startMs) / 1000);
    const localEnd = Math.min((endMs - startMs), (cue.end_ms - startMs) / 1000);
    if (localEnd <= 0.05) return;
    const id = `cap${i}`;
    centre(id, cue.text, W / 2, cy, size, color);
    const a = Math.max(0, localStart - 0.02), b = Math.min((endMs - startMs), localEnd);
    anim(id, 'opacity', [[a, a > 0.06 ? 0 : 0], [Math.min(b, a + 0.12), 1], [Math.max(a + 0.14, b - 0.12), 1], [b, 0]], 'linear');
  });
}

// --- scene lifecycle --------------------------------------------------------
export function begin() { nodes = []; anims = []; uid = 0; }

export function finish({ name, duration, camera, canvas = { width: W, height: H } }) {
  const byKey = new Map();
  for (const a of anims) byKey.set(`${a.node_id}|${a.property}`, a);
  const out = [];
  for (const a of byKey.values()) {
    const clamped = [];
    for (const k of a.keyframes) {
      const time = Math.min(Math.max(k.at.numerator / k.at.denominator, 0), duration);
      const prev = clamped.at(-1);
      if (prev && time <= prev.at.numerator / prev.at.denominator) { prev.value = k.value; continue; }
      clamped.push({ at: at(time), value: k.value });
    }
    if (!clamped.length) continue;
    // A property that starts animating later must hold its first value from 0.
    // Without this, the renderer uses the node's base value until the first key
    // and a delayed clip appears at full opacity before its entrance.
    if (clamped[0].at.numerator / clamped[0].at.denominator > 0) {
      clamped.unshift({ at: at(0), value: clamped[0].value });
    }
    if (clamped.length === 1) clamped.push({ at: at(duration), value: clamped[0].value });
    out.push({ ...a, keyframes: clamped });
  }
  const scene = {
    format: 'opencut-reinforced-motion-scene',
    schema_version: 1,
    scene_id: randomUUID(),
    name,
    duration: { numerator: Math.round(duration * 1000), denominator: 1000 },
    canvas,
    assets: [],
    nodes: nodes.slice(),
    animations: out,
  };
  if (camera) scene.camera = { keyframes: camera.map((k) => ({ at: at(k.t), ...Object.fromEntries(Object.entries(k).filter(([key]) => key !== 't')) })), easing: 'ease_in_out' };
  return scene;
}

export function mark() { return nodes.length; }
export function since(m) { return nodes.slice(m).map((n) => n.node_id); }
export function translation(ids, property, keys, easing = 'ease_in_out') { ids.forEach((id) => anim(id, property, keys, easing)); return ids; }

// --- camera ----------------------------------------------------------------
// Updated per scene. `zoom` never drops below 0.94 so a flat backdrop can never
// show the canvas edge; pan is in canvas pixels.
export const cam = {
  push: (d) => [{ t: 0, zoom: 1.0 }, { t: d, zoom: 1.08, pan_y: -10 }],
  pull: (d) => [{ t: 0, zoom: 1.1, pan_y: 8 }, { t: d, zoom: 1.0 }],
  panLeft: (d) => [{ t: 0, zoom: 1.05, pan_x: 60 }, { t: d, zoom: 1.05, pan_x: -60 }],
  panRight: (d) => [{ t: 0, zoom: 1.05, pan_x: -60 }, { t: d, zoom: 1.05, pan_x: 60 }],
  fly: (d) => [
    { t: 0, zoom: 0.99, pan_x: 30 },
    { t: d * 0.55, zoom: 1.14, pan_x: -24, pan_y: -14 },
    { t: d, zoom: 1.02, pan_x: 16 },
  ],
  dive: (d, target) => [
    { t: 0, zoom: 1.04 },
    { t: d * 0.7, zoom: 1.02 },
    { t: d, zoom: Math.max(1, target ?? 1.3), pan_x: 0, pan_y: 0 },
  ],
  settle: (d) => [{ t: 0, zoom: 1.02, pan_y: 6 }, { t: d, zoom: 1.0 }],
};

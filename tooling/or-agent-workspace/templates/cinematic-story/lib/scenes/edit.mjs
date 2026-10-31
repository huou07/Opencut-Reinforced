// Editing chapter: the real workspace, then the exact-frame edits that make it a
// tool rather than a mock-up, then preview and export.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';
import { windowFrame, viewer, mediaBin, inspector, timeline, playhead, waveformBlock, tagChip, UI } from '../ui.mjs';

export function editorBuild(duration) {
  begin();
  const d = duration;
  const tWin = 0.04 * d;
  const tPanels = 0.2 * d;
  const tTl = 0.38 * d;
  const tClips = 0.56 * d;
  const tPush = 0.82 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', -300, -400, 1400, 1100, A(C.blue, 9));
  ell('glowB', 1450, 500, 1100, 900, A(C.violet, 7));
  for (let i = 0; i < 10; i += 1) rect(`gx${i}`, 120 + i * 170, 0, 1, 1080, A(C.edge, 18));

  const win = windowFrame('w');
  const winIds = ['wBg', 'wBar', 'wBarLine', 'wDot0', 'wDot1', 'wDot2', 'wTitle', 'wFile', 'wMode', 'wModeDot', 'wShadow'];
  winIds.forEach((id) => anim(id, 'opacity', [[tWin, 0], [tWin + 0.25, 1]], 'ease_out'));
  anim('wBg', 'scale', [[tWin, 0.96], [tWin + 0.35, 1]], 'ease_out');
  anim('wBg', 'translation_y', [[tWin, -30], [tWin + 0.35, 0]], 'ease_out');

  // viewer first: it is the thing a beginner recognizes
  viewer('v', 430, 142, 800, 420, { tc: '00:00:12:08' });
  const vIds = ['vCase', 'vBg', 'vSky', 'vHalo', 'vSun', 'vSunCore', 'vRidgeA', 'vRidgeB', 'vWater', 'vHorizon', 'vHead', 'vBody', 'vRim', 'vRule', 'vTc', 'vRes'];
  vIds.forEach((id) => anim(id, 'opacity', [[tWin + 0.12, 0], [tWin + 0.34, 1]], 'ease_out'));
  anim('vSun', 'translation_y', [[0, 12], [d, -8]], 'ease_in_out');
  anim('vRidgeA', 'translation_x', [[0, -10], [d, 8]], 'ease_in_out');

  // bin
  mediaBin('b', 140, 142, 270, 420, {});
  ['bBg', 'bTitle', 'bRule', 'bImport', 'bImportT'].forEach((id) => anim(id, 'opacity', [[tPanels, 0], [tPanels + 0.25, 1]], 'ease_out'));
  for (let i = 0; i < 5; i += 1) {
    ['Thumb', 'ThumbBar', 'Name', 'Dur'].forEach((s) => anim(`b${s}${i}`, 'opacity', [[tPanels + 0.06 + i * 0.04, 0], [tPanels + 0.2 + i * 0.04, 1]], 'ease_out'));
    anim(`bThumb${i}`, 'translation_x', [[tPanels + 0.06 + i * 0.04, -20], [tPanels + 0.26 + i * 0.04, 0]], 'ease_out');
    anim(`bName${i}`, 'translation_x', [[tPanels + 0.06 + i * 0.04, -20], [tPanels + 0.26 + i * 0.04, 0]], 'ease_out');
  }

  // inspector
  inspector('i', 1250, 142, 526, 420, {});
  ['iBg', 'iTitle', 'iRule', 'iColour'].forEach((id) => anim(id, 'opacity', [[tPanels + 0.1, 0], [tPanels + 0.3, 1]], 'ease_out'));
  for (let i = 0; i < 4; i += 1) {
    [`iL${i}`, `iTrack${i}`, `iFill${i}`, `iKnob${i}`].forEach((id) => anim(id, 'opacity', [[tPanels + 0.14 + i * 0.04, 0], [tPanels + 0.3 + i * 0.04, 1]], 'ease_out'));
    anim(`iKnob${i}`, 'translation_x', [[tPanels + 0.3, 0], [tPanels + 0.8, 12 + i * 6]], 'ease_in_out');
  }
  for (let i = 0; i < 5; i += 1) anim(`iSw${i}`, 'opacity', [[tPanels + 0.3 + i * 0.02, 0], [tPanels + 0.4 + i * 0.02, 1]], 'ease_out');

  // timeline frame and lanes
  const T = timeline('t', 140, 590, 1640, 250, {
    tc: '00:00:12:08',
    clips: [],
  });
  ['tBg', 'tHead', 'tTitle', 'tTc'].forEach((id) => anim(id, 'opacity', [[tTl, 0], [tTl + 0.22, 1]], 'ease_out'));
  for (let i = 0; i < 5; i += 1) {
    [`tLane${i}`, `tLaneTag${i}`, `tLaneT${i}`].forEach((id) => anim(id, 'opacity', [[tTl + 0.04 + i * 0.05, 0], [tTl + 0.18 + i * 0.05, 1]], 'ease_out'));
    anim(`tLane${i}`, 'width', [[tTl + 0.04 + i * 0.05, 20], [tTl + 0.3 + i * 0.05, 1616]], 'ease_out');
  }

  // clips drop onto the lanes
  const clips = [
    [2, 240, 300, C.amber, 'shot-01'],
    [2, 560, 250, C.teal, 'shot-02'],
    [2, 830, 380, C.amber, 'shot-03'],
    [3, 240, 520, C.cyan],
    [4, 240, 1100, C.blue],
    [1, 700, 220, C.violet, 'title'],
    [0, 560, 320, C.blue],
  ];
  clips.forEach(([lane, x, w, color, label], i) => {
    const ly = 590 + 34 + lane * T.laneH;
    const h = T.laneH - 7;
    const t0 = tClips + (i / clips.length) * 0.26 * d;
    rect(`k${i}`, x, ly + 2, w, h, A(color, 185));
    rect(`k${i}Cap`, x, ly + 2, w, 2, A(color, 245));
    if (label) txt(`k${i}T`, label, x + 8, ly + h / 2 + 3, 9, A(C.bg, 185));
    anim(`k${i}`, 'translation_y', [[t0, -140], [t0 + 0.13 * d, 0]], 'ease_out');
    anim(`k${i}Cap`, 'translation_y', [[t0, -140], [t0 + 0.13 * d, 0]], 'ease_out');
    if (label) anim(`k${i}T`, 'translation_y', [[t0, -140], [t0 + 0.13 * d, 0]], 'ease_out');
    anim(`k${i}`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 1]], 'ease_out');
    anim(`k${i}Cap`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 1]], 'ease_out');
    if (label) anim(`k${i}T`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 1]], 'ease_out');
  });

  // playhead sweeps the timeline
  playhead('t', 158 + 18, 624, 210);
  ['tPlay', 'tPlayHead'].forEach((id) => anim(id, 'opacity', [[tTl + 0.2, 0], [tTl + 0.3, 1]], 'ease_out'));
  anim('tPlay', 'translation_x', [[tClips, 0], [d, 1180]], 'linear');
  anim('tPlayHead', 'translation_x', [[tClips, 0], [d, 1180]], 'linear');

  // a selection and an undo chip: human decisions remain visible
  rect('sel', 830, 590 + 34 + 2 * T.laneH + 1, 380, T.laneH - 9, A(C.ink, 0), { stroke: A(C.ink, 210), stroke_width: 2 });
  anim('sel', 'opacity', [[0.7 * d, 0], [0.74 * d, 1], [0.9 * d, 1], [0.95 * d, 0]], 'ease_in_out');
  tagChip('undo', '⌘Z  UNDO', 1580, 566, C.dim, 12);
  ['undoBg', 'undoT'].forEach((id) => anim(id, 'opacity', [[0.78 * d, 0], [0.82 * d, 1]], 'ease_out'));

  // camera: wide → right rail → timeline
  return finish({
    name: 'Dựng phim — không gian làm việc',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_y: 10 },
      { t: 0.2 * d, zoom: 1.0 },
      { t: 0.42 * d, zoom: 1.04, pan_y: -10 },
      { t: 0.62 * d, zoom: 1.02, pan_y: 20 },
      { t: tPush, zoom: 1.1, pan_x: -10, pan_y: 0 },
      { t: d, zoom: 1.18, pan_x: -20, pan_y: 0 },
    ],
  });
}

export function editActions(duration) {
  begin();
  const d = duration;
  const T = { x: 160, y: 210, w: 1600, h: 560 };
  const laneCount = 5;
  const laneH = (T.h - 50) / laneCount;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 300, 120, 1500, 1000, A(C.amber, 8));
  ell('glowB', 1200, 600, 1000, 900, A(C.cyan, 7));

  // a large, close timeline — this beat is about the edit, not the window
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('tlHead', T.x, T.y, T.w, 40, C.panel2);
  txt('tlTitle', 'TIMELINE · 00:00:14:02', T.x + 20, T.y + 23, 13, A(C.mute, 235));
  txt('tlMode', 'SNAP ON', T.x + T.w - 92, T.y + 23, 11, A(C.green, 225));
  const lanes = [
    ['C1', C.blue], ['V2', C.violet], ['V1', C.amber], ['A1', C.cyan], ['A2', C.blue],
  ];
  lanes.forEach(([name, color], i) => {
    const y = T.y + 44 + i * laneH;
    rect(`lane${i}`, T.x + 14, y, T.w - 28, laneH - 4, A(C.panel2, 255));
    rect(`laneTag${i}`, T.x + 14, y, 46, laneH - 4, A(C.bg2, 255));
    txt(`laneT${i}`, name, T.x + 26, y + laneH / 2, 11, A(color, 235));
  });
  const laneY = (i) => T.y + 44 + i * laneH + 2;

  // clips, large and readable
  const base = [
    [2, 230, 300, C.amber, 'shot-01'],
    [2, 540, 250, C.teal, 'shot-02'],
    [2, 800, 200, C.amber, 'shot-03'],
    [2, 1010, 280, C.teal, 'shot-04'],
    [2, 1300, 380, C.amber, 'shot-05'],
    [3, 230, 700, C.cyan],
    [3, 950, 720, C.cyan],
    [4, 230, 1400, C.blue],
    [1, 860, 250, C.violet, 'title'],
  ];
  base.forEach(([lane, x, w, color, label], i) => {
    rect(`c${i}`, x, laneY(lane), w, laneH - 8, A(color, 180));
    rect(`c${i}Cap`, x, laneY(lane), w, 2, A(color, 245));
    if (label) txt(`c${i}T`, label, x + 9, laneY(lane) + 18, 10, A(C.bg, 190));
  });
  // waveform inside audio clips
  for (let i = 0; i < 52; i += 1) {
    const bh = 4 + ((i * 31) % 20);
    rect(`aw${i}`, 238 + i * 13.2, laneY(3) + 18 - bh / 2, 3, bh, A(C.bg, 115));
  }

  // playhead stops at a cut point; selection, split, ripple
  const cutX = 880;
  rect('play', cutX, T.y + 40, 2, T.h - 40, A(C.red, 225));
  rect('playHead', cutX - 9, T.y + 40, 20, 7, A(C.red, 235));
  anim('playHead', 'translation_x', [[0, -500], [0.1 * d, 0]], 'ease_out');
  anim('play', 'opacity', [[0.05 * d, 0], [0.1 * d, 1]], 'ease_out');
  anim('playHead', 'opacity', [[0.05 * d, 0], [0.1 * d, 1]], 'ease_out');

  rect('sel', 800, laneY(2) - 2, 200, laneH - 4, A(C.ink, 0), { stroke: A(C.ink, 225), stroke_width: 3 });
  anim('sel', 'opacity', [[0.08 * d, 0], [0.12 * d, 1], [0.2 * d, 1], [0.24 * d, 0]], 'ease_in_out');

  // split: the selected clip becomes two
  rect('splitLine', cutX, laneY(2), 2, laneH - 8, A(C.ink, 240));
  anim('splitLine', 'opacity', [[0.2 * d, 0], [0.22 * d, 1], [0.32 * d, 1], [0.34 * d, 0]], 'ease_out');
  const half = cutX - 800;
  rect('c2b', 800, laneY(2), half - 2, laneH - 8, A(C.teal, 205));
  rect('c2bCap', 800, laneY(2), half - 2, 2, A(C.teal, 245));
  rect('c2c', cutX + 2, laneY(2), 200 - half - 2, laneH - 8, A(C.amber, 205));
  rect('c2cCap', cutX + 2, laneY(2), 200 - half - 2, 2, A(C.amber, 245));
  ['c2b', 'c2bCap', 'c2c', 'c2cCap'].forEach((id) => anim(id, 'opacity', [[0.24 * d, 0], [0.27 * d, 1]], 'ease_out'));
  // the old clip is covered by the two halves
  anim('c2', 'opacity', [[0.24 * d, 1], [0.27 * d, 0]], 'ease_in');

  // ripple delete the right half and slide the rest left
  anim('c2c', 'opacity', [[0.38 * d, 1], [0.42 * d, 0]], 'ease_in');
  anim('c2cCap', 'opacity', [[0.38 * d, 1], [0.42 * d, 0]], 'ease_in');
  const shift = (id, dx, t0) => anim(id, 'translation_x', [[t0, 0], [t0 + 0.16 * d, dx]], 'ease_in_out');
  ['c3', 'c3Cap', 'c4', 'c4Cap', 'c5', 'c5Cap', 'c2b', 'c2bCap', 'c7', 'c7Cap', 'c8', 'c8Cap'].forEach((id) => shift(id, -half, 0.42 * d));
  for (let i = 0; i < 52; i += 1) shift(`aw${i}`, -half, 0.42 * d);
  anim('c2b', 'width', [[0.42 * d, half - 2], [0.52 * d, 260]], 'ease_out');

  // move a clip up to V2 so it overlays; trim its tail
  anim('c5', 'translation_y', [[0.55 * d, 0], [0.63 * d, -laneH]], 'ease_in_out');
  anim('c5', 'translation_x', [[0.55 * d, 0], [0.63 * d, 120]], 'ease_in_out');
  anim('c5Cap', 'translation_y', [[0.55 * d, 0], [0.63 * d, -laneH]], 'ease_in_out');
  anim('c5Cap', 'translation_x', [[0.55 * d, 0], [0.63 * d, 120]], 'ease_in_out');
  anim('c5', 'width', [[0.7 * d, 380], [0.8 * d, 260]], 'ease_in_out');
  anim('c5Cap', 'width', [[0.7 * d, 380], [0.8 * d, 260]], 'ease_in_out');
  rect('trimHandle', 1680, laneY(1), 4, laneH - 8, A(C.ink, 230));
  anim('trimHandle', 'opacity', [[0.7 * d, 0], [0.72 * d, 1], [0.82 * d, 1], [0.84 * d, 0]], 'ease_out');
  anim('trimHandle', 'translation_x', [[0.7 * d, 0], [0.8 * d, -140]], 'ease_in_out');

  // history: each action is a card, the stack grows
  const hist = [
    ['SPLIT', C.cyan, 0.24],
    ['RIPPLE', C.amber, 0.42],
    ['MOVE', C.violet, 0.55],
    ['TRIM', C.green, 0.7],
  ];
  hist.forEach(([label, color, t], i) => {
    const hx = 1640 - i * 12;
    const hy = 130 + i * 34;
    rect(`h${i}`, hx, hy, 190, 26, A(C.panel, 245), { stroke: A(color, 150), stroke_width: 1 });
    txt(`h${i}T`, label, hx + 12, hy + 18, 11, A(color, 230));
    anim(`h${i}`, 'opacity', [[t * d, 0], [t * d + 0.1 * d, 1]], 'ease_out');
    anim(`h${i}`, 'translation_x', [[t * d, 40], [t * d + 0.1 * d, 0]], 'ease_out');
    anim(`h${i}T`, 'opacity', [[t * d, 0], [t * d + 0.1 * d, 1]], 'ease_out');
  });
  tagChip('undoH', '⌘Z  UNDO · 4 BƯỚC', 160, 130, C.dim, 12);
  ['undoHBg', 'undoHT'].forEach((id) => anim(id, 'opacity', [[0.85 * d, 0], [0.88 * d, 1]], 'ease_out'));

  // a small preview stays visible above the timeline
  const V = { x: 640, y: 74, w: 420, h: 118 };
  rect('pv', V.x, V.y, V.w, V.h, { red: 10, green: 13, blue: 18, alpha: 255 }, { stroke: A(C.edge2, 180), stroke_width: 1 });
  rect('pvSky', V.x + 2, V.y + 2, V.w - 4, 70, { red: 15, green: 19, blue: 26, alpha: 255 });
  ell('pvSun', V.x + 300, V.y + 14, 44, 44, A(C.amber, 235));
  rect('pvGround', V.x + 2, V.y + 72, V.w - 4, 44, { red: 7, green: 9, blue: 12, alpha: 255 });
  rect('pvBar', V.x + 2, V.y + 100, 180, 3, A(C.amber, 220));
  txt('pvLabel', 'PREVIEW', V.x + 8, V.y + 14, 9, A(C.mute, 220));
  anim('pvBar', 'width', [[0.1 * d, 30], [d, 400]], 'linear');

  particles('dust', 1500, 240, 18, 320, C.amber, d, { size: 1.6, speed: 14 });

  return finish({
    name: 'Cắt, ripple, move, trim',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -30 },
      { t: 0.25 * d, zoom: 1.08, pan_x: 60 },
      { t: 0.45 * d, zoom: 1.06, pan_x: 90 },
      { t: 0.65 * d, zoom: 1.1, pan_x: 30, pan_y: -20 },
      { t: 0.85 * d, zoom: 1.04, pan_x: 0 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function previewExport(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 200, 120, 1400, 1000, A(C.amber, 9));
  ell('glowB', 1300, 480, 1100, 900, A(C.violet, 7));

  const V = { x: 180, y: 150, w: 940, h: 540 };
  rect('vCase', V.x - 12, V.y - 12, V.w + 24, V.h + 24, C.panel);
  rect('vBg', V.x, V.y, V.w, V.h, { red: 8, green: 10, blue: 13, alpha: 255 }, { stroke: A(C.edge2, 200), stroke_width: 1 });

  // three shots crossfading inside the viewer
  const shots = [
    ['s0', C.amber, { red: 16, green: 20, blue: 27 }],
    ['s1', C.cyan, { red: 12, green: 22, blue: 28 }],
    ['s2', C.violet, { red: 21, green: 15, blue: 28 }],
  ];
  shots.forEach(([prefix, accent, sky], i) => {
    rect(`${prefix}Sky`, V.x, V.y, V.w, V.h * 0.66, { red: sky.red, green: sky.green, blue: sky.blue, alpha: 255 });
    ell(`${prefix}Sun`, V.x + V.w * 0.66, V.y + V.h * 0.1, V.h * 0.24, V.h * 0.24, A(accent, 236));
    ell(`${prefix}RidgeA`, V.x - 40, V.y + V.h * 0.5, V.w * 0.62, V.h * 0.4, { red: 9, green: 11, blue: 15, alpha: 255 });
    ell(`${prefix}RidgeB`, V.x + V.w * 0.42, V.y + V.h * 0.54, V.w * 0.64, V.h * 0.38, { red: 12, green: 15, blue: 20, alpha: 255 });
    rect(`${prefix}Ground`, V.x, V.y + V.h * 0.66, V.w, V.h * 0.34, { red: 7, green: 9, blue: 12, alpha: 255 });
    if (i === 1) for (let k = 0; k < 9; k += 1) {
      const h2 = 30 + ((k * 47) % 150);
      rect(`${prefix}City${k}`, V.x + 70 + k * 96, V.y + V.h * 0.62 - h2, 54, h2, { red: 14, green: 26, blue: 34, alpha: 255 });
      for (let w = 0; w < 3; w += 1) rect(`${prefix}Win${k}_${w}`, V.x + 80 + k * 96, V.y + V.h * 0.6 - h2 + w * 22, 9, 4, A(C.cyan, 145));
    }
    if (i === 2) {
      ell(`${prefix}Head`, V.x + V.w * 0.45, V.y + V.h * 0.34, 90, 90, { red: 26, green: 20, blue: 36, alpha: 255 });
      rect(`${prefix}Body`, V.x + V.w * 0.38, V.y + V.h * 0.52, 230, 130, { red: 22, green: 17, blue: 31, alpha: 255 });
      ell(`${prefix}Rim`, V.x + V.w * 0.45, V.y + V.h * 0.34, 90, 90, A(C.violet, 0), { stroke: A(C.violet, 120), stroke_width: 3 });
    }
  });
  const groups = [
    ['s0', 0.02, 0.3],
    ['s1', 0.28, 0.56],
    ['s2', 0.54, 0.82],
  ];
  groups.forEach(([prefix, a, b]) => {
    const ids = ['Sky', 'Sun'].concat(prefix === 's1' ? Array.from({ length: 9 }, (_, k) => `City${k}`).concat(Array.from({ length: 27 }, (_, k) => `Win${k}`)) : []).concat(prefix === 's2' ? ['Head', 'Body', 'Rim'] : []).concat(['RidgeA', 'RidgeB', 'Ground']);
    ids.forEach((suffix) => {
      if (prefix === 's1' && suffix.startsWith('Win')) return;
      const id = `${prefix}${suffix}`;
      anim(id, 'opacity', [[a * d, 0], [a * d + 0.06 * d, 1], [b * d - 0.05 * d, 1], [b * d, 0]], 'ease_in_out');
    });
    anim(`${prefix}Sun`, 'translation_x', [[a * d, 14], [b * d, -12]], 'ease_in_out');
  });
  // windows of the city
  for (let k = 0; k < 9; k += 1) for (let w = 0; w < 3; w += 1) {
    anim(`s1Win${k}_${w}`, 'opacity', [[0.3 * d, 0], [0.36 * d, 1], [0.52 * d, 1], [0.56 * d, 0]], 'ease_in_out');
  }

  // frame furniture
  rect('vRule', V.x, V.y + V.h + 22, V.w, 1, A(C.edge2, 160));
  txt('vTc', '00:00:42:18', V.x, V.y + V.h + 36, 12, A(C.mute, 210));
  txt('vRes', '1920 × 1080 · 30p', V.x + V.w - textW('1920 × 1080 · 30p', 12), V.y + V.h + 36, 12, A(C.mute, 170));
  rect('vPlay', V.x, V.y + V.h + 12, 2, 2, A(C.red, 225));

  // ------------------------------------------------------------- export panel
  const E = { x: 1200, y: 150, w: 540, h: 620 };
  rect('eShadow', E.x + 12, E.y + 14, E.w, E.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('eBg', E.x, E.y, E.w, E.h, C.panel, { stroke: A(C.edge2, 215), stroke_width: 1 });
  rect('eBar', E.x, E.y, E.w, 42, C.panel2);
  ell('eDot', E.x + 18, E.y + 17, 8, 8, A(C.green, 235));
  txt('eTitle', 'EXPORT', E.x + 34, E.y + 20, 12, A(C.ink, 232));
  txt('ePreset', 'MATROSKA · 1080p', E.x + E.w - 18 - textW('MATROSKA · 1080p', 11), E.y + 21, 11, A(C.dim, 210));
  const rows = [
    ['RESOLUTION', '1920 × 1080'],
    ['FRAME RATE', '30 fps'],
    ['VIDEO', 'H.264 · high'],
    ['AUDIO', 'AAC · 48 kHz'],
    ['RANGE', 'Cả timeline'],
  ];
  rows.forEach(([k, v], i) => {
    const y = E.y + 74 + i * 44;
    txt(`eK${i}`, k, E.x + 22, y, 10, A(C.dim, 220));
    txt(`eV${i}`, v, E.x + E.w - 22 - textW(v, 12), y, 12, A(C.mute, 235));
    rect(`eR${i}`, E.x + 22, y + 16, E.w - 44, 1, A(C.edge, 140));
  });
  rect('ePath', E.x + 22, E.y + 300, E.w - 44, 32, A(C.panel2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
  txt('ePathT', '~/Movies/gioi-thieu-or.mkv', E.x + 34, E.y + 312, 11, A(C.mute, 215));

  tagChip('eGo', 'BẮT ĐẦU XUẤT', E.x + 22, E.y + 372, C.amber, 14);
  ['eGoBg', 'eGoT'].forEach((id) => anim(id, 'opacity', [[0.16 * d, 0], [0.2 * d, 1]], 'ease_out'));
  rect('eProgBg', E.x + 22, E.y + 410, E.w - 44, 4, A(C.edge2, 200));
  rect('eProg', E.x + 22, E.y + 408, 4, 8, A(C.amber, 235));
  anim('eProg', 'width', [[0.22 * d, 4], [0.74 * d, E.w - 44]], 'linear');
  const pct = [['42%', 0.45], ['78%', 0.62], ['100%', 0.76]];
  pct.forEach(([label, t], i) => {
    txt(`eP${i}`, label, E.x + 22 + i * 24, E.y + 432, 12, A(C.mute, 220));
    anim(`eP${i}`, 'opacity', [[t * d, 0], [t * d + 0.04 * d, 1]], 'ease_out');
  });

  // the finished file card; the viewer folds into it
  const fc = { x: E.x + 22, y: E.y + 474, w: E.w - 44, h: 112 };
  rect('fileShadow', fc.x + 8, fc.y + 10, fc.w, fc.h, { red: 0, green: 0, blue: 0, alpha: 120 });
  rect('file', fc.x, fc.y, fc.w, fc.h, C.panel2, { stroke: A(C.amber, 180), stroke_width: 1 });
  rect('fileBar', fc.x, fc.y, 4, fc.h, A(C.amber, 230));
  txt('fileName', 'gioi-thieu-or.mkv', fc.x + 22, fc.y + 34, 17, A(C.ink, 240));
  txt('fileMeta', '48.2 MB · 00:05:00 · 1080p30', fc.x + 22, fc.y + 62, 12, A(C.mute, 220));
  tagChip('fileOk', 'XUẤT XONG', fc.x + 22, fc.y + 92, C.green, 11);
  ['fileShadow', 'file', 'fileBar', 'fileName', 'fileMeta', 'fileOkBg', 'fileOkT'].forEach((id) => {
    anim(id, 'opacity', [[0.78 * d, 0], [0.82 * d, 1]], 'ease_out');
  });
  anim('vBg', 'scale', [[0.84 * d, 1], [0.94 * d, 0.25]], 'ease_in_out');
  anim('vBg', 'translation_x', [[0.84 * d, 0], [0.94 * d, 740]], 'ease_in_out');
  anim('vBg', 'translation_y', [[0.84 * d, 0], [0.94 * d, 300]], 'ease_in_out');
  anim('vBg', 'opacity', [[0.9 * d, 1], [0.96 * d, 0]], 'ease_in');

  particles('dust', 900, 380, 20, 400, C.amber, d, { size: 1.6, speed: 16 });

  return finish({
    name: 'Xem trước và xuất',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: -20 },
      { t: 0.2 * d, zoom: 1.0 },
      { t: 0.55 * d, zoom: 1.02, pan_x: 20 },
      { t: 0.8 * d, zoom: 1.06, pan_x: 50 },
      { t: d, zoom: 1.12, pan_x: 80 },
    ],
  });
}

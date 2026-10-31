// Opening chapter: a finished shot becomes its layers, the layers become a
// timeline, and the timeline becomes the product. No title card, no feature list.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since, cam } from '../kit.mjs';
import { miniShot, waveformBlock, tagChip } from '../ui.mjs';

const W = 1920;

export function coldOpen(duration) {
  begin();
  const d = duration;
  const T_HUD = 0.24 * d;
  const T_BREAK = 0.46 * d;
  const T_BACK = 0.74 * d;
  const T_LINE = 0.88 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', -400, -500, 1500, 1200, A(C.blue, 10));
  ell('glowB', 1350, 520, 1100, 900, A(C.amber, 8));

  // --------------------------------------------------------------- the shot
  const HORIZON = 690;
  rect('shotSky', 0, 0, W, HORIZON, { red: 15, green: 19, blue: 26, alpha: 255 });
  rect('shotSkyTop', 0, 0, W, 360, { red: 12, green: 16, blue: 22, alpha: 255 });
  ell('shotHalo', 1100, 60, 620, 620, A(C.amber, 15));
  ell('shotSun', 1300, 190, 270, 270, A(C.amber, 238));
  ell('shotSunCore', 1342, 232, 186, 186, A({ red: 251, green: 209, blue: 136, alpha: 255 }, 242));
  rect('shotWater', 0, HORIZON, W, 1080 - HORIZON, { red: 8, green: 10, blue: 13, alpha: 255 });
  for (let i = 0; i < 10; i += 1) {
    const w = 210 - i * 17;
    rect(`shotRefl${i}`, 1434 - w / 2, HORIZON + 30 + i * 30, w, 3 + (i % 3) * 2, A(C.amber, 64 - i * 5));
  }
  const ridges = [
    ['shotM1', -520, 492, 1520, 230, -5, { red: 9, green: 11, blue: 14, alpha: 255 }],
    ['shotM2', 260, 470, 1640, 220, 3, { red: 10, green: 12, blue: 15, alpha: 255 }],
    ['shotM3', 980, 508, 1320, 210, -4, { red: 8, green: 10, blue: 13, alpha: 255 }],
  ];
  ridges.forEach(([id, x, y, w, h, rot, fill]) => {
    rect(id, x, y, w, h, fill, { rotation_degrees: rot });
    rect(`${id}Lit`, x + 14, y + 1, w - 30, 2, A(C.amber, 68), { rotation_degrees: rot });
  });
  for (let i = 0; i < 4; i += 1) rect(`shotHaze${i}`, 980 + i * 32, 575 + i * 24, 760 - i * 120, 2, A(C.amber, 28 - i * 6));
  rect('shotFg', -80, 900, 2120, 320, { red: 4, green: 5, blue: 7, alpha: 255 }, { rotation_degrees: -1.2 });
  rect('shotFgLit', -80, 900, 2120, 2, A(C.amber, 42), { rotation_degrees: -1.2 });
  ell('shotHead', 610, 386, 26, 26, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('shotTorso', 598, 410, 50, 58, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('shotLegA', 602, 466, 10, 26, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('shotLegB', 618, 466, 10, 26, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('shotRimHead', 634, 390, 3, 20, A(C.amber, 190));
  rect('shotRimTorso', 646, 411, 3, 54, A(C.amber, 190));
  particles('shotDust', 1150, 360, 34, 420, C.amber, d, { size: 2, speed: 38, squash: 0.92 });
  for (let i = 0; i < 3; i += 1) {
    rect(`shotBird${i}`, 720 + i * 42, 310 + (i % 2) * 26, 24, 2, A(C.ink, 112), { rotation_degrees: -18 });
    rect(`shotBird${i}b`, 728 + i * 42, 312 + (i % 2) * 26, 24, 2, A(C.ink, 112), { rotation_degrees: 18 });
    anim(`shotBird${i}`, 'translation_x', [[0, 0], [d * 0.55, -260 - i * 30]], 'linear');
    anim(`shotBird${i}b`, 'translation_x', [[0, 0], [d * 0.55, -260 - i * 30]], 'linear');
  }
  anim('shotSun', 'scale', [[0, 1], [d * 0.45, 1.05]], 'ease_in_out');
  anim('shotSunCore', 'scale', [[0, 1], [d * 0.45, 1.04]], 'ease_in_out');
  anim('shotHalo', 'scale', [[0, 1], [d * 0.45, 1.12]], 'ease_in_out');

  // already a film: frame lines, sequence label, progress
  rect('frameT', 0, 44, W, 1, A(C.edge2, 150));
  rect('frameB', 0, 1034, W, 1, A(C.edge2, 150));
  txt('slug', 'SEQUENCE 01 — MỘT BỘ PHIM', 110, 104, 14, A(C.ink, 170));
  txt('tc', '00:00:08:12', W - 120 - textW('00:00:08:12', 14), 104, 14, A(C.mute, 205));
  rect('progBg', 64, 1004, 560, 2, A(C.ink, 42));
  rect('progFill', 64, 1002, 60, 5, A(C.amber, 240));
  anim('progFill', 'width', [[0.1, 60], [d * 0.46, 560]], 'linear');

  // --------------------------------------------------------------- HUD layers
  const hud = [];
  hud.push(txt('hudPic', 'HÌNH ẢNH', 66, 796, 12, A(C.amber, 210)));
  hud.push(txt('hudSnd', 'ÂM THANH', 66, 826, 12, A(C.cyan, 210)));
  hud.push(txt('hudCap', 'PHỤ ĐỀ', 66, 840, 12, A(C.blue, 210)));
  for (let i = 0; i < 44; i += 1) {
    const bh = 5 + ((i * 37) % 46);
    hud.push(rect(`hudW${i}`, 190 + i * 9, 826 - bh / 2, 3, bh, A(C.cyan, 150)));
  }
  hud.push(rect('hudWaveRule', 184, 842, 420, 1, A(C.edge2, 140)));
  hud.push(txt('hudCapText', 'Một video được ghép từ nhiều lớp.', 190, 850, 17, A(C.ink, 215)));
  hud.push(rect('hudTlBg', 720, 760, 700, 54, A(C.bg2, 220), { stroke: A(C.edge2, 170), stroke_width: 1 }));
  [C.amber, C.cyan, C.violet, C.blue].forEach((color, i) => {
    hud.push(rect(`hudClip${i}`, 734, 768 + i * 11, 180 + i * 96, 7, A(color, 190)));
  });
  hud.push(rect('hudPlay', 736, 764, 2, 46, A(C.red, 220)));
  anim('hudPlay', 'translation_x', [[0, 0], [d * 0.46, 600]], 'linear');
  hud.push(txt('hudTc', 'MULTI-TRACK · 30 FPS', 720, 846, 11, A(C.dim, 210)));
  const hudIds = hud.filter(Boolean);
  hudIds.forEach((id) => {
    anim(id, 'opacity', [[T_HUD - 0.35, 0], [T_HUD, 1]], 'ease_out');
    anim(id, 'opacity', [[T_BREAK - 0.3, 1], [T_BREAK - 0.05, 0]], 'ease_in');
  });

  // --------------------------------------------------------------- break apart
  const shotPieces = ['shotSky', 'shotSkyTop', 'shotHalo', 'shotSun', 'shotSunCore', 'shotWater',
    ...Array.from({ length: 10 }, (_, i) => `shotRefl${i}`),
    ...ridges.flatMap(([id]) => [id, `${id}Lit`]),
    ...Array.from({ length: 4 }, (_, i) => `shotHaze${i}`),
    'shotFg', 'shotFgLit', 'shotHead', 'shotTorso', 'shotLegA', 'shotLegB', 'shotRimHead', 'shotRimTorso',
    ...Array.from({ length: 3 }, (_, i) => `shotBird${i}`), ...Array.from({ length: 3 }, (_, i) => `shotBird${i}b`),
    'frameT', 'frameB', 'slug', 'tc', 'progBg', 'progFill'];
  rect('flash', 0, 0, W, 1080, A(C.ink, 0));
  anim('flash', 'opacity', [[T_BREAK - 0.04, 0], [T_BREAK + 0.02, 0.55], [T_BREAK + 0.22, 0]], 'ease_out');
  shotPieces.forEach((id) => anim(id, 'opacity', [[T_BREAK, 1], [T_BREAK + 0.16, 0]], 'ease_in'));

  // four layer cards, laid out off-axis so it is not the template grid
  const cards = [
    ['L1', 296, 196, 'HÌNH ẢNH', C.amber],
    ['L2', 1064, 168, 'ÂM THANH', C.cyan],
    ['L3', 232, 520, 'GIỌNG ĐỌC', C.violet],
    ['L4', 1096, 512, 'PHỤ ĐỀ', C.blue],
  ];
  cards.forEach(([prefix, x, y, name, accent], idx) => {
    const cw = 500, ch = 258;
    rect(`${prefix}Shadow`, x + 12, y + 14, cw, ch, { red: 0, green: 0, blue: 0, alpha: 140 });
    rect(`${prefix}Panel`, x, y, cw, ch, C.panel, { stroke: A(C.edge2, 215), stroke_width: 1 });
    rect(`${prefix}Top`, x, y, cw, 3, accent);
    ell(`${prefix}Dot`, x + 16, y + 18, 7, 7, accent);
    txt(`${prefix}Name`, name, x + 32, y + 22, 13, A(C.mute, 238));
    txt(`${prefix}Num`, `0${idx + 1}`, x + cw - 38, y + 22, 13, A(C.dim, 210));
    const dirX = x < 960 ? -1 : 1;
    const dirY = y < 400 ? -1 : 1;
    [''].forEach(() => {});
    const cardIds = [`${prefix}Shadow`, `${prefix}Panel`, `${prefix}Top`, `${prefix}Dot`, `${prefix}Name`, `${prefix}Num`];
    cardIds.forEach((id) => {
      anim(id, 'translation_x', [[T_BREAK + 0.04, dirX * 170], [T_BREAK + 0.5, 0]], 'ease_out');
      anim(id, 'translation_y', [[T_BREAK + 0.04, dirY * 140], [T_BREAK + 0.5, 0]], 'ease_out');
      anim(id, 'opacity', [[T_BREAK + 0.05, 0], [T_BREAK + 0.3, 1]], 'ease_out');
      anim(id, 'translation_x', [[T_BACK, 0], [T_BACK + 0.4, dirX * 120]], 'ease_in');
      anim(id, 'translation_y', [[T_BACK, 0], [T_BACK + 0.4, dirY * 100]], 'ease_in');
      anim(id, 'opacity', [[T_BACK + 0.26, 1], [T_BACK + 0.44, 0]], 'ease_in');
    });
    if (idx === 0) {
      const m = miniShot('cf', x + 18, y + 44, cw - 36, ch - 62, accent);
      const ids = since(mark() - 0);
      // miniShot nodes were just added; animate the ones in this call by prefix
      const shotIds = ['Sky', 'Sun', 'RidgeA', 'RidgeB', 'Ground', 'Head', 'Body', 'Rim'].map((s) => `cf${s}`);
      shotIds.forEach((id) => {
        anim(id, 'opacity', [[T_BREAK + 0.22, 0], [T_BREAK + 0.5, 1]], 'ease_out');
        anim(id, 'opacity', [[T_BACK + 0.28, 1], [T_BACK + 0.44, 0]], 'ease_in');
      });
      anim('cfSun', 'translation_y', [[T_BREAK, 10], [d * 0.7, -6]], 'ease_in_out');
    }
    if (idx === 1) {
      for (let i = 0; i < 36; i += 1) {
        const bh = 8 + ((i * 43) % 72);
        rect(`${prefix}W${i}`, x + 24 + i * 12.4, y + 128 - bh / 2, 4, bh, A(accent, 200));
        anim(`${prefix}W${i}`, 'scale', [[T_BREAK + 0.24 + i * 0.01, 0.2], [T_BREAK + 0.5 + i * 0.01, 1]], 'ease_out');
        anim(`${prefix}W${i}`, 'opacity', [[T_BREAK + 0.1, 0], [T_BREAK + 0.3, 1], [T_BACK + 0.2, 1], [T_BACK + 0.4, 0]], 'ease_in_out');
      }
      rect(`${prefix}Base`, x + 20, y + 176, cw - 40, 1, A(C.edge2, 180));
      txt(`${prefix}Tag`, 'WAVEFORM · 48 kHz', x + 20, y + 196, 12, A(C.dim, 210));
    }
    if (idx === 2) {
      const cx = x + 150, cy = y + 132;
      for (let i = 0; i < 4; i += 1) {
        const r = 30 + i * 28;
        ell(`${prefix}R${i}`, cx - r, cy - r, r * 2, r * 2, A(accent, 0), { stroke: A(accent, 160 - i * 34), stroke_width: 1.4 });
        anim(`${prefix}R${i}`, 'scale', [[T_BREAK + 0.3 + i * 0.2, 0.6], [T_BREAK + 0.3 + i * 0.2 + 1.4, 1.25]], 'ease_out');
        anim(`${prefix}R${i}`, 'opacity', [[T_BREAK + 0.3 + i * 0.2, 0], [T_BREAK + 0.5 + i * 0.2, 1], [T_BREAK + 1.7 + i * 0.2, 0]], 'ease_out');
      }
      ell(`${prefix}Mic`, cx - 22, cy - 22, 44, 44, A(accent, 235));
      rect(`${prefix}Stem`, cx - 4, cy + 20, 8, 20, A(accent, 210));
      rect(`${prefix}Base`, cx - 16, cy + 38, 32, 3, A(accent, 210));
      for (let i = 0; i < 12; i += 1) {
        const bh = 8 + ((i * 31) % 40);
        rect(`${prefix}B${i}`, x + 290 + i * 14, cy - bh / 2, 5, bh, A(accent, 180));
      }
      txt(`${prefix}Tag`, 'VIENEU-TTS · LOCAL', x + 20, y + 214, 12, A(C.green, 215));
    }
    if (idx === 3) {
      const lines = ['Một video', 'được ghép', 'từ nhiều lớp.'];
      lines.forEach((line, i) => {
        txt(`${prefix}T${i}`, line, x + 36, y + 88 + i * 42, 23, i === 1 ? A(C.ink, 245) : A(C.mute, 220));
        anim(`${prefix}T${i}`, 'opacity', [[T_BREAK + 0.35 + i * 0.12, 0], [T_BREAK + 0.6 + i * 0.12, 1], [T_BACK + 0.2, 1], [T_BACK + 0.36, 0]], 'ease_in_out');
      });
      rect(`${prefix}Bar`, x + 24, y + 72, 3, 116, A(accent, 220));
      rect(`${prefix}BarBg`, x + 24, y + 208, 240, 2, A(C.edge, 200));
      rect(`${prefix}BarFill`, x + 24, y + 206, 20, 5, A(accent, 235));
      anim(`${prefix}BarFill`, 'width', [[T_BREAK + 0.4, 20], [T_BREAK + 1.6, 240]], 'linear');
      txt(`${prefix}Tag`, 'WHISPER · ĐỒNG BỘ', x + 24, y + 226, 12, A(C.dim, 210));
    }
  });

  // Content that has no entrance animation of its own still belongs to its
  // card: hide it until the card opens, then take it away with the card.
  const lateCard = [
    'L2Base', 'L2Tag',
    'L3Base', 'L3Tag', 'L3Mic', 'L3Stem', ...Array.from({ length: 12 }, (_, i) => `L3B${i}`),
    'L4Bar', 'L4BarBg', 'L4BarFill', 'L4Tag',
  ];
  lateCard.forEach((id) => {
    anim(id, 'opacity', [[T_BREAK + 0.18, 0], [T_BREAK + 0.34, 1], [T_BACK + 0.22, 1], [T_BACK + 0.38, 0]], 'ease_in_out');
  });

  // ------------------------------------------------------------- fold to a line
  const lineY = 700;
  rect('foldLine', 620, lineY, 0, 2, A(C.amber, 230));
  anim('foldLine', 'width', [[T_BACK + 0.24, 0], [T_BACK + 0.62, 680]], 'ease_out');
  anim('foldLine', 'translation_x', [[T_BACK + 0.24, 340], [T_BACK + 0.62, 0]], 'ease_out');
  anim('foldLine', 'opacity', [[T_LINE, 1], [d - 0.12, 1], [d, 0]], 'ease_in');
  rect('foldGlow', 620, lineY - 8, 0, 18, A(C.amber, 24));
  anim('foldGlow', 'width', [[T_BACK + 0.3, 0], [T_BACK + 0.7, 680]], 'ease_out');
  anim('foldGlow', 'translation_x', [[T_BACK + 0.3, 340], [T_BACK + 0.7, 0]], 'ease_out');
  anim('foldGlow', 'opacity', [[T_LINE, 0.5], [d, 0]], 'ease_out');
  txt('foldLabel', 'MỘT TIMELINE', 960 - textW('MỘT TIMELINE', 15) / 2, lineY - 34, 15, A(C.mute, 220));
  anim('foldLabel', 'opacity', [[T_BACK + 0.55, 0], [T_BACK + 0.8, 1]], 'ease_out');
  anim('foldLabel', 'opacity', [[T_LINE, 1], [d, 0.6]], 'ease_out');

  return finish({
    name: 'Mở — một bộ phim tách lớp',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02 },
      { t: 0.3 * d, zoom: 1.1, pan_x: 30, pan_y: 10 },
      { t: 0.46 * d, zoom: 1.0, pan_x: 0, pan_y: 0 },
      { t: 0.72 * d, zoom: 1.04, pan_y: -10 },
      { t: 0.88 * d, zoom: 1.08, pan_y: 20 },
      { t: d, zoom: 1.2, pan_y: 40 },
    ],
  });
}

export function layerFlow(duration) {
  begin();
  const d = duration;
  const t0 = 0.06 * d;
  const tBuild = 0.3 * d;
  const tPlay = 0.62 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 1100, 140, 1300, 1000, A(C.amber, 9));
  ell('glowB', 120, 540, 1000, 800, A(C.blue, 9));

  // the fold line from the previous beat becomes the first track
  rect('seed', 460, 700, 1000, 2, A(C.amber, 210));

  // --------------------------------------------------- exploded track planes
  const planes = [
    ['P0', 560, 610, 1200, 44, C.blue, 'C1'],
    ['P1', 520, 662, 1280, 56, C.violet, 'V2'],
    ['P2', 480, 730, 1360, 76, C.amber, 'V1'],
    ['P3', 440, 824, 1440, 56, C.cyan, 'A1'],
  ];
  planes.forEach(([prefix, x, y, w, h, color, name], i) => {
    rect(`${prefix}Bg`, x, y, w, h, A(C.panel2, 245), { stroke: A(C.edge2, 170), stroke_width: 1 });
    rect(`${prefix}Tag`, x, y, 42, h, A(C.bg2, 255));
    txt(`${prefix}TagT`, name, x + 10, y + h / 2 + 4, 11, A(color, 235));
    anim(`${prefix}Bg`, 'width', [[t0 + i * 0.1, 20], [t0 + 0.4 + i * 0.1, w]], 'ease_out');
    anim(`${prefix}Bg`, 'translation_x', [[t0 + i * 0.1, (w - 20) / 2], [t0 + 0.4 + i * 0.1, 0]], 'ease_out');
    anim(`${prefix}Tag`, 'opacity', [[t0 + i * 0.1, 0], [t0 + 0.3 + i * 0.1, 1]], 'ease_out');
  });

  // clips drop into each plane and snap
  const drops = [
    ['c0', 0, 620, 250, C.blue, 0.34],
    ['c1', 0, 890, 320, C.blue, 0.42],
    ['c2', 1, 560, 300, C.violet, 0.48],
    ['c3', 2, 520, 380, C.amber, 0.52],
    ['c4', 2, 920, 300, C.teal, 0.6],
    ['c5', 2, 1240, 360, C.amber, 0.68],
    ['c6', 3, 500, 620, C.cyan, 0.74],
    ['c7', 3, 1140, 420, C.cyan, 0.8],
  ];
  drops.forEach(([prefix, lane, x, w, color, t], i) => {
    const [, px, py, pw, ph] = planes[lane];
    const y = py + 4;
    rect(`${prefix}`, x, y, w, ph - 8, A(color, 190));
    rect(`${prefix}Cap`, x, y, w, 2, A(color, 245));
    if (lane === 2) for (let k = 0; k < Math.floor(w / 28); k += 1) rect(`${prefix}W${k}`, x + 8 + k * 28, y + ph / 2 - 7, 3, 14, A(C.bg, 95));
    anim(prefix, 'translation_y', [[t * d, -180], [(t + 0.35) * d, 0]], 'ease_out');
    anim(`${prefix}Cap`, 'translation_y', [[t * d, -180], [(t + 0.35) * d, 0]], 'ease_out');
    anim(prefix, 'opacity', [[t * d, 0], [t * d + 0.08, 1]], 'ease_out');
    anim(`${prefix}Cap`, 'opacity', [[t * d, 0], [t * d + 0.08, 1]], 'ease_out');
    anim(prefix, 'scale', [[t * d, 0.96], [(t + 0.4) * d, 1]], 'ease_out');
  });

  // gaps close after the last drop
  const shift = (prefix, from, dx) => {
    anim(prefix, 'translation_x', [[from, 0], [from + 0.4 * d, dx]], 'ease_in_out');
  };
  shift('c4', 0.84 * d, -80);
  shift('c4Cap', 0.84 * d, -80);

  // ---------------------------------------------------------- flatten to 2D
  const F = { x: 300, y: 660, w: 1320, h: 220 };
  rect('flatBg', F.x, F.y, F.w, F.h, A(C.panel, 250), { stroke: A(C.edge2, 190), stroke_width: 1 });
  anim('flatBg', 'opacity', [[tPlay - 0.4, 0], [tPlay - 0.1, 1]], 'ease_out');

  // viewer appears above; camera dives into it at the end
  const V = { x: 560, y: 174, w: 800, h: 420 };
  rect('viewCase', V.x - 10, V.y - 10, V.w + 20, V.h + 20, C.panel);
  anim('viewCase', 'opacity', [[tPlay - 0.2, 0], [tPlay + 0.1, 1]], 'ease_out');
  const shot = miniShot('v', V.x, V.y, V.w, V.h, C.amber);
  const vIds = ['vSky', 'vSun', 'vRidgeA', 'vRidgeB', 'vGround', 'vHead', 'vBody', 'vRim'];
  vIds.forEach((id) => anim(id, 'opacity', [[tPlay, 0], [tPlay + 0.25, 1]], 'ease_out'));
  anim('vSun', 'translation_y', [[tPlay, 40], [d, -6]], 'ease_in_out');
  anim('vRidgeA', 'translation_x', [[tPlay, -40], [d, 10]], 'ease_in_out');
  anim('vRidgeB', 'translation_x', [[tPlay, 50], [d, -8]], 'ease_in_out');
  anim('vRim', 'opacity', [[tPlay, 0.3], [tPlay + 0.8, 1]], 'ease_out');

  // playhead sweeps the flat timeline, clips light as it passes
  const playX = F.x + 18;
  rect('play', playX, F.y + 6, 2, F.h - 12, A(C.red, 225));
  rect('playHead', playX - 8, F.y + 6, 18, 6, A(C.red, 235));
  anim('play', 'translation_x', [[tPlay, 0], [d, F.w - 70]], 'linear');
  anim('playHead', 'translation_x', [[tPlay, 0], [d, F.w - 70]], 'linear');

  // a second, deeper camera: the "3D" grid behind the planes
  for (let i = 0; i < 9; i += 1) {
    rect(`gridV${i}`, 480 + i * 140, 600, 1, 300, A(C.edge, 26));
  }
  for (let i = 0; i < 5; i += 1) {
    rect(`gridH${i}`, 380, 620 + i * 58, 1360, 1, A(C.edge, 22));
  }
  const gridIds = [...Array.from({ length: 9 }, (_, i) => `gridV${i}`), ...Array.from({ length: 5 }, (_, i) => `gridH${i}`)];
  gridIds.forEach((id) => anim(id, 'opacity', [[0, 0], [0.2 * d, 0.6], [0.6 * d, 0.2], [0.9 * d, 0]], 'ease_in_out'));

  return finish({
    name: 'Bốn lớp thành một timeline',
    duration: d,
    camera: [
      { t: 0, zoom: 1.06, pan_y: 30 },
      { t: 0.3 * d, zoom: 1.02, pan_y: 10 },
      { t: 0.55 * d, zoom: 1.0 },
      { t: 0.7 * d, zoom: 1.03, pan_y: -20 },
      { t: 0.86 * d, zoom: 1.1, pan_y: 30 },
      { t: d, zoom: 1.30, pan_y: 30 },
    ],
  });
}

export function reveal(duration) {
  begin();
  const d = duration;
  const tMark = 0.18 * d;
  const tWord = 0.42 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 720, 240, 1300, 1100, A(C.amber, 10));
  ell('glowB', 1050, 360, 1000, 900, A(C.violet, 8));
  for (let i = 0; i < 10; i += 1) {
    rect(`gx${i}`, 240 + i * 160, 0, 1, 1080, A(C.edge, 20));
    anim(`gx${i}`, 'translation_x', [[0, 0], [d, 14]], 'linear');
  }

  // the timeline bars from the previous beat fly to the centre and become the mark
  const bars = [];
  for (let i = 0; i < 12; i += 1) {
    const x = 360 + i * 100, y = 840;
    bars.push(rect(`bar${i}`, x, y, 84, 8, A(i % 3 === 0 ? C.amber : i % 3 === 1 ? C.cyan : C.violet, 220)));
    anim(`bar${i}`, 'translation_x', [[0, 0], [tMark, 960 - (x + 42)]], 'ease_in_out');
    anim(`bar${i}`, 'translation_y', [[0, 0], [tMark, 470 - y]], 'ease_in_out');
    anim(`bar${i}`, 'width', [[tMark, 84], [tMark + 0.3 * d, 18]], 'ease_in');
    anim(`bar${i}`, 'height', [[tMark, 8], [tMark + 0.3 * d, 18]], 'ease_in');
    anim(`bar${i}`, 'rotation', [[tMark, 0], [tMark + 0.4 * d, 45]], 'ease_in');
    anim(`bar${i}`, 'opacity', [[tMark + 0.28 * d, 1], [tMark + 0.36 * d, 0]], 'ease_in');
  }

  // mark
  const cx = 960, cy = 430;
  ell('markRing', cx - 58, cy - 58, 116, 116, A(C.amber, 0), { stroke: A(C.amber, 235), stroke_width: 3 });
  rect('markDia', cx - 26, cy - 26, 52, 52, A(C.amber, 225), { rotation_degrees: 45 });
  rect('markDiaIn', cx - 12, cy - 12, 24, 24, A(C.bg, 255), { rotation_degrees: 45 });
  rect('markL', cx - 92, cy - 2, 24, 4, A(C.amber, 210));
  rect('markR', cx + 68, cy - 2, 24, 4, A(C.amber, 210));
  const markIds = ['markRing', 'markDia', 'markDiaIn', 'markL', 'markR'];
  markIds.forEach((id, i) => {
    anim(id, 'opacity', [[tMark + 0.24 * d + i * 0.02 * d, 0], [tMark + 0.36 * d + i * 0.02 * d, 1]], 'ease_out');
    anim(id, 'scale', [[tMark + 0.24 * d, 0.4], [tMark + 0.46 * d, 1]], 'ease_out');
  });
  anim('markRing', 'rotation', [[tMark, -40], [tMark + 0.8 * d, 0]], 'ease_out');

  // wordmark
  const word = 'OpenCut Reinforced';
  const wsize = 86;
  const total = textW(word, wsize);
  txt('word', word, 960 - total / 2, 582, wsize, A(C.ink, 252));
  anim('word', 'opacity', [[tWord, 0], [tWord + 0.14 * d, 1]], 'ease_out');
  anim('word', 'translation_y', [[tWord, 26], [tWord + 0.2 * d, 0]], 'ease_out');
  rect('wordUnder', 960 - 170, 640, 0, 3, A(C.amber, 215));
  anim('wordUnder', 'width', [[tWord + 0.12 * d, 0], [tWord + 0.3 * d, 340]], 'ease_out');
  anim('wordUnder', 'translation_x', [[tWord + 0.12 * d, 170], [tWord + 0.3 * d, 0]], 'ease_out');

  const tag = 'TRÌNH BIÊN TẬP VIDEO · MIỄN PHÍ · MÃ NGUỒN MỞ';
  txt('tag', tag, 960 - textW(tag, 19) / 2, 694, 19, A(C.mute, 238));
  anim('tag', 'opacity', [[tWord + 0.24 * d, 0], [tWord + 0.36 * d, 1]], 'ease_in_out');

  // licence line, small and honest
  tagChip('lic', 'MIT', 690, 760, C.green, 14);
  tagChip('loc', 'LOCAL-FIRST', 790, 760, C.cyan, 14);
  tagChip('har', 'ACP HARNESS', 960, 760, C.violet, 14);
  ['licBg', 'licT', 'locBg', 'locT', 'harBg', 'harT'].forEach((id, i) => {
    anim(id, 'opacity', [[tWord + 0.3 * d + i * 0.015 * d, 0], [tWord + 0.4 * d + i * 0.015 * d, 1]], 'ease_out');
  });

  // underline shoots out, handing off to the editor
  rect('handoff', 960, 850, 0, 2, A(C.amber, 220));
  anim('handoff', 'width', [[0.88 * d, 0], [d, 1680]], 'ease_in');
  anim('handoff', 'translation_x', [[0.88 * d, -840], [d, 0]], 'ease_in');

  particles('dust', 960, 430, 26, 480, C.mute, d, { size: 1.8, speed: 20 });

  return finish({
    name: 'Lộ diện — OpenCut Reinforced',
    duration: d,
    camera: [
      { t: 0, zoom: 1.05, pan_y: 30 },
      { t: 0.25 * d, zoom: 1.08, pan_y: 10 },
      { t: 0.5 * d, zoom: 1.0 },
      { t: 0.8 * d, zoom: 1.02 },
      { t: d, zoom: 1.1, pan_y: -10 },
    ],
  });
}

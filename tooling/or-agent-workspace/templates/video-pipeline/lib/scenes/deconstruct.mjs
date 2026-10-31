// S1 — 0–5 s. A finished shot holds the full frame, then the picture breaks into
// the four layers it is made of — picture, sound, voice, captions — and closes
// back into one film that folds into a line.
import { begin, finish, cornerMarks, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since, captions } from '../kit.mjs';

const CARD_W = 520, CARD_H = 306;
const CARD = {
  image: { x: 214, y: 150, accent: C.amber },
  audio: { x: 214, y: 592, accent: C.cyan },
  voice: { x: 1186, y: 150, accent: C.violet },
  captions: { x: 1186, y: 592, accent: C.blue },
};

const T_OPEN = 1.30;   // the shot breaks
const T_LIVE = 1.62;   // layer contents wake
const T_BACK = 3.74;   // layers return
const T_FILM = 4.02;   // final shot
const T_FOLD = 4.42;   // folds to a line

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', -500, -420, 1300, 1100, A(C.blue, 9));
  ell('glowB', 1180, 520, 1100, 900, A(C.violet, 8));

  // ------------------------------------------------------------- the finished shot
  // ------------------------------------------------------------- the finished shot
  const HORIZON = 640;
  const DARK = { red: 5, green: 6, blue: 8, alpha: 255 };
  rect('shotSky', 0, 0, 1920, HORIZON, { red: 14, green: 18, blue: 24, alpha: 255 });
  rect('shotSkyTop', 0, 0, 1920, 330, { red: 13, green: 17, blue: 22, alpha: 255 });
  ell('shotHalo', 1070, 40, 540, 540, A(C.amber, 14));
  ell('shotSun', 1210, 162, 250, 250, A(C.amber, 238));
  ell('shotSunCore', 1250, 202, 170, 170, A({ red: 251, green: 208, blue: 134, alpha: 255 }, 242));
  rect('shotWater', 0, HORIZON, 1920, 1080 - HORIZON, { red: 8, green: 10, blue: 13, alpha: 255 });
  for (let i = 0; i < 9; i += 1) {
    const w = 190 - i * 16;
    rect(`shotRefl${i}`, 1360 - w / 2, HORIZON + 28 + i * 34, w, 3 + (i % 3) * 2, A(C.amber, 66 - i * 5));
  }
  const ridges = [
    ['shotM1', -500, 468, 1500, 220, -5, { red: 9, green: 11, blue: 14, alpha: 255 }],
    ['shotM2', 260, 450, 1600, 210, 3, { red: 10, green: 12, blue: 15, alpha: 255 }],
    ['shotM3', 900, 486, 1400, 220, -4, { red: 9, green: 11, blue: 14, alpha: 255 }],
  ];
  ridges.forEach(([id, x, y, w, h, rot, fill]) => {
    rect(id, x, y, w, h, fill, { rotation_degrees: rot });
    rect(`${id}Lit`, x + 10, y + 1, w - 20, 2, A(C.amber, 62), { rotation_degrees: rot });
  });
  for (let i = 0; i < 4; i += 1) rect(`shotHaze${i}`, 900 + i * 30, 552 + i * 26, 720 - i * 110, 2, A(C.amber, 30 - i * 6));
  // foreground ridge and the figure standing on it
  rect('shotFg', -80, 872, 2100, 340, { red: 4, green: 5, blue: 7, alpha: 255 }, { rotation_degrees: -1.4 });
  rect('shotFgLit', -80, 872, 2100, 2, A(C.amber, 40), { rotation_degrees: -1.4 });
  ell('shotHead', 794, 366, 26, 26, DARK);
  rect('shotTorso', 782, 390, 50, 56, DARK);
  rect('shotLegA', 786, 444, 10, 26, DARK);
  rect('shotLegB', 802, 444, 10, 26, DARK);
    rect('shotRimHead', 818, 370, 3, 20, A(C.amber, 185));
  rect('shotRimTorso', 830, 391, 3, 52, A(C.amber, 185));
  rect('shotRimLeg', 810, 446, 3, 22, A(C.amber, 120));
  particles('shotStars', 1500, 200, 26, 300, C.ink, d, { size: 1.6, speed: 10, squash: 0.6 });
  for (let i = 0; i < 3; i += 1) {
    rect(`shotBird${i}`, 880 + i * 40, 300 + (i % 2) * 24, 24, 2, A(C.ink, 110), { rotation_degrees: -18 });
    rect(`shotBird${i}b`, 888 + i * 40, 302 + (i % 2) * 24, 24, 2, A(C.ink, 110), { rotation_degrees: 18 });
    anim(`shotBird${i}`, 'translation_x', [[0, 0], [4.4, -240 - i * 28]], 'linear');
    anim(`shotBird${i}b`, 'translation_x', [[0, 0], [4.4, -240 - i * 28]], 'linear');
    anim(`shotBird${i}`, 'translation_y', [[0, 0], [2.2, -14], [4.4, 4]], 'ease_in_out');
    anim(`shotBird${i}b`, 'translation_y', [[0, 0], [2.2, -14], [4.4, 4]], 'ease_in_out');
  }
  particles('shotDust', 1400, 400, 34, 380, C.amber, d, { size: 2, speed: 40, squash: 0.9 });
  rect('shotStreak', -200, -140, 7, 1500, A(C.ink, 12), { rotation_degrees: 24 });
  anim('shotStreak', 'translation_x', [[0, 0], [1.4, 520]], 'linear');
  rect('shotStreak2', -60, -140, 2, 1500, A(C.ink, 8), { rotation_degrees: 24 });
  anim('shotStreak2', 'translation_x', [[0, 0], [1.4, 420]], 'linear');

  anim('shotSun', 'scale', [[0, 1], [1.3, 1.06], [4.2, 1.0]], 'ease_in_out');
  anim('shotSunCore', 'scale', [[0, 1], [1.3, 1.04], [4.2, 1.02]], 'ease_in_out');
  anim('shotHalo', 'scale', [[0, 1], [1.3, 1.12], [4.2, 1.02]], 'ease_in_out');
  ['shotHead', 'shotTorso', 'shotLegA', 'shotLegB', 'shotRimHead', 'shotRimTorso', 'shotRimLeg'].forEach((id) => {
    anim(id, 'translation_x', [[0, 0], [1.3, -10], [4.2, 5]], 'ease_in_out');
  });

  rect('shotFrameT', 0, 46, 1920, 1, A(C.edge2, 150));
  rect('shotFrameB', 0, 1034, 1920, 1, A(C.edge2, 150));
  txt('shotSlug', 'SEQUENCE 01 — OPEN CUT', 64, 104, 15, A(C.ink, 165));
  txt('shotTc', '00:00:59', 1920 - 64 - textW('00:00:59', 15), 104, 15, A(C.mute, 200));
  rect('shotProg', 64, 1004, 640, 2, A(C.ink, 40));
  rect('shotProgFill', 64, 1002, 70, 5, A(C.amber, 235));
  anim('shotProgFill', 'width', [[0.1, 70], [1.3, 640]], 'linear');

  const shotPieces = ['shotSky', 'shotSkyTop', 'shotHalo', 'shotSun', 'shotSunCore', 'shotWater',
    ...Array.from({ length: 9 }, (_, i) => `shotRefl${i}`),
    ...ridges.flatMap(([id]) => [id, `${id}Lit`]),
    ...Array.from({ length: 4 }, (_, i) => `shotHaze${i}`),
    'shotFg', 'shotFgLit', 'shotHead', 'shotTorso', 'shotLegA', 'shotLegB', 'shotRimHead', 'shotRimTorso', 'shotRimLeg',
    ...Array.from({ length: 3 }, (_, i) => `shotBird${i}`), ...Array.from({ length: 3 }, (_, i) => `shotBird${i}b`),
    'shotStreak', 'shotStreak2', 'shotSlug', 'shotTc', 'shotProg', 'shotProgFill', 'shotFrameT', 'shotFrameB',
    ...Array.from({ length: 26 }, (_, i) => `shotStars${i}`)];
  shotPieces.forEach((id) => anim(id, 'opacity', [[0, 0], [0.3, 1]], 'ease_out'));

  rect('flash', 0, 0, 1920, 1080, A(C.ink, 0));
  anim('flash', 'opacity', [[T_OPEN - 0.04, 0], [T_OPEN + 0.02, 0.62], [T_OPEN + 0.24, 0]], 'ease_out');
  shotPieces.forEach((id) => anim(id, 'opacity', [[T_OPEN, 1], [T_OPEN + 0.16, 0]], 'ease_in'));

  // ------------------------------------------------------------ the four layers
  function layerFrame(prefix, box, name) {
    const { x, y, accent } = box;
    const m = mark();
    rect(`${prefix}Shadow`, x + 12, y + 14, CARD_W, CARD_H, { red: 0, green: 0, blue: 0, alpha: 130 });
    rect(`${prefix}Panel`, x, y, CARD_W, CARD_H, C.panel, { stroke: A(C.edge2, 215), stroke_width: 1 });
    rect(`${prefix}Top`, x, y, CARD_W, 3, accent);
    rect(`${prefix}HeadBg`, x, y + 3, CARD_W, 32, C.panel2);
    rect(`${prefix}Dot`, x + 14, y + 17, 7, 7, accent);
    txt(`${prefix}Name`, name, x + 30, y + 20, 14, A(C.mute, 240));
    const ids = since(m);
    const dirX = x < 960 ? -1 : 1;
    const dirY = y < 540 ? -1 : 1;
    ids.forEach((id) => {
      anim(id, 'translation_x', [[T_OPEN + 0.02, dirX * 200], [T_OPEN + 0.44, 0]], 'ease_out');
      anim(id, 'translation_y', [[T_OPEN + 0.02, dirY * 160], [T_OPEN + 0.44, 0]], 'ease_out');
      anim(id, 'opacity', [[T_OPEN + 0.04, 0], [T_OPEN + 0.24, 1]], 'ease_out');
      anim(id, 'translation_x', [[T_BACK, 0], [T_BACK + 0.46, dirX * 170]], 'ease_in');
      anim(id, 'translation_y', [[T_BACK, 0], [T_BACK + 0.46, dirY * 130]], 'ease_in');
      anim(id, 'opacity', [[T_BACK + 0.24, 1], [T_BACK + 0.48, 0]], 'ease_in');
    });
  }
  layerFrame('im', CARD.image, 'HÌNH ẢNH');
  layerFrame('au', CARD.audio, 'ÂM THANH');
  layerFrame('vo', CARD.voice, 'GIỌNG ĐỌC');
  layerFrame('ca', CARD.captions, 'PHỤ ĐỀ');

  // ------------------------------------------------------------------ picture
  {
    const { x, y } = CARD.image;
    const m = mark();
    rect('imSky', x + 2, y + 36, CARD_W - 4, 160, { red: 18, green: 23, blue: 31, alpha: 255 });
    ell('imSun', x + 326, y + 58, 104, 104, A(C.amber, 232));
    ell('imSunCore', x + 346, y + 78, 64, 64, A({ red: 251, green: 210, blue: 140, alpha: 255 }, 235));
    ell('imHillA', x + 0, y + 128, 320, 150, { red: 10, green: 12, blue: 16, alpha: 255 });
    ell('imHillB', x + 232, y + 150, 280, 130, { red: 12, green: 15, blue: 20, alpha: 255 });
    rect('imShore', x + 2, y + 210, CARD_W - 4, 2, A(C.amber, 110));
    rect('imWater', x + 2, y + 212, CARD_W - 4, 62, { red: 9, green: 11, blue: 14, alpha: 255 });
    for (let i = 0; i < 5; i += 1) rect(`imRefl${i}`, x + 328 - i * 8, y + 218 + i * 11, 84 + i * 10, 3, A(C.amber, 58 - i * 8));
    ell('imHead', x + 132, y + 158, 22, 22, { red: 5, green: 6, blue: 8, alpha: 255 });
    rect('imBody', x + 124, y + 180, 38, 32, { red: 5, green: 6, blue: 8, alpha: 255 });
    rect('imRim', x + 160, y + 180, 3, 30, A(C.amber, 170));
    rect('imStreak', x + 70, y + 40, 3, 230, A(C.ink, 60), { rotation_degrees: 26 });
    anim('imStreak', 'translation_x', [[T_LIVE, 0], [T_LIVE + 1.7, 260]], 'ease_in_out');
    ell('imSunDot', x + 466, y + 60, 24, 24, A(C.amber, 40), { stroke: A(C.amber, 190), stroke_width: 1 });
    const ids = since(m);
    ids.forEach((id) => anim(id, 'opacity', [[0, 0], [T_LIVE, 0], [T_LIVE + 0.26, 1]], 'ease_out'));
    ids.forEach((id) => anim(id, 'opacity', [[T_BACK - 0.3, 1], [T_BACK - 0.12, 0]], 'ease_in'));
  }

  // -------------------------------------------------------------------- sound
  {
    const { x, y, accent } = CARD.audio;
    const m = mark();
    rect('auLane1', x + 18, y + 52, 250, 26, A(accent, 200));
    rect('auLane2', x + 18, y + 86, 380, 26, A(C.blue, 180));
    rect('auLane3', x + 18, y + 120, 200, 26, A(C.violet, 170));
    rect('auPlay', x + 36, y + 46, 2, 110, A(C.ink, 220));
    anim('auPlay', 'translation_x', [[T_LIVE, 0], [T_LIVE + 1.9, 420]], 'linear');
    for (let i = 0; i < 46; i += 1) {
      const base = 10 + ((i * 53 + 9) % 66);
      const id = `auW${i}`;
      rect(id, x + 26 + i * 10.2, y + 196, 4, base * 0.72, A(accent, 205));
      const keys = [];
      for (let k = 0; k <= 6; k += 1) {
        const t = T_LIVE + (1.9 * k) / 6;
        const wave = 0.3 + 0.7 * Math.abs(Math.sin((k + 1) * (0.9 + ((i * 7919) % 97) / 60) + i * 0.5));
        keys.push([t, Math.max(4, base * 0.72 * wave)]);
      }
      anim(id, 'height', keys, 'ease_in_out');
    }
    rect('auBase', x + 26, y + 246, 460, 1, A(C.edge2, 170));
    txt('auTag', 'HÌNH ẢNH + TIẾNG ĐỘNG', x + 18, y + 268, 14, A(C.mute, 210));
    const ids = since(m);
    ids.forEach((id) => anim(id, 'opacity', [[0, 0], [T_LIVE + 0.08, 0], [T_LIVE + 0.36, 1]], 'ease_out'));
    ids.forEach((id) => anim(id, 'opacity', [[T_BACK - 0.28, 1], [T_BACK - 0.1, 0]], 'ease_in'));
  }

  // -------------------------------------------------------------------- voice
  {
    const { x, y, accent } = CARD.voice;
    const m = mark();
    const cx = x + 170, cy = y + 172;
    for (let i = 0; i < 4; i += 1) {
      const r = 34 + i * 30;
      ell(`voR${i}`, cx - r, cy - r, r * 2, r * 2, A(accent, 0), { stroke: A(accent, 170 - i * 32), stroke_width: 1.6 });
      anim(`voR${i}`, 'scale', [[T_LIVE + i * 0.24, 0.66], [T_LIVE + i * 0.24 + 1.4, 1.22]], 'ease_out');
      anim(`voR${i}`, 'opacity', [[T_LIVE + i * 0.24, 0], [T_LIVE + i * 0.24 + 0.24, 1], [T_LIVE + i * 0.24 + 1.4, 0]], 'ease_out');
    }
    ell('voMic', cx - 26, cy - 26, 52, 52, A(accent, 235));
    rect('voMicBar', cx - 5, cy + 22, 10, 22, A(accent, 205));
    rect('voMicBase', cx - 20, cy + 44, 40, 4, A(accent, 205));
    for (let i = 0; i < 16; i += 1) {
      const h = 8 + ((i * 31) % 42);
      rect(`voBar${i}`, x + 300 + i * 11, cy - h / 2, 4, h, A(accent, 170));
      anim(`voBar${i}`, 'scale', [[T_LIVE + i * 0.06, 0.3], [T_LIVE + i * 0.06 + 0.4, 1]], 'ease_out');
    }
    txt('voTag', 'VIENEU-TTS', x + 18, y + 268, 15, A(C.mute, 225));
    rect('voTagDot', x + 140, y + 262, 7, 7, A(C.green, 235));
    txt('voLocal', 'LOCAL', x + 154, y + 268, 15, A(C.green, 215));
    const ids = since(m);
    ids.forEach((id) => anim(id, 'opacity', [[0, 0], [T_LIVE + 0.16, 0], [T_LIVE + 0.44, 1]], 'ease_out'));
    ids.forEach((id) => anim(id, 'opacity', [[T_BACK - 0.32, 1], [T_BACK - 0.14, 0]], 'ease_in'));
  }

  // ----------------------------------------------------------------- captions
  {
    const { x, y, accent } = CARD.captions;
    const m = mark();
    const lines = ['Một video hoàn chỉnh,', 'rồi tách ra', 'thành bốn lớp.'];
    lines.forEach((line, i) => {
      txt(`caT${i}`, line, x + 46, y + 92 + i * 50, 25, i === 1 ? A(C.ink, 255) : A(C.mute, 225));
      anim(`caT${i}`, 'opacity', [[T_LIVE + 0.12 + i * 0.12, 0], [T_LIVE + 0.42 + i * 0.12, 1]], 'ease_out');
    });
    rect('caMark', x + 22, y + 76, 4, 132, A(accent, 235));
    rect('caBar', x + 22, y + 250, 260, 3, A(C.panel3, 255));
    rect('caBarFill', x + 22, y + 248, 4, 7, A(accent, 235));
    anim('caBarFill', 'width', [[T_LIVE + 0.4, 4], [T_LIVE + 1.8, 396]], 'linear');
    txt('caTag', 'WHISPER · ĐỒNG BỘ', x + 22, y + 274, 14, A(C.mute, 205));
    const ids = since(m);
    ids.forEach((id) => anim(id, 'opacity', [[0, 0], [T_LIVE + 0.26, 0], [T_LIVE + 0.56, 1]], 'ease_out'));
    ids.forEach((id) => anim(id, 'opacity', [[T_BACK - 0.34, 1], [T_BACK - 0.16, 0]], 'ease_in'));
  }

  const labels = [
    ['imLab', '01 — HÌNH ẢNH', CARD.image.x, CARD.image.y + CARD_H + 30, C.amber],
    ['auLab', '02 — ÂM THANH', CARD.audio.x, CARD.audio.y + CARD_H + 30, C.cyan],
    ['voLab', '03 — GIỌNG ĐỌC', CARD.voice.x, CARD.voice.y + CARD_H + 30, C.violet],
    ['caLab', '04 — PHỤ ĐỀ', CARD.captions.x, CARD.captions.y + CARD_H + 30, C.blue],
  ];
  labels.forEach(([id, text, x, y, accent]) => {
    txt(id, text, x, y, 16, A(accent, 225));
    anim(id, 'opacity', [[T_OPEN + 0.42, 0], [T_OPEN + 0.66, 1]], 'ease_out');
    anim(id, 'opacity', [[T_BACK - 0.4, 1], [T_BACK - 0.22, 0]], 'ease_in');
  });

  const sx = 760, sy = 500;
  rect('coreBg', sx, sy, 400, 58, A(C.bg2, 246), { stroke: A(C.edge2, 200), stroke_width: 1 });
  rect('coreLane1', sx + 12, sy + 12, 150, 12, A(C.amber, 215));
  rect('coreLane2', sx + 168, sy + 12, 96, 12, A(C.teal, 195));
  rect('coreLane3', sx + 12, sy + 30, 300, 5, A(C.cyan, 155));
  rect('coreLane4', sx + 12, sy + 42, 230, 5, A(C.violet, 155));
  rect('corePlay', sx + 12, sy + 8, 2, 44, A(C.ink, 215));
  anim('corePlay', 'translation_x', [[T_LIVE, 0], [T_BACK - 0.1, 376]], 'linear');
  ['coreBg', 'coreLane1', 'coreLane2', 'coreLane3', 'coreLane4', 'corePlay'].forEach((id) => {
    anim(id, 'opacity', [[T_OPEN + 0.5, 0], [T_OPEN + 0.8, 1]], 'ease_out');
    anim(id, 'opacity', [[T_BACK, 1], [T_BACK + 0.2, 0]], 'ease_in');
  });
  const wires = [
    ['wireTL', CARD.image.x + CARD_W, 420, 760 - (CARD.image.x + CARD_W) - 8],
    ['wireTR', 1160, 420, CARD.voice.x - 1160 - 8],
    ['wireBL', CARD.audio.x + CARD_W, 668, 760 - (CARD.audio.x + CARD_W) - 8],
    ['wireBR', 1160, 668, CARD.captions.x - 1160 - 8],
  ];
  wires.forEach(([id, x, y, w]) => {
    rect(id, x, y, w, 1, A(C.edge2, 160));
    anim(id, 'opacity', [[T_OPEN + 0.5, 0], [T_OPEN + 0.85, 1]], 'ease_out');
    anim(id, 'opacity', [[T_BACK, 1], [T_BACK + 0.2, 0]], 'ease_in');
  });

  shotPieces.forEach((id) => anim(id, 'opacity', [[T_FILM, 0], [T_FILM + 0.26, 1]], 'ease_in'));
  anim('flash', 'opacity', [[T_FILM, 0], [T_FILM + 0.02, 0.4], [T_FILM + 0.2, 0]], 'ease_out');
  for (let i = 0; i < 34; i += 1) anim(`shotDust${i}`, 'opacity', [[T_OPEN, 0.6], [T_OPEN + 0.2, 0], [T_FILM, 0], [T_FILM + 0.3, 0.5]], 'ease_in_out');

  anim('shotSky', 'height', [[T_FOLD, 640], [T_FOLD + 0.44, 4]], 'ease_in_out');
  anim('shotSky', 'translation_y', [[T_FOLD, 0], [T_FOLD + 0.44, 320]], 'ease_in_out');
  anim('shotSkyTop', 'height', [[T_FOLD, 330], [T_FOLD + 0.44, 4]], 'ease_in_out');
  anim('shotSkyTop', 'translation_y', [[T_FOLD, 0], [T_FOLD + 0.44, 330]], 'ease_in_out');
  anim('shotWater', 'height', [[T_FOLD, 440], [T_FOLD + 0.44, 4]], 'ease_in_out');
  anim('shotWater', 'translation_y', [[T_FOLD, 0], [T_FOLD + 0.44, -220]], 'ease_in_out');
  anim('shotFrameT', 'translation_y', [[T_FOLD, 0], [T_FOLD + 0.44, -46]], 'ease_in_out');
  anim('shotFrameB', 'translation_y', [[T_FOLD, 0], [T_FOLD + 0.44, -492]], 'ease_in_out');
  shotPieces.filter((id) => !['shotSky', 'shotSkyTop', 'shotWater', 'shotFrameT', 'shotFrameB'].includes(id)).forEach((id) => {
    anim(id, 'opacity', [[T_FOLD, id.startsWith('shotRefl') ? 0.7 : 1], [T_FOLD + 0.16, 0]], 'ease_in');
  });

  return finish({
    name: opts.name ?? 'Deconstruct',
    duration: d,
    camera: [
      { t: 0, zoom: 1.0 },
      { t: T_OPEN, zoom: 1.09, pan_x: 10 },
      { t: 2.3, zoom: 1.01, pan_x: -4 },
      { t: 3.7, zoom: 1.03, pan_x: 8 },
      { t: 4.4, zoom: 1.08 },
      { t: 5, zoom: 1.2 },
    ],
  });
}

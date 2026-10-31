// MotionScene chapter: code becomes geometry, geometry becomes a shot, and the
// shot becomes an ordinary clip beside real footage.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';
import { codePane, tagChip, miniShot } from '../ui.mjs';

export function motionCode(duration) {
  begin();
  const d = duration;
  const tCode = 0.02 * d;
  const tFly = 0.32 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 1250, 200, 1200, 900, A(C.cyan, 10));
  ell('glowB', 200, 620, 900, 800, A(C.violet, 9));
  for (let i = 0; i < 11; i += 1) rect(`gx${i}`, 100 + i * 172, 0, 1, 1080, A(C.edge, 20));
  for (let i = 0; i < 7; i += 1) rect(`gy${i}`, 0, 100 + i * 130, 1920, 1, A(C.edge, 20));

  // perspective floor: lines converge on a vanishing point
  const VP = { x: 1250, y: 470 };
  for (let i = 0; i < 10; i += 1) {
    const x = 780 + i * 120;
    rect(`floorV${i}`, x, 1080, 1, 700, A(C.edge2, 30), { rotation_degrees: (x - VP.x) * 0.02 });
  }
  for (let i = 0; i < 6; i += 1) {
    rect(`floorH${i}`, 700, 560 + i * 70, 1220 - i * 60, 1, A(C.edge2, 28));
  }

  const lines = [
    ['scene.begin({ size: [1920, 1080] })', C.violet],
    ['scene.sun({ x: 1180, y: 250, r: 96 })', C.ink],
    ['scene.ridge({ y: 560, layers: 3 })', C.ink],
    ['scene.figure({ x: 620 })', C.ink],
    ['scene.birds({ count: 3 })', C.ink],
    ['scene.particles({ count: 40 })', C.ink],
    ['scene.camera.push({ zoom: 1.18 })', C.cyan],
    ['motion.render({ fps: 30 })', C.cyan],
  ];
  codePane('c', 110, 200, 660, 500, lines.map(([text, color], i) => [text, color, tCode + i * 0.045 * d]), { accent: C.cyan, title: 'MOTION SCENE', file: 'canh-mo-dau.ormotion.json' });
  for (let i = 0; i < 8; i += 1) {
    anim(`cL${i}`, 'opacity', [[tCode + i * 0.04 * d, 0], [tCode + 0.12 * d + i * 0.04 * d, 1]], 'ease_out');
    anim(`cL${i}`, 'translation_x', [[tCode + i * 0.04 * d, -12], [tCode + 0.12 * d + i * 0.04 * d, 0]], 'ease_out');
  }
  rect('cCaret', 170, 244, 2, 22, A(C.cyan, 230));
  anim('cCaret', 'translation_y', [[tCode, 0], [0.3 * d, 240]], 'linear');
  anim('cCaret', 'opacity', [[0.3 * d, 1], [0.34 * d, 0]], 'linear');

  // tokens leave the code and become geometry on the stage
  const stage = { x: 880, y: 190, w: 860, h: 520 };
  rect('stBg', stage.x, stage.y, stage.w, stage.h, { red: 10, green: 13, blue: 18, alpha: 230 }, { stroke: A(C.edge2, 180), stroke_width: 1 });
  anim('stBg', 'opacity', [[0.18 * d, 0], [0.26 * d, 1]], 'ease_out');
  txt('stLabel', 'STAGE · 0.0 s', stage.x + 14, stage.y + 24, 11, A(C.dim, 220));
  anim('stLabel', 'opacity', [[0.2 * d, 0], [0.28 * d, 1]], 'ease_out');

  const geo = [
    ['sun', 'ell', 1180, 300, 120, 120, C.amber],
    ['ridgeA', 'rect', 900, 520, 400, 160, { red: 12, green: 15, blue: 20, alpha: 255 }],
    ['ridgeB', 'rect', 1240, 540, 380, 150, { red: 10, green: 13, blue: 18, alpha: 255 }],
    ['figure', 'rect', 1010, 430, 34, 90, C.ink],
    ['birdA', 'rect', 1350, 320, 24, 2, C.ink],
    ['birdB', 'rect', 1380, 336, 24, 2, C.ink],
  ];
  // token chips fly from each code line toward the corresponding geometry
  for (let i = 0; i < 8; i += 1) {
    const y = 246 + i * 27;
    const color = i === 1 ? C.amber : i === 2 ? C.teal : i === 3 ? C.violet : C.cyan;
    rect(`tok${i}`, 700, y - 7, 18, 14, A(color, 230));
    const t0 = tFly + i * 0.045 * d;
    anim(`tok${i}`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 1], [t0 + 0.22 * d, 1], [t0 + 0.26 * d, 0]], 'ease_out');
    anim(`tok${i}`, 'translation_x', [[t0, 0], [t0 + 0.18 * d, 180 + (i % 3) * 120]], 'ease_in_out');
    anim(`tok${i}`, 'translation_y', [[t0, 0], [t0 + 0.18 * d, -60 + (i % 4) * 80]], 'ease_in_out');
    anim(`tok${i}`, 'rotation', [[t0, 0], [t0 + 0.2 * d, 90]], 'ease_in_out');
  }

  // geometry materializes as the tokens land
  const geoIds = [];
  geo.forEach(([id, kind, x, y, w, h, fill], i) => {
    if (kind === 'ell') ell(`g${id}`, x, y, w, h, A(C.amber, 235));
    else rect(`g${id}`, x, y, w, h, fill);
    geoIds.push(`g${id}`);
    const t0 = 0.56 * d + i * 0.05 * d;
    anim(`g${id}`, 'opacity', [[t0, 0], [t0 + 0.08 * d, 1]], 'ease_out');
    anim(`g${id}`, 'scale', [[t0, 0.2], [t0 + 0.16 * d, 1]], 'ease_out');
    anim(`g${id}`, 'translation_x', [[t0, 80], [t0 + 0.16 * d, 0]], 'ease_out');
  });
  anim('gsun', 'translation_y', [[0.7 * d, 30], [d, -6]], 'ease_in_out');

  // ruler + timecode keeps the determinism visible
  for (let i = 0; i < 14; i += 1) {
    rect(`tick${i}`, stage.x + 16 + i * 60, stage.y + stage.h - 22, 1, i % 3 === 0 ? 10 : 5, A(C.edge2, 180));
  }
  txt('tc', 't = 0,000 → 8,000 s', stage.x + 16, stage.y + stage.h - 40, 10, A(C.dim, 210));

  return finish({
    name: 'Motion Scene — code thành hình',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: 30 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.04, pan_x: -30 },
      { t: 0.85 * d, zoom: 1.1, pan_x: -80, pan_y: 20 },
      { t: d, zoom: 1.22, pan_x: -140, pan_y: 40 },
    ],
  });
}

export function motionBuild(duration) {
  begin();
  const d = duration;
  const tRise = 0.08 * d;
  const tRidge = 0.3 * d;
  const tFigure = 0.5 * d;
  const tPush = 0.7 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 100, 1500, 1200, A(C.amber, 10));
  ell('glowB', 1300, 500, 1000, 900, A(C.violet, 7));

  // full-frame scene, built in layers
  const HORIZON = 700;
  rect('sky', 0, 0, 1920, HORIZON, { red: 14, green: 18, blue: 25, alpha: 255 });
  ell('skyGlow', 1020, HORIZON - 430, 1100, 500, A(C.amber, 16));
  rect('skyBand', 0, 470, 1920, 2, A(C.amber, 18));
  rect('skyBand2', 0, 520, 1920, 1, A(C.ink, 10));
  rect('skyTop', 0, 0, 1920, 340, { red: 11, green: 15, blue: 21, alpha: 255 });
  ell('halo', 1030, 40, 600, 600, A(C.amber, 15));
  ell('sun', 1280, 170, 260, 260, A(C.amber, 238));
  ell('sunCore', 1320, 210, 180, 180, A({ red: 251, green: 210, blue: 140, alpha: 255 }, 240));
  rect('water', 0, HORIZON, 1920, 380, { red: 9, green: 12, blue: 16, alpha: 255 });
  for (let i = 0; i < 6; i += 1) rect(`waterLine${i}`, 0, HORIZON + 70 + i * 44, 1920 - i * 60, 1, A(C.blue, 16 - i * 2));
  for (let i = 0; i < 8; i += 1) {
    const w = 200 - i * 18;
    rect(`refl${i}`, 1410 - w / 2, HORIZON + 26 + i * 32, w, 3 + (i % 3) * 2, A(C.amber, 62 - i * 5));
  }
  anim('sun', 'translation_y', [[0, 160], [tRise + 0.4 * d, 0], [d, -10]], 'ease_out');
  anim('sunCore', 'translation_y', [[0, 160], [tRise + 0.4 * d, 0], [d, -10]], 'ease_out');
  anim('halo', 'scale', [[0, 0.7], [tRise + 0.5 * d, 1.12], [d, 1.04]], 'ease_out');
  anim('sun', 'scale', [[0, 0.86], [tRise + 0.4 * d, 1.04], [d, 1]], 'ease_in_out');

  // ridges slide in from opposite edges at different speeds
  rect('ridgeFar', -420, 512, 1500, 210, { red: 12, green: 15, blue: 21, alpha: 255 }, { rotation_degrees: -4 });
  rect('ridgeFarLit', -420, 512, 1500, 2, A(C.amber, 95), { rotation_degrees: -4 });
  rect('ridgeMid', 500, 540, 1700, 200, { red: 14, green: 18, blue: 24, alpha: 255 }, { rotation_degrees: 3 });
  rect('ridgeMidLit2', 520, 566, 1500, 1, A(C.cyan, 22), { rotation_degrees: 3 });
  rect('ridgeMidLit', 500, 540, 1700, 2, A(C.amber, 115), { rotation_degrees: 3 });
  rect('ridgeNear', 300, 600, 1700, 220, { red: 10, green: 13, blue: 18, alpha: 255 }, { rotation_degrees: -2 });
  rect('ridgeNearLit', 300, 600, 1700, 2, A(C.amber, 130), { rotation_degrees: -2 });
  anim('ridgeFarLit', 'opacity', [[0, 0.5], [d, 1]], 'ease_in_out');
  anim('ridgeFar', 'translation_x', [[0, -520], [tRidge + 0.4 * d, 0]], 'ease_out');
  anim('ridgeFarLit', 'translation_x', [[0, -520], [tRidge + 0.4 * d, 0]], 'ease_out');
  anim('ridgeMid', 'translation_x', [[0, 620], [tRidge + 0.5 * d, 0]], 'ease_out');
  anim('ridgeMidLit', 'translation_x', [[0, 620], [tRidge + 0.5 * d, 0]], 'ease_out');
  anim('ridgeNear', 'translation_x', [[0, 520], [tRidge + 0.6 * d, 0]], 'ease_out');
  anim('ridgeNearLit', 'translation_x', [[0, 520], [tRidge + 0.6 * d, 0]], 'ease_out');

  // foreground and figure
  rect('fg', -80, 900, 2120, 320, { red: 4, green: 5, blue: 7, alpha: 255 }, { rotation_degrees: -1.2 });
  rect('fgLit', -80, 900, 2120, 2, A(C.amber, 70), { rotation_degrees: -1.2 });
  for (let i = 0; i < 26; i += 1) {
    const gx = 40 + i * 76;
    const gh = 10 + ((i * 37) % 26);
    rect(`grass${i}`, gx, 900 - gh, 2, gh, { red: 5, green: 6, blue: 8, alpha: 255 }, { rotation_degrees: (i % 2 ? -8 : 7) });
    rect(`grassLit${i}`, gx, 900 - gh, 1, gh, A(C.amber, 35), { rotation_degrees: (i % 2 ? -8 : 7) });
  }
  ell('head', 760, 430, 28, 28, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('torso', 748, 456, 52, 62, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('legA', 752, 516, 10, 30, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('legB', 770, 516, 10, 30, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('rimHead', 786, 434, 3, 22, A(C.amber, 195));
  rect('rimTorso', 798, 457, 3, 58, A(C.amber, 195));
  ['head', 'torso', 'legA', 'legB', 'rimHead', 'rimTorso'].forEach((id, i) => {
    anim(id, 'opacity', [[tFigure + i * 0.04 * d, 0], [tFigure + 0.1 * d + i * 0.04 * d, 1]], 'ease_out');
    anim(id, 'translation_y', [[tFigure, 30], [tFigure + 0.2 * d, 0]], 'ease_out');
  });

  // birds cross the frame
  for (let i = 0; i < 3; i += 1) {
    rect(`bird${i}`, 840 + i * 60, 320 + (i % 2) * 34, 30, 2, A(C.ink, 120), { rotation_degrees: -18 });
    rect(`bird${i}b`, 850 + i * 60, 322 + (i % 2) * 34, 30, 2, A(C.ink, 120), { rotation_degrees: 18 });
    anim(`bird${i}`, 'translation_x', [[tFigure, 0], [d, -520 - i * 60]], 'linear');
    anim(`bird${i}b`, 'translation_x', [[tFigure, 0], [d, -520 - i * 60]], 'linear');
    anim(`bird${i}`, 'translation_y', [[tFigure, 0], [d * 0.5, -22], [d, 12]], 'ease_in_out');
    anim(`bird${i}b`, 'translation_y', [[tFigure, 0], [d * 0.5, -22], [d, 12]], 'ease_in_out');
  }
  particles('dust', 1300, 400, 44, 460, C.amber, d, { size: 2, speed: 44, squash: 0.9 });

  // layer guides, thin and technical, then they leave
  const guides = [];
  ['sky', 'ridgeFar', 'ridgeMid', 'ridgeNear', 'fg'].forEach((id, i) => {
    rect(`guide${i}`, 0, 300 + i * 130, 1920, 1, A(C.cyan, 60));
    guides.push(`guide${i}`);
  });
  guides.forEach((id) => anim(id, 'opacity', [[0.08 * d, 0], [0.2 * d, 0.5], [0.55 * d, 0.3], [0.66 * d, 0]], 'ease_in_out'));
  tagChip('layerTag', 'LỚP · PARALLAX · CAMERA', 160, 180, C.cyan, 12);
  ['layerTagBg', 'layerTagT'].forEach((id) => anim(id, 'opacity', [[0.1 * d, 0], [0.2 * d, 1], [0.64 * d, 1], [0.7 * d, 0]], 'ease_in_out'));

  return finish({
    name: 'Motion Scene — lớp và camera',
    duration: d,
    camera: [
      { t: 0, zoom: 1.0, pan_y: 20 },
      { t: 0.3 * d, zoom: 1.03, pan_y: 0 },
      { t: 0.55 * d, zoom: 1.06, pan_x: -20 },
      { t: 0.75 * d, zoom: 1.12, pan_x: -40, pan_y: 20 },
      { t: d, zoom: 1.34, pan_x: -60, pan_y: 60 },
    ],
  });
}

export function motionMorph(duration) {
  begin();
  const d = duration;
  const tMorph = 0.16 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 180, 1400, 1100, A(C.violet, 9));
  ell('glowB', 1200, 500, 1000, 900, A(C.amber, 8));
  for (let i = 0; i < 9; i += 1) rect(`gx${i}`, 240 + i * 180, 0, 1, 1080, A(C.edge, 18));

  // the same picture, mid-transform: abstract pieces becoming a landscape
  const cx = 960, cy = 450;
  // circle → sun
  ell('shapeCircle', cx + 190, cy - 250, 130, 130, A(C.violet, 0), { stroke: A(C.violet, 160), stroke_width: 3 });
  ell('sun', cx + 220, cy - 220, 220, 220, A(C.amber, 235));
  anim('shapeCircle', 'opacity', [[0, 1], [0.3 * d, 0.8], [0.42 * d, 0]], 'ease_in');
  anim('shapeCircle', 'scale', [[0, 1], [0.4 * d, 1.7]], 'ease_in');
  anim('sun', 'opacity', [[0.22 * d, 0], [0.4 * d, 1]], 'ease_out');
  anim('sun', 'scale', [[0.22 * d, 0.4], [0.5 * d, 1.05], [d, 1]], 'ease_out');

  // two lines → ridges
  rect('lineA', 360, 520, 420, 4, A(C.cyan, 180));
  rect('lineB', 820, 560, 480, 4, A(C.blue, 180));
  rect('ridgeA', 300, 520, 460, 190, { red: 10, green: 13, blue: 17, alpha: 255 }, { rotation_degrees: -3 });
  rect('ridgeB', 760, 548, 520, 200, { red: 8, green: 11, blue: 15, alpha: 255 }, { rotation_degrees: 2 });
  ['lineA', 'lineB'].forEach((id, i) => {
    anim(id, 'opacity', [[0, 1], [0.36 * d, 1], [0.46 * d, 0]], 'ease_in');
    anim(id, 'height', [[0.3 * d, 4], [0.5 * d, 190]], 'ease_in_out');
    anim(id, 'translation_y', [[0.3 * d, 0], [0.5 * d, -i * 12]], 'ease_in_out');
  });
  ['ridgeA', 'ridgeB'].forEach((id, i) => {
    anim(id, 'opacity', [[0.34 * d, 0], [0.5 * d, 1]], 'ease_out');
    anim(id, 'translation_x', [[0.34 * d, i ? 60 : -60], [0.55 * d, 0]], 'ease_out');
  });

  // rectangle → figure
  rect('shapeBox', cx - 30, cy + 20, 60, 120, A(C.violet, 0), { stroke: A(C.violet, 170), stroke_width: 2 });
  ell('head', cx - 14, cy + 4, 28, 28, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('torso', cx - 26, cy + 34, 52, 70, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('rim', cx + 22, cy + 38, 3, 60, A(C.amber, 190));
  anim('shapeBox', 'opacity', [[0.1 * d, 1], [0.3 * d, 0.7], [0.42 * d, 0]], 'ease_in');
  anim('shapeBox', 'rotation', [[0.1 * d, 0], [0.42 * d, 12]], 'ease_in');
  ['head', 'torso', 'rim'].forEach((id, i) => {
    anim(id, 'opacity', [[0.28 * d + i * 0.03 * d, 0], [0.44 * d + i * 0.03 * d, 1]], 'ease_out');
    anim(id, 'translation_y', [[0.28 * d, 40], [0.5 * d, 0]], 'ease_out');
  });

  // a text layer becomes a title inside the scene, then dissolves into particles
  txt('morphText', 'MOTION', cx - textW('MOTION', 42) / 2, cy + 220, 42, A(C.ink, 240));
  anim('morphText', 'opacity', [[0.05 * d, 0], [0.16 * d, 1], [0.3 * d, 1], [0.42 * d, 0]], 'ease_in_out');
  anim('morphText', 'translation_y', [[0.05 * d, 30], [0.3 * d, 0]], 'ease_out');
  particles('mtDust', cx, cy + 220, 24, 160, C.ink, 4.5, { size: 1.6, speed: 26 });

  // frame edges snap in at the end: the scene becomes a clip
  const F = { x: 360, y: 150, w: 1200, h: 600 };
  ['top', 'bottom', 'left', 'right'].forEach((edge, i) => {
    const horizontal = i < 2;
    rect(`edge${i}`, F.x, F.y + (i === 1 ? F.h : 0), horizontal ? F.w : 1, horizontal ? 1 : F.h, A(C.ink, 190));
    anim(`edge${i}`, horizontal ? 'width' : 'height', [[0.72 * d, i < 2 ? F.w : F.h], [0.84 * d, i < 2 ? F.w : F.h]], 'ease_out');
    anim(`edge${i}`, 'opacity', [[0.72 * d, 0], [0.78 * d, 1], [0.9 * d, 1], [0.96 * d, 0]], 'ease_in_out');
    if (horizontal) {
      anim(`edge${i}`, 'width', [[0.62 * d, 0], [0.78 * d, F.w]], 'ease_out');
      anim(`edge${i}`, 'translation_x', [[0.62 * d, i === 0 ? -F.w / 2 : F.w / 2], [0.78 * d, 0]], 'ease_out');
    } else {
      anim(`edge${i}`, 'height', [[0.62 * d, 0], [0.78 * d, F.h]], 'ease_out');
      anim(`edge${i}`, 'translation_y', [[0.62 * d, i === 2 ? -F.h / 2 : F.h / 2], [0.78 * d, 0]], 'ease_out');
    }
  });
  txt('timeLbl', 'FRAME 001 · 00:00:00,000', F.x, F.y - 22, 10, A(C.dim, 210));
  anim('timeLbl', 'opacity', [[0.7 * d, 0], [0.8 * d, 1]], 'ease_out');
  anim('timeLbl', 'translation_x', [[0.7 * d, -20], [0.85 * d, 0]], 'ease_out');

  // slow colour sweep across the geometry
  rect('sweep', F.x - 200, F.y, 60, F.h, A(C.ink, 12), { rotation_degrees: 12 });
  anim('sweep', 'translation_x', [[0.2 * d, 0], [0.9 * d, 1500]], 'linear');
  anim('sweep', 'opacity', [[0.2 * d, 0], [0.3 * d, 1], [0.85 * d, 1], [0.95 * d, 0]], 'ease_in_out');

  return finish({
    name: 'Motion Scene — biến đổi',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: -30, rotation_degrees: -1 },
      { t: 0.3 * d, zoom: 1.0, pan_x: 0, rotation_degrees: 0 },
      { t: 0.55 * d, zoom: 1.06, pan_x: 40, rotation_degrees: 1 },
      { t: 0.8 * d, zoom: 1.02, pan_x: 0 },
      { t: d, zoom: 1.12, pan_y: 10 },
    ],
  });
}

export function motionTimeline(duration) {
  begin();
  const d = duration;
  const tClip = 0.3 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 1200, 100, 1200, 900, A(C.cyan, 9));
  ell('glowB', 300, 500, 900, 800, A(C.amber, 8));

  // the finished scene, full size, then it becomes a clip
  const V = { x: 360, y: 120, w: 1200, h: 620 };
  rect('frame', V.x, V.y, V.w, V.h, { red: 10, green: 13, blue: 18, alpha: 255 }, { stroke: A(C.edge2, 190), stroke_width: 1 });
  const shot = miniShot('sc', V.x, V.y, V.w, V.h, C.amber);
  ['scSky', 'scSun', 'scRidgeA', 'scRidgeB', 'scGround', 'scHead', 'scBody', 'scRim'].forEach((id) => anim(id, 'opacity', [[0, 0], [0.08 * d, 1]], 'ease_out'));
  anim('scSun', 'translation_y', [[0, 20], [d, -10]], 'ease_in_out');
  txt('scLabel', 'MOTION SCENE · 8.0 s', V.x + 16, V.y + 28, 12, A(C.ink, 220));
  txt('scTc', '00:00:08:00', V.x + V.w - 16 - textW('00:00:08:00', 12), V.y + 28, 12, A(C.mute, 220));
  const all = ['frame', 'scSky', 'scSun', 'scRidgeA', 'scRidgeB', 'scGround', 'scHead', 'scBody', 'scRim', 'scLabel', 'scTc'];
  all.forEach((id) => anim(id, 'opacity', [[tClip, 1], [tClip + 0.12 * d, 0]], 'ease_in'));
  all.forEach((id) => anim(id, 'translation_y', [[tClip, 0], [tClip + 0.12 * d, -420]], 'ease_in'));
  // Text keeps its raster size: a tiny scale makes glyph shaping fail in the
  // bundled text path. It leaves by opacity and position instead.
  all.filter((id) => !['scLabel', 'scTc'].includes(id))
    .forEach((id) => anim(id, 'scale', [[tClip, 1], [tClip + 0.12 * d, 0.18]], 'ease_in'));

  // timeline with real footage, motion clip and sound/captions
  const T = { x: 200, y: 500, w: 1520, h: 340 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('tlHead', T.x, T.y, T.w, 34, C.panel2);
  txt('tlTitle', 'TIMELINE · 00:00:24:00', T.x + 16, T.y + 21, 11, A(C.mute, 232));
  const lanes = [['C1', C.blue], ['V2', C.violet], ['V1', C.amber], ['A1', C.cyan]];
  const laneH = (T.h - 44) / 4;
  lanes.forEach(([name, color], i) => {
    const y = T.y + 44 + i * laneH;
    rect(`lane${i}`, T.x + 12, y, T.w - 24, laneH - 3, A(C.panel2, 255));
    rect(`laneTag${i}`, T.x + 12, y, 42, laneH - 3, A(C.bg2, 255));
    txt(`laneT${i}`, name, T.x + 22, y + laneH / 2, 10, A(color, 235));
  });
  const laneY = (i) => T.y + 44 + i * laneH + 2;
  rect('footA', 300, laneY(2), 340, laneH - 7, A(C.amber, 185));
  rect('footB', 680, laneY(2), 320, laneH - 7, A(C.teal, 185));
  rect('footC', 1040, laneY(2), 420, laneH - 7, A(C.amber, 185));
  rect('voice', 300, laneY(3), 700, laneH - 7, A(C.cyan, 175));
  for (let i = 0; i < 50; i += 1) {
    const bh = 4 + ((i * 29) % 20);
    rect(`wv${i}`, 310 + i * 13.6, laneY(3) + (laneH - 7) / 2, 3, bh, A(C.bg, 100));
  }
  // the motion clip lands on V2
  rect('motionClip', 760, laneY(1), 300, laneH - 7, A(C.violet, 205));
  rect('motionClipCap', 760, laneY(1), 300, 2, A(C.ink, 240));
  txt('motionClipT', 'scene-01', 772, laneY(1) + 20, 10, A(C.bg, 190));
  anim('motionClip', 'opacity', [[tClip + 0.06 * d, 0], [tClip + 0.16 * d, 1]], 'ease_out');
  anim('motionClipCap', 'opacity', [[tClip + 0.06 * d, 0], [tClip + 0.16 * d, 1]], 'ease_out');
  anim('motionClipT', 'opacity', [[tClip + 0.06 * d, 0], [tClip + 0.16 * d, 1]], 'ease_out');
  anim('motionClip', 'translation_y', [[tClip + 0.06 * d, -160], [tClip + 0.2 * d, 0]], 'ease_out');

  // caption track appears under the voice
  for (let i = 0; i < 6; i += 1) {
    rect(`cap${i}`, 320 + i * 128, laneY(0), 112, laneH - 7, A(C.blue, 150));
    anim(`cap${i}`, 'opacity', [[0.68 * d + i * 0.02 * d, 0], [0.72 * d + i * 0.02 * d, 1]], 'ease_out');
  }

  // playhead crosses the whole programme
  rect('play', 300, T.y + 40, 2, T.h - 40, A(C.red, 225));
  rect('playHead', 292, T.y + 40, 18, 6, A(C.red, 235));
  anim('play', 'translation_x', [[0.62 * d, 0], [d, 1080]], 'linear');
  anim('playHead', 'translation_x', [[0.62 * d, 0], [d, 1080]], 'linear');

  // blend label
  tagChip('blend', 'CLIP BÌNH THƯỜNG · CẮT · GHÉP · XUẤT', 200, 460, C.green, 11);
  ['blendBg', 'blendT'].forEach((id) => anim(id, 'opacity', [[0.74 * d, 0], [0.82 * d, 1]], 'ease_out'));

  return finish({
    name: 'Motion Scene — vào timeline',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_y: 20 },
      { t: 0.3 * d, zoom: 1.06, pan_y: 60 },
      { t: 0.55 * d, zoom: 1.0, pan_y: 30 },
      { t: 0.75 * d, zoom: 1.04, pan_y: 20 },
      { t: d, zoom: 1.1, pan_y: 40 },
    ],
  });
}

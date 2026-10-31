// S4 — 22–31 s. Motion Scene: code becomes geometry, geometry becomes a shot.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 1000, 100, 1200, 900, A(C.cyan, 10));
  ell('glowB', 200, 600, 900, 800, A(C.violet, 10));
  for (let i = 0; i < 12; i += 1) rect(`gx${i}`, 120 + i * 150, 0, 1, 1080, A(C.edge, 18));
  for (let i = 0; i < 8; i += 1) rect(`gy${i}`, 0, 90 + i * 120, 1920, 1, A(C.edge, 18));

  // ------------------------------------------------------------------ code pane
  const P = { x: 110, y: 240, w: 780, h: 520 };
  rect('codeShadow', P.x + 14, P.y + 16, P.w, P.h, { red: 0, green: 0, blue: 0, alpha: 140 });
  rect('code', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('codeBar', P.x, P.y, P.w, 40, C.panel2);
  ell('codeDot', P.x + 18, P.y + 16, 8, 8, A(C.cyan, 235));
  txt('codeTitle', 'MOTION SCENE', P.x + 34, P.y + 19, 14, A(C.ink, 230));
  txt('codeFile', 'scene.ormotion', P.x + P.w - 20 - textW('scene.ormotion', 12), P.y + 20, 12, A(C.dim, 210));
  rect('codeRule', P.x, P.y + 40, P.w, 1, A(C.edge2, 180));
  for (let i = 0; i < 9; i += 1) rect(`lnG${i}`, P.x + 20, P.y + 56 + i * 30, P.w - 40, 1, A(C.edge, 60));

  const lines = [
    ['const scene = motion.begin()', [0, 14], C.violet],
    ['scene.sun({ x: 340, r: 90 })', [0, 10], C.ink],
    ['scene.ridge({ y: 210 })', [0, 12], C.ink],
    ['scene.figure({ x: 120 })', [0, 13], C.ink],
    ['scene.birds(3)', [0, 12], C.ink],
    ['scene.particles(40)', [0, 17], C.ink],
    ['scene.camera.push(1.1)', [0, 18], C.ink],
    ['motion.render(scene)', [0, 17], C.cyan],
  ];
  const codeIds = [];
  lines.forEach(([text, [start, end], color], i) => {
    const y = P.y + 76 + i * 30;
    const tx = P.x + 52;
    txt(`codeL${i}`, text, tx, y, 19, A(C.mute, 225));
    rect(`codeHl${i}`, P.x + 20, y - 10, P.w - 40, 24, A(C.ink, 0));
    codeIds.push(`codeL${i}`, `codeHl${i}`);
    anim(`codeL${i}`, 'opacity', [[0.1 + i * 0.08, 0], [0.34 + i * 0.08, 1]], 'ease_out');
    anim(`codeHl${i}`, 'opacity', [[0.1 + i * 0.08, 0], [0.2 + i * 0.08, 0.06], [0.9 + i * 0.1, 0.06], [1.1 + i * 0.1, 0]], 'linear');
    // one highlighted token per line
    rect(`codeTok${i}`, tx + textW(text.slice(0, start), 19), y - 7, textW(text.slice(start, end), 19), 18, A(color, 40));
  });
  // caret and selection
  rect('codeCursor', P.x + 52 + 240, P.y + 76 - 12, 2, 24, A(C.cyan, 230));
  anim('codeCursor', 'translation_y', [[0.1, 0], [0.5, 60], [0.9, 120], [1.3, 180], [1.9, 240], [2.3, 300]], 'linear');
  anim('codeCursor', 'opacity', [[2.9, 1], [3.05, 0]], 'linear');

  // shapes fly out of the code to the stage
  const stage = { x: 1060, y: 228, w: 700, h: 432 };
  const T_ASM = 2.3;

  // stage frame
  rect('stageBg', stage.x - 12, stage.y - 12, stage.w + 24, stage.h + 24, C.panel);
  rect('stage', stage.x, stage.y, stage.w, stage.h, { red: 12, green: 15, blue: 20, alpha: 255 });
  rect('stageSky', stage.x, stage.y, stage.w, 300, { red: 17, green: 22, blue: 30, alpha: 255 });
  // border draws itself
  const border = 2, bcol = A(C.cyan, 200);
  rect('stB0', stage.x, stage.y, 1, stage.h, bcol);
  rect('stB1', stage.x + stage.w - 1, stage.y, 1, stage.h, bcol);
  rect('stB2', stage.x, stage.y, stage.w, 1, bcol);
  rect('stB3', stage.x, stage.y + stage.h - 1, stage.w, 1, bcol);
  anim('stB0', 'height', [[T_ASM, 1], [T_ASM + 0.5, stage.h]], 'ease_out');
  anim('stB1', 'height', [[T_ASM, 1], [T_ASM + 0.5, stage.h]], 'ease_out');
  anim('stB2', 'width', [[T_ASM, 1], [T_ASM + 0.5, stage.w]], 'ease_out');
  anim('stB3', 'width', [[T_ASM, 1], [T_ASM + 0.5, stage.w]], 'ease_out');
  // the picture, built in order
  ell('stHalo', stage.x + 300, stage.y + 40, 300, 300, A(C.amber, 12));
  ell('stSun', stage.x + 400, stage.y + 76, 150, 150, A(C.amber, 232));
  ell('stSunC', stage.x + 424, stage.y + 100, 102, 102, A({ red: 251, green: 208, blue: 138, alpha: 255 }, 235));
  rect('stRidge1', stage.x, stage.y + 232, 500, 190, { red: 10, green: 12, blue: 16, alpha: 255 }, { rotation_degrees: -5 });
  rect('stRidge1L', stage.x, stage.y + 232, 500, 2, A(C.amber, 90), { rotation_degrees: -5 });
  rect('stRidge2', stage.x + 300, stage.y + 250, 400, 180, { red: 12, green: 15, blue: 20, alpha: 255 }, { rotation_degrees: 4 });
  rect('stRidge2L', stage.x + 300, stage.y + 250, 400, 2, A(C.amber, 70), { rotation_degrees: 4 });
  rect('stGround', stage.x, stage.y + 310, stage.w, 122, { red: 8, green: 10, blue: 13, alpha: 255 });
  for (let i = 0; i < 5; i += 1) rect(`stRefl${i}`, stage.x + 448 - i * 7, stage.y + 322 + i * 18, 70 - i * 8, 3, A(C.amber, 55 - i * 8));
  ell('stHead', stage.x + 176, stage.y + 218, 20, 20, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('stBody', stage.x + 170, stage.y + 234, 32, 52, { red: 5, green: 6, blue: 8, alpha: 255 });
  rect('stRim', stage.x + 200, stage.y + 236, 3, 48, A(C.amber, 170));
  for (let i = 0; i < 3; i += 1) {
    rect(`stBird${i}`, stage.x + 250 + i * 34, stage.y + 70 + i * 14, 18, 2, A(C.ink, 110), { rotation_degrees: -16 });
    rect(`stBird${i}b`, stage.x + 258 + i * 34, stage.y + 72 + i * 14, 18, 2, A(C.ink, 110), { rotation_degrees: 16 });
  }
  particles('stDust', stage.x + 480, stage.y + 180, 22, 260, C.amber, d, { size: 1.8, speed: 22 });
  const stageIds = ['stSun', 'stSunC', 'stRidge1', 'stRidge1L', 'stRidge2', 'stRidge2L', 'stGround', 'stHead', 'stBody', 'stRim',
    ...Array.from({ length: 5 }, (_, i) => `stRefl${i}`), ...Array.from({ length: 3 }, (_, i) => `stBird${i}`), ...Array.from({ length: 3 }, (_, i) => `stBird${i}b`)];
  stageIds.forEach((id, i) => {
    const t0 = T_ASM + 0.1 + i * 0.07;
    anim(id, 'opacity', [[t0, 0], [t0 + 0.3, 1]], 'ease_out');
    anim(id, 'translation_x', [[t0, -420], [t0 + 0.5, 0]], 'ease_out');
    anim(id, 'scale', [[t0, 0.3], [t0 + 0.55, 1]], 'ease_out');
  });

  // construction: wireframe ticks orbit the stage while it builds
  for (let i = 0; i < 8; i += 1) {
    const t0 = 2.5 + i * 0.16;
    rect(`tick${i}`, stage.x - 30, stage.y + 40 + i * 52, 18, 2, A(C.cyan, 150));
    anim(`tick${i}`, 'opacity', [[t0, 0], [t0 + 0.2, 1], [t0 + 0.8, 0]], 'ease_out');
    anim(`tick${i}`, 'translation_x', [[t0, 0], [t0 + 0.8, -40]], 'ease_out');
  }

  // camera pushes into the built scene
  anim('stSun', 'translation_y', [[T_ASM, 20], [T_ASM + 1.4, -10], [d, 0]], 'ease_in_out');
  anim('stSunC', 'translation_y', [[T_ASM, 20], [T_ASM + 1.4, -10], [d, 0]], 'ease_in_out');
  anim('stHalo', 'scale', [[0, 1], [3.2, 1.05], [6.2, 1.0]], 'ease_in_out');
  anim('stHead', 'translation_x', [[3.2, 0], [5.4, -14]], 'ease_in_out');
  anim('stBody', 'translation_x', [[3.2, 0], [5.4, -14]], 'ease_in_out');
  anim('stRim', 'translation_x', [[3.2, 0], [5.4, -14]], 'ease_in_out');

  return finish({
    name: opts.name ?? 'Motion Scene',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: 12 },
      { t: 1.2, zoom: 1.0 },
      { t: 2.3, zoom: 1.06, pan_x: -40 },
      { t: 3.4, zoom: 1.02, pan_x: -60 },
      { t: 4.8, zoom: 1.1, pan_x: -70 },
      { t: 5.2, zoom: 1.16, pan_x: -160, pan_y: 40 },
      { t: 6.6, zoom: 1.42, pan_x: -600, pan_y: 128 },
      { t: 9, zoom: 1.5, pan_x: -630, pan_y: 132 },
    ],
  });
}

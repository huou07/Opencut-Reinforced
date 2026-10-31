// Reality and community chapter: what ships, what does not, and the invitation.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since, arrow } from '../kit.mjs';
import { tagChip, miniShot, viewer } from '../ui.mjs';

export function reality(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);

  // a room, one desk, one machine
  ell('lamp', 620, 80, 900, 700, A(C.amber, 14));
  rect('floor', -40, 760, 2000, 360, { red: 12, green: 12, blue: 13, alpha: 255 });
  rect('wall', -40, -40, 2000, 800, { red: 14, green: 15, blue: 17, alpha: 255 });
  for (let i = 0; i < 9; i += 1) rect(`wallLine${i}`, -40 + i * 240, -40, 1, 800, A(C.edge, 35));

  // the desk
  rect('deskTop', 220, 620, 1480, 40, { red: 34, green: 28, blue: 22, alpha: 255 });
  rect('deskEdge', 220, 656, 1480, 10, { red: 24, green: 20, blue: 16, alpha: 255 });
  rect('legL', 300, 666, 24, 200, { red: 22, green: 19, blue: 16, alpha: 255 });
  rect('legR', 1580, 666, 24, 200, { red: 22, green: 19, blue: 16, alpha: 255 });

  // laptop, screen lit
  const L = { x: 620, y: 300, w: 700, h: 330 };
  rect('lapBase', L.x - 50, L.y + L.h, L.w + 100, 16, { red: 38, green: 36, blue: 34, alpha: 255 });
  rect('lapLid', L.x, L.y, L.w, L.h, { red: 17, green: 17, blue: 18, alpha: 255 }, { stroke: A(C.edge2, 150), stroke_width: 1 });
  rect('screen', L.x + 12, L.y + 12, L.w - 24, L.h - 24, C.bg2);
  rect('screenBar', L.x + 12, L.y + 12, L.w - 24, 26, C.panel2);
  ell('screenDot', L.x + 26, L.y + 23, 6, 6, A(C.amber, 230));
  txt('screenTitle', 'OpenCut Reinforced', L.x + 40, L.y + 26, 9, A(C.ink, 215));
  rect('screenViewer', L.x + 26, L.y + 50, 400, 170, { red: 10, green: 13, blue: 18, alpha: 255 });
  ell('screenSun', L.x + 300, L.y + 74, 54, 54, A(C.amber, 228));
  rect('screenGround', L.x + 26, L.y + 162, 400, 58, { red: 7, green: 9, blue: 12, alpha: 255 });
  for (let i = 0; i < 4; i += 1) rect(`screenTrack${i}`, L.x + 26, L.y + 238 + i * 18, 660, 10, A(C.panel2, 255));
  rect('screenClipA', L.x + 36, L.y + 240, 170, 7, A(C.amber, 180));
  rect('screenClipB', L.x + 230, L.y + 258, 200, 7, A(C.cyan, 180));
  rect('screenClipC', L.x + 460, L.y + 276, 160, 7, A(C.blue, 180));
  rect('screenPlay', L.x + 300, L.y + 234, 2, 78, A(C.red, 215));
  anim('screenPlay', 'translation_x', [[0, 0], [d, 180]], 'linear');

  // release card, pinned beside the screen
  const R = { x: 1330, y: 310, w: 350, h: 230 };
  rect('relShadow', R.x + 10, R.y + 12, R.w, R.h, { red: 0, green: 0, blue: 0, alpha: 130 });
  rect('relBg', R.x, R.y, R.w, R.h, C.panel, { stroke: A(C.amber, 170), stroke_width: 1 });
  rect('relTop', R.x, R.y, R.w, 3, C.amber);
  txt('relTag', 'BẢN PHÁT HÀNH HIỆN TẠI', R.x + 18, R.y + 30, 9, A(C.dim, 220));
  txt('relVer', '0.1.0', R.x + 18, R.y + 78, 38, A(C.ink, 245));
  txt('relOs', 'macOS · Apple Silicon', R.x + 18, R.y + 112, 13, A(C.mute, 230));
  rect('relRule', R.x + 18, R.y + 130, R.w - 36, 1, A(C.edge, 160));
  txt('relNote', 'ad-hoc signed · not notarized', R.x + 18, R.y + 156, 10, A(C.amber, 215));
  txt('relNote2', 'một bản chạy được để thử', R.x + 18, R.y + 180, 10, A(C.dim, 210));
  tagChip('relTry', 'THỬ TRÊN MAC', R.x + 18, R.y + 212, C.green, 10);

  // a single chair, a single desk: this is one person's setup
  ellipseChair();
  function ellipseChair() {
    ell('chairBack', 980, 700, 220, 160, { red: 20, green: 20, blue: 22, alpha: 255 });
    rect('chairPost', 1082, 830, 16, 70, { red: 18, green: 18, blue: 20, alpha: 255 });
    rect('chairBase', 1010, 896, 160, 10, { red: 18, green: 18, blue: 20, alpha: 255 });
  }
  txt('onePerson', 'một người · một chiếc Mac', 620, 866, 13, A(C.dim, 220));
  anim('onePerson', 'opacity', [[0.62 * d, 0], [0.7 * d, 1]], 'ease_out');

  // slow pull from the screen to the room
  return finish({
    name: 'Thực tế — một chiếc Mac',
    duration: d,
    camera: [
      { t: 0, zoom: 1.22, pan_x: 40, pan_y: 80 },
      { t: 0.45 * d, zoom: 1.08, pan_x: 0, pan_y: 20 },
      { t: 0.75 * d, zoom: 1.0, pan_y: 0 },
      { t: d, zoom: 1.05, pan_y: 10 },
    ],
  });
}

export function platformGap(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 300, 200, 1300, 1000, A(C.blue, 7));
  ell('glowB', 1500, 500, 1000, 900, A(C.violet, 6));

  const panels = [
    ['macOS', 'Apple Silicon', C.green, 'ĐANG PHÁT HÀNH', 1],
    ['Windows', 'x86_64 / arm64', C.blue, 'CHƯA CÓ BẢN PHÁT HÀNH', 0],
    ['Linux', 'x86_64 / arm64', C.amber, 'CHƯA CÓ BẢN PHÁT HÀNH', 0],
  ];
  panels.forEach(([name, arch, color, state, live], i) => {
    const x = 130 + i * 570;
    const W = 520, H = 620;
    rect(`p${i}Shadow`, x + 12, 190, W, H, { red: 0, green: 0, blue: 0, alpha: 130 });
    rect(`p${i}`, x, 180, W, H, live ? C.panel : C.bg2, { stroke: A(color, live ? 190 : 90), stroke_width: 1 });
    rect(`p${i}Top`, x, 180, W, 4, A(color, live ? 240 : 100));
    ell(`p${i}Dot`, x + 22, 206, 9, 9, A(color, live ? 240 : 110));
    txt(`p${i}Name`, name, x + 42, 214, 20, A(live ? C.ink : C.mute, 240));
    txt(`p${i}Arch`, arch, x + 22, 252, 11, A(C.dim, 215));
    txt(`p${i}State`, state, x + 22, 284, 10, A(live ? C.green : C.amber, 225));
    const rows = ['GIAO DIỆN', 'CODEC / MEDIA', 'HIỆU NĂNG', 'CÀI ĐẶT'];
    rows.forEach((row, k) => {
      const y = 330 + k * 66;
      txt(`p${i}R${k}`, row, x + 22, y, 10, A(C.dim, 220));
      if (live) {
        rect(`p${i}Ck${k}`, x + W - 52, y - 12, 14, 14, A(C.green, 0), { stroke: A(C.green, 200), stroke_width: 1 });
        rect(`p${i}TkA${k}`, x + W - 48, y - 5, 6, 2, A(C.green, 230), { rotation_degrees: 40 });
        rect(`p${i}TkB${k}`, x + W - 44, y - 7, 9, 2, A(C.green, 230), { rotation_degrees: -45 });
      } else {
        rect(`p${i}Open${k}`, x + W - 52, y - 6, 22, 2, A(C.amber, 130));
        txt(`p${i}Wait${k}`, 'cần máy thật', x + W - 190, y, 9, A(C.amber, 170));
      }
    });
    // a thin progress line that is complete only on macOS
    rect(`p${i}Bar`, x + 22, 600, W - 44, 3, A(C.edge2, 170));
    rect(`p${i}Fill`, x + 22, 598, live ? W - 44 : 90, 7, A(color, 210));
    if (!live) anim(`p${i}Fill`, 'width', [[0.5 * d + i * 0.05 * d, 40], [0.7 * d + i * 0.05 * d, 120]], 'ease_in_out');
    txt(`p${i}Foot`, live ? 'build + test trên máy thật' : 'cần tester thiết bị thật', x + 22, 630, 10, A(C.dim, 210));
    [`p${i}Shadow`, `p${i}`, `p${i}Top`, `p${i}Dot`, `p${i}Name`, `p${i}Arch`, `p${i}State`].forEach((id) => anim(id, 'opacity', [[0.04 * d + i * 0.06 * d, 0], [0.12 * d + i * 0.06 * d, 1]], 'ease_out'));
    for (let k = 0; k < 4; k += 1) {
      [`p${i}R${k}`, `p${i}Bar`, `p${i}Fill`, `p${i}Foot`].forEach((id) => anim(id, 'opacity', [[0.16 * d + i * 0.06 * d + k * 0.02 * d, 0], [0.24 * d + i * 0.06 * d + k * 0.02 * d, 1]], 'ease_out'));
    }
  });

  // the ask, across all three
  const ask = 'KHÔNG CHỈ LÀ BIÊN DỊCH — CẦN NGƯỜI THỬ TRÊN MÁY THẬT';
  centre('ask', ask, 960, 860, 15, A(C.ink, 235));
  anim('ask', 'opacity', [[0.72 * d, 0], [0.8 * d, 1]], 'ease_out');
  rect('askLine', 960 - 300, 880, 600, 2, A(C.amber, 190));
  anim('askLine', 'width', [[0.72 * d, 0], [0.85 * d, 600]], 'ease_out');
  anim('askLine', 'translation_x', [[0.72 * d, -300], [0.85 * d, 0]], 'ease_out');

  return finish({
    name: 'Nền tảng — cái gì chạy, cái gì cần người',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: -80 },
      { t: 0.3 * d, zoom: 1.0, pan_x: 0 },
      { t: 0.6 * d, zoom: 1.04, pan_x: 60 },
      { t: 0.85 * d, zoom: 1.06, pan_x: 100 },
      { t: d, zoom: 1.0, pan_x: 0 },
    ],
  });
}

export function mobileQuestion(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 200, 1400, 1100, A(C.violet, 8));
  ell('glowB', 1250, 600, 1000, 900, A(C.blue, 6));

  // two phones, one problem
  const phones = [
    ['Android', 330, C.green],
    ['iOS', 1430, C.ink],
  ];
  phones.forEach(([name, x, color], i) => {
    rect(`ph${i}Case`, x, 220, 250, 500, { red: 18, green: 19, blue: 22, alpha: 255 }, { stroke: A(C.edge2, 180), stroke_width: 2 });
    rect(`ph${i}Screen`, x + 14, 234, 222, 472, { red: 10, green: 12, blue: 15, alpha: 255 });
    rect(`ph${i}Notch`, x + 95, 242, 60, 8, { red: 26, green: 27, blue: 30, alpha: 255 });
    txt(`ph${i}Name`, name, x + 24, 286, 12, A(color, 225));
    rect(`ph${i}Bar`, x + 24, 310, 202, 26, A(C.panel2, 255));
    rect(`ph${i}Prev`, x + 24, 352, 202, 130, { red: 14, green: 18, blue: 24, alpha: 255 });
    ell(`ph${i}Sun`, x + 150, 374, 40, 40, A(C.amber, 220));
    rect(`ph${i}Ground`, x + 24, 452, 202, 30, { red: 7, green: 9, blue: 12, alpha: 255 });
    for (let k = 0; k < 3; k += 1) rect(`ph${i}Track${k}`, x + 24, 500 + k * 34, 202, 16, A(C.panel2, 255));
    rect(`ph${i}No`, x + 24, 610, 202, 42, A(C.panel3, 255), { stroke: A(C.amber, 100), stroke_width: 1 });
    txt(`ph${i}NoT`, 'CHƯA CÓ TRỢ LÝ', x + 48, 636, 10, A(C.amber, 210));
    [`ph${i}Case`, `ph${i}Screen`, `ph${i}Notch`, `ph${i}Name`, `ph${i}Bar`, `ph${i}Prev`, `ph${i}Sun`, `ph${i}Ground`, `ph${i}No`, `ph${i}NoT`].forEach((id) => anim(id, 'opacity', [[0.04 * d + i * 0.06 * d, 0], [0.14 * d + i * 0.06 * d, 1]], 'ease_out'));
  });

  // the missing piece: a harness solution, drawn as a puzzle gap
  const Cx = 960;
  rect('gap', Cx - 130, 300, 260, 200, A(C.bg2, 220), { stroke: A(C.violet, 110), stroke_width: 1 });
  txt('gapT', '?', Cx - 22, 430, 72, A(C.violet, 150));
  txt('gapLabel', 'GIẢI PHÁP HARNESS', Cx - textW('GIẢI PHÁP HARNESS', 10) / 2, 524, 10, A(C.dim, 220));
  anim('gapT', 'scale', [[0.2 * d, 0.8], [0.35 * d, 1.1], [0.5 * d, 1]], 'ease_in_out');
  anim('gapT', 'opacity', [[0.2 * d, 0.3], [0.35 * d, 0.9], [0.5 * d, 0.5]], 'ease_in_out');

  // dotted ACP line that does not connect
  for (let i = 0; i < 12; i += 1) {
    rect(`dotL${i}`, 610 + i * 24, 392, 10, 3, A(C.violet, 130));
    rect(`dotR${i}`, 1120 + i * 24, 392, 10, 3, A(C.violet, 130));
  }

  // one boundary, many providers: the maintenance trap
  const cx2 = 960, cy2 = 700;
  ell('hub', cx2 - 26, cy2 - 26, 52, 52, C.panel3, { stroke: A(C.violet, 210), stroke_width: 1.5 });
  txt('hubT', 'ACP', cx2 - 14, cy2 + 6, 12, A(C.ink, 230));
  for (let i = 0; i < 9; i += 1) {
    const a = Math.PI + (i / 8) * Math.PI;
    const x = cx2 + Math.cos(a) * 190;
    const y = cy2 + Math.sin(a) * 100;
    ell(`prov${i}`, x - 9, y - 9, 18, 18, A(C.edge2, 190));
    rect(`line${i}`, Math.min(cx2, x), y, Math.abs(cx2 - x), 1, A(C.edge2, 90));
    anim(`prov${i}`, 'opacity', [[0.5 * d + i * 0.03 * d, 0], [0.56 * d + i * 0.03 * d, 1]], 'ease_out');
    anim(`line${i}`, 'opacity', [[0.5 * d + i * 0.03 * d, 0], [0.56 * d + i * 0.03 * d, 0.7]], 'ease_out');
  }
  txt('trap', 'một hệ agent riêng = hàng chục tích hợp phải nuôi', cx2 - textW('một hệ agent riêng = hàng chục tích hợp phải nuôi', 12) / 2, 816, 12, A(C.dim, 215));
  anim('trap', 'opacity', [[0.62 * d, 0], [0.7 * d, 1]], 'ease_out');

  tagChip('research', 'HƯỚNG NGHIÊN CỨU · CHƯA PHẢI LỜI HỨA', 120, 836, C.amber, 11);
  ['researchBg', 'researchT'].forEach((id) => anim(id, 'opacity', [[0.8 * d, 0], [0.86 * d, 1]], 'ease_out'));

  return finish({
    name: 'Mobile — bài toán harness',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: 50 },
      { t: 0.3 * d, zoom: 1.0, pan_x: 0 },
      { t: 0.55 * d, zoom: 1.04, pan_y: 30 },
      { t: 0.8 * d, zoom: 1.05, pan_y: 70 },
      { t: d, zoom: 1.0, pan_y: 0 },
    ],
  });
}

export function notarize(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 960, 260, 1400, 1000, A(C.amber, 7));

  // a package with a seal that is not a notarisation seal
  const P = { x: 620, y: 170, w: 680, h: 560 };
  rect('pkgShadow', P.x + 16, P.y + 18, P.w, P.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('pkg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('pkgBar', P.x, P.y, P.w, 46, C.panel2);
  ell('pkgDot', P.x + 20, P.y + 19, 9, 9, A(C.amber, 235));
  txt('pkgTitle', 'OPENCUT REINFORCED 0.1.0', P.x + 38, P.y + 23, 13, A(C.ink, 232));
  txt('pkgDmg', '.dmg · arm64', P.x + P.w - 18 - textW('.dmg · arm64', 10), P.y + 24, 10, A(C.dim, 210));

  // the app icon as a simple mark
  const cx = P.x + P.w / 2, cy = P.y + 220;
  ell('iconRing', cx - 74, cy - 74, 148, 148, A(C.amber, 0), { stroke: A(C.amber, 220), stroke_width: 2 });
  rect('iconDia', cx - 34, cy - 34, 68, 68, A(C.amber, 220), { rotation_degrees: 45 });
  rect('iconDiaIn', cx - 16, cy - 16, 32, 32, C.bg, { rotation_degrees: 45 });
  txt('pkgName', 'OpenCut Reinforced', cx - textW('OpenCut Reinforced', 24) / 2, P.y + 356, 24, A(C.ink, 240));
  txt('pkgSub', 'macOS · Apple Silicon', cx - textW('macOS · Apple Silicon', 12) / 2, P.y + 386, 12, A(C.mute, 220));

  // signing status, two separate lines
  rect('sigBox', P.x + 60, P.y + 426, P.w - 120, 82, A(C.panel2, 255), { stroke: A(C.edge, 170), stroke_width: 1 });
  ell('sigDot', P.x + 80, P.y + 446, 8, 8, A(C.amber, 235));
  txt('sig1', 'AD-HOC SIGNED', P.x + 96, P.y + 454, 11, A(C.amber, 225));
  txt('sig2', 'NOT NOTARIZED', P.x + 96, P.y + 478, 11, A(C.red, 220));
  txt('sig3', 'chưa có Apple Developer account', P.x + 96, P.y + 498, 9, A(C.dim, 210));

  // the dashed gate: a real release process is still ahead
  rect('gate', 320, 250, 2, 520, A(C.edge2, 120));
  for (let i = 0; i < 16; i += 1) rect(`gd${i}`, 320, 260 + i * 32, 2, 14, A(C.edge2, 170));
  txt('gateL', 'QUY TRÌNH', 150, 300, 12, A(C.dim, 220));
  txt('gateL2', 'PHÁT HÀNH', 150, 324, 12, A(C.dim, 220));
  txt('gateL3', 'ĐẦY ĐỦ', 150, 348, 12, A(C.dim, 220));
  rect('gateArrow', 210, 420, 80, 2, A(C.dim, 160));
  anim('gateArrow', 'translation_x', [[0.4 * d, 0], [0.8 * d, 40]], 'ease_in_out');
  txt('honest', 'một bản chạy được — không phải một quy trình đã hoàn chỉnh', 960 - textW('một bản chạy được — không phải một quy trình đã hoàn chỉnh', 15) / 2, 800, 15, A(C.mute, 230));
  anim('honest', 'opacity', [[0.62 * d, 0], [0.72 * d, 1]], 'ease_out');

  return finish({
    name: 'Notarize — điều chưa có',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: 20 },
      { t: 0.4 * d, zoom: 1.0 },
      { t: 0.7 * d, zoom: 1.03, pan_x: -20 },
      { t: d, zoom: 1.08, pan_y: 10 },
    ],
  });
}

export function community(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 960, 300, 1500, 1100, A(C.amber, 9));
  ell('glowB', 1200, 500, 1100, 900, A(C.violet, 7));

  // one main branch grows; contributors join from all directions
  const cx = 960, cy = 560;
  rect('main', cx - 2, 160, 4, 700, A(C.amber, 180));
  rect('mainTop', cx - 80, 160, 160, 4, A(C.amber, 220));
  ell('mainDot', cx - 10, 156, 24, 24, A(C.amber, 235));
  txt('mainT', 'main', cx + 22, 176, 11, A(C.amber, 225));

  const branches = [
    ['ui', -1, 280, C.violet, 0.1],
    ['media', 1, 360, C.cyan, 0.16],
    ['perf', -1, 460, C.blue, 0.22],
    ['package', 1, 540, C.green, 0.28],
    ['docs', -1, 620, C.pink, 0.34],
  ];
  branches.forEach(([name, side, y, color, t], i) => {
    const x2 = cx + side * 250;
    rect(`br${i}`, Math.min(cx, x2), y, 250, 3, A(color, 170));
    rect(`br${i}V`, x2, y, 3, 80, A(color, 170));
    ell(`br${i}Dot`, x2 - 8, y + 72, 16, 16, A(color, 230));
    txt(`br${i}T`, name, x2 + (side < 0 ? -textW(name, 11) - 16 : 16), y + 26, 11, A(color, 225));
    anim(`br${i}`, 'width', [[t * d, 0], [t * d + 0.08 * d, 250]], 'ease_out');
    anim(`br${i}V`, 'height', [[t * d + 0.05 * d, 0], [t * d + 0.1 * d, 80]], 'ease_out');
    anim(`br${i}Dot`, 'opacity', [[t * d + 0.08 * d, 0], [t * d + 0.12 * d, 1]], 'ease_out');
    anim(`br${i}T`, 'opacity', [[t * d + 0.02 * d, 0], [t * d + 0.08 * d, 1]], 'ease_out');
  });

  // contributor avatars on a loose ring
  for (let i = 0; i < 14; i += 1) {
    const a = (i / 14) * Math.PI * 2;
    const r = 340 + (i % 3) * 40;
    const x = cx + Math.cos(a) * r;
    const y = cy + Math.sin(a) * r * 0.55;
    ell(`av${i}`, x - 18, y - 18, 36, 36, A(i % 3 === 0 ? C.amber : i % 3 === 1 ? C.cyan : C.violet, 80 + (i % 4) * 30));
    ell(`av${i}R`, x - 18, y - 18, 36, 36, A(C.edge2, 0), { stroke: A(C.edge2, 120), stroke_width: 1 });
    const t0 = 0.36 * d + i * 0.02 * d;
    anim(`av${i}`, 'opacity', [[t0, 0], [t0 + 0.06 * d, 1]], 'ease_out');
    anim(`av${i}R`, 'opacity', [[t0, 0], [t0 + 0.06 * d, 1]], 'ease_out');
    anim(`av${i}`, 'scale', [[t0, 0.5], [t0 + 0.1 * d, 1]], 'ease_out');
  }

  // pull requests fly in, get a check, and merge
  const prs = [
    ['#12', 'fix: Windows codec probe', C.cyan, 0.46],
    ['#18', 'perf: preview pacing', C.blue, 0.54],
    ['#24', 'ui: compact timeline', C.violet, 0.62],
    ['#31', 'build: Linux package', C.green, 0.7],
  ];
  prs.forEach(([id, title, color, t], i) => {
    const x = 250 + i * 370;
    const y = 760;
    rect(`pr${i}`, x, y, 330, 74, A(C.panel, 245), { stroke: A(color, 160), stroke_width: 1 });
    txt(`pr${i}Id`, id, x + 14, y + 24, 10, A(color, 230));
    txt(`pr${i}T`, title, x + 14, y + 48, 10, A(C.mute, 220));
    ell(`pr${i}Dot`, x + 300, y + 20, 8, 8, A(C.green, 200));
    anim(`pr${i}`, 'opacity', [[t * d, 0], [t * d + 0.05 * d, 1]], 'ease_out');
    anim(`pr${i}`, 'translation_y', [[t * d, 30], [t * d + 0.08 * d, 0]], 'ease_out');
    anim(`pr${i}Dot`, 'scale', [[t * d + 0.06 * d, 0.4], [t * d + 0.1 * d, 1.2], [t * d + 0.14 * d, 1]], 'ease_out');
  });

  txt('comLine', 'một người bắt đầu · nhiều người làm tốt hơn', 960 - textW('một người bắt đầu · nhiều người làm tốt hơn', 18) / 2, 880, 18, A(C.ink, 235));
  anim('comLine', 'opacity', [[0.76 * d, 0], [0.84 * d, 1]], 'ease_out');

  return finish({
    name: 'Cộng đồng — nhiều nhánh, một sản phẩm',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_y: 20 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.65 * d, zoom: 1.05, pan_y: -10 },
      { t: 0.85 * d, zoom: 1.02 },
      { t: d, zoom: 1.08, pan_y: 10 },
    ],
  });
}

export function contribPath(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 600, 260, 1400, 1000, A(C.amber, 8));
  ell('glowB', 1500, 500, 1000, 900, A(C.violet, 7));

  // the repository, real and readable
  const repo = 'github.com/huou07/Opencut-Reinforced';
  const rsize = 31;
  const rx = 960 - textW(repo, rsize) / 2;
  txt('repo', repo, rx, 160, rsize, A(C.ink, 245));
  rect('repoUnder', 960 - 330, 184, 0, 3, A(C.amber, 220));
  anim('repoUnder', 'width', [[0.02 * d, 0], [0.14 * d, 660]], 'ease_out');
  anim('repoUnder', 'translation_x', [[0.02 * d, -330], [0.14 * d, 0]], 'ease_out');
  txt('repoLabel', 'MỞ · MIT · APPLE SILICON', 960 - textW('MỞ · MIT · APPLE SILICON', 11) / 2, 214, 11, A(C.dim, 220));

  // six steps, one path
  const steps = [
    ['FORK', 'repo của bạn', C.violet],
    ['CHANGE', 'một thay đổi nhỏ', C.cyan],
    ['TEST', 'kiểm tra phù hợp', C.blue],
    ['PULL REQUEST', 'mô tả rõ ràng', C.amber],
    ['REVIEW', 'trao đổi', C.green],
    ['MERGE', 'vào release', C.ink],
  ];
  const startX = 110, stepW = 270, gap = 24, y = 360;
  steps.forEach(([name, sub, color], i) => {
    const x = startX + i * (stepW + gap);
    rect(`s${i}Shadow`, x + 10, y + 12, stepW, 200, { red: 0, green: 0, blue: 0, alpha: 120 });
    rect(`s${i}`, x, y, stepW, 200, C.panel, { stroke: A(color, 150), stroke_width: 1 });
    rect(`s${i}Top`, x, y, stepW, 3, A(color, 220));
    txt(`s${i}N`, name, x + 20, y + 44, 18, A(C.ink, 238));
    txt(`s${i}D`, sub, x + 20, y + 76, 11, A(C.dim, 215));
    txt(`s${i}Num`, `0${i + 1}`, x + stepW - 36, y + 44, 12, A(color, 180));
    // a small diagram per step
    if (i === 0) {
      ell(`s${i}DiaA`, x + 30, y + 120, 26, 26, A(color, 0), { stroke: A(color, 170), stroke_width: 1 });
      ell(`s${i}DiaB`, x + 80, y + 120, 26, 26, A(color, 60));
      rect(`s${i}DiaL`, x + 58, y + 132, 20, 2, A(color, 140));
    } else if (i === 1) {
      for (let k = 0; k < 4; k += 1) rect(`s${i}Code${k}`, x + 28, y + 116 + k * 14, 120 - k * 18, 4, A(color, 110));
    } else if (i === 2) {
      rect(`s${i}Box`, x + 28, y + 116, 130, 54, A(C.bg2, 255), { stroke: A(color, 120), stroke_width: 1 });
      txt(`s${i}Ok`, 'OK', x + 78, y + 150, 13, A(color, 210));
    } else if (i === 3) {
      rect(`s${i}Pr`, x + 28, y + 112, 130, 58, A(C.bg2, 255), { stroke: A(color, 120), stroke_width: 1 });
      txt(`s${i}PrT`, '#42', x + 38, y + 136, 12, A(color, 220));
      txt(`s${i}PrD`, 'diff + notes', x + 38, y + 154, 9, A(C.dim, 210));
    } else if (i === 4) {
      ell(`s${i}B1`, x + 28, y + 118, 54, 34, A(color, 50));
      ell(`s${i}B2`, x + 88, y + 128, 54, 34, A(color, 30));
      txt(`s${i}Bt1`, 'review', x + 38, y + 140, 9, A(color, 220));
    } else {
      rect(`s${i}Merge`, x + 28, y + 116, 130, 2, A(color, 180));
      ell(`s${i}Md`, x + 88, y + 110, 14, 14, A(color, 230));
      txt(`s${i}Mt`, 'main', x + 108, y + 122, 9, A(C.dim, 210));
    }
    const t0 = 0.1 * d + i * 0.1 * d;
    [`s${i}Shadow`, `s${i}`, `s${i}Top`, `s${i}N`, `s${i}D`, `s${i}Num`].forEach((id) => anim(id, 'opacity', [[t0, 0], [t0 + 0.06 * d, 1]], 'ease_out'));
    anim(`s${i}`, 'translation_y', [[t0, 24], [t0 + 0.08 * d, 0]], 'ease_out');
    const diagram = i === 0
      ? [`s${i}DiaA`, `s${i}DiaB`, `s${i}DiaL`]
      : i === 1
        ? Array.from({ length: 4 }, (_, k) => `s${i}Code${k}`)
        : i === 2
          ? [`s${i}Box`, `s${i}Ok`]
          : i === 3
            ? [`s${i}Pr`, `s${i}PrT`, `s${i}PrD`]
            : i === 4
              ? [`s${i}B1`, `s${i}B2`, `s${i}Bt1`]
              : [`s${i}Merge`, `s${i}Md`, `s${i}Mt`];
    diagram.forEach((id) => anim(id, 'opacity', [[t0 + 0.02 * d, 0], [t0 + 0.08 * d, 1]], 'ease_out'));
  });

  // the packet travels the path
  for (let i = 0; i < 6; i += 1) {
    const x = startX + i * (stepW + gap) + stepW / 2;
    const t0 = 0.24 * d + i * 0.1 * d;
    ell(`pk${i}`, x - 8, y + 214, 16, 16, A(C.amber, 230));
    anim(`pk${i}`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 1], [t0 + 0.08 * d, 0]], 'ease_out');
    anim(`pk${i}`, 'scale', [[t0, 0.6], [t0 + 0.05 * d, 1.5]], 'ease_out');
    if (i < 5) {
      const nx = startX + (i + 1) * (stepW + gap) + stepW / 2;
      rect(`conn${i}`, x + 8, y + 220, nx - x - 16, 2, A(C.edge2, 100));
      anim(`conn${i}`, 'opacity', [[t0, 0], [t0 + 0.04 * d, 0.8]], 'ease_out');
    }
  }

  // what a contributor can pick
  const areas = ['GIAO DIỆN', 'MEDIA PIPELINE', 'HIỆU NĂNG', 'AI INTEGRATION', 'PACKAGING', 'TÀI LIỆU'];
  areas.forEach((area, i) => {
    const x = 170 + i * 272;
    tagChip(`area${i}`, area, x, 700, i % 2 ? C.cyan : C.dim, 10);
    ['Bg', 'T'].forEach((s) => anim(`area${i}${s}`, 'opacity', [[0.58 * d + i * 0.03 * d, 0], [0.64 * d + i * 0.03 * d, 1]], 'ease_out'));
  });

  // the honest check line
  txt('checkLine', 'chạy phần kiểm tra phù hợp · báo rõ phần chưa chạy được', 960 - textW('chạy phần kiểm tra phù hợp · báo rõ phần chưa chạy được', 14) / 2, 770, 14, A(C.mute, 230));
  anim('checkLine', 'opacity', [[0.7 * d, 0], [0.78 * d, 1]], 'ease_out');

  // final emphasis on PR
  rect('prRing', startX + 3 * (stepW + gap) - 6, y - 6, stepW + 12, 212, A(C.amber, 0), { stroke: A(C.amber, 180), stroke_width: 1 });
  anim('prRing', 'opacity', [[0.8 * d, 0], [0.86 * d, 1]], 'ease_out');

  return finish({
    name: 'Đóng góp — fork đến merge',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -60 },
      { t: 0.3 * d, zoom: 1.0, pan_x: 0 },
      { t: 0.6 * d, zoom: 1.03, pan_x: 60 },
      { t: 0.85 * d, zoom: 1.02, pan_x: 80 },
      { t: d, zoom: 1.0, pan_x: 0 },
    ],
  });
}

export function commitment(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 200, 1400, 1000, A(C.green, 7));
  ell('glowB', 1300, 500, 1000, 900, A(C.amber, 7));

  // an inbox of PRs, read one by one
  const inbox = [
    ['#42', 'media: Windows codec probe', 'đọc xong', C.cyan, 0.08],
    ['#43', 'ui: compact timeline', 'đang review', C.violet, 0.16],
    ['#44', 'docs: Linux build notes', 'đọc xong', C.amber, 0.24],
  ];
  inbox.forEach(([id, title, state, color, t], i) => {
    const x = 150, y = 200 + i * 110;
    rect(`i${i}`, x, y, 620, 88, A(C.panel, 245), { stroke: A(color, 140), stroke_width: 1 });
    txt(`i${i}Id`, id, x + 18, y + 28, 11, A(color, 230));
    txt(`i${i}T`, title, x + 18, y + 58, 13, A(C.ink, 235));
    txt(`i${i}S`, state, x + 480, y + 58, 10, A(C.dim, 215));
    anim(`i${i}`, 'opacity', [[t * d, 0], [t * d + 0.05 * d, 1]], 'ease_out');
    anim(`i${i}`, 'translation_x', [[t * d, -30], [t * d + 0.07 * d, 0]], 'ease_out');
  });

  // the review card: a diff, a comment, a merge
  const R = { x: 860, y: 180, w: 900, h: 540 };
  rect('rBg', R.x, R.y, R.w, R.h, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('rBar', R.x, R.y, R.w, 40, C.panel2);
  ell('rDot', R.x + 18, R.y + 16, 8, 8, A(C.green, 235));
  txt('rTitle', 'PULL REQUEST #42', R.x + 34, R.y + 20, 11, A(C.ink, 232));
  txt('rState', 'OPEN', R.x + R.w - 18 - textW('OPEN', 10), R.y + 21, 10, A(C.green, 215));
  const diff = [
    ['- if (target == "windows") return', C.red],
    ['+ probe_codec(target, profile)', C.green],
    ['+ if (!ok) report_platform_gap()', C.green],
  ];
  diff.forEach(([line, color], i) => {
    txt(`d${i}`, line, R.x + 30, R.y + 90 + i * 36, 15, A(color, 225));
    rect(`dBg${i}`, R.x + 20, R.y + 74 + i * 36, R.w - 40, 30, A(color, 12));
    anim(`dBg${i}`, 'opacity', [[0.34 * d + i * 0.05 * d, 0], [0.4 * d + i * 0.05 * d, 1]], 'ease_out');
    anim(`d${i}`, 'opacity', [[0.34 * d + i * 0.05 * d, 0], [0.4 * d + i * 0.05 * d, 1]], 'ease_out');
  });
  rect('rComment', R.x + 30, R.y + 220, R.w - 60, 60, A(C.panel2, 255), { stroke: A(C.edge, 170), stroke_width: 1 });
  txt('rCommentT', 'Cảm ơn bạn. Phần này chạy được trên Windows của bạn chứ?', R.x + 44, R.y + 256, 12, A(C.mute, 225));
  anim('rComment', 'opacity', [[0.5 * d, 0], [0.56 * d, 1]], 'ease_out');
  anim('rCommentT', 'opacity', [[0.52 * d, 0], [0.58 * d, 1]], 'ease_out');

  // merge button, pressed
  const B = { x: R.x + 30, y: R.y + 320, w: 240, h: 52 };
  rect('mergeBtn', B.x, B.y, B.w, B.h, A(C.green, 20), { stroke: A(C.green, 180), stroke_width: 1 });
  txt('mergeT', 'MERGE PULL REQUEST', B.x + 26, B.y + 32, 12, A(C.green, 235));
  anim('mergeBtn', 'opacity', [[0.74 * d, 0], [0.78 * d, 1]], 'ease_out');
  anim('mergeBtn', 'scale', [[0.8 * d, 1], [0.84 * d, 0.96], [0.88 * d, 1]], 'ease_out');
  tagChip('merged', 'ĐÃ HỢP NHẤT', B.x + 270, B.y + 30, C.green, 11);
  ['mergedBg', 'mergedT'].forEach((id) => anim(id, 'opacity', [[0.86 * d, 0], [0.9 * d, 1]], 'ease_out'));

  // mainline advances, release tag at the end
  rect('mainLine', R.x + 30, R.y + 430, R.w - 60, 3, A(C.edge2, 150));
  for (let i = 0; i < 5; i += 1) ell(`md${i}`, R.x + 60 + i * 150, R.y + 420, 14, 14, A(i === 4 ? C.amber : C.green, 210));
  anim('md4', 'opacity', [[0.88 * d, 0], [0.92 * d, 1]], 'ease_out');
  txt('mainLabel', 'main → release tiếp theo', R.x + 30, R.y + 466, 11, A(C.mute, 220));
  anim('mainLabel', 'opacity', [[0.88 * d, 0], [0.92 * d, 1]], 'ease_out');

  // the promise, in three verbs
  ['ĐỌC', 'REVIEW', 'MERGE'].forEach((word, i) => {
    const x = 260 + i * 210;
    txt(`pw${i}`, word, x, 790, 22, A(i === 2 ? C.amber : C.ink, 235));
    rect(`pwL${i}`, x, 810, 0, 2, A(i === 2 ? C.amber : C.dim, 180));
    anim(`pwL${i}`, 'width', [[0.7 * d + i * 0.05 * d, 0], [0.78 * d + i * 0.05 * d, 150]], 'ease_out');
    anim(`pw${i}`, 'opacity', [[0.66 * d + i * 0.05 * d, 0], [0.72 * d + i * 0.05 * d, 1]], 'ease_out');
  });
  txt('promise', 'Mình không hứa mọi PR đều được merge. Mình hứa sẽ đọc và nói rõ lý do.', 960 - textW('Mình không hứa mọi PR đều được merge. Mình hứa sẽ đọc và nói rõ lý do.', 14) / 2, 870, 14, A(C.mute, 230));
  anim('promise', 'opacity', [[0.82 * d, 0], [0.9 * d, 1]], 'ease_out');

  return finish({
    name: 'Cam kết — đọc, review, merge',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: -30 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.04, pan_x: 40 },
      { t: 0.85 * d, zoom: 1.06, pan_x: 60 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function close(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 700, 300, 1400, 1100, A(C.amber, 11));
  ell('glowB', 1050, 340, 1100, 900, A(C.violet, 8));

  const cx = 960, cy = 330;
  for (let i = 0; i < 4; i += 1) {
    const r = 150 + i * 110;
    ell(`ring${i}`, cx - r, cy - r, r * 2, r * 2, A(C.edge2, 0), { stroke: A(C.edge2, 35 - i * 6), stroke_width: 1 });
    anim(`ring${i}`, 'rotation', [[0, 0], [d, i % 2 ? 16 : -14]], 'linear');
  }
  // converging particles
  for (let i = 0; i < 44; i += 1) {
    const a = (i / 44) * Math.PI * 2;
    const dist = 620 + (i % 5) * 50;
    const x = cx + Math.cos(a) * dist;
    const y = cy + Math.sin(a) * dist * 0.6;
    ell(`p${i}`, x, y, 3, 3, A(i % 3 === 0 ? C.amber : C.mute, 120));
    anim(`p${i}`, 'translation_x', [[0, 0], [0.8 * d, Math.cos(a) * -dist * 0.86]], 'ease_in');
    anim(`p${i}`, 'translation_y', [[0, 0], [0.8 * d, Math.sin(a) * -dist * 0.6 * 0.86]], 'ease_in');
    anim(`p${i}`, 'opacity', [[0, 0.9], [0.7 * d, 0.9], [0.9 * d, 0]], 'ease_in');
  }

  // mark
  ell('markRing', cx - 48, cy - 48, 96, 96, A(C.amber, 0), { stroke: A(C.amber, 230), stroke_width: 3 });
  rect('markDia', cx - 21, cy - 21, 42, 42, A(C.amber, 220), { rotation_degrees: 45 });
  rect('markDiaIn', cx - 10, cy - 10, 20, 20, A(C.bg, 255), { rotation_degrees: 45 });
  rect('markL', cx - 78, cy - 2, 18, 4, A(C.amber, 200));
  rect('markR', cx + 60, cy - 2, 18, 4, A(C.amber, 200));
  ['markRing', 'markDia', 'markDiaIn', 'markL', 'markR'].forEach((id) => anim(id, 'opacity', [[0.12 * d, 0], [0.32 * d, 1]], 'ease_out'));
  anim('markRing', 'rotation', [[0.12 * d, -30], [0.6 * d, 0]], 'ease_out');
  anim('markRing', 'scale', [[0.12 * d, 0.8], [0.36 * d, 1]], 'ease_out');

  const word = 'OpenCut Reinforced';
  const wsize = 78;
  txt('word', word, cx - textW(word, wsize) / 2, 470, wsize, A(C.ink, 250));
  anim('word', 'opacity', [[0.3 * d, 0], [0.46 * d, 1]], 'ease_out');
  anim('word', 'translation_y', [[0.3 * d, 22], [0.46 * d, 0]], 'ease_out');

  const repo = 'github.com/huou07/Opencut-Reinforced';
  const rsize = 27;
  txt('repo', repo, cx - textW(repo, rsize) / 2, 550, rsize, A(C.amber, 240));
  anim('repo', 'opacity', [[0.46 * d, 0], [0.58 * d, 1]], 'ease_out');
  rect('repoUnder', cx - 280, 574, 0, 3, A(C.amber, 210));
  anim('repoUnder', 'width', [[0.5 * d, 0], [0.66 * d, 560]], 'ease_out');
  anim('repoUnder', 'translation_x', [[0.5 * d, -280], [0.66 * d, 0]], 'ease_out');

  tagChip('free', 'MIỄN PHÍ', 720, 640, C.green, 13);
  tagChip('mit', 'MIT', 850, 640, C.amber, 13);
  tagChip('local', 'LOCAL-FIRST', 940, 640, C.cyan, 13);
  tagChip('open', 'MÃ NGUỒN MỞ', 1090, 640, C.violet, 13);
  ['freeBg', 'freeT', 'mitBg', 'mitT', 'localBg', 'localT', 'openBg', 'openT'].forEach((id, i) => {
    anim(id, 'opacity', [[0.58 * d + i * 0.012 * d, 0], [0.66 * d + i * 0.012 * d, 1]], 'ease_out');
  });

  const line = 'một người bắt đầu · cộng đồng cùng xây';
  txt('line', line, cx - textW(line, 19) / 2, 730, 19, A(C.mute, 235));
  anim('line', 'opacity', [[0.7 * d, 0], [0.8 * d, 1]], 'ease_out');

  // the programme ends on the mark, still moving
  anim('glowA', 'scale', [[0, 1], [d, 1.1]], 'ease_in_out');
  anim('markDia', 'rotation', [[0.6 * d, 45], [d, 50]], 'ease_in_out');
  rect('endRule', cx - 160, 790, 320, 2, A(C.edge2, 160));
  anim('endRule', 'width', [[0.8 * d, 0], [d, 320]], 'ease_out');
  anim('endRule', 'translation_x', [[0.8 * d, -160], [d, 0]], 'ease_out');

  return finish({
    name: 'Kết — OpenCut Reinforced',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04 },
      { t: 0.4 * d, zoom: 1.0 },
      { t: 0.75 * d, zoom: 1.03 },
      { t: d, zoom: 1.08 },
    ],
  });
}

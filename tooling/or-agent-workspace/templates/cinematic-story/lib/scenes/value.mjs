// Value chapter: the licence and the local bill, then the feeling of owning the
// workspace. Free is stated precisely, including what still costs.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';
import { viewer, timeline, tagChip } from '../ui.mjs';

export function freeLicense(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 300, 200, 1300, 1000, A(C.green, 7));
  ell('glowB', 1300, 500, 1100, 900, A(C.amber, 7));

  // MIT licence, as a document rather than a badge
  const L = { x: 140, y: 140, w: 700, h: 700 };
  rect('lShadow', L.x + 16, L.y + 18, L.w, L.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('lPaper', L.x, L.y, L.w, L.h, { red: 236, green: 232, blue: 222, alpha: 255 });
  rect('lFold', L.x + L.w - 70, L.y, 70, 70, { red: 214, green: 209, blue: 197, alpha: 255 });
  rect('lFoldLine', L.x + L.w - 70, L.y + 70, 70, 2, { red: 190, green: 184, blue: 172, alpha: 255 }, { rotation_degrees: -45 });
  txt('lTitle', 'MIT License', L.x + 54, L.y + 92, 34, { red: 32, green: 34, blue: 38, alpha: 255 });
  txt('lCopy', 'Copyright (c) 2026 huou07', L.x + 54, L.y + 132, 13, { red: 92, green: 94, blue: 98, alpha: 255 });
  rect('lRule', L.x + 54, L.y + 158, L.w - 108, 1, { red: 196, green: 190, blue: 178, alpha: 255 });
  const body = [
    'Permission is hereby granted, free of charge,',
    'to any person obtaining a copy of this software',
    'and associated documentation files, to deal in',
    'the Software without restriction, including',
    'without limitation the rights to use, copy,',
    'modify, merge, publish, distribute, sublicense,',
    'and/or sell copies of the Software…',
  ];
  body.forEach((line, i) => {
    txt(`lB${i}`, line, L.x + 54, L.y + 196 + i * 26, 13, { red: 48, green: 50, blue: 54, alpha: 235 });
    anim(`lB${i}`, 'opacity', [[0.05 * d + i * 0.035 * d, 0], [0.12 * d + i * 0.035 * d, 1]], 'ease_out');
  });
  // seal
  ell('seal', L.x + L.w - 150, L.y + L.h - 150, 96, 96, A(C.green, 0), { stroke: A(C.green, 170), stroke_width: 2 });
  ell('sealIn', L.x + L.w - 134, L.y + L.h - 134, 64, 64, A(C.green, 0), { stroke: A(C.green, 110), stroke_width: 1 });
  txt('sealT', 'MIT', L.x + L.w - 126, L.y + L.h - 96, 22, A(C.green, 190));
  anim('seal', 'rotation', [[0.4 * d, -20], [0.9 * d, 0]], 'ease_out');
  ['seal', 'sealIn', 'sealT'].forEach((id) => anim(id, 'opacity', [[0.38 * d, 0], [0.55 * d, 1]], 'ease_out'));

  // cost ledger: what is zero, and what is not
  const R = { x: 980, y: 140, w: 800, h: 700 };
  rect('rShadow', R.x + 14, R.y + 16, R.w, R.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('rBg', R.x, R.y, R.w, R.h, C.panel, { stroke: A(C.edge2, 205), stroke_width: 1 });
  rect('rBar', R.x, R.y, R.w, 44, C.panel2);
  ell('rDot', R.x + 18, R.y + 18, 8, 8, A(C.green, 235));
  txt('rTitle', 'CHI PHÍ THEO TỪNG LẦN XỬ LÝ', R.x + 34, R.y + 22, 12, A(C.ink, 232));
  txt('rMoney', 'USD', R.x + R.w - 18 - textW('USD', 10), R.y + 23, 10, A(C.dim, 210));

  const ledger = [
    ['Giấy phép MIT', '0', C.green, 'dùng · sửa · chia sẻ'],
    ['VieNeu-TTS local', '0', C.cyan, 'mỗi câu đọc'],
    ['Whisper local', '0', C.blue, 'mỗi phút audio'],
    ['Chat Assistant', 'tuỳ provider', C.violet, 'khi nhà cung cấp cho phép'],
  ];
  ledger.forEach(([name, price, color, note], i) => {
    const y = R.y + 92 + i * 104;
    rect(`lrow${i}`, R.x + 24, y - 22, R.w - 48, 82, A(C.panel2, 240));
    rect(`lbar${i}`, R.x + 24, y - 22, 4, 82, A(color, 230));
    txt(`ln${i}`, name, R.x + 46, y, 17, A(C.ink, 238));
    txt(`lnote${i}`, note, R.x + 46, y + 26, 11, A(C.dim, 215));
    txt(`lprice${i}`, price, R.x + R.w - 46 - textW(price, 20), y + 10, 20, A(color, 230));
    [`lrow${i}`, `lbar${i}`, `ln${i}`, `lnote${i}`, `lprice${i}`].forEach((id) => anim(id, 'opacity', [[0.08 * d + i * 0.06 * d, 0], [0.14 * d + i * 0.06 * d, 1]], 'ease_out'));
    anim(`lrow${i}`, 'translation_x', [[0.08 * d + i * 0.06 * d, 24], [0.16 * d + i * 0.06 * d, 0]], 'ease_out');
  });

  // the conditions, small and clear
  const caveats = [
    ['PHẦN CỨNG', 'CPU / GPU đủ mạnh'],
    ['DUNG LƯỢNG', 'model và media'],
    ['TẢI MODEL', 'một lần, có thể lớn'],
    ['GIẤY PHÉP', 'theo từng model'],
  ];
  caveats.forEach(([k, v], i) => {
    const x = R.x + 24 + i * 188;
    txt(`cvK${i}`, k, x, R.y + 560, 9, A(C.dim, 220));
    txt(`cvV${i}`, v, x, R.y + 584, 11, A(C.mute, 220));
    anim(`cvK${i}`, 'opacity', [[0.72 * d + i * 0.03 * d, 0], [0.78 * d + i * 0.03 * d, 1]], 'ease_out');
    anim(`cvV${i}`, 'opacity', [[0.72 * d + i * 0.03 * d, 0], [0.78 * d + i * 0.03 * d, 1]], 'ease_out');
  });
  txt('rFoot', 'Không có gói thuê bao, watermark hay giới hạn số lần xuất.', R.x + 24, R.y + 638, 13, A(C.dim, 220));
  anim('rFoot', 'opacity', [[0.6 * d, 0], [0.68 * d, 1]], 'ease_out');

  // a thin line ties the licence to the machine
  rect('tie', L.x + L.w, 490, R.x - L.x - L.w, 1, A(C.green, 90));
  anim('tie', 'width', [[0.56 * d, 0], [0.7 * d, R.x - L.x - L.w]], 'ease_out');
  ell('tieDot', L.x + L.w - 4, 486, 9, 9, A(C.green, 220));

  particles('dust', 900, 400, 18, 420, C.green, d, { size: 1.5, speed: 12 });

  return finish({
    name: 'MIT — miễn phí, và cái gì không miễn phí',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -20 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.03, pan_x: 20 },
      { t: 0.85 * d, zoom: 1.05, pan_x: 40 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function ownership(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, { red: 9, green: 9, blue: 10, alpha: 255 });

  // a desk seen from above, lit by one warm pool
  ell('lampPool', 480, 120, 1300, 900, A(C.amber, 10));
  ell('lampPool2', 900, 300, 900, 650, A(C.amber, 6));
  rect('desk', -40, 560, 2000, 560, { red: 24, green: 20, blue: 16, alpha: 255 });
  rect('deskEdge', -40, 560, 2000, 2, { red: 54, green: 44, blue: 34, alpha: 255 });
  for (let i = 0; i < 9; i += 1) {
    rect(`grain${i}`, -40 + i * 240, 600 + (i % 3) * 110, 2000, 1, { red: 34, green: 28, blue: 22, alpha: 130 });
  }

  // laptop with OR open
  const L = { x: 500, y: 330, w: 760, h: 430 };
  rect('lapBase', L.x - 40, L.y + L.h, L.w + 80, 18, { red: 38, green: 36, blue: 34, alpha: 255 });
  rect('lapLid', L.x, L.y, L.w, L.h, { red: 18, green: 18, blue: 19, alpha: 255 }, { stroke: A(C.edge2, 150), stroke_width: 1 });
  rect('lapScreen', L.x + 14, L.y + 14, L.w - 28, L.h - 28, C.bg2);
  rect('lapBar', L.x + 14, L.y + 14, L.w - 28, 26, C.panel2);
  ell('lapDot', L.x + 28, L.y + 25, 6, 6, A(C.amber, 230));
  txt('lapTitle', 'OpenCut Reinforced', L.x + 42, L.y + 28, 9, A(C.ink, 210));
  // mini editor on the laptop
  rect('lapViewer', L.x + 30, L.y + 54, 420, 180, { red: 10, green: 13, blue: 18, alpha: 255 });
  ell('lapSun', L.x + 340, L.y + 76, 50, 50, A(C.amber, 225));
  rect('lapGround', L.x + 30, L.y + 176, 420, 58, { red: 7, green: 9, blue: 12, alpha: 255 });
  rect('lapBin', L.x + 470, L.y + 54, 120, 180, C.panel);
  for (let i = 0; i < 3; i += 1) rect(`lapBinRow${i}`, L.x + 482, L.y + 70 + i * 40, 96, 24, A(C.edge2, 150));
  for (let i = 0; i < 3; i += 1) {
    rect(`lapTrack${i}`, L.x + 30, L.y + 260 + i * 36, 560, 22, A(C.panel2, 255));
    rect(`lapClip${i}`, L.x + 40 + i * 40, L.y + 264 + i * 36, 180 + i * 60, 14, A(i === 0 ? C.amber : i === 1 ? C.cyan : C.blue, 180));
  }
  rect('lapPlay', L.x + 230, L.y + 254, 2, 104, A(C.red, 210));
  anim('lapPlay', 'translation_x', [[0, 0], [d, 260]], 'linear');

  // objects: a drive, headphones, a stack of project files, a phone
  rect('drive', 180, 700, 200, 120, { red: 32, green: 33, blue: 36, alpha: 255 }, { stroke: A(C.edge2, 140), stroke_width: 1 });
  rect('driveSlot', 200, 730, 120, 6, { red: 14, green: 15, blue: 17, alpha: 255 });
  ell('driveLed', 350, 760, 7, 7, A(C.green, 200));
  txt('driveT', 'PROJECTS', 200, 792, 9, A(C.dim, 210));

  // headphones as two circles and an arc
  ell('hpL', 1320, 640, 90, 90, { red: 28, green: 28, blue: 30, alpha: 255 }, { stroke: A(C.edge2, 170), stroke_width: 2 });
  ell('hpR', 1500, 640, 90, 90, { red: 28, green: 28, blue: 30, alpha: 255 }, { stroke: A(C.edge2, 170), stroke_width: 2 });
  rect('hpArc', 1360, 600, 190, 8, { red: 28, green: 28, blue: 30, alpha: 255 });
  rect('hpArcL', 1320, 600, 40, 8, { red: 28, green: 28, blue: 30, alpha: 255 }, { rotation_degrees: -30 });
  rect('hpArcR', 1550, 600, 40, 8, { red: 28, green: 28, blue: 30, alpha: 255 }, { rotation_degrees: 30 });

  // papers: projects, models, media
  const stack = [
    ['model', C.violet, 1400, 300, -6],
    ['media', C.cyan, 1460, 330, 4],
    ['project', C.amber, 1430, 360, -2],
  ];
  stack.forEach(([label, color, x, y, rot], i) => {
    rect(`paper${i}`, x, y, 250, 150, { red: 30, green: 27, blue: 23, alpha: 255 }, { rotation_degrees: rot, stroke: A(C.edge2, 110), stroke_width: 1 });
    rect(`paper${i}Tab`, x + 18, y + 20, 5, 110, A(color, 200), { rotation_degrees: rot });
    txt(`paper${i}T`, label.toUpperCase(), x + 36, y + 46, 11, A(color, 220), { rotation_degrees: rot });
  });

  // unplugged network cable: the point is local ownership
  rect('cable', 1560, 780, 300, 6, { red: 30, green: 30, blue: 32, alpha: 255 }, { rotation_degrees: -12 });
  rect('plug', 1530, 762, 34, 26, { red: 44, green: 44, blue: 46, alpha: 255 }, { rotation_degrees: -12 });
  rect('jack', 1840, 690, 44, 34, { red: 22, green: 22, blue: 24, alpha: 255 }, { stroke: A(C.edge2, 140), stroke_width: 1 });
  rect('jackGlow', 1838, 688, 48, 38, A(C.amber, 0), { stroke: A(C.amber, 100), stroke_width: 1 });
  txt('offline', 'OFFLINE', 1750, 760, 10, A(C.green, 220));

  // labels, small, tied to things
  const labels = [
    ['DỰ ÁN', 560, 300, C.amber, 500, 330],
    ['MEDIA', 260, 660, C.cyan, 280, 700],
    ['MODEL', 1420, 260, C.violet, 1450, 300],
    ['CÔNG CỤ', 1260, 760, C.green, 1320, 700],
  ];
  labels.forEach(([text, x, y, color, tx, ty], i) => {
    txt(`lb${i}`, text, x, y, 11, A(color, 225));
    rect(`lbLine${i}`, tx, ty - 12, 1, 42, A(color, 120));
    anim(`lb${i}`, 'opacity', [[0.18 * d + i * 0.08 * d, 0], [0.25 * d + i * 0.08 * d, 1]], 'ease_out');
    anim(`lbLine${i}`, 'opacity', [[0.18 * d + i * 0.08 * d, 0], [0.25 * d + i * 0.08 * d, 1]], 'ease_out');
  });

  txt('ownLine', 'Dự án, media, công cụ và quá trình sản xuất — trên máy của bạn.', 960 - textW('Dự án, media, công cụ và quá trình sản xuất — trên máy của bạn.', 18) / 2, 878, 18, A(C.mute, 235));
  anim('ownLine', 'opacity', [[0.7 * d, 0], [0.8 * d, 1]], 'ease_out');

  // slow warm sweep, no dead frame
  rect('sweep', -200, 0, 120, 1080, A(C.amber, 8), { rotation_degrees: 8 });
  anim('sweep', 'translation_x', [[0, 0], [d, 2200]], 'linear');

  return finish({
    name: 'Trên máy của bạn',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: -30, pan_y: 20 },
      { t: 0.3 * d, zoom: 1.0, pan_x: 0, pan_y: 0 },
      { t: 0.6 * d, zoom: 1.03, pan_x: 40, pan_y: 10 },
      { t: 0.85 * d, zoom: 1.02, pan_x: 10 },
      { t: d, zoom: 1.08, pan_x: 30, pan_y: 20 },
    ],
  });
}

// Extra beats created when the measured narration made a few windows longer than
// one composer should carry. Each one owns a single idea.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since, arrow } from '../kit.mjs';
import { timeline, playhead, viewer, tagChip, miniShot, waveformBlock } from '../ui.mjs';

export function layersAlign(duration) {
  begin();
  const d = duration;
  const tSnap = 0.3 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 160, 1400, 1000, A(C.blue, 8));
  ell('glowB', 300, 500, 900, 800, A(C.amber, 7));

  // four strips, offset in time; they must line up
  const strips = [
    ['pic', 300, C.amber, 'HÌNH ẢNH'],
    ['snd', 400, C.cyan, 'ÂM THANH'],
    ['voi', 500, C.violet, 'GIỌNG ĐỌC'],
    ['cap', 600, C.blue, 'PHỤ ĐỀ'],
  ];
  const syncX = 860;
  strips.forEach(([prefix, y, color, name], i) => {
    rect(`${prefix}Bg`, 200, y, 1520, 70, A(C.panel2, 245), { stroke: A(C.edge, 160), stroke_width: 1 });
    rect(`${prefix}Tag`, 200, y, 58, 70, A(C.bg2, 255));
    txt(`${prefix}TagT`, name.slice(0, 5), 210, y + 40, 9, A(color, 230));
    // content sits offset at first
    const off = [-180, 120, -60, 260][i];
    rect(`${prefix}Bar`, 300 + off, y + 20, 360, 30, A(color, 190));
    rect(`${prefix}Bar2`, 710 + off, y + 20, 250, 30, A(color, 130));
    anim(`${prefix}Bar`, 'translation_x', [[0, 0], [tSnap + i * 0.02 * d, -off]], 'ease_in_out');
    anim(`${prefix}Bar2`, 'translation_x', [[0, 0], [tSnap + i * 0.02 * d, -off]], 'ease_in_out');
    // after snapping, a pulse runs along the strip
    rect(`${prefix}Pulse`, syncX, y + 10, 3, 50, A(C.ink, 0));
    anim(`${prefix}Pulse`, 'opacity', [[0.56 * d + i * 0.04 * d, 0], [0.62 * d + i * 0.04 * d, 0.9], [0.76 * d + i * 0.04 * d, 0]], 'ease_out');
    anim(`${prefix}Pulse`, 'translation_x', [[0.56 * d, 0], [0.8 * d, 680]], 'linear');
  });

  // vertical sync line: the exact frame where they line up
  rect('sync', syncX, 260, 2, 450, A(C.red, 210));
  rect('syncTop', syncX - 8, 260, 18, 6, A(C.red, 225));
  anim('sync', 'opacity', [[0, 0.3], [tSnap, 1]], 'ease_in_out');
  txt('syncT', 'đồng bộ · cùng một khung', syncX + 16, 250, 11, A(C.mute, 220));

  // preview above: the combined result
  const V = { x: 1240, y: 150, w: 480, h: 260 };
  rect('pvCase', V.x - 8, V.y - 8, V.w + 16, V.h + 16, C.panel);
  miniShot('pv', V.x, V.y, V.w, V.h, C.amber);
  rect('pvFrame', V.x, V.y, V.w, V.h, A(C.ink, 0), { stroke: A(C.edge2, 180), stroke_width: 1 });
  txt('pvLabel', 'PREVIEW', V.x + 10, V.y + 18, 9, A(C.mute, 220));
  anim('pvSun', 'translation_y', [[0, 10], [d, -6]], 'ease_in_out');

  // bottom timeline: the four strips collapse into tracks
  const T = { x: 200, y: 710, w: 1520, h: 180 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 180), stroke_width: 1 });
  [C.amber, C.cyan, C.violet, C.blue].forEach((color, i) => {
    rect(`tlLane${i}`, T.x + 12, T.y + 14 + i * 40, T.w - 24, 32, A(C.panel2, 255));
    rect(`tlClip${i}`, T.x + 300 + i * 20, T.y + 18 + i * 40, 380 - i * 30, 24, A(color, 180));
  });
  rect('tlPlay', T.x + 300, T.y + 10, 2, T.h - 20, A(C.red, 220));
  anim('tlPlay', 'translation_x', [[0.5 * d, 0], [d, 620]], 'linear');
  txt('tlLabel', 'MỘT TIMELINE · BỐN LỚP', T.x + 12, T.y + 136, 10, A(C.dim, 215));

  // after alignment, a single compound ring closes
  ell('lockRing', syncX - 18, 470, 36, 36, A(C.green, 0), { stroke: A(C.green, 190), stroke_width: 2 });
  anim('lockRing', 'scale', [[0.62 * d, 0.4], [0.74 * d, 1]], 'ease_out');
  anim('lockRing', 'opacity', [[0.62 * d, 0], [0.7 * d, 1]], 'ease_out');

  return finish({
    name: 'Bốn lớp thẳng hàng',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: -20 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.03, pan_y: 10 },
      { t: 0.85 * d, zoom: 1.06, pan_y: 30 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function cutRipple(duration) {
  begin();
  const d = duration;
  const Y = 420, H = 210;
  const cut = 900;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 500, 120, 1500, 1000, A(C.amber, 8));
  ell('glowB', 1200, 600, 1000, 900, A(C.cyan, 7));

  // one clip, macro scale: the cut is the whole picture
  rect('clip', 180, Y, 1560, H, A(C.amber, 175));
  rect('clipCap', 180, Y, 1560, 3, A(C.amber, 245));
  for (let i = 0; i < 70; i += 1) {
    const bh = 10 + ((i * 37) % 10);
    rect(`cw${i}`, 200 + i * 21.6, Y + H / 2 - bh / 2, 4, bh, A(C.bg, 60));
  }
  txt('clipName', 'shot-03 · 00:00:08:04', 200, Y + 38, 12, A(C.bg, 180));

  // playhead arrives at the cut
  rect('play', cut, 260, 2, 480, A(C.red, 225));
  rect('playHead', cut - 9, 260, 20, 7, A(C.red, 235));
  anim('play', 'translation_x', [[0, -420], [0.18 * d, 0]], 'ease_out');
  anim('playHead', 'translation_x', [[0, -420], [0.18 * d, 0]], 'ease_out');

  // split
  rect('split', cut, Y - 10, 2, H + 20, A(C.ink, 240));
  anim('split', 'opacity', [[0.18 * d, 0], [0.2 * d, 1], [0.34 * d, 1], [0.38 * d, 0]], 'ease_out');
  rect('rightHalf', cut + 2, Y, 740, H, A(C.teal, 175));
  rect('rightCap', cut + 2, Y, 740, 3, A(C.teal, 245));
  anim('rightHalf', 'opacity', [[0.2 * d, 0], [0.23 * d, 1]], 'ease_out');
  anim('rightCap', 'opacity', [[0.2 * d, 0], [0.23 * d, 1]], 'ease_out');
  // the right half is removed; the next clip ripples in
  anim('rightHalf', 'opacity', [[0.38 * d, 1], [0.42 * d, 0]], 'ease_in');
  anim('rightCap', 'opacity', [[0.38 * d, 1], [0.42 * d, 0]], 'ease_in');
  rect('nextClip', 1110, Y, 630, H, A(C.amber, 160));
  rect('nextCap', 1110, Y, 630, 3, A(C.amber, 220));
  anim('nextClip', 'translation_x', [[0.38 * d, 400], [0.56 * d, 0]], 'ease_in_out');
  anim('nextCap', 'translation_x', [[0.38 * d, 400], [0.56 * d, 0]], 'ease_in_out');

  // move a piece to an overlay track
  const V2 = 250;
  rect('overlay', 620, V2, 420, 120, A(C.violet, 175));
  rect('overlayCap', 620, V2, 420, 3, A(C.violet, 240));
  anim('overlay', 'opacity', [[0.6 * d, 0], [0.64 * d, 1]], 'ease_out');
  anim('overlayCap', 'opacity', [[0.6 * d, 0], [0.64 * d, 1]], 'ease_out');
  anim('overlay', 'translation_y', [[0.6 * d, 120], [0.72 * d, 0]], 'ease_out');
  anim('overlayCap', 'translation_y', [[0.6 * d, 120], [0.72 * d, 0]], 'ease_out');

  // trim the right edge
  rect('trim', 1740, Y - 8, 4, H + 16, A(C.ink, 235));
  anim('trim', 'translation_x', [[0.78 * d, 0], [0.88 * d, -160]], 'ease_in_out');
  anim('trim', 'opacity', [[0.76 * d, 0], [0.78 * d, 1], [0.9 * d, 1], [0.92 * d, 0]], 'ease_out');

  // action labels, small, at the top
  const labels = [
    ['SPLIT', C.cyan, 0.18],
    ['RIPPLE DELETE', C.amber, 0.38],
    ['MOVE TO V2', C.violet, 0.6],
    ['TRIM', C.green, 0.78],
  ];
  labels.forEach(([label, color, t], i) => {
    tagChip(`ac${i}`, label, 180 + i * 330, 180, color, 12);
    ['Bg', 'T'].forEach((s) => anim(`ac${i}${s}`, 'opacity', [[t * d, 0], [t * d + 0.02 * d, 1], [(t + 0.14) * d, 1], [(t + 0.18) * d, 0.35]], 'ease_out'));
  });

  // timecode and a history chip
  txt('tcBig', '00:00:14:02', 180, 300, 25, A(C.ink, 235));
  tagChip('undo', '⌘Z  UNDO', 1500, 300, C.dim, 12);
  ['undoBg', 'undoT'].forEach((id) => anim(id, 'opacity', [[0.86 * d, 0], [0.9 * d, 1]], 'ease_out'));

  particles('dust', 900, 700, 18, 360, C.amber, d, { size: 1.5, speed: 14 });

  return finish({
    name: 'Cắt và ripple — cận cảnh',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: -40, pan_y: 30 },
      { t: 0.3 * d, zoom: 1.0, pan_x: 0, pan_y: 0 },
      { t: 0.6 * d, zoom: 1.05, pan_x: 30, pan_y: -10 },
      { t: 0.85 * d, zoom: 1.08, pan_x: 50 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function assistantRole(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 400, 200, 1300, 1000, A(C.violet, 8));
  ell('glowB', 1300, 500, 900, 800, A(C.amber, 7));

  // human timeline on the left
  const T = { x: 154, y: 300, w: 900, h: 300 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 180), stroke_width: 1 });
  const lanes = [['V2', C.violet], ['V1', C.amber], ['A1', C.cyan]];
  const laneH = (T.h - 20) / 3;
  lanes.forEach(([name, color], i) => {
    const y = T.y + 10 + i * laneH;
    rect(`lane${i}`, T.x + 12, y, T.w - 24, laneH - 4, A(C.panel2, 255));
    txt(`laneT${i}`, name, T.x + 22, y + laneH / 2, 10, A(color, 230));
  });
  rect('clipA', 220, T.y + 10 + laneH, 260, laneH - 8, A(C.amber, 185));
  rect('clipB', 500, T.y + 10 + laneH, 220, laneH - 8, A(C.teal, 185));
  rect('clipC', 740, T.y + 10 + laneH, 250, laneH - 8, A(C.amber, 185));
  rect('capA', 220, T.y + 10, 420, laneH - 8, A(C.blue, 150));
  rect('voiceA', 220, T.y + 10 + laneH * 2, 620, laneH - 8, A(C.cyan, 170));

  // hand cursor grabs the playhead and moves it deliberately
  rect('play', 480, T.y + 10, 2, T.h - 10, A(C.red, 220));
  anim('play', 'translation_x', [[0.1 * d, 0], [0.4 * d, 180], [0.7 * d, 120]], 'ease_in_out');
  Cursor('cur', 470, 470, C.ink);
  function Cursor(prefix, x, y, color) {
    rect(`${prefix}A`, x, y, 6, 26, color, { rotation_degrees: -18 });
    rect(`${prefix}B`, x + 4, y + 16, 6, 18, color, { rotation_degrees: 32 });
    ell(`${prefix}Dot`, x - 4, y - 4, 12, 12, A(color, 220));
  }
  anim('curA', 'translation_x', [[0.1 * d, 0], [0.4 * d, 180], [0.7 * d, 120]], 'ease_in_out');
  anim('curA', 'translation_y', [[0.1 * d, 0], [0.4 * d, -20], [0.7 * d, 0]], 'ease_in_out');
  ['curB', 'curDot'].forEach((id) => anim(id, 'translation_x', [[0.1 * d, 0], [0.4 * d, 180], [0.7 * d, 120]], 'ease_in_out'));

  // chat on the right: a suggestion, not a command
  const P = { x: 1220, y: 220, w: 540, h: 480 };
  rect('pBg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('pHead', P.x, P.y, P.w, 40, C.panel2);
  ell('pDot', P.x + 18, P.y + 16, 8, 8, A(C.violet, 230));
  txt('pTitle', 'CHAT ASSISTANT', P.x + 34, P.y + 20, 11, A(C.ink, 230));
  rect('pBubble', P.x + 24, P.y + 70, P.w - 48, 80, A(C.panel3, 255));
  txt('pBubbleT1', 'Mình có thể dựng cảnh này', P.x + 40, P.y + 98, 14, A(C.ink, 232));
  txt('pBubbleT2', 'và thêm phụ đề. Bạn duyệt nhé?', P.x + 40, P.y + 122, 14, A(C.ink, 232));
  tagChip('approve', 'DUYỆT', P.x + 24, P.y + 186, C.green, 12);
  tagChip('edit', 'SỬA', P.x + 130, P.y + 186, C.amber, 12);
  tagChip('skip', 'BỎ QUA', P.x + 220, P.y + 186, C.dim, 12);
  ['approveBg', 'approveT', 'editBg', 'editT', 'skipBg', 'skipT'].forEach((id) => anim(id, 'opacity', [[0.5 * d, 0], [0.58 * d, 1]], 'ease_out'));
  txt('pFoot', 'trợ lý làm việc lặp lại', P.x + 24, P.y + 300, 12, A(C.dim, 215));
  txt('pFoot2', 'bạn giữ nhịp điệu', P.x + 24, P.y + 324, 12, A(C.mute, 225));

  // a dashed line: the assistant proposes, the human decides
  for (let i = 0; i < 14; i += 1) rect(`dash${i}`, 1060 + i * 12, 460, 7, 2, A(C.violet, 120));
  txt('dashT', 'ĐỀ XUẤT', 1080, 440, 9, A(C.violet, 220));

  return finish({
    name: 'Trợ lý đề xuất, bạn quyết định',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: -30 },
      { t: 0.4 * d, zoom: 1.0 },
      { t: 0.75 * d, zoom: 1.05, pan_x: 40 },
      { t: d, zoom: 1.08, pan_x: 60 },
    ],
  });
}

export function harnessRail(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 400, 200, 1300, 1000, A(C.violet, 8));
  ell('glowB', 1400, 500, 900, 800, A(C.blue, 7));

  // settings card: the harness list, with only the installed one live
  const P = { x: 140, y: 160, w: 720, h: 650 };
  rect('pShadow', P.x + 12, P.y + 14, P.w, P.h, { red: 0, green: 0, blue: 0, alpha: 140 });
  rect('pBg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 205), stroke_width: 1 });
  rect('pBar', P.x, P.y, P.w, 42, C.panel2);
  ell('pDot', P.x + 18, P.y + 17, 8, 8, A(C.violet, 235));
  txt('pTitle', 'ASSISTANT · HARNESS', P.x + 34, P.y + 21, 12, A(C.ink, 232));
  txt('pMode', 'ACP STDIO', P.x + P.w - 18 - textW('ACP STDIO', 10), P.y + 22, 10, A(C.dim, 210));
  txt('pLabel', 'HARNESS', P.x + 24, P.y + 72, 10, A(C.dim, 220));
  const rows = [
    ['DeepSeek Harness', 'đã cài · đang dùng', C.violet, 1],
    ['Claude Code (ACP)', 'cần cài trên máy', C.amber, 0],
    ['Codex (ACP)', 'cần cài trên máy', C.cyan, 0],
    ['OpenCode (ACP)', 'cần cài trên máy', C.green, 0],
    ['Custom ACP command', 'trỏ tới một lệnh', C.dim, 0],
  ];
  rows.forEach(([name, state, color, live], i) => {
    const y = P.y + 92 + i * 74;
    rect(`h${i}`, P.x + 24, y, P.w - 48, 62, A(live ? C.panel3 : C.panel2, 245), { stroke: A(color, live ? 190 : 85), stroke_width: 1 });
    ell(`h${i}Dot`, P.x + 42, y + 20, 9, 9, A(color, live ? 240 : 100));
    txt(`h${i}N`, name, P.x + 62, y + 26, 15, A(live ? C.ink : C.mute, 235));
    txt(`h${i}S`, state, P.x + 62, y + 48, 10, A(live ? color : C.dim, 210));
    if (live) {
      tagChip(`h${i}Tag`, 'ĐANG CHẠY', P.x + P.w - 132, y + 38, color, 9);
    }
    [`h${i}`, `h${i}Dot`, `h${i}N`, `h${i}S`].forEach((id) => anim(id, 'opacity', [[0.08 * d + i * 0.04 * d, 0], [0.14 * d + i * 0.04 * d, 1]], 'ease_out'));
  });
  txt('pFoot', 'OR không cài hộ harness, không giữ khoá.', P.x + 24, P.y + 500, 12, A(C.dim, 220));
  txt('pFoot2', 'Chọn harness nào đã có trên máy bạn.', P.x + 24, P.y + 524, 12, A(C.mute, 225));
  anim('pFoot', 'opacity', [[0.45 * d, 0], [0.52 * d, 1]], 'ease_out');
  anim('pFoot2', 'opacity', [[0.45 * d, 0], [0.52 * d, 1]], 'ease_out');

  // credentials stay outside; the project history stays inside
  const O = { x: 980, y: 190, w: 780, h: 250 };
  rect('oBg', O.x, O.y, O.w, O.h, C.bg2, { stroke: A(C.edge2, 180), stroke_width: 1 });
  txt('oTitle', 'DỰ ÁN · LỊCH SỬ', O.x + 20, O.y + 28, 10, A(C.ink, 225));
  const cmds = ['timeline.insert-clip', 'timeline.split-clip', 'motion.publish', 'captions.import'];
  cmds.forEach((cmd, i) => {
    const y = O.y + 56 + i * 38;
    txt(`cmd${i}`, cmd, O.x + 34, y + 14, 11, A(C.mute, 220));
    ell(`cmd${i}Dot`, O.x + O.w - 36, y + 6, 8, 8, A(C.green, 200));
    anim(`cmd${i}`, 'opacity', [[0.5 * d + i * 0.04 * d, 0], [0.56 * d + i * 0.04 * d, 1]], 'ease_out');
    anim(`cmd${i}Dot`, 'opacity', [[0.5 * d + i * 0.04 * d, 0], [0.56 * d + i * 0.04 * d, 1]], 'ease_out');
  });

  // a key outside the boundary, crossed out: OR never receives it
  const K = { x: 1180, y: 500, w: 420, h: 250 };
  rect('kBound', K.x - 20, K.y - 20, 2, K.h + 40, A(C.edge2, 110));
  for (let i = 0; i < 10; i += 1) rect(`kd${i}`, K.x - 20, K.y - 10 + i * 26, 2, 10, A(C.edge2, 160));
  ell('keyRing', K.x + 110, K.y + 70, 60, 60, A(C.amber, 0), { stroke: A(C.amber, 190), stroke_width: 2 });
  rect('keyStem', K.x + 166, K.y + 96, 90, 8, A(C.amber, 190));
  rect('keyToothA', K.x + 230, K.y + 104, 8, 20, A(C.amber, 190));
  rect('keyToothB', K.x + 250, K.y + 104, 8, 14, A(C.amber, 190));
  rect('keyCrossA', K.x + 70, K.y + 120, 160, 3, A(C.red, 200), { rotation_degrees: -24 });
  rect('keyCrossB', K.x + 70, K.y + 118, 160, 3, A(C.red, 200), { rotation_degrees: 24 });
  txt('keyLabel', 'KHOÁ Ở LẠI HARNESS', K.x + 44, K.y + 190, 11, A(C.mute, 225));
  anim('keyRing', 'rotation', [[0.6 * d, 0], [0.9 * d, 18]], 'ease_in_out');

  return finish({
    name: 'Harness — cái gì có sẵn, cái gì cần cài',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: -20 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.03, pan_x: 40 },
      { t: 0.85 * d, zoom: 1.05, pan_x: 60 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function mitDoc(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 700, 200, 1300, 1000, A(C.green, 7));
  ell('glowB', 1200, 500, 1000, 900, A(C.amber, 6));

  const L = { x: 360, y: 120, w: 760, h: 740 };
  rect('lShadow', L.x + 18, L.y + 20, L.w, L.h, { red: 0, green: 0, blue: 0, alpha: 155 });
  rect('lPaper', L.x, L.y, L.w, L.h, { red: 236, green: 232, blue: 222, alpha: 255 });
  rect('lFold', L.x + L.w - 76, L.y, 76, 76, { red: 214, green: 209, blue: 197, alpha: 255 });
  txt('lTitle', 'MIT License', L.x + 60, L.y + 100, 38, { red: 32, green: 34, blue: 38, alpha: 255 });
  txt('lSub', 'OpenCut Reinforced · 2026', L.x + 60, L.y + 142, 13, { red: 92, green: 94, blue: 98, alpha: 255 });
  rect('lRule', L.x + 60, L.y + 170, L.w - 120, 1, { red: 196, green: 190, blue: 178, alpha: 255 });
  const body = [
    'Permission is hereby granted, free of charge, to any',
    'person obtaining a copy of this software and associated',
    'documentation files, to deal in the Software without',
    'restriction, including without limitation the rights.',
  ];
  body.forEach((line, i) => txt(`lB${i}`, line, L.x + 60, L.y + 210 + i * 26, 13, { red: 48, green: 50, blue: 54, alpha: 235 }));

  // three freedoms, large enough to read but not a title card
  const freedoms = [
    ['DÙNG', 'cho bất kỳ mục đích nào', C.green],
    ['SỬA', 'và giữ bản của bạn', C.cyan],
    ['CHIA SẺ', 'bản gốc hoặc đã sửa', C.amber],
  ];
  freedoms.forEach(([verb, note, color], i) => {
    const y = L.y + 360 + i * 92;
    txt(`f${i}V`, verb, L.x + 60, y, 26, { red: 28, green: 30, blue: 34, alpha: 255 });
    txt(`f${i}N`, note, L.x + 220, y - 2, 13, { red: 84, green: 86, blue: 90, alpha: 240 });
    rect(`f${i}Bar`, L.x + 60, y + 14, 0, 3, A(color, 210));
    anim(`f${i}Bar`, 'width', [[0.3 * d + i * 0.08 * d, 0], [0.46 * d + i * 0.08 * d, 130]], 'ease_out');
    anim(`f${i}V`, 'opacity', [[0.24 * d + i * 0.08 * d, 0], [0.34 * d + i * 0.08 * d, 1]], 'ease_out');
    anim(`f${i}N`, 'opacity', [[0.28 * d + i * 0.08 * d, 0], [0.38 * d + i * 0.08 * d, 1]], 'ease_out');
  });

  // seal
  ell('seal', L.x + L.w - 150, L.y + L.h - 150, 100, 100, A(C.green, 0), { stroke: A(C.green, 170), stroke_width: 2 });
  ell('sealIn', L.x + L.w - 132, L.y + L.h - 132, 64, 64, A(C.green, 0), { stroke: A(C.green, 110), stroke_width: 1 });
  txt('sealT', 'MIT', L.x + L.w - 125, L.y + L.h - 96, 22, A(C.green, 190));
  ['seal', 'sealIn', 'sealT'].forEach((id) => anim(id, 'opacity', [[0.64 * d, 0], [0.74 * d, 1]], 'ease_out'));

  // a small counterpoint: not everything a video touches is free
  const R = { x: 1200, y: 220, w: 560, h: 540 };
  rect('rBg', R.x, R.y, R.w, R.h, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  txt('rTitle', 'MIỄN PHÍ Ở ĐÂU', R.x + 24, R.y + 44, 12, A(C.dim, 225));
  const rows = [
    ['Phần mềm', 'MIT', C.green],
    ['Track & xuất', 'không giới hạn', C.green],
    ['Watermark', 'không có', C.green],
    ['Model local', 'giấy phép riêng', C.amber],
    ['Cloud AI', 'tuỳ provider', C.amber],
  ];
  rows.forEach(([k, v, color], i) => {
    const y = R.y + 90 + i * 64;
    txt(`rK${i}`, k, R.x + 24, y, 13, A(C.mute, 230));
    txt(`rV${i}`, v, R.x + R.w - 24 - textW(v, 13), y, 13, A(color, 235));
    rect(`rL${i}`, R.x + 24, y + 20, R.w - 48, 1, A(C.edge, 120));
    anim(`rK${i}`, 'opacity', [[0.4 * d + i * 0.04 * d, 0], [0.48 * d + i * 0.04 * d, 1]], 'ease_out');
    anim(`rV${i}`, 'opacity', [[0.4 * d + i * 0.04 * d, 0], [0.48 * d + i * 0.04 * d, 1]], 'ease_out');
  });
  txt('rFoot', 'MIT không có nghĩa mọi thứ khác đều miễn phí.', R.x + 24, R.y + 450, 12, A(C.dim, 220));
  anim('rFoot', 'opacity', [[0.74 * d, 0], [0.82 * d, 1]], 'ease_out');

  // camera drifts like a hand turning a page
  return finish({
    name: 'MIT — ba quyền tự do',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_x: 30, rotation_degrees: 1 },
      { t: 0.4 * d, zoom: 1.0, pan_x: 0, rotation_degrees: 0 },
      { t: 0.7 * d, zoom: 1.03, pan_x: -20, rotation_degrees: -0.6 },
      { t: d, zoom: 1.07, pan_x: -30 },
    ],
  });
}

export function localCost(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 200, 1400, 1000, A(C.cyan, 8));
  ell('glowB', 300, 500, 900, 800, A(C.amber, 6));

  // ledger, read line by line
  const rows = [
    ['VieNeu-TTS local', '0', 'mỗi câu đọc', C.cyan],
    ['Whisper local', '0', 'mỗi phút audio', C.blue],
    ['Chat Assistant', 'tuỳ provider', 'khi nhà cung cấp cho phép', C.violet],
    ['Xuất video', 'không giới hạn', 'không watermark', C.green],
  ];
  rows.forEach(([name, price, note, color], i) => {
    const y = 150 + i * 140;
    rect(`row${i}`, 140, y, 1640, 118, A(C.panel, 245), { stroke: A(C.edge2, 150), stroke_width: 1 });
    rect(`row${i}Bar`, 140, y, 5, 118, A(color, 225));
    txt(`row${i}N`, name, 180, y + 44, 22, A(C.ink, 238));
    txt(`row${i}D`, note, 180, y + 78, 12, A(C.dim, 220));
    txt(`row${i}P`, price, 1700 - textW(price, 28), y + 60, 28, A(color, 235));
    rect(`row${i}Rule`, 180, y + 98, 1400, 1, A(C.edge, 100));
    anim(`row${i}`, 'opacity', [[0.06 * d + i * 0.1 * d, 0], [0.14 * d + i * 0.1 * d, 1]], 'ease_out');
    anim(`row${i}`, 'translation_x', [[0.06 * d + i * 0.1 * d, 40], [0.16 * d + i * 0.1 * d, 0]], 'ease_out');
  });

  // conditions, as a small strip beneath
  const caveats = [
    ['PHẦN CỨNG', 'CPU / GPU'],
    ['DUNG LƯỢNG', 'model + media'],
    ['TẢI MODEL', 'một lần'],
    ['GIẤY PHÉP', 'theo từng model'],
  ];
  caveats.forEach(([k, v], i) => {
    const x = 160 + i * 410;
    tagChip(`cv${i}`, k, x, 770, C.dim, 10);
    txt(`cv${i}V`, v, x + 4, 812, 12, A(C.mute, 225));
    ['Bg', 'T'].forEach((s) => anim(`cv${i}${s}`, 'opacity', [[0.6 * d + i * 0.03 * d, 0], [0.68 * d + i * 0.03 * d, 1]], 'ease_out'));
    anim(`cv${i}V`, 'opacity', [[0.6 * d + i * 0.03 * d, 0], [0.68 * d + i * 0.03 * d, 1]], 'ease_out');
  });

  // a small machine that keeps the counter at zero
  const M = { x: 900, y: 660, w: 120, h: 70 };
  rect('mBase', M.x, M.y + 50, M.w, 6, A(C.edge2, 180));
  rect('mLid', M.x + 12, M.y, M.w - 24, 50, A(C.panel2, 255), { stroke: A(C.edge2, 180), stroke_width: 1 });
  rect('mScreen', M.x + 26, M.y + 12, 44, 26, A(C.bg, 255), { stroke: A(C.cyan, 130), stroke_width: 1 });
  ell('mLed', M.x + 82, M.y + 22, 7, 7, A(C.green, 220));
  txt('mLabel', 'LOCAL', M.x - 4, M.y + 82, 9, A(C.dim, 210));

  return finish({
    name: 'Chi phí local — con số thật',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_y: -20 },
      { t: 0.3 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.03, pan_y: 30 },
      { t: 0.85 * d, zoom: 1.02, pan_y: 20 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function releaseCard(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 300, 1300, 1000, A(C.amber, 9));

  const Cx = 960;
  const card = { x: Cx - 400, y: 160, w: 800, h: 560 };
  rect('cShadow', card.x + 16, card.y + 18, card.w, card.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('cBg', card.x, card.y, card.w, card.h, C.panel, { stroke: A(C.amber, 170), stroke_width: 1 });
  rect('cTop', card.x, card.y, card.w, 4, C.amber);
  txt('cTag', 'RELEASE', card.x + 32, card.y + 44, 11, A(C.dim, 225));
  txt('cVer', '0.1.0', card.x + 32, card.y + 128, 64, A(C.ink, 248));
  txt('cDate', '30 · 10 · 2026', card.x + 340, card.y + 96, 14, A(C.mute, 225));
  rect('cRule', card.x + 32, card.y + 168, card.w - 64, 1, A(C.edge, 160));
  txt('cOs', 'macOS · Apple Silicon', card.x + 32, card.y + 216, 20, A(C.ink, 235));
  txt('cArch', 'arm64 · .dmg', card.x + 32, card.y + 248, 12, A(C.dim, 215));

  // chip drawing
  rect('chip', card.x + 500, card.y + 196, 190, 130, A(C.panel2, 255), { stroke: A(C.edge2, 180), stroke_width: 1 });
  for (let i = 0; i < 6; i += 1) {
    rect(`pinT${i}`, card.x + 520 + i * 26, card.y + 184, 8, 12, A(C.edge2, 180));
    rect(`pinB${i}`, card.x + 520 + i * 26, card.y + 326, 8, 12, A(C.edge2, 180));
  }
  txt('chipT', 'M', card.x + 568, card.y + 276, 34, A(C.ink, 230));

  // the honest lines
  tagChip('signed', 'AD-HOC SIGNED', card.x + 32, card.y + 330, C.amber, 11);
  tagChip('notar', 'NOT NOTARIZED', card.x + 220, card.y + 330, C.red, 11);
  txt('cNote', 'chưa có Apple Developer account', card.x + 32, card.y + 380, 11, A(C.dim, 215));
  txt('cNote2', 'một điểm bắt đầu, không phải đích đến', card.x + 32, card.y + 410, 13, A(C.mute, 225));

  // a small progress mark: start, not finish
  rect('prog', card.x + 32, card.y + 470, card.w - 64, 3, A(C.edge2, 160));
  rect('progFill', card.x + 32, card.y + 468, 80, 7, A(C.amber, 220));
  txt('progT', 'bản đầu tiên', card.x + 32, card.y + 498, 10, A(C.dim, 210));

  return finish({
    name: '0.1.0 — một điểm bắt đầu',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_y: 10 },
      { t: 0.4 * d, zoom: 1.0 },
      { t: 0.75 * d, zoom: 1.03 },
      { t: d, zoom: 1.07, pan_y: -10 },
    ],
  });
}

export function providerTrap(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 960, 300, 1400, 1000, A(C.violet, 8));

  const Cx = 960, Cy = 470;
  ell('hub', Cx - 46, Cy - 46, 92, 92, C.panel3, { stroke: A(C.violet, 230), stroke_width: 2 });
  txt('hubT', 'ACP', Cx - 18, Cy + 8, 16, A(C.ink, 235));

  // one harness fans out to many providers
  const providers = [];
  for (let i = 0; i < 12; i += 1) {
    const a = (i / 12) * Math.PI * 2;
    const x = Cx + Math.cos(a) * 420;
    const y = Cy + Math.sin(a) * 240;
    providers.push([x, y, i]);
    const color = i % 3 === 0 ? C.amber : i % 3 === 1 ? C.cyan : C.violet;
    rect(`p${i}`, x - 2, Math.min(Cy, y), 2, Math.abs(Cy - y), A(C.edge2, 80));
    ell(`p${i}Node`, x - 22, y - 22, 44, 44, A(C.panel2, 255), { stroke: A(color, 160), stroke_width: 1 });
    txt(`p${i}T`, `${i + 1}`, x - 6, y + 6, 12, A(color, 225));
    // each provider needs its own maintenance mark
    ell(`p${i}W`, x + 24, y - 26, 18, 18, A(C.red, 210));
    txt(`p${i}WT`, '!', x + 30, y - 16, 10, A(C.bg, 240));
    const t0 = 0.08 * d + i * 0.02 * d;
    anim(`p${i}Node`, 'opacity', [[t0, 0], [t0 + 0.04 * d, 1]], 'ease_out');
    anim(`p${i}T`, 'opacity', [[t0, 0], [t0 + 0.04 * d, 1]], 'ease_out');
    anim(`p${i}Node`, 'scale', [[t0, 0.4], [t0 + 0.08 * d, 1]], 'ease_out');
    anim(`p${i}W`, 'opacity', [[t0 + 0.06 * d, 0], [t0 + 0.1 * d, 1]], 'ease_out');
    anim(`p${i}WT`, 'opacity', [[t0 + 0.06 * d, 0], [t0 + 0.1 * d, 1]], 'ease_out');
  }

  txt('count', '1 harness · 12 tích hợp · 12 thứ phải nuôi', Cx - textW('1 harness · 12 tích hợp · 12 thứ phải nuôi', 16) / 2, 790, 16, A(C.ink, 235));
  anim('count', 'opacity', [[0.5 * d, 0], [0.6 * d, 1]], 'ease_out');

  // a wall of small integration cards piling up
  for (let i = 0; i < 5; i += 1) {
    const x = 200 + i * 320, y = 800;
    rect(`w${i}`, x, y, 280, 32, A(C.panel, 230), { stroke: A(C.edge2, 100), stroke_width: 1 });
    txt(`w${i}T`, `provider-${i + 1}`, x + 12, y + 21, 9, A(C.dim, 210));
    anim(`w${i}`, 'opacity', [[0.65 * d + i * 0.03 * d, 0], [0.7 * d + i * 0.03 * d, 1]], 'ease_out');
    anim(`w${i}T`, 'opacity', [[0.65 * d + i * 0.03 * d, 0], [0.7 * d + i * 0.03 * d, 1]], 'ease_out');
  }

  return finish({
    name: 'Cái bẫy provider',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04, pan_y: -20 },
      { t: 0.4 * d, zoom: 1.0 },
      { t: 0.7 * d, zoom: 1.05, pan_y: 20 },
      { t: d, zoom: 1.1, pan_y: 30 },
    ],
  });
}

export function checks(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 300, 200, 1300, 1000, A(C.green, 7));
  ell('glowB', 1400, 500, 900, 800, A(C.cyan, 6));

  // a terminal with honest results
  const T = { x: 120, y: 150, w: 760, h: 560 };
  rect('tShadow', T.x + 12, T.y + 14, T.w, T.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('tBg', T.x, T.y, T.w, T.h, { red: 10, green: 12, blue: 14, alpha: 255 }, { stroke: A(C.edge2, 200), stroke_width: 1 });
  rect('tBar', T.x, T.y, T.w, 38, C.panel2);
  ell('tDot', T.x + 16, T.y + 15, 8, 8, A(C.green, 235));
  txt('tTitle', 'CHECKS', T.x + 32, T.y + 18, 12, A(C.ink, 232));
  const lines = [
    ['$ bash scripts/check-repo.sh', C.mute, ''],
    ['  ok  required files · links · whitespace', C.green, 'PASS'],
    ['$ cargo test -p or_core', C.mute, ''],
    ['  ok  128 passed · 0 failed', C.green, 'PASS'],
    ['$ flutter test', C.mute, ''],
    ['  ok  64 passed', C.green, 'PASS'],
    ['$ build Windows package', C.mute, ''],
    ['  x  SDK not on this machine', C.amber, 'BLOCKED'],
    ['  LOCAL ENVIRONMENT BLOCKED', C.dim, ''],
  ];
  lines.forEach(([line, color, state], i) => {
    const y = T.y + 80 + i * 48;
    txt(`tL${i}`, line, T.x + 28, y, 13, A(color, 230));
    if (state) {
      const c = state === 'PASS' ? C.green : C.amber;
      tagChip(`tS${i}`, state, T.x + T.w - 130, y - 6, c, 9);
      ['Bg', 'T'].forEach((s) => anim(`tS${i}${s}`, 'opacity', [[0.16 * d + i * 0.045 * d, 0], [0.2 * d + i * 0.045 * d, 1]], 'ease_out'));
    }
    anim(`tL${i}`, 'opacity', [[0.08 * d + i * 0.04 * d, 0], [0.12 * d + i * 0.04 * d, 1]], 'ease_out');
  });

  // GitHub Actions is the canonical evidence for a blocked platform
  const G = { x: 940, y: 150, w: 820, h: 230 };
  rect('gBg', G.x, G.y, G.w, G.h, C.panel, { stroke: A(C.edge2, 180), stroke_width: 1 });
  txt('gTitle', 'GITHUB ACTIONS', G.x + 22, G.y + 36, 11, A(C.dim, 225));
  txt('gLine', 'cùng commit · cùng lệnh · trên runner thật', G.x + 22, G.y + 76, 14, A(C.mute, 230));
  tagChip('gPass', 'macOS', G.x + 22, G.y + 130, C.green, 10);
  tagChip('gWin', 'Windows', G.x + 140, G.y + 130, C.amber, 10);
  tagChip('gLinux', 'Linux', G.x + 258, G.y + 130, C.amber, 10);
  txt('gFoot', 'báo rõ phần chưa chạy được — không giả vờ đã chạy', G.x + 22, G.y + 186, 11, A(C.dim, 215));
  anim('gBg', 'opacity', [[0.5 * d, 0], [0.58 * d, 1]], 'ease_out');
  anim('gTitle', 'opacity', [[0.5 * d, 0], [0.58 * d, 1]], 'ease_out');
  anim('gLine', 'opacity', [[0.5 * d, 0], [0.58 * d, 1]], 'ease_out');
  anim('gFoot', 'opacity', [[0.66 * d, 0], [0.72 * d, 1]], 'ease_out');

  // a PR description
  const P = { x: 940, y: 420, w: 820, h: 420 };
  rect('pBg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.amber, 160), stroke_width: 1 });
  rect('pBar', P.x, P.y, P.w, 38, C.panel2);
  ell('pDot', P.x + 16, P.y + 15, 8, 8, A(C.amber, 235));
  txt('pTitle', 'PULL REQUEST · MÔ TẢ', P.x + 32, P.y + 18, 11, A(C.ink, 232));
  const fields = [
    ['WHAT', 'probe codec theo nền tảng'],
    ['WHY', 'Windows chọn sai profile'],
    ['IMPACT', 'chỉ media pipeline'],
    ['CHECKS', 'cargo test · flutter test'],
    ['RISKS', 'cần tester Windows xác nhận'],
  ];
  fields.forEach(([k, v], i) => {
    const y = P.y + 66 + i * 60;
    txt(`pK${i}`, k, P.x + 24, y, 9, A(C.dim, 220));
    txt(`pV${i}`, v, P.x + 130, y - 2, 13, A(C.mute, 230));
    rect(`pL${i}`, P.x + 24, y + 18, P.w - 48, 1, A(C.edge, 100));
    anim(`pK${i}`, 'opacity', [[0.58 * d + i * 0.05 * d, 0], [0.64 * d + i * 0.05 * d, 1]], 'ease_out');
    anim(`pV${i}`, 'opacity', [[0.58 * d + i * 0.05 * d, 0], [0.64 * d + i * 0.05 * d, 1]], 'ease_out');
  });
  tagChip('pOk', 'SẴN SÀNG REVIEW', P.x + P.w - 210, P.y + 386, C.green, 10);
  ['pOkBg', 'pOkT'].forEach((id) => anim(id, 'opacity', [[0.86 * d, 0], [0.92 * d, 1]], 'ease_out'));

  return finish({
    name: 'Đóng góp — kiểm tra và mô tả',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: -20 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.65 * d, zoom: 1.03, pan_y: 30 },
      { t: 0.85 * d, zoom: 1.04, pan_y: 50 },
      { t: d, zoom: 1.0 },
    ],
  });
}

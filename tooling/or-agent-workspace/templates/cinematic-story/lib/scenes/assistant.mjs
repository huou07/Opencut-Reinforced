// Assistant chapter: a client of an outside harness, not a new agent. The chat,
// the ACP boundary and the return path are shown as one loop.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';
import { viewer, timeline, playhead, tagChip, miniShot } from '../ui.mjs';

const BOUNDARY = 1160;

export function assistantScope(duration) {
  begin();
  const d = duration;
  const tOr = 0.06 * d;
  const tBound = 0.28 * d;
  const tHarness = 0.42 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 300, 180, 1400, 1000, A(C.violet, 9));
  ell('glowB', 1400, 400, 1000, 900, A(C.blue, 8));

  // OR: the project and its editing surface, enclosed
  rect('orPanel', 120, 150, 960, 620, C.bg2, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('orHead', 120, 150, 960, 42, C.panel2);
  ell('orDot', 140, 166, 8, 8, A(C.amber, 235));
  txt('orTitle', 'OPENCUT REINFORCED', 158, 170, 12, A(C.ink, 235));
  txt('orTag', 'DỰ ÁN ĐANG MỞ', 120 + 960 - 18 - textW('DỰ ÁN ĐANG MỞ', 10), 171, 10, A(C.dim, 210));
  ['orPanel', 'orHead', 'orDot', 'orTitle', 'orTag'].forEach((id) => {
    anim(id, 'opacity', [[tOr, 0], [tOr + 0.2, 1]], 'ease_out');
    anim(id, 'scale', [[tOr, 0.97], [tOr + 0.25, 1]], 'ease_out');
  });

  // inside: viewer + tracks + chat dock
  viewer('sv', 160, 220, 520, 260, { tc: '00:00:09:12', res: '1920 × 1080' });
  ['svCase', 'svBg', 'svSky', 'svHalo', 'svSun', 'svSunCore', 'svRidgeA', 'svRidgeB', 'svWater', 'svHorizon', 'svHead', 'svBody', 'svRim', 'svRule', 'svTc', 'svRes'].forEach((id) => anim(id, 'opacity', [[tOr + 0.1, 0], [tOr + 0.3, 1]], 'ease_out'));
  const T = timeline('st', 160, 550, 700, 190, {
    lanes: [['C1', C.blue], ['V1', C.amber], ['A1', C.cyan]],
    clips: [[0, 260, 260, C.blue], [1, 260, 300, C.amber], [1, 580, 200, C.teal], [2, 260, 480, C.cyan]],
  });
  ['stBg', 'stHead', 'stTitle', 'stTc'].forEach((id) => anim(id, 'opacity', [[tOr + 0.14, 0], [tOr + 0.32, 1]], 'ease_out'));

  // chat dock is part of OR, not outside it
  rect('dockBg', 710, 220, 350, 520, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('dockHead', 710, 220, 350, 38, C.panel2);
  ell('dockDot', 726, 234, 8, 8, A(C.violet, 235));
  txt('dockTitle', 'CHAT ASSISTANT', 742, 237, 11, A(C.ink, 232));
  txt('dockHint', 'yêu cầu bằng lời', 730, 284, 13, A(C.mute, 210));
  rect('dockBubble', 730, 306, 250, 64, A(C.panel3, 255));
  txt('dockText1', 'Viết cảnh mở đầu', 744, 330, 15, A(C.ink, 230));
  txt('dockText2', 'và làm phụ đề.', 744, 352, 15, A(C.ink, 230));
  for (let i = 0; i < 3; i += 1) ell(`dockType${i}`, 740 + i * 14, 396, 6, 6, A(C.violet, 200));
  ['dockBg', 'dockHead', 'dockDot', 'dockTitle', 'dockHint', 'dockBubble', 'dockText1', 'dockText2', ...Array.from({ length: 3 }, (_, i) => `dockType${i}`)].forEach((id) => {
    anim(id, 'opacity', [[tOr + 0.2, 0], [tOr + 0.4, 1]], 'ease_out');
  });
  for (let i = 0; i < 3; i += 1) anim(`dockType${i}`, 'opacity', [[0.6 * d + i * 0.1, 0.2], [0.66 * d + i * 0.1, 1], [0.72 * d + i * 0.1, 0.2]], 'linear');

  // boundary: OR is the environment; the harness is a guest
  rect('boundLine', BOUNDARY, 80, 2, 860, A(C.edge2, 120));
  for (let i = 0; i < 20; i += 1) {
    rect(`boundDash${i}`, BOUNDARY, 90 + i * 42, 2, 18, A(C.edge2, 180));
    anim(`boundDash${i}`, 'opacity', [[tBound + i * 0.01, 0], [tBound + 0.2 + i * 0.01, 1]], 'ease_out');
  }
  txt('boundLabel', 'RANH GIỚI DỰ ÁN', BOUNDARY - 150, 108, 10, A(C.dim, 210));
  anim('boundLabel', 'opacity', [[tBound + 0.1, 0], [tBound + 0.3, 1]], 'ease_out');

  // harness capsule, outside
  const H = { x: 1290, y: 258, w: 470, h: 220 };
  rect('hShadow', H.x + 12, H.y + 14, H.w, H.h, { red: 0, green: 0, blue: 0, alpha: 140 });
  rect('hBg', H.x, H.y, H.w, H.h, C.panel, { stroke: A(C.violet, 190), stroke_width: 1 });
  rect('hTop', H.x, H.y, H.w, 3, C.violet);
  ell('hDot', H.x + 20, H.y + 24, 10, 10, A(C.violet, 240));
  txt('hName', 'DEEPSEEK HARNESS', H.x + 40, H.y + 30, 16, A(C.ink, 240));
  txt('hKind', 'ACP · STDIO', H.x + H.w - 20 - textW('ACP · STDIO', 11), H.y + 31, 11, A(C.dim, 210));
  txt('hModel', 'model + provider của bạn', H.x + 20, H.y + 78, 14, A(C.mute, 220));
  rect('hRule', H.x + 20, H.y + 98, H.w - 40, 1, A(C.edge, 180));
  txt('hCred', 'khoá & phiên nằm trong harness', H.x + 20, H.y + 124, 12, A(C.dim, 220));
  txt('hTool', 'gọi công cụ qua CLI / MCP', H.x + 20, H.y + 150, 12, A(C.dim, 220));
  for (let i = 0; i < 3; i += 1) {
    ell(`hPulse${i}`, H.x + H.w - 42, H.y + 150, 8, 8, A(C.violet, 200));
    anim(`hPulse${i}`, 'opacity', [[0.55 * d + i * 0.12, 0.15], [0.62 * d + i * 0.12, 1], [0.7 * d + i * 0.12, 0.15]], 'linear');
  }
  ['hShadow', 'hBg', 'hTop', 'hDot', 'hName', 'hKind', 'hModel', 'hRule', 'hCred', 'hTool', ...Array.from({ length: 3 }, (_, i) => `hPulse${i}`)].forEach((id) => {
    anim(id, 'opacity', [[tHarness, 0], [tHarness + 0.22, 1]], 'ease_out');
    anim(id, 'translation_x', [[tHarness, 80], [tHarness + 0.3, 0]], 'ease_out');
  });

  // ACP beam out; CLI and MCP lanes back
  rect('acpBeam', BOUNDARY + 4, 366, H.x - BOUNDARY - 8, 2, A(C.violet, 100));
  anim('acpBeam', 'width', [[0.4 * d, 0], [0.52 * d, H.x - BOUNDARY - 8]], 'ease_out');
  txt('acpLabel', 'ACP', BOUNDARY + 46, 348, 12, A(C.violet, 235));
  anim('acpLabel', 'opacity', [[0.44 * d, 0], [0.5 * d, 1]], 'ease_out');
  for (let i = 0; i < 5; i += 1) {
    ell(`pkt${i}`, BOUNDARY + 10, 362, 10, 10, A(C.ink, 235));
    const t0 = 0.46 * d + i * 0.09 * d;
    anim(`pkt${i}`, 'translation_x', [[t0, 0], [t0 + 0.16 * d, H.x - BOUNDARY - 20]], 'linear');
    anim(`pkt${i}`, 'opacity', [[t0, 0], [t0 + 0.02 * d, 1], [t0 + 0.14 * d, 0.8], [t0 + 0.17 * d, 0]], 'linear');
  }
  rect('cliLane', 1280, 520, 460, 2, A(C.cyan, 110));
  rect('mcpLane', 1280, 548, 460, 2, A(C.amber, 110));
  txt('cliLabel', 'CLI', 1320, 510, 11, A(C.cyan, 225));
  txt('mcpLabel', 'MCP', 1320, 578, 11, A(C.amber, 225));
  ['cliLane', 'mcpLane', 'cliLabel', 'mcpLabel'].forEach((id) => anim(id, 'opacity', [[0.56 * d, 0], [0.62 * d, 1]], 'ease_out'));
  for (let i = 0; i < 4; i += 1) {
    ell(`ret${i}`, 1280, 516, 8, 8, A(i % 2 ? C.amber : C.cyan, 235));
    const t0 = 0.62 * d + i * 0.07 * d;
    anim(`ret${i}`, 'translation_x', [[t0, 0], [t0 + 0.16 * d, -(1280 - BOUNDARY - 40)]], 'linear');
    anim(`ret${i}`, 'opacity', [[t0, 0], [t0 + 0.02 * d, 1], [t0 + 0.17 * d, 0]], 'linear');
  }

  // the claim, small and literal
  tagChip('notAgent', 'KHÔNG PHẢI AGENT MỚI', 120, 812, C.ink, 13);
  tagChip('isEnv', 'LÀ MÔI TRƯỜNG LÀM VIỆC', 120 + 260, 812, C.amber, 13);
  ['notAgentBg', 'notAgentT', 'isEnvBg', 'isEnvT'].forEach((id) => anim(id, 'opacity', [[0.72 * d, 0], [0.78 * d, 1]], 'ease_out'));

  return finish({
    name: 'Assistant là khách của dự án',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: 40 },
      { t: 0.25 * d, zoom: 1.0, pan_x: 0 },
      { t: 0.45 * d, zoom: 1.05, pan_x: -60 },
      { t: 0.65 * d, zoom: 1.03, pan_x: -20 },
      { t: 0.85 * d, zoom: 1.07, pan_x: -90 },
      { t: d, zoom: 1.0, pan_x: 0 },
    ],
  });
}

export function chatAsk(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 1400, 120, 1100, 900, A(C.violet, 10));
  ell('glowB', 300, 500, 1000, 900, A(C.blue, 7));

  // editor stays visible on the left: the assistant works on this project
  rect('edBg', 110, 110, 1040, 760, C.bg2, { stroke: A(C.edge2, 180), stroke_width: 1 });
  rect('edHead', 110, 110, 1040, 40, C.panel2);
  txt('edTitle', 'OpenCut Reinforced', 128, 134, 13, A(C.ink, 230));
  txt('edFile', 'gioi-thieu-or.orproj', 330, 135, 11, A(C.dim, 200));
  viewer('cv', 150, 180, 560, 268, { tc: '00:00:12:08', res: '1920 × 1080' });
  rect('ctxClip', 150, 470, 560, 32, A(C.panel3, 255), { stroke: A(C.edge, 190), stroke_width: 1 });
  txt('ctxT', 'CLIP ĐANG CHỌN · shot-03 · 00:00:08:04', 162, 490, 10, A(C.mute, 220));
  timeline('ct', 150, 520, 960, 320, {
    lanes: [['C1', C.blue], ['V2', C.violet], ['V1', C.amber], ['A1', C.cyan], ['A2', C.blue]],
    clips: [[1, 250, 200, C.violet], [2, 250, 320, C.amber], [2, 590, 260, C.teal], [3, 250, 550, C.cyan], [4, 250, 900, C.blue]],
    tc: '00:00:12:08',
  });
  ['edBg', 'edHead', 'edTitle', 'edFile', 'ctxClip', 'ctxT'].forEach((id) => anim(id, 'opacity', [[0, 0], [0.12 * d, 1]], 'ease_out'));
  ['ctBg', 'ctHead', 'ctTitle', 'ctTc'].forEach((id) => anim(id, 'opacity', [[0.04 * d, 0], [0.16 * d, 1]], 'ease_out'));
  for (let i = 0; i < 5; i += 1) [`ctLane${i}`, `ctLaneTag${i}`, `ctLaneT${i}`].forEach((id) => anim(id, 'opacity', [[0.06 * d + i * 0.02 * d, 0], [0.18 * d + i * 0.02 * d, 1]], 'ease_out'));

  // chat panel: the actual request and the tool trail
  const P = { x: 1200, y: 110, w: 600, h: 760 };
  rect('pShadow', P.x + 14, P.y + 16, P.w, P.h, { red: 0, green: 0, blue: 0, alpha: 150 });
  rect('pBg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 215), stroke_width: 1 });
  rect('pHead', P.x, P.y, P.w, 46, C.panel2);
  ell('pDot', P.x + 20, P.y + 19, 9, 9, A(C.violet, 240));
  txt('pTitle', 'CHAT ASSISTANT', P.x + 38, P.y + 23, 13, A(C.ink, 232));
  txt('pModel', 'DEEPSEEK HARNESS · ACP', P.x + P.w - 20 - textW('DEEPSEEK HARNESS · ACP', 10), P.y + 24, 10, A(C.dim, 210));

  // context strip
  rect('pCtx', P.x + 20, P.y + 66, P.w - 40, 30, A(C.panel2, 255), { stroke: A(C.edge, 170), stroke_width: 1 });
  txt('pCtxT', 'clip: shot-03  ·  playhead 00:00:12:08', P.x + 32, P.y + 85, 10, A(C.mute, 215));

  // user request, typed in three lines
  rect('userBub', P.x + 58, P.y + 116, P.w - 78, 104, A(C.panel3, 255), { stroke: A(C.edge, 190), stroke_width: 1 });
  txt('userLabel', 'BẠN', P.x + 24, P.y + 132, 10, A(C.dim, 220));
  const req = ['Dựng cảnh mở đầu 8 giây,', 'thêm giọng Ngọc Huyền', 'và phụ đề từ audio.'];
  req.forEach((line, i) => {
    txt(`req${i}`, line, P.x + 76, P.y + 146 + i * 26, 18, A(C.ink, 238));
    anim(`req${i}`, 'opacity', [[0.16 * d + i * 0.07 * d, 0], [0.2 * d + i * 0.07 * d, 1]], 'ease_out');
    anim(`req${i}`, 'translation_x', [[0.16 * d + i * 0.07 * d, 8], [0.2 * d + i * 0.07 * d, 0]], 'ease_out');
  });
  rect('caret', P.x + 470, P.y + 178, 2, 22, A(C.violet, 230));
  anim('caret', 'opacity', [[0.38 * d, 0], [0.4 * d, 1], [0.42 * d, 0], [0.44 * d, 1], [0.46 * d, 0]], 'linear');

  // assistant acknowledgement
  rect('aiBub', P.x + 20, P.y + 244, P.w - 42, 42, A(C.panel2, 255), { stroke: A(C.edge, 170), stroke_width: 1 });
  txt('aiLabel', 'OR', P.x + 34, P.y + 258, 10, A(C.violet, 235));
  txt('aiText', 'Đang dựng cảnh và tạo giọng đọc…', P.x + 70, P.y + 270, 14, A(C.mute, 232));
  ['aiBub', 'aiLabel', 'aiText'].forEach((id) => anim(id, 'opacity', [[0.42 * d, 0], [0.47 * d, 1]], 'ease_out'));

  // tool cards: real command names, real order
  const tools = [
    ['motion.publish', 'cảnh 8.0 s · 1920×1080', C.cyan, 0.5],
    ['voiceover · VieNeu-TTS', 'Ngọc Huyền · local', C.violet, 0.62],
    ['captions.transcribe', 'Whisper · tiếng Việt', C.blue, 0.72],
  ];
  tools.forEach(([name, detail, color, t], i) => {
    const y = P.y + 312 + i * 84;
    rect(`tool${i}`, P.x + 20, y, P.w - 42, 66, A(C.bg2, 245), { stroke: A(C.edge, 180), stroke_width: 1 });
    rect(`tool${i}Bar`, P.x + 20, y, 3, 66, A(color, 230));
    txt(`tool${i}N`, name, P.x + 36, y + 24, 13, A(C.ink, 230));
    txt(`tool${i}D`, detail, P.x + 36, y + 46, 11, A(C.dim, 215));
    ell(`tool${i}Dot`, P.x + P.w - 54, y + 24, 8, 8, A(color, 235));
    txt(`tool${i}S`, i === 2 ? 'DONE' : 'RUNNING', P.x + P.w - 122, y + 46, 9, A(i === 2 ? C.green : C.mute, 220));
    [`tool${i}`, `tool${i}Bar`, `tool${i}N`, `tool${i}D`, `tool${i}Dot`, `tool${i}S`].forEach((id) => anim(id, 'opacity', [[t * d, 0], [t * d + 0.04 * d, 1]], 'ease_out'));
    anim(`tool${i}`, 'translation_x', [[t * d, 24], [t * d + 0.08 * d, 0]], 'ease_out');
    anim(`tool${i}Dot`, 'scale', [[t * d + 0.04 * d, 0.6], [t * d + 0.12 * d, 1.2], [t * d + 0.2 * d, 1]], 'ease_out');
  });

  // status line and permission chip
  tagChip('perm', 'CHO PHÉP CHẠY CÔNG CỤ', P.x + 20, P.y + 612, C.amber, 11);
  tagChip('undo', 'MỌI BƯỚC ĐỀU HOÀN TÁC ĐƯỢC', P.x + 20, P.y + 660, C.dim, 11);
  ['permBg', 'permT', 'undoBg', 'undoT'].forEach((id) => anim(id, 'opacity', [[0.78 * d, 0], [0.83 * d, 1]], 'ease_out'));

  particles('dust', 1600, 720, 14, 220, C.violet, d, { size: 1.5, speed: 12 });

  return finish({
    name: 'Chat — một yêu cầu thật',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -40 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.05, pan_x: 40 },
      { t: 0.8 * d, zoom: 1.08, pan_x: 70 },
      { t: d, zoom: 1.02, pan_x: 20 },
    ],
  });
}

export function acpBridge(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 500, 260, 1300, 1000, A(C.violet, 9));
  ell('glowB', 1500, 500, 1000, 900, A(C.blue, 7));
  for (let i = 0; i < 10; i += 1) rect(`gx${i}`, 140 + i * 175, 0, 1, 1080, A(C.edge, 20));

  const X = 960;
  // project core on the left
  rect('core', 150, 250, 640, 470, C.bg2, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('coreHead', 150, 250, 640, 38, C.panel2);
  ell('coreDot', 168, 264, 8, 8, A(C.amber, 235));
  txt('coreTitle', 'DỰ ÁN · OR', 186, 267, 11, A(C.ink, 232));
  const T = timeline('mt', 170, 310, 600, 250, {
    lanes: [['V2', C.violet], ['V1', C.amber], ['A1', C.cyan], ['C1', C.blue]],
    clips: [[0, 260, 180, C.violet], [1, 260, 240, C.amber], [1, 520, 200, C.teal], [2, 260, 430, C.cyan], [3, 260, 340, C.blue]],
    tc: '00:00:18:02',
  });
  const hist = ['timeline.insert-clip', 'timeline.set-text', 'motion.publish'];
  hist.forEach((s, i) => {
    const y = 590 + i * 34;
    txt(`hs${i}`, s, 182, y, 11, A(C.dim, 215));
    rect(`hsBar${i}`, 170, y - 12, 3, 16, A(C.green, 190));
  });

  // boundary line
  rect('bound', X, 100, 2, 820, A(C.edge2, 110));
  for (let i = 0; i < 18; i += 1) rect(`dash${i}`, X, 110 + i * 46, 2, 20, A(C.edge2, 175));
  txt('boundT', 'ACP', X + 14, 150, 12, A(C.violet, 230));

  // harness on the right
  const H = { x: 1240, y: 250, w: 520, h: 300 };
  rect('hShadow', H.x + 12, H.y + 14, H.w, H.h, { red: 0, green: 0, blue: 0, alpha: 140 });
  rect('hBg', H.x, H.y, H.w, H.h, C.panel, { stroke: A(C.violet, 200), stroke_width: 1 });
  rect('hTop', H.x, H.y, H.w, 3, C.violet);
  ell('hGlow', H.x + 200, H.y + 70, 120, 120, A(C.violet, 24));
  ell('hNode', H.x + 226, H.y + 84, 68, 68, C.panel3, { stroke: A(C.violet, 235), stroke_width: 2 });
  txt('hNodeT', 'H', H.x + 250, H.y + 126, 26, A(C.ink, 245));
  txt('hName', 'DEEPSEEK HARNESS', H.x + 318, H.y + 108, 17, A(C.ink, 240));
  txt('hSub', 'ACP v1 · stdio', H.x + 318, H.y + 132, 11, A(C.dim, 215));
  rect('hRule', H.x + 20, H.y + 186, H.w - 40, 1, A(C.edge, 180));
  txt('hCred', 'provider + khoá ở lại đây', H.x + 20, H.y + 214, 12, A(C.mute, 220));
  txt('hCall', 'gọi lại dự án bằng CLI / MCP', H.x + 20, H.y + 242, 12, A(C.mute, 220));
  txt('hPerm', 'permission do harness hỏi', H.x + 20, H.y + 270, 11, A(C.dim, 210));

  // request packet out
  for (let i = 0; i < 6; i += 1) {
    ell(`out${i}`, X + 6, 470, 10, 10, A(C.ink, 235));
    const t0 = 0.12 * d + i * 0.06 * d;
    anim(`out${i}`, 'translation_x', [[t0, 0], [t0 + 0.18 * d, H.x - X - 24]], 'linear');
    anim(`out${i}`, 'opacity', [[t0, 0], [t0 + 0.02 * d, 1], [t0 + 0.16 * d, 0.8], [t0 + 0.19 * d, 0]], 'linear');
  }
  rect('beamOut', X + 4, 468, 0, 2, A(C.violet, 90));
  anim('beamOut', 'width', [[0.1 * d, 0], [0.3 * d, H.x - X - 8]], 'ease_out');

  // return lanes: CLI and MCP
  rect('cli', H.x - 20, 600, X - H.x + 60, 2, A(C.cyan, 100));
  rect('mcp', H.x - 20, 650, X - H.x + 60, 2, A(C.amber, 100));
  txt('cliT', 'CLI', H.x - 64, 592, 11, A(C.cyan, 230));
  txt('mcpT', 'MCP', H.x - 64, 642, 11, A(C.amber, 230));
  ['cli', 'mcp', 'cliT', 'mcpT'].forEach((id) => anim(id, 'opacity', [[0.3 * d, 0], [0.36 * d, 1]], 'ease_out'));
  for (let i = 0; i < 5; i += 1) {
    ell(`back${i}`, H.x - 24, i % 2 ? 646 : 596, 8, 8, A(i % 2 ? C.amber : C.cyan, 235));
    const t0 = 0.34 * d + i * 0.07 * d;
    anim(`back${i}`, 'translation_x', [[t0, 0], [t0 + 0.2 * d, -(H.x - X - 8)]], 'linear');
    anim(`back${i}`, 'opacity', [[t0, 0], [t0 + 0.02 * d, 1], [t0 + 0.19 * d, 0]], 'linear');
  }

  // harness rail: only DSH is live here; the rest are settings entries
  const rail = [
    ['DSH', 'ĐANG CHẠY', C.violet, 1],
    ['CLAUDE CODE', 'NẾU ĐÃ CÀI', C.amber, 0],
    ['CODEX', 'NẾU ĐÃ CÀI', C.cyan, 0],
    ['OPENCODE', 'NẾU ĐÃ CÀI', C.green, 0],
  ];
  rail.forEach(([name, state, color, live], i) => {
    const x = 250 + i * 370, y = 760;
    rect(`r${i}`, x, y, 340, 58, A(live ? C.panel2 : C.bg2, 245), { stroke: A(color, live ? 200 : 80), stroke_width: 1 });
    ell(`r${i}Dot`, x + 18, y + 16, 8, 8, A(color, live ? 240 : 110));
    txt(`r${i}N`, name, x + 34, y + 24, 12, A(live ? C.ink : C.dim, 230));
    txt(`r${i}S`, state, x + 18, y + 44, 9, A(live ? color : C.dim, 210));
    [`r${i}`, `r${i}Dot`, `r${i}N`, `r${i}S`].forEach((id) => anim(id, 'opacity', [[0.5 * d + i * 0.04 * d, 0], [0.55 * d + i * 0.04 * d, 1]], 'ease_out'));
  });
  tagChip('railT', 'ĐỔI HARNESS TRONG CÀI ĐẶT', 250, 720, C.dim, 11);
  ['railTBg', 'railTT'].forEach((id) => anim(id, 'opacity', [[0.55 * d, 0], [0.6 * d, 1]], 'ease_out'));

  // a small honesty line: the harness must exist on this machine
  txt('honest', 'OR không cài hộ harness, không giữ khoá của bạn.', 250, 826, 13, A(C.dim, 220));
  anim('honest', 'opacity', [[0.7 * d, 0], [0.76 * d, 1]], 'ease_out');

  return finish({
    name: 'ACP — dự án là môi trường, harness là khách',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -80 },
      { t: 0.3 * d, zoom: 1.0, pan_x: -20 },
      { t: 0.5 * d, zoom: 1.03, pan_x: 30 },
      { t: 0.7 * d, zoom: 1.06, pan_x: 70, pan_y: 40 },
      { t: 0.9 * d, zoom: 1.0, pan_x: 0 },
      { t: d, zoom: 1.04, pan_y: 10 },
    ],
  });
}

export function chatResult(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 900, 200, 1400, 1100, A(C.amber, 8));
  ell('glowB', 1300, 400, 1000, 900, A(C.violet, 8));

  // chat result panel
  const P = { x: 120, y: 120, w: 520, h: 700 };
  rect('pBg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 205), stroke_width: 1 });
  rect('pHead', P.x, P.y, P.w, 42, C.panel2);
  ell('pDot', P.x + 18, P.y + 16, 8, 8, A(C.green, 235));
  txt('pTitle', 'CHAT ASSISTANT', P.x + 34, P.y + 20, 11, A(C.ink, 232));
  rect('pUser', P.x + 30, P.y + 64, P.w - 60, 42, A(C.panel3, 255));
  txt('pUserT', 'Dựng cảnh mở đầu 8 giây…', P.x + 44, P.y + 90, 13, A(C.mute, 220));
  rect('pAi', P.x + 30, P.y + 120, P.w - 60, 72, A(C.panel2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
  txt('pAiT1', 'Xong. Đã thêm 4 thứ', P.x + 44, P.y + 146, 14, A(C.ink, 235));
  txt('pAiT2', 'vào dự án đang mở.', P.x + 44, P.y + 168, 14, A(C.ink, 235));
  tagChip('pUndo', 'HOÀN TÁC ĐƯỢC', P.x + 30, P.y + 218, C.green, 10);

  // four artifact cards, then they land on their real tracks
  const cards = [
    ['SCRIPT', '8 dòng · tiếng Việt', C.amber],
    ['MOTION', '8.0 s · 1920×1080', C.cyan],
    ['VOICE', 'Ngọc Huyền · WAV', C.violet],
    ['CAPTIONS', '24 cue · SRT', C.blue],
  ];
  cards.forEach(([name, detail, color], i) => {
    const y = P.y + 262 + i * 94;
    rect(`card${i}`, P.x + 30, y, P.w - 60, 76, A(C.bg2, 245), { stroke: A(C.edge2, 180), stroke_width: 1 });
    rect(`card${i}Bar`, P.x + 30, y, 4, 76, A(color, 230));
    txt(`card${i}N`, name, P.x + 48, y + 28, 12, A(color, 235));
    txt(`card${i}D`, detail, P.x + 48, y + 50, 11, A(C.dim, 215));
    rect(`card${i}Line`, P.x + 48, y + 62, 0, 2, A(color, 180));
    anim(`card${i}Line`, 'width', [[0.1 * d + i * 0.05 * d, 0], [0.2 * d + i * 0.05 * d, 260]], 'ease_out');
    [`card${i}`, `card${i}Bar`, `card${i}N`, `card${i}D`, `card${i}Line`].forEach((id) => anim(id, 'opacity', [[0.06 * d + i * 0.05 * d, 0], [0.12 * d + i * 0.05 * d, 1]], 'ease_out'));
  });

  // viewer with the composed shot
  viewer('rv', 760, 140, 700, 340, { tc: '00:00:20:04', res: '1920 × 1080' });
  ['rvCase', 'rvBg', 'rvSky', 'rvHalo', 'rvSun', 'rvSunCore', 'rvRidgeA', 'rvRidgeB', 'rvWater', 'rvHorizon', 'rvHead', 'rvBody', 'rvRim', 'rvRule', 'rvTc', 'rvRes'].forEach((id) => anim(id, 'opacity', [[0.34 * d, 0], [0.42 * d, 1]], 'ease_out'));
  anim('rvSun', 'translation_y', [[0.4 * d, 10], [d, -8]], 'ease_in_out');

  // timeline where the four results become editable clips
  const T = timeline('rt', 700, 540, 1100, 340, {
    lanes: [['C1', C.blue], ['V2', C.violet], ['V1', C.amber], ['A1', C.cyan]],
    clips: [[2, 820, 300, C.amber], [2, 1140, 260, C.teal]],
    tc: '00:00:20:04',
  });
  ['rtBg', 'rtHead', 'rtTitle', 'rtTc'].forEach((id) => anim(id, 'opacity', [[0.2 * d, 0], [0.3 * d, 1]], 'ease_out'));
  for (let i = 0; i < 4; i += 1) [`rtLane${i}`, `rtLaneTag${i}`, `rtLaneT${i}`].forEach((id) => anim(id, 'opacity', [[0.22 * d + i * 0.03 * d, 0], [0.32 * d + i * 0.03 * d, 1]], 'ease_out'));

  const lands = [
    ['landScript', 0, 620, 200, C.amber, 0.46],
    ['landMotion', 2, 1120, 260, C.cyan, 0.54],
    ['landVoice', 3, 820, 420, C.violet, 0.62],
    ['landCap', 0, 900, 380, C.blue, 0.7],
  ];
  lands.forEach(([prefix, lane, x, w, color, t], i) => {
    const y = 540 + 34 + lane * ((340 - 44) / 4) + 2;
    const h = (340 - 44) / 4 - 7;
    rect(prefix, x, y, w, h, A(color, 185));
    rect(`${prefix}Cap`, x, y, w, 2, A(color, 245));
    anim(prefix, 'opacity', [[t * d, 0], [t * d + 0.05 * d, 1]], 'ease_out');
    anim(`${prefix}Cap`, 'opacity', [[t * d, 0], [t * d + 0.05 * d, 1]], 'ease_out');
    anim(prefix, 'translation_y', [[t * d, -180], [t * d + 0.1 * d, 0]], 'ease_out');
    anim(`${prefix}Cap`, 'translation_y', [[t * d, -180], [t * d + 0.1 * d, 0]], 'ease_out');
  });
  playhead('rt', 830, 574, 300);
  anim('rtPlay', 'translation_x', [[0.45 * d, 0], [d, 620]], 'linear');
  anim('rtPlayHead', 'translation_x', [[0.45 * d, 0], [d, 620]], 'linear');

  txt('endT', 'KẾT QUẢ LÀ CLIP, WAV, CAPTION TRACK — KHÔNG PHẢI CÂU TRẢ LỜI SUÔNG', 700, 862, 13, A(C.dim, 220));
  anim('endT', 'opacity', [[0.82 * d, 0], [0.88 * d, 1]], 'ease_out');

  return finish({
    name: 'Kết quả quay về timeline',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: -30 },
      { t: 0.3 * d, zoom: 1.0 },
      { t: 0.55 * d, zoom: 1.04, pan_y: 20 },
      { t: 0.8 * d, zoom: 1.06, pan_y: 50 },
      { t: d, zoom: 1.0 },
    ],
  });
}

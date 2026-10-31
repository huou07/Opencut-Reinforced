// Local production chapter: script → local voice → local transcript → captions,
// then the closed loop on one machine.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since, waveform } from '../kit.mjs';
import { viewer, timeline, playhead, tagChip, miniShot } from '../ui.mjs';

export function ttsScript(duration) {
  begin();
  const d = duration;
  const tStart = 0.42 * d;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 300, 180, 1400, 1000, A(C.cyan, 9));
  ell('glowB', 1350, 500, 1000, 900, A(C.violet, 8));

  // script card
  const S = { x: 120, y: 160, w: 760, h: 500 };
  rect('sShadow', S.x + 12, S.y + 14, S.w, S.h, { red: 0, green: 0, blue: 0, alpha: 145 });
  rect('sBg', S.x, S.y, S.w, S.h, C.panel, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('sBar', S.x, S.y, S.w, 40, C.panel2);
  ell('sDot', S.x + 18, S.y + 16, 8, 8, A(C.cyan, 235));
  txt('sTitle', 'KỊCH BẢN', S.x + 34, S.y + 20, 12, A(C.ink, 232));
  txt('sFile', 'script.json', S.x + S.w - 18 - textW('script.json', 10), S.y + 21, 10, A(C.dim, 210));
  const lines = [
    ['Một video hoàn chỉnh', C.ink],
    ['không bắt đầu bằng', C.ink],
    ['một cú máy.', C.ink],
    ['Nó bắt đầu bằng', C.mute],
    ['những quyết định nhỏ.', C.mute],
    ['Và một timeline.', C.mute],
  ];
  lines.forEach(([line, color], i) => {
    txt(`sL${i}`, line, S.x + 40, S.y + 92 + i * 54, 28, A(color, 235));
    anim(`sL${i}`, 'opacity', [[0.02 * d + i * 0.04 * d, 0], [0.12 * d + i * 0.04 * d, 1]], 'ease_out');
  });
  rect('sKara', S.x + 28, S.y + 70, 0, 46, A(C.cyan, 30));
  anim('sKara', 'width', [[0.1 * d, 0], [0.3 * d, 390], [0.46 * d, 390], [0.52 * d, 0]], 'ease_in_out');
  rect('sCursor', S.x + 28, S.y + 66, 3, 44, A(C.cyan, 210));
  anim('sCursor', 'translation_x', [[0.1 * d, 0], [0.5 * d, 380]], 'ease_in_out');
  txt('sWord', 'Ngọc Huyền', S.x + 40, S.y + 446, 15, A(C.violet, 235));
  txt('sWordL', 'GIỌNG ĐỌC ĐÃ CHỌN', S.x + 40, S.y + 470, 10, A(C.dim, 210));

  // voice picker / engine card
  const E = { x: 960, y: 160, w: 820, h: 610 };
  rect('eShadow', E.x + 14, E.y + 16, E.w, E.h, { red: 0, green: 0, blue: 0, alpha: 145 });
  rect('eBg', E.x, E.y, E.w, E.h, C.panel, { stroke: A(C.edge2, 210), stroke_width: 1 });
  rect('eBar', E.x, E.y, E.w, 44, C.panel2);
  ell('eDot', E.x + 18, E.y + 18, 9, 9, A(C.green, 235));
  txt('eTitle', 'VIENEU-TTS', E.x + 36, E.y + 22, 14, A(C.ink, 234));
  txt('eLocal', 'LOCAL · ONNX · 48 kHz', E.x + E.w - 18 - textW('LOCAL · ONNX · 48 kHz', 11), E.y + 23, 11, A(C.green, 225));
  rect('eRule', E.x, E.y + 44, E.w, 1, A(C.edge2, 180));

  // engine status
  ell('eEngineDot', E.x + 24, E.y + 68, 10, 10, A(C.green, 235));
  txt('eEngine', 'ENGINE: SẴN SÀNG', E.x + 44, E.y + 78, 11, A(C.mute, 225));
  txt('eEngineUrl', '127.0.0.1:8000', E.x + E.w - 18 - textW('127.0.0.1:8000', 10), E.y + 78, 10, A(C.dim, 200));
  rect('eHealthBg', E.x + 24, E.y + 96, E.w - 48, 3, A(C.edge2, 170));
  rect('eHealth', E.x + 24, E.y + 95, 60, 5, A(C.green, 220));
  anim('eHealth', 'width', [[0.1 * d, 60], [0.3 * d, E.w - 48]], 'ease_out');

  // voice list: one chosen, the rest are other installed voices
  const voices = [
    ['Ngọc Huyền', 'Nữ · Bắc · tự nhiên', C.violet, 1],
    ['Trúc Ly', 'Nữ · Bắc · tự nhiên', C.dim, 0],
    ['Thiện Minh', 'Nam · Bắc · kể chuyện', C.dim, 0],
    ['Mai Anh', 'Nữ · Bắc · tin tức', C.dim, 0],
  ];
  voices.forEach(([name, desc, color, active], i) => {
    const y = E.y + 124 + i * 64;
    rect(`v${i}`, E.x + 24, y, E.w - 48, 54, A(active ? C.panel3 : C.panel2, 250), { stroke: A(color, active ? 190 : 90), stroke_width: 1 });
    ell(`v${i}Dot`, E.x + 42, y + 22, 9, 9, A(color, active ? 240 : 110));
    txt(`v${i}N`, name, E.x + 62, y + 22, 15, A(active ? C.ink : C.mute, 235));
    txt(`v${i}D`, desc, E.x + 62, y + 42, 10, A(active ? C.dim : C.faint, 210));
    if (active) {
      rect(`v${i}Check`, E.x + E.w - 62, y + 20, 16, 16, A(color, 0), { stroke: A(color, 220), stroke_width: 1.4 });
      rect(`v${i}TickA`, E.x + E.w - 58, y + 27, 7, 2, A(color, 230), { rotation_degrees: 40 });
      rect(`v${i}TickB`, E.x + E.w - 53, y + 25, 10, 2, A(color, 230), { rotation_degrees: -45 });
      anim(`v${i}Dot`, 'scale', [[0.16 * d, 1], [0.2 * d, 1.4], [0.26 * d, 1]], 'ease_out');
    }
  });

  // start button and log
  const btn = { x: E.x + 24, y: E.y + 400, w: 240, h: 44 };
  rect('btn', btn.x, btn.y, btn.w, btn.h, A(C.amber, 24), { stroke: A(C.amber, 190), stroke_width: 1 });
  txt('btnT', 'TẠO GIỌNG ĐỌC', btn.x + 26, btn.y + 28, 14, A(C.amber, 240));
  rect('btnBar', btn.x, btn.y, 4, btn.h, A(C.amber, 230));
  anim('btnBar', 'width', [[tStart, 4], [tStart + 0.1 * d, btn.w]], 'ease_out');
  anim('btnT', 'opacity', [[tStart, 0.5], [tStart + 0.08 * d, 1]], 'ease_out');

  rect('logBg', E.x + 24, E.y + 462, E.w - 48, 124, A(C.bg2, 250), { stroke: A(C.edge, 170), stroke_width: 1 });
  const logs = [
    '› engine online · onnx',
    '› voice: Ngọc Huyền',
    '› synthesising 6 câu…',
    '› wav 48 kHz · 4,98 s',
  ];
  logs.forEach((line, i) => {
    txt(`log${i}`, line, E.x + 40, E.y + 488 + i * 25, 11, A(i === 3 ? C.green : C.dim, 220));
    anim(`log${i}`, 'opacity', [[tStart + 0.05 * d + i * 0.1 * d, 0], [tStart + 0.09 * d + i * 0.1 * d, 1]], 'ease_out');
  });

  // voice orb pulses with the generated speech
  const O = { cx: 1720, cy: 690 };
  ell('orb', O.cx - 30, O.cy - 30, 60, 60, A(C.violet, 30));
  ell('orb2', O.cx - 16, O.cy - 16, 32, 32, A(C.violet, 200));
  anim('orb', 'scale', [[tStart, 0.7], [tStart + 0.5 * d, 1.3], [d, 1]], 'ease_in_out');
  anim('orb2', 'scale', [[tStart, 0.8], [tStart + 0.3 * d, 1.2], [d, 1]], 'ease_in_out');
  for (let i = 0; i < 3; i += 1) {
    ell(`ring${i}`, O.cx - 30 - i * 18, O.cy - 30 - i * 18, 60 + i * 36, 60 + i * 36, A(C.violet, 0), { stroke: A(C.violet, 90 - i * 25), stroke_width: 1 });
    anim(`ring${i}`, 'scale', [[tStart + i * 0.12 * d, 0.6], [tStart + 0.5 * d + i * 0.12 * d, 1.6]], 'ease_out');
    anim(`ring${i}`, 'opacity', [[tStart + i * 0.12 * d, 0], [tStart + 0.2 * d + i * 0.12 * d, 1], [tStart + 0.5 * d + i * 0.12 * d, 0]], 'ease_out');
  }

  return finish({
    name: 'VieNeu-TTS — kịch bản thành giọng',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -20 },
      { t: 0.35 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.05, pan_x: -40 },
      { t: 0.85 * d, zoom: 1.08, pan_x: -70, pan_y: 10 },
      { t: d, zoom: 1.02, pan_x: -20 },
    ],
  });
}

export function ttsWave(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 800, 240, 1400, 1000, A(C.cyan, 10));
  ell('glowB', 1300, 480, 1000, 900, A(C.violet, 7));

  // a large waveform, generated locally
  const A1 = { x: 160, y: 200, w: 1600, h: 300 };
  rect('audBg', A1.x - 20, A1.y - 30, A1.w + 40, A1.h + 100, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('audBar', A1.x - 20, A1.y - 30, A1.w + 40, 40, C.panel2);
  ell('audDot', A1.x, A1.y - 14, 8, 8, A(C.cyan, 235));
  txt('audTitle', 'VIENEU-TTS · NGỌC HUYỀN', A1.x + 18, A1.y - 11, 12, A(C.ink, 232));
  txt('audLocal', 'LOCAL · 48 kHz · WAV', A1.x + A1.w - 10 - textW('LOCAL · 48 kHz · WAV', 11), A1.y - 11, 11, A(C.green, 225));
  const BARS = 96;
  for (let i = 0; i < BARS; i += 1) {
    const base = 14 + ((i * 73 + 19) % 150);
    const id = `w${i}`;
    rect(id, A1.x + i * 16.2, A1.y + 150 - base / 2, 6, base, A(i % 9 === 0 ? C.blue : C.cyan, 205));
    const keys = [];
    for (let k = 0; k <= 6; k += 1) {
      const t = d * k / 6;
      const wave = 0.2 + 0.8 * Math.abs(Math.sin((k + 1) * (0.7 + ((i * 7919) % 89) / 55) + i * 0.4));
      keys.push([t, Math.max(4, base * wave)]);
    }
    anim(id, 'height', keys, 'ease_in_out');
    anim(id, 'translation_y', keys.map(([t, v]) => [t, (base - v) / 2]), 'ease_in_out');
  }
  rect('audRule', A1.x, A1.y + 300, A1.w, 1, A(C.edge2, 180));
  txt('audTime', '00:00:00', A1.x, A1.y + 326, 10, A(C.dim, 210));
  txt('audTime2', '00:00:04,98', A1.x + A1.w - textW('00:00:04,98', 10), A1.y + 326, 10, A(C.dim, 210));

  // a playhead travels the waveform, then becomes a clip on the audio track
  rect('wavePlay', A1.x, A1.y - 10, 2, A1.h + 20, A(C.red, 225));
  anim('wavePlay', 'translation_x', [[0.1 * d, 0], [0.62 * d, A1.w - 4]], 'linear');

  const T = { x: 160, y: 620, w: 1600, h: 250 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 180), stroke_width: 1 });
  rect('tlHead', T.x, T.y, T.w, 32, C.panel2);
  txt('tlTitle', 'TIMELINE', T.x + 16, T.y + 20, 10, A(C.dim, 230));
  const lanes = [['V1', C.amber], ['A1', C.cyan], ['A2', C.blue]];
  const laneH = (T.h - 40) / 3;
  lanes.forEach(([name, color], i) => {
    const y = T.y + 40 + i * laneH;
    rect(`lane${i}`, T.x + 12, y, T.w - 24, laneH - 4, A(C.panel2, 255));
    rect(`laneTag${i}`, T.x + 12, y, 42, laneH - 4, A(C.bg2, 255));
    txt(`laneT${i}`, name, T.x + 22, y + laneH / 2, 10, A(color, 235));
  });
  rect('vClip', 300, T.y + 42, 300, laneH - 8, A(C.amber, 180));
  rect('vClip2', 620, T.y + 42, 280, laneH - 8, A(C.teal, 180));

  // the narration clip appears on A1
  const ay = T.y + 40 + laneH + 2;
  rect('voiceClip', 300, ay, 480, laneH - 8, A(C.cyan, 190));
  rect('voiceClipCap', 300, ay, 480, 2, A(C.cyan, 245));
  txt('voiceClipT', 'voice · Ngọc Huyền', 310, ay + 18, 9, A(C.bg, 190));
  for (let i = 0; i < 36; i += 1) {
    const bh = 3 + ((i * 31) % 16);
    rect(`vw${i}`, 312 + i * 13, ay + (laneH - 8) / 2, 3, bh, A(C.bg, 100));
  }
  ['voiceClip', 'voiceClipCap', 'voiceClipT'].forEach((id) => anim(id, 'opacity', [[0.5 * d, 0], [0.56 * d, 1]], 'ease_out'));
  for (let i = 0; i < 36; i += 1) anim(`vw${i}`, 'opacity', [[0.5 * d, 0], [0.56 * d, 1]], 'ease_out');
  anim('voiceClip', 'translation_y', [[0.5 * d, -60], [0.58 * d, 0]], 'ease_out');
  anim('voiceClipCap', 'translation_y', [[0.5 * d, -60], [0.58 * d, 0]], 'ease_out');
  anim('voiceClipT', 'translation_y', [[0.5 * d, -60], [0.58 * d, 0]], 'ease_out');

  // cost counter: no per-request bill
  tagChip('zero', 'KHÔNG GỬI CLOUD · 0 LƯỢT GỌI', 160, 560, C.green, 12);
  ['zeroBg', 'zeroT'].forEach((id) => anim(id, 'opacity', [[0.62 * d, 0], [0.68 * d, 1]], 'ease_out'));

  return finish({
    name: 'VieNeu-TTS — âm thanh vào timeline',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: 40 },
      { t: 0.3 * d, zoom: 1.0 },
      { t: 0.55 * d, zoom: 1.04, pan_y: 30 },
      { t: 0.8 * d, zoom: 1.06, pan_y: 20 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function whisperListen(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 700, 220, 1400, 1000, A(C.violet, 9));
  ell('glowB', 1300, 520, 1000, 900, A(C.blue, 8));

  // audio clip goes into the analyser
  const A1 = { x: 140, y: 250, w: 620, h: 260 };
  rect('aCard', A1.x, A1.y, A1.w, A1.h, C.panel, { stroke: A(C.edge2, 190), stroke_width: 1 });
  txt('aTitle', 'VOICE · Ngọc Huyền', A1.x + 16, A1.y + 24, 11, A(C.ink, 230));
  txt('aDur', '4,98 s', A1.x + A1.w - 16 - textW('4,98 s', 10), A1.y + 24, 10, A(C.dim, 210));
  for (let i = 0; i < 44; i += 1) {
    const bh = 8 + ((i * 37) % 70);
    rect(`aw${i}`, A1.x + 18 + i * 13.4, A1.y + 140 - bh / 2, 4, bh, A(C.cyan, 190));
  }
  rect('aRule', A1.x, A1.y + 220, A1.w, 1, A(C.edge2, 170));
  txt('aTag', 'WAV 48 kHz', A1.x + 16, A1.y + 242, 10, A(C.dim, 210));

  // the clip slides into a large analyser
  const AN = { x: 820, y: 160, w: 900, h: 640 };
  rect('anBg', AN.x, AN.y, AN.w, AN.h, C.panel, { stroke: A(C.violet, 160), stroke_width: 1 });
  rect('anBar', AN.x, AN.y, AN.w, 42, C.panel2);
  ell('anDot', AN.x + 18, AN.y + 17, 9, 9, A(C.violet, 235));
  txt('anTitle', 'WHISPER · LOCAL', AN.x + 36, AN.y + 21, 12, A(C.ink, 232));
  txt('anLang', 'VI · TỰ NHẬN DIỆN', AN.x + AN.w - 18 - textW('VI · TỰ NHẬN DIỆN', 10), AN.y + 22, 10, A(C.dim, 210));

  // spectrogram bands
  for (let i = 0; i < 26; i += 1) {
    const y = AN.y + 70 + i * 16;
    rect(`band${i}`, AN.x + 30, y, AN.w - 60, 8, A(i % 4 === 0 ? C.violet : C.blue, 40 + (i % 5) * 12));
    const t0 = 0.2 * d + i * 0.02 * d;
    anim(`band${i}`, 'width', [[t0, 10], [t0 + 0.3 * d, AN.w - 60]], 'ease_out');
    anim(`band${i}`, 'opacity', [[t0, 0], [t0 + 0.1 * d, 1]], 'ease_out');
  }
  // waveform overlay, moving through the analyser
  for (let i = 0; i < 70; i += 1) {
    const bh = 6 + ((i * 41) % 54);
    rect(`anW${i}`, AN.x + 30 + i * 12.2, AN.y + 330 - bh / 2, 4, bh, A(C.ink, 155));
  }
  rect('scan', AN.x + 30, AN.y + 60, 2, 380, A(C.cyan, 200));
  anim('scan', 'translation_x', [[0.2 * d, 0], [0.75 * d, AN.w - 70]], 'linear');
  anim('scan', 'opacity', [[0.2 * d, 0], [0.25 * d, 1], [0.78 * d, 1], [0.84 * d, 0]], 'ease_in_out');

  // rings around the model node
  const N = { cx: AN.x + 200, cy: AN.y + 560 };
  ell('nodeGlow', N.cx - 60, N.cy - 60, 120, 120, A(C.violet, 22));
  ell('node', N.cx - 32, N.cy - 32, 64, 64, C.panel3, { stroke: A(C.violet, 230), stroke_width: 2 });
  txt('nodeT', 'W', N.cx - 9, N.cy + 8, 22, A(C.ink, 240));
  for (let i = 0; i < 3; i += 1) {
    ell(`nr${i}`, N.cx - 40, N.cy - 40, 80, 80, A(C.violet, 0), { stroke: A(C.violet, 120 - i * 30), stroke_width: 1.4 });
    anim(`nr${i}`, 'scale', [[0.25 * d + i * 0.15 * d, 0.6], [0.5 * d + i * 0.15 * d, 2.4]], 'ease_out');
    anim(`nr${i}`, 'opacity', [[0.25 * d + i * 0.15 * d, 0], [0.32 * d + i * 0.15 * d, 1], [0.5 * d + i * 0.15 * d, 0]], 'ease_out');
  }

  // transcript tokens emerge with timestamps
  const tokens = [
    ['Một video', '00:00,00'],
    ['hoàn chỉnh', '00:00,42'],
    ['được ghép', '00:01,10'],
    ['từ nhiều lớp.', '00:01,72'],
  ];
  tokens.forEach(([word, time], i) => {
    const y = AN.y + 430 + i * 44;
    txt(`tkT${i}`, time, AN.x + 440, y, 10, A(C.dim, 210));
    txt(`tk${i}`, word, AN.x + 540, y, 18, A(C.ink, 235));
    const t0 = 0.5 * d + i * 0.08 * d;
    anim(`tkT${i}`, 'opacity', [[t0, 0], [t0 + 0.06 * d, 1]], 'ease_out');
    anim(`tk${i}`, 'opacity', [[t0, 0], [t0 + 0.06 * d, 1]], 'ease_out');
    anim(`tk${i}`, 'translation_x', [[t0, 16], [t0 + 0.08 * d, 0]], 'ease_out');
  });
  rect('tkRule', AN.x + 430, AN.y + 414, 400, 1, A(C.edge, 170));
  anim('tkRule', 'width', [[0.46 * d, 0], [0.56 * d, 400]], 'ease_out');
  txt('review', 'BẠN XEM LẠI TRƯỚC KHI ÁP DỤNG', AN.x + 440, AN.y + 610, 11, A(C.amber, 225));
  anim('review', 'opacity', [[0.82 * d, 0], [0.88 * d, 1]], 'ease_out');

  // audio card travels into the analyser
  anim('aCard', 'translation_x', [[0.1 * d, 0], [0.24 * d, 400]], 'ease_in_out');
  anim('aCard', 'scale', [[0.1 * d, 1], [0.24 * d, 0.7]], 'ease_in_out');
  anim('aCard', 'opacity', [[0.22 * d, 1], [0.3 * d, 0]], 'ease_in');
  for (let i = 0; i < 44; i += 1) anim(`aw${i}`, 'opacity', [[0.1 * d, 1], [0.28 * d, 0]], 'ease_in');

  return finish({
    name: 'Whisper — nghe và nhận diện',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: 30 },
      { t: 0.3 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.05, pan_x: 50 },
      { t: 0.85 * d, zoom: 1.08, pan_x: 70, pan_y: 20 },
      { t: d, zoom: 1.02, pan_x: 20 },
    ],
  });
}

export function whisperCues(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 400, 160, 1400, 1000, A(C.blue, 9));
  ell('glowB', 1400, 500, 1000, 900, A(C.violet, 8));

  // transcript review panel
  const P = { x: 120, y: 150, w: 760, h: 560 };
  rect('pBg', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 200), stroke_width: 1 });
  rect('pBar', P.x, P.y, P.w, 42, C.panel2);
  ell('pDot', P.x + 18, P.y + 17, 8, 8, A(C.violet, 235));
  txt('pTitle', 'WHISPER · VĂN BẢN ĐỀ XUẤT', P.x + 34, P.y + 21, 12, A(C.ink, 232));
  txt('pLang', 'VI · 24 CUE', P.x + P.w - 18 - textW('VI · 24 CUE', 10), P.y + 22, 10, A(C.dim, 210));
  const rows = [
    ['00:00,00', 'Một video hoàn chỉnh', C.ink],
    ['00:00,42', 'không bắt đầu bằng', C.mute],
    ['00:01,10', 'một cú máy.', C.ink],
    ['00:01,72', 'Nó bắt đầu bằng những', C.mute],
    ['00:02,48', 'quyết định nhỏ.', C.ink],
    ['00:03,10', 'Và một timeline.', C.mute],
  ];
  rows.forEach(([time, text, color], i) => {
    const y = P.y + 78 + i * 58;
    rect(`row${i}`, P.x + 20, y - 20, P.w - 40, 44, A(C.panel2, i === 2 ? 255 : 0));
    txt(`rowT${i}`, time, P.x + 34, y, 10, A(C.dim, 210));
    txt(`rowW${i}`, text, P.x + 140, y, 17, A(color, 235));
    rect(`rowLine${i}`, P.x + 20, y + 20, P.w - 40, 1, A(C.edge, 100));
    anim(`rowW${i}`, 'opacity', [[0.06 * d + i * 0.05 * d, 0], [0.12 * d + i * 0.05 * d, 1]], 'ease_out');
    anim(`rowT${i}`, 'opacity', [[0.06 * d + i * 0.05 * d, 0], [0.12 * d + i * 0.05 * d, 1]], 'ease_out');
  });
  // review checkbox and apply
  rect('checkBox', P.x + 24, P.y + 442, 18, 18, A(C.bg2, 255), { stroke: A(C.green, 200), stroke_width: 1 });
  rect('checkA', P.x + 28, P.y + 450, 7, 2, A(C.green, 0), { rotation_degrees: 40 });
  rect('checkB', P.x + 32, P.y + 448, 10, 2, A(C.green, 0), { rotation_degrees: -45 });
  txt('checkT', 'Đã xem lại nội dung', P.x + 54, P.y + 456, 12, A(C.mute, 225));
  anim('checkA', 'opacity', [[0.46 * d, 0], [0.48 * d, 1]], 'ease_out');
  anim('checkB', 'opacity', [[0.46 * d, 0], [0.48 * d, 1]], 'ease_out');
  rect('apply', P.x + P.w - 200, P.y + 430, 176, 42, A(C.green, 22), { stroke: A(C.green, 180), stroke_width: 1 });
  txt('applyT', 'ÁP DỤNG CAPTION', P.x + P.w - 176, P.y + 456, 12, A(C.green, 235));
  ['apply', 'applyT'].forEach((id) => anim(id, 'opacity', [[0.5 * d, 0.4], [0.54 * d, 1]], 'ease_out'));
  rect('applyFill', P.x + P.w - 200, P.y + 430, 0, 42, A(C.green, 45));
  anim('applyFill', 'width', [[0.56 * d, 0], [0.66 * d, 176]], 'ease_out');

  // preview with the reviewed subtitle inside the frame (not over the real caption band)
  const V = { x: 960, y: 150, w: 700, h: 360 };
  rect('pvCase', V.x - 8, V.y - 8, V.w + 16, V.h + 16, C.panel);
  miniShot('pv', V.x, V.y, V.w, V.h, C.amber);
  rect('pvFrame', V.x, V.y, V.w, V.h, A(C.ink, 0), { stroke: A(C.edge2, 190), stroke_width: 1 });
  rect('pvSubBg', V.x + 80, V.y + V.h - 86, V.w - 160, 44, A(C.bg, 170));
  txt('pvSub', 'Nó bắt đầu bằng những quyết định nhỏ.', V.x + 100, V.y + V.h - 56, 17, A(C.ink, 235));
  anim('pvSub', 'opacity', [[0.62 * d, 0], [0.7 * d, 1]], 'ease_out');
  txt('pvLabel', 'PREVIEW · PHỤ ĐỀ ĐÃ DUYỆT', V.x, V.y + V.h + 30, 10, A(C.dim, 210));

  // the cue blocks become a real caption track
  const T = { x: 960, y: 540, w: 800, h: 260 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 180), stroke_width: 1 });
  rect('tlHead', T.x, T.y, T.w, 32, C.panel2);
  txt('tlTitle', 'CAPTION TRACK · C1', T.x + 16, T.y + 20, 10, A(C.mute, 230));
  rect('laneC', T.x + 12, T.y + 46, T.w - 24, 80, A(C.panel2, 255));
  rect('laneTag', T.x + 12, T.y + 46, 42, 80, A(C.bg2, 255));
  txt('laneT', 'C1', T.x + 22, T.y + 92, 10, A(C.blue, 235));
  for (let i = 0; i < 5; i += 1) {
    const x = T.x + 70 + i * 138;
    rect(`cue${i}`, x, T.y + 56, 128, 60, A(C.blue, 170));
    rect(`cue${i}Top`, x, T.y + 56, 128, 2, A(C.blue, 240));
    txt(`cue${i}T`, ['Một video', 'hoàn chỉnh', 'không bắt đầu', 'bằng một', 'cú máy.'][i], x + 8, T.y + 78, 10, A(C.bg, 200));
    const t0 = 0.66 * d + i * 0.04 * d;
    anim(`cue${i}`, 'opacity', [[t0, 0], [t0 + 0.05 * d, 1]], 'ease_out');
    anim(`cue${i}Top`, 'opacity', [[t0, 0], [t0 + 0.05 * d, 1]], 'ease_out');
    anim(`cue${i}T`, 'opacity', [[t0, 0], [t0 + 0.05 * d, 1]], 'ease_out');
    anim(`cue${i}`, 'translation_y', [[t0, 40], [t0 + 0.07 * d, 0]], 'ease_out');
    anim(`cue${i}Top`, 'translation_y', [[t0, 40], [t0 + 0.07 * d, 0]], 'ease_out');
    anim(`cue${i}T`, 'translation_y', [[t0, 40], [t0 + 0.07 * d, 0]], 'ease_out');
  }
  rect('play', T.x + 60, T.y + 40, 2, 200, A(C.red, 220));
  anim('play', 'translation_x', [[0.6 * d, 0], [d, 620]], 'linear');
  tagChip('editTag', 'SỬA TỪNG CHỮ · TỪNG MỐC THỜI GIAN', T.x, T.y + 216, C.cyan, 11);
  ['editTagBg', 'editTagT'].forEach((id) => anim(id, 'opacity', [[0.78 * d, 0], [0.84 * d, 1]], 'ease_out'));

  return finish({
    name: 'Whisper — duyệt và áp dụng caption',
    duration: d,
    camera: [
      { t: 0, zoom: 1.02, pan_x: -20 },
      { t: 0.3 * d, zoom: 1.0 },
      { t: 0.55 * d, zoom: 1.04, pan_y: 20 },
      { t: 0.8 * d, zoom: 1.06, pan_y: 40 },
      { t: d, zoom: 1.0 },
    ],
  });
}

export function workflowLoop(duration) {
  begin();
  const d = duration;
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 960, 300, 1500, 1000, A(C.cyan, 8));
  ell('glowB', 1200, 600, 1000, 900, A(C.violet, 7));

  // an arc of six stations: one loop, one machine
  const stations = [
    ['SCRIPT', 'kịch bản', C.amber],
    ['VOICE', 'VieNeu-TTS', C.violet],
    ['WHISPER', 'nhận diện', C.blue],
    ['CAPTIONS', 'caption track', C.cyan],
    ['TIMELINE', 'ghép & sửa', C.green],
    ['EXPORT', 'tệp video', C.ink],
  ];
  const n = stations.length;
  const cx = 960, cy = 660, radius = 470;
  stations.forEach(([name, sub, color], i) => {
    const a = Math.PI + (i / (n - 1)) * Math.PI;
    const x = cx + Math.cos(a) * radius;
    const y = cy + Math.sin(a) * radius * 0.72;
    const t0 = 0.06 * d + i * 0.035 * d;
    rect(`node${i}Shadow`, x - 74, y - 46, 148, 92, { red: 0, green: 0, blue: 0, alpha: 120 });
    rect(`node${i}`, x - 80, y - 52, 160, 96, A(C.panel, 250), { stroke: A(color, 160), stroke_width: 1 });
    rect(`node${i}Top`, x - 80, y - 52, 160, 3, A(color, 220));
    ell(`node${i}Dot`, x - 62, y - 32, 8, 8, A(color, 235));
    txt(`node${i}N`, name, x - 46, y - 26, 12, A(C.ink, 235));
    txt(`node${i}S`, sub, x - 62, y + 6, 10, A(C.dim, 210));
    [`node${i}Shadow`, `node${i}`, `node${i}Top`, `node${i}Dot`, `node${i}N`, `node${i}S`].forEach((id) => anim(id, 'opacity', [[t0, 0], [t0 + 0.05 * d, 1]], 'ease_out'));
    anim(`node${i}`, 'translation_y', [[t0, 24], [t0 + 0.08 * d, 0]], 'ease_out');
  });

  // a connecting path
  for (let i = 0; i < n - 1; i += 1) {
    const a1 = Math.PI + (i / (n - 1)) * Math.PI;
    const a2 = Math.PI + ((i + 1) / (n - 1)) * Math.PI;
    const x1 = cx + Math.cos(a1) * radius, y1 = cy + Math.sin(a1) * radius * 0.72;
    const x2 = cx + Math.cos(a2) * radius, y2 = cy + Math.sin(a2) * radius * 0.72;
    const mx = (x1 + x2) / 2, my = (y1 + y2) / 2;
    const len = Math.hypot(x2 - x1, y2 - y1);
    const angle = Math.atan2(y2 - y1, x2 - x1) * 180 / Math.PI;
    rect(`link${i}`, mx - len / 2, my, len, 2, A(C.edge2, 120), { rotation_degrees: angle });
    anim(`link${i}`, 'opacity', [[0.14 * d + i * 0.03 * d, 0], [0.2 * d + i * 0.03 * d, 1]], 'ease_out');
  }

  // a packet travels the arc, lighting each station
  for (let i = 0; i < n; i += 1) {
    const a = Math.PI + (i / (n - 1)) * Math.PI;
    const x = cx + Math.cos(a) * radius;
    const y = cy + Math.sin(a) * radius * 0.72;
    const t0 = 0.22 * d + i * 0.1 * d;
    ell(`pulse${i}`, x - 8, y - 8, 16, 16, A(C.ink, 235));
    anim(`pulse${i}`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 1], [t0 + 0.08 * d, 0]], 'ease_out');
    anim(`pulse${i}`, 'scale', [[t0, 0.6], [t0 + 0.05 * d, 1.6]], 'ease_out');
    rect(`glow${i}`, x - 80, y - 52, 160, 96, A(C.ink, 0), { stroke: A(C.ink, 200), stroke_width: 1 });
    anim(`glow${i}`, 'opacity', [[t0, 0], [t0 + 0.03 * d, 0.8], [t0 + 0.12 * d, 0]], 'ease_out');
  }
  // the packet path in one continuous line
  for (let i = 0; i < 26; i += 1) {
    const a = Math.PI + (i / 25) * Math.PI;
    const x = cx + Math.cos(a) * radius;
    const y = cy + Math.sin(a) * radius * 0.72;
    ell(`trail${i}`, x - 2, y - 2, 5, 5, A(C.cyan, 90));
    const t0 = 0.22 * d + (i / 25) * 0.5 * d;
    anim(`trail${i}`, 'opacity', [[t0, 0], [t0 + 0.02 * d, 0.8], [t0 + 0.12 * d, 0]], 'linear');
  }

  // the loop closes: everything ends on one timeline
  const T = { x: 480, y: 720, w: 960, h: 140 };
  rect('tlBg', T.x, T.y, T.w, T.h, C.panel, { stroke: A(C.edge2, 180), stroke_width: 1 });
  anim('tlBg', 'opacity', [[0.78 * d, 0], [0.84 * d, 1]], 'ease_out');
  [C.amber, C.violet, C.blue, C.cyan, C.green].forEach((color, i) => {
    rect(`tl${i}`, T.x + 40 + i * 170, T.y + 30 + (i % 2) * 34, 150, 20, A(color, 190));
    anim(`tl${i}`, 'opacity', [[0.82 * d + i * 0.02 * d, 0], [0.86 * d + i * 0.02 * d, 1]], 'ease_out');
  });
  txt('loopLabel', 'MỘT VÒNG LẶP · MỘT CHIẾC MÁY', cx - textW('MỘT VÒNG LẶP · MỘT CHIẾC MÁY', 15) / 2, 866, 15, A(C.mute, 230));
  anim('loopLabel', 'opacity', [[0.84 * d, 0], [0.9 * d, 1]], 'ease_out');

  particles('dust', 960, 520, 30, 400, C.cyan, d, { size: 1.6, speed: 20 });

  return finish({
    name: 'Quy trình khép kín trên máy',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_y: 20 },
      { t: 0.3 * d, zoom: 1.0 },
      { t: 0.6 * d, zoom: 1.04, pan_y: -20 },
      { t: 0.85 * d, zoom: 1.02, pan_y: 10 },
      { t: d, zoom: 1.08, pan_y: -10 },
    ],
  });
}

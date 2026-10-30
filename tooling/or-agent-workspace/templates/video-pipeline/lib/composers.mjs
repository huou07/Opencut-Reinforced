// The beat library. Each composer receives (duration, ctx, tag) and draws one
// layered composition. ctx.phase(dur, fraction) resolves a phase boundary so
// animations line up with the narration lines inside the beat.
import {
  C, A, at, textW, nid, r, e, t, an, fadeIn, fadeOut, pop, centred, moveAll, pulse,
  world, card3d, waveform, laneStrip,
} from './world.mjs';

export const COMPOSER = {
  open(dur, ctx) {
    world('grid', dur, '01');
    // an empty preview frame assembles, then fills with a finished frame
    const fx = 480, fy = 190, fw = 960, fh = 540;
    r('frame', fx, fy, fw, fh, A(C.card, 255), { stroke: A(C.edge, 230), stroke_width: 2 });
    r('frameInner', fx + 16, fy + 16, fw - 32, fh - 32, A(C.bg2, 255));
    t('emptyMark', 'chưa có gì', fx + fw / 2 - 60, fy + fh / 2 - 18, 26, C.dim);
    fadeIn('frame', 0.2);
    an('frame', 'scale', [[0, 0.94], [1.2, 1]], 'ease_out');
    // caret blinking in the empty timeline
    const ty = 780;
    r('tl', 300, ty, 1320, 150, A(C.card2, 255), { stroke: A(C.edge, 200), stroke_width: 1 });
    for (let i = 0; i < 3; i += 1) r(`tlLane${i}`, 320, ty + 18 + i * 42, 1280, 34, A(C.bg, 255));
    r('caret', 322, ty + 18, 3, 118, C.gold);
    an('caret', 'opacity', [[0, 1], [0.5, 0], [1, 1], [1.5, 0], [2, 1]]);
    fadeIn('tl', 0.6);
    // finished content arrives: three lanes fill, frame pops into life
    const arrive = ctx.phase(dur, 0.45);
    for (let i = 0; i < 3; i += 1) {
      r(`filled${i}`, 320, ty + 18 + i * 42, 0, 34, A([C.blue, C.brown, C.teal][i], 210));
      an(`filled${i}`, 'width', [[arrive + i * 0.25, 0], [arrive + 1.2 + i * 0.25, 1180]], 'ease_out');
    }
    const dots = [];
    for (let i = 0; i < 5; i += 1) dots.push(e(`fd${i}`, fx + 120 + i * 150, fy + 180, 60 + i * 6, 60 + i * 6, A([C.teal, C.gold, C.purple, C.blue, C.green][i], 120)));
    dots.forEach((id, i) => { pop(id, arrive + 0.5 + i * 0.14, 16); an(id, 'translation_x', [[arrive + 0.5, 0], [dur, 40 + i * 12]]); });
    fadeOut('emptyMark', arrive - 0.2, 0.3);
  },

  chatLink(dur, ctx) {
    world('dots', dur);
    // boundary line: OR on the left, the harness machine on the right
    for (let i = 0; i < 34; i += 1) r(`bnd${i}`, 1150, 120 + i * 24, 2, 12, A(C.edge, 180));
    t('bndLabel', 'OR', 1080, 96, 13, C.dim);
    t('bndOut', 'trên máy của bạn', 1268, 96, 13, C.dim);

    // OR project card
    card3d('orCard', 110, 220, 560, 520);
    t('orT', 'project · 7', 134, 234, 12, C.dim);
    for (let i = 0; i < 3; i += 1) {
      r(`laneBg${i}`, 134, 280 + i * 88, 512, 56, A(C.bg, 255));
      r(`laneFill${i}`, 146, 292 + i * 88, 0, 32, A([C.blue, C.brown, C.teal][i], 190));
      an(`laneFill${i}`, 'width', [[1.4 + i * 0.2, 0], [2.4 + i * 0.2, 380 - i * 60]], 'ease_out');
    }
    r('playhead', 146, 272, 2, 300, A(C.gold, 210));
    an('playhead', 'translation_x', [[0, 0], [dur, 480]], 'linear');

    // chat panel with a real conversation
    card3d('chatPanel', 720, 200, 400, 470, { stroke: A(C.teal, 170) });
    t('chatT', 'chat assistant', 744, 214, 12, C.teal);
    const msgs = [
      ['viết kịch bản 5 phút', C.blue, 44],
      ['đã đọc timeline', C.card2, 255],
      ['chia 74 cảnh, đặt mốc', C.card2, 255],
      ['xong · chờ bạn duyệt', C.teal, 30],
    ];
    msgs.forEach(([text, color, alphaValue], i) => {
      const y = 258 + i * 84;
      r(`msg${i}`, 744, y, 352, 56, A(color, color === C.card2 ? 255 : alphaValue), { stroke: A(C.edge, 170), stroke_width: 1 });
      t(`msgT${i}`, text, 760, y + 18, 13, C.ink);
      pop(`msg${i}`, 0.3 + i * 0.55, 14);
      fadeIn(`msgT${i}`, 0.35 + i * 0.55, 0.25);
    });

    // harness card outside the boundary
    card3d('harness', 1250, 300, 520, 300, { stroke: A(C.purple, 190) });
    t('harnessT', 'harness đã cài', 1274, 314, 12, C.purple);
    t('harnessS', 'model + tài khoản của bạn', 1274, 338, 11, C.dim);
    ['đọc · sửa · chạy công cụ', 'không giữ dự án', 'không giữ khoá trong OR'].forEach((line, i) => {
      t(`harnessL${i}`, line, 1274, 380 + i * 40, 12, C.mute);
      fadeIn(`harnessL${i}`, 0.9 + i * 0.3, 0.3);
    });
    r('mcpTag', 1274, 520, 200, 34, A(C.teal, 28), { stroke: A(C.teal, 170), stroke_width: 1 });
    t('mcpT', 'MCP · công cụ', 1288, 528, 12, C.teal);
    fadeIn('mcpTag', 1.6); fadeIn('mcpT', 1.7);

    // tokens leave the chat, cross the boundary, and return with a result
    const out = ctx.phase(dur, 0.32), back = ctx.phase(dur, 0.58);
    for (let i = 0; i < 4; i += 1) {
      const dot = e(`tok${i}`, 1100, 330 + i * 20, 9, 9, C.gold);
      an(dot, 'translation_x', [[out + i * 0.1, 0], [out + 0.5 + i * 0.1, 220]], 'ease_in_out');
      an(dot, 'translation_y', [[out + i * 0.1, 0], [out + 0.5 + i * 0.1, 80 + i * 40]], 'ease_in_out');
      fadeIn(dot, out + i * 0.1, 0.15);
      fadeOut(dot, out + 0.55 + i * 0.1, 0.2);
    }
    for (let i = 0; i < 3; i += 1) {
      const dot = e(`ret${i}`, 1350, 520, 8, 8, C.teal);
      an(dot, 'translation_x', [[back + i * 0.15, 0], [back + 0.5 + i * 0.15, -260]], 'ease_in_out');
      an(dot, 'translation_y', [[back + i * 0.15, 0], [back + 0.5 + i * 0.15, -160]], 'ease_in_out');
      fadeIn(dot, back + i * 0.15, 0.15);
      fadeOut(dot, back + 0.6 + i * 0.15, 0.2);
    }
  },

  chatRun(dur, ctx) {
    world('grid', dur, '01');
    card3d('panel', 640, 140, 640, 700, { stroke: A(C.teal, 160) });
    t('ph', 'trợ lý', 666, 154, 13, C.teal);
    const msgs = [
      ['viết kịch bản 5 phút về OR', C.blue],
      ['đang đọc kịch bản cũ…', C.card2],
      ['chia thành 42 cảnh', C.card2],
      ['đã ghi script.json', C.card2],
    ];
    msgs.forEach(([text, fill], i) => {
      const y = 200 + i * 96;
      r(`m${i}`, 664, y, 560, 62, A(fill, fill === C.card2 ? 255 : 48), { stroke: A(C.edge, 170), stroke_width: 1 });
      t(`mT${i}`, text, 682, y + 20, 14, C.ink);
      pop(`m${i}`, 0.3 + i * 0.7, 14);
      fadeIn(`mT${i}`, 0.35 + i * 0.7, 0.25);
    });
    // permission card
    r('perm', 664, 600, 560, 84, A(C.gold, 24), { stroke: A(C.gold, 190), stroke_width: 1 });
    t('permT', 'cho phép ghi tệp trong workspace?', 682, 616, 13, C.gold);
    r('permYes', 682, 644, 120, 28, A(C.teal, 40), { stroke: C.teal, stroke_width: 1 });
    t('permYesT', 'Cho phép', 700, 650, 12, C.teal);
    fadeIn('perm', 3.2); fadeIn('permT', 3.3); fadeIn('permYes', 3.4); fadeIn('permYesT', 3.45);
    // script file appears beside
    card3d('script', 1340, 220, 400, 420, { stroke: A(C.gold, 170) });
    t('scriptT', 'script.json · 42 cảnh', 1364, 234, 12, C.gold);
    for (let i = 0; i < 8; i += 1) {
      r(`sl${i}`, 1364, 274 + i * 34, 0, 7, A(C.ink, 150));
      an(`sl${i}`, 'width', [[3.8 + i * 0.12, 0], [4.3 + i * 0.12, 330]], 'ease_out');
    }
  },

  assembly(dur, ctx) {
    world('rings', dur);
    const sx = 460, sy = 150, sw = 1000, sh = 540;
    r('stage', sx, sy, sw, sh, A(C.card, 255), { stroke: A(C.edge, 210), stroke_width: 2 });
    // the mini-scene that assembles: heading bar, two cards, a chart, an arrow
    const parts = [
      ['p1', 'r', sx + 60, sy + 70, 420, 14, C.gold, -160, -90],
      ['p2', 'r', sx + 60, sy + 120, 520, 150, C.blue, -260, 60],
      ['p3', 'r', sx + 640, sy + 120, 300, 150, C.card2, 240, -70],
      ['p4', 'r', sx + 60, sy + 330, 90, 120, C.teal, -120, 150],
      ['p5', 'r', sx + 190, sy + 330, 90, 80, C.purple, 0, 180],
      ['p6', 'r', sx + 320, sy + 330, 90, 40, C.green, 120, 200],
    ];
    parts.forEach(([id, , x, y, w, h, color, fx, fy], i) => {
      r(id, x, y, w, h, A(color, 215));
      an(id, 'translation_x', [[0.35 + i * 0.17, fx], [1.15 + i * 0.17, 0]], 'ease_out');
      an(id, 'translation_y', [[0.35 + i * 0.17, fy], [1.15 + i * 0.17, 0]], 'ease_out');
      fadeIn(id, 0.35 + i * 0.17, 0.35);
    });
    // an arrow and a caption band inside the scene
    r('arrow', sx + 470, sy + 400, 220, 8, A(C.ink, 200));
    r('arrowHead', sx + 680, sy + 386, 36, 36, A(C.ink, 200), { rotation_degrees: 45 });
    fadeIn('arrow', 1.5, 0.4); fadeIn('arrowHead', 1.5, 0.4);
    r('sceneBand', sx + 60, sy + 470, 880, 30, A(C.ink, 40));
    fadeIn('sceneBand', 1.8, 0.4);

    // the whole stage pulls back into a clip on a timeline strip
    const fly = ctx.phase(dur, 0.66);
    const ids = ['stage', 'arrow', 'arrowHead', 'sceneBand', ...parts.map(([id]) => id)];
    ids.forEach((id) => an(id, 'scale', [[fly, 1], [fly + 0.9, 0.2]], 'ease_in_out'));
    ids.forEach((id) => an(id, 'translation_y', [[fly, 0], [fly + 0.9, 300]], 'ease_in_out'));
    ids.forEach((id) => fadeOut(id, fly + 0.8, 0.3));

    r('tlBg', 300, 720, 1320, 170, A(C.card2, 255), { stroke: A(C.edge, 190), stroke_width: 1 });
    fadeIn('tlBg', fly + 0.5, 0.4);
    for (let i = 0; i < 3; i += 1) r(`tlRow${i}`, 320, 738 + i * 46, 1280, 36, A(C.bg, 255));
    const clip = r('asmClip', 700, 738, 40, 36, A(C.purple, 225));
    an('asmClip', 'width', [[fly + 0.8, 40], [fly + 1.4, 420]], 'ease_out');
    fadeIn('asmClip', fly + 0.8, 0.2);
    t('asmClipT', 'scene.mkv · clip', 716, 748, 12, { red: 10, green: 11, blue: 13, alpha: 235 });
    fadeIn('asmClipT', fly + 1.1, 0.3);
  },

  motionDetail(dur, ctx) {
    world('dots', dur);
    // camera looks through a composition: bars grow, a shape orbits, a rule holds
    card3d('sheet', 420, 120, 1080, 620, { stroke: A(C.purple, 160) });
    t('sheetT', 'scene.ormotion.json', 444, 134, 12, C.mute);
    for (let i = 0; i < 5; i += 1) {
      const hh = 40 + i * 34;
      r(`bar${i}`, 540 + i * 120, 560 - hh, 66, hh, A([C.teal, C.blue, C.purple, C.gold, C.green][i], 210));
      an(`bar${i}`, 'height', [[0.4 + i * 0.16, 6], [1.2 + i * 0.16, hh]], 'ease_out');
      an(`bar${i}`, 'translation_y', [[0.4 + i * 0.16, hh - 6], [1.2 + i * 0.16, 0]], 'ease_out');
    }
    const orbit = e('orb', 1180, 300, 54, 54, C.gold);
    an('orb', 'translation_x', [[0.6, -60], [dur, 60]], 'ease_in_out');
    an('orb', 'translation_y', [[0.6, 20], [dur, -20]], 'ease_in_out');
    const ring = e('ringTrack', 1120, 240, 190, 190, A(C.bg, 0), { stroke: A(C.gold, 90), stroke_width: 2 });
    fadeIn('ringTrack', 0.4);
    // static preview frame (PNG still) flies out to the right
    const out = ctx.phase(dur, 0.55);
    card3d('still', 1380, 200, 360, 210, { stroke: A(C.edge, 190) });
    t('stillT', 'xem trước', 1400, 214, 11, C.dim);
    for (let i = 0; i < 4; i += 1) r(`stillRow${i}`, 1400, 250 + i * 34, 320, 12, A(C.edge, 160));
    an('still', 'translation_x', [[out, 240], [out + 0.7, 0]], 'ease_out');
    fadeIn('still', out, 0.4);
    t('ruleTag', 'không tràn khung', 1396, 432, 12, C.teal);
    fadeIn('ruleTag', out + 0.5, 0.4);
  },

  motionLibrary(dur, ctx) {
    world('grid', dur, '01');
    t('libT', 'thư viện cảnh', 120, 120, 13, C.dim);
    const thumbs = [];
    for (let i = 0; i < 6; i += 1) {
      const x = 120 + (i % 3) * 300, y = 170 + Math.floor(i / 3) * 220;
      card3d(`th${i}`, x, y, 260, 170, { head: false, stroke: A(C.edge, 190) });
      e(`thDot${i}`, x + 40, y + 50, 70, 70, A([C.teal, C.gold, C.purple][i % 3], 110));
      r(`thBar${i}`, x + 24, y + 130, 120, 8, A(C.ink, 120));
      thumbs.push(`th${i}`);
      pop(`th${i}`, 0.2 + i * 0.12, 18);
    }
    // one card lifts, duplicates, parameter chips change
    const pick = ctx.phase(dur, 0.35);
    an('th1', 'translation_y', [[pick, 0], [pick + 0.6, -40]], 'ease_out');
    an('th1', 'scale', [[pick, 1], [pick + 0.8, 1.35]], 'ease_out');
    an('th1', 'translation_x', [[pick + 0.6, 0], [pick + 1.4, 420]], 'ease_in_out');
    const dup = ctx.phase(dur, 0.62);
    card3d('dup', 840, 420, 340, 220, { head: false, stroke: A(C.gold, 190) });
    e('dupDot', 900, 470, 100, 100, A(C.gold, 120));
    r('dupBar', 864, 590, 150, 10, A(C.ink, 140));
    an('dup', 'opacity', [[dup, 0], [dup + 0.6, 1]]);
    an('dup', 'translation_x', [[dup, 260], [dup + 0.6, 0]], 'ease_out');
    for (let i = 0; i < 3; i += 1) {
      r(`pchip${i}`, 840 + i * 116, 660, 104, 34, A(C.card2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
      t(`pchipT${i}`, ['chữ', 'màu', 'tốc độ'][i], 862 + i * 116, 668, 12, C.mute);
      fadeIn(`pchip${i}`, dup + 0.6 + i * 0.2, 0.3);
    }
  },

  voiceGen(dur, ctx) {
    world('dots', dur);
    // script lines on the left turn into a waveform on the right
    card3d('script', 140, 200, 520, 520, { stroke: A(C.edge, 200) });
    t('scT', 'kịch bản', 164, 214, 12, C.mute);
    for (let i = 0; i < 7; i += 1) {
      r(`sr${i}`, 168, 258 + i * 56, 0, 9, A(C.ink, 160));
      an(`sr${i}`, 'width', [[0.3 + i * 0.18, 0], [0.8 + i * 0.18, 440]], 'ease_out');
    }
    const server = ctx.phase(dur, 0.4);
    card3d('srv', 760, 210, 300, 120, { stroke: A(C.teal, 180) });
    t('srvT', 'máy chủ giọng nói', 784, 224, 12, C.teal);
    t('srvS', 'trên máy này', 784, 258, 11, C.dim);
    fadeIn('srv', server); fadeIn('srvT', server + 0.1); fadeIn('srvS', server + 0.15);
    // audio wave grows to the right of the server
    const waveStart = ctx.phase(dur, 0.55);
    waveform('vw', 1140, 330, 620, 26, C.brown, 90, 5);
    for (let i = 0; i < 26; i += 1) fadeIn(`vw${i}`, waveStart + i * 0.035, 0.25);
    // clip lands in the audio lane at the bottom
    laneStrip('aud', 300, 800, 1320, [], [C.brown]);
    r('voiceClip', 320, 803, 30, 28, A(C.brown, 225));
    an('voiceClip', 'width', [[waveStart + 0.9, 30], [waveStart + 1.8, 520]], 'ease_out');
    fadeIn('voiceClip', waveStart + 0.9, 0.2);
    t('voiceClipT', 'dòng 01', 332, 810, 11, { red: 12, green: 12, blue: 12, alpha: 235 });
  },

  voiceChoose(dur, ctx) {
    world('grid', dur, '01');
    const opts = [['giọng hệ thống', C.mute], ['máy chủ cục bộ', C.teal], ['đám mây', C.blue]];
    opts.forEach(([label, color], i) => {
      const y = 240 + i * 150;
      card3d(`vo${i}`, 300, y, 620, 110, { stroke: A(color, 190) });
      e(`voDot${i}`, 330, y + 34, 40, 40, A(color, 160));
      t(`voT${i}`, label, 400, y + 40, 18, C.ink);
      pop(`vo${i}`, 0.2 + i * 0.3, 16);
    });
    // selection moves onto the local server
    const sel = ctx.phase(dur, 0.45);
    r('selRing', 288, 378, 644, 134, A(C.bg, 0), { stroke: C.gold, stroke_width: 3 });
    an('selRing', 'opacity', [[sel, 0], [sel + 0.4, 1]]);
    pulse('selRing', sel + 0.4, 3, 1.0);
    // cost / key chips
    const chips = [['không cần khoá', C.teal], ['chi phí tuỳ dịch vụ', C.gold]];
    chips.forEach(([label, color], i) => {
      r(`cost${i}`, 300 + i * 320, 700, 300, 44, A(color, 26), { stroke: A(color, 170), stroke_width: 1 });
      t(`costT${i}`, label, 320 + i * 320, 712, 13, color);
      fadeIn(`cost${i}`, sel + 0.6 + i * 0.4, 0.4);
      fadeIn(`costT${i}`, sel + 0.7 + i * 0.4, 0.4);
    });
  },

  whisperListen(dur, ctx) {
    world('dots', dur);
    // a big waveform, a playhead sweeping, words popping out
    waveform('ww', 300, 330, 1320, 44, C.teal, 110, 9);
    const ph = r('wph', 300, 230, 3, 200, C.gold);
    an('wph', 'translation_x', [[0, 0], [dur * 0.85, 1320]], 'linear');
    t('wlabel', 'âm thanh', 300, 200, 12, C.dim);
    const words = ['video', 'này', 'bắt', 'đầu', 'từ', 'một', 'câu', 'hỏi'];
    words.forEach((w, i) => {
      const wx = 360 + i * 150;
      t(`wd${i}`, w, wx, 470, 26, C.ink);
      pop(`wd${i}`, 0.5 + i * 0.28, 16);
      const tick = r(`tick${i}`, wx + 6, 400, 2, 30, A(C.gold, 180));
      fadeIn(`tick${i}`, 0.5 + i * 0.28, 0.2);
    });
    // caption chips appear below, aligned
    for (let i = 0; i < 4; i += 1) {
      r(`cc${i}`, 360 + i * 300, 700, 260, 56, A(C.card2, 255), { stroke: A(C.teal, 170), stroke_width: 1 });
      r(`ccBar${i}`, 376 + i * 300, 726, 0, 8, A(C.teal, 190));
      an(`ccBar${i}`, 'width', [[1.6 + i * 0.4, 0], [2.2 + i * 0.4, 220]], 'ease_out');
      fadeIn(`cc${i}`, 1.5 + i * 0.4, 0.3);
    }
  },

  captionSync(dur, ctx) {
    world('grid', dur, '01');
    // preview frame with subtitles appearing in sync, caption lane below
    card3d('prev', 380, 150, 1160, 480, { stroke: A(C.edge, 200) });
    r('prevIn', 400, 170, 1120, 440, A(C.bg, 255));
    const subs = ['Video này bắt đầu', 'từ một câu hỏi', 'làm sao kể cho ai đó', 'hiểu OR là gì'];
    subs.forEach((s, i) => {
      centred(`sub${i}`, s, 960, 520, 30, C.ink);
      an(`sub${i}`, 'opacity', [[0.3 + i * 1.0, 0], [0.45 + i * 1.0, 1], [1.15 + i * 1.0, 1], [1.3 + i * 1.0, 0]]);
    });
    for (let i = 0; i < 4; i += 1) {
      r(`sc${i}`, 420 + i * 280, 720, 240, 40, A(C.teal, 60), { stroke: A(C.teal, 170), stroke_width: 1 });
      an(`sc${i}`, 'opacity', [[0.3 + i * 1.0, 0.2], [0.5 + i * 1.0, 1]]);
    }
    // style panel slides in at the end
    const style = ctx.phase(dur, 0.7);
    card3d('sty', 1380, 240, 400, 300, { stroke: A(C.gold, 170) });
    t('styT', 'kiểu phụ đề', 1404, 254, 12, C.gold);
    ['cỡ chữ', 'vị trí', 'nền'].forEach((label, i) => {
      t(`styL${i}`, label, 1404, 300 + i * 66, 13, C.mute);
      r(`styS${i}`, 1520, 304 + i * 66, 220, 8, A(C.edge, 200));
      e(`styK${i}`, 1540 + i * 70, 296 + i * 66, 16, 16, C.gold);
      fadeIn(`styL${i}`, style + i * 0.2, 0.3);
      fadeIn(`styS${i}`, style + i * 0.2, 0.3);
      fadeIn(`styK${i}`, style + 0.1 + i * 0.2, 0.3);
    });
  },

  editOps(dur, ctx) {
    world('grid', dur, '01');
    // a zoomed timeline where clips are dragged, trimmed, split, rippled
    card3d('tlz', 160, 200, 1600, 560, { stroke: A(C.edge, 200) });
    t('tlzT', 'timeline', 184, 214, 12, C.dim);
    const lanes = [['video', 280], ['audio', 400], ['caption', 520]];
    lanes.forEach(([label, y], i) => {
      t(`ln${i}`, label, 184, y - 22, 11, C.mute);
      r(`lnB${i}`, 300, y, 1400, 76, A(C.bg, 255));
    });
    const clips = [[320, 320, C.blue], [660, 260, C.teal], [940, 300, C.purple]];
    clips.forEach(([x, w, color], i) => {
      r(`ec${i}`, x, 290, w, 56, A(color, 215));
      t(`ecT${i}`, `clip_0${i + 1}`, x + 12, 312, 12, { red: 12, green: 13, blue: 15, alpha: 240 });
    });
    // 1) drag clip 2
    an('ec1', 'translation_x', [[0.6, 0], [1.4, 140]], 'ease_in_out');
    // 2) trim clip 3 right edge
    an('ec2', 'width', [[2.0, 300], [2.8, 180]], 'ease_in_out');
    // 3) split clip 1 at the playhead
    const split = 3.4;
    const ph = r('phl', 560, 270, 3, 300, C.gold);
    an('phl', 'translation_x', [[0, 0], [dur, 900]], 'linear');
    r('blade', 560, 280, 3, 76, C.gold);
    an('blade', 'opacity', [[split - 0.3, 0], [split, 1], [split + 0.5, 0]]);
    r('splitR', 563, 290, 4, 56, A(C.blue, 215));
    an('splitR', 'width', [[split, 4], [split + 0.7, 76]], 'ease_out');
    // 4) ripple: clip 3 slides left after a delete
    an('ec2', 'opacity', [[5.2, 1], [5.6, 0]], 'ease_in');
    an('ec0', 'translation_x', [[5.6, 0], [6.3, -120]], 'ease_in_out');
    t('opLabel', 'kéo · cắt · tách · xoá', 640, 700, 14, C.dim);
    fadeIn('opLabel', 0.5, 0.5);
  },

  previewSync(dur, ctx) {
    world('dots', dur);
    // left: timeline zoom, right: preview updating, wipe between before/after
    card3d('leftTl', 160, 240, 760, 420, { stroke: A(C.edge, 190) });
    for (let i = 0; i < 3; i += 1) r(`ltl${i}`, 190, 300 + i * 96, 700, 60, A(C.bg, 255));
    r('ltlClip', 200, 308, 260, 44, A(C.blue, 215));
    an('ltlClip', 'width', [[0.8, 260], [2.0, 420]], 'ease_in_out');
    card3d('rightPv', 1000, 240, 760, 420, { stroke: A(C.teal, 170) });
    r('pvA', 1024, 268, 712, 364, A(C.blue, 50));
    r('pvB', 1024, 268, 0, 364, A(C.teal, 70));
    an('pvB', 'width', [[1.0, 0], [2.2, 712]], 'ease_in_out');
    t('pvL', 'trước', 1040, 290, 12, C.mute);
    t('pvR', 'sau', 1660, 290, 12, C.teal);
    // playhead sweep across both
    const ph = r('syncPh', 200, 240, 2, 420, A(C.gold, 200));
    an('syncPh', 'translation_x', [[0, 0], [dur, 1480]], 'linear');
    t('syncLabel', 'cùng một lúc', 860, 700, 14, C.dim);
    fadeIn('syncLabel', 1.4, 0.5);
  },

  layers(dur, ctx) {
    world('grid', dur, '01');
    // three planes separate, then snap into the preview frame
    const planes = [
      ['layerV', 520, 210, 880, 300, C.blue, 'hình'],
      ['layerA', 560, 330, 880, 200, C.brown, 'tiếng'],
      ['layerC', 600, 430, 880, 120, C.teal, 'phụ đề'],
    ];
    planes.forEach(([id, x, y, w, h, color, label], i) => {
      r(id, x, y, w, h, A(color, 90), { stroke: A(color, 200), stroke_width: 2 });
      t(`${id}T`, label, x + 16, y + 12, 13, C.ink);
      an(id, 'translation_y', [[0.3 + i * 0.25, 24], [1.2 + i * 0.25, 0]], 'ease_out');
      fadeIn(id, 0.3 + i * 0.25, 0.4);
      an(id, 'translation_x', [[dur * 0.55 + i * 0.1, 0], [dur * 0.55 + 1.0 + i * 0.1, -60 + i * 60]], 'ease_in_out');
    });
    // music duck dip visual
    waveform('mix', 420, 780, 1080, 36, C.purple, 50, 3);
    r('duckBand', 420, 740, 0, 80, A(C.teal, 26));
    an('duckBand', 'width', [[2.2, 0], [3.4, 1080]], 'ease_in_out');
    t('duckT', 'nhạc hạ dưới giọng đọc', 420, 700, 13, C.mute);
    fadeIn('duckT', 2.4);
  },

  automation(dur, ctx) {
    world('dots', dur);
    card3d('chatS', 160, 200, 560, 520, { stroke: A(C.teal, 160) });
    t('chatST', 'chat', 184, 214, 12, C.teal);
    r('req', 190, 270, 500, 58, A(C.blue, 40), { stroke: A(C.blue, 170), stroke_width: 1 });
    t('reqT', 'dựng nhạc, phụ đề, mốc chương', 208, 290, 13, C.ink);
    fadeIn('req', 0.2); fadeIn('reqT', 0.3);
    const tools = ['add music', 'duck −12 dB', 'import captions', 'add markers', 'read timeline'];
    // fan out to the right, then fly into tracks
    tools.forEach((label, i) => {
      const y = 240 + i * 92;
      r(`tool${i}`, 820, y, 420, 56, A(C.card2, 255), { stroke: A(C.purple, 170), stroke_width: 1 });
      t(`toolT${i}`, label, 844, y + 18, 13, C.ink);
      an(`tool${i}`, 'translation_x', [[0.8 + i * 0.22, -300], [1.2 + i * 0.22, 0], [3.2 + i * 0.1, 0], [3.7 + i * 0.1, 340]], 'ease_in_out');
      fadeIn(`tool${i}`, 0.8 + i * 0.22, 0.3);
      fadeIn(`toolT${i}`, 0.9 + i * 0.22, 0.3);
      fadeOut(`tool${i}`, 3.5 + i * 0.1, 0.3);
      fadeOut(`toolT${i}`, 3.5 + i * 0.1, 0.3);
    });
    // tracks fill as tools land
    laneStrip('aLane', 1260, 720, 520, [], [C.brown]);
    laneStrip('cLane', 1260, 800, 520, [], [C.teal]);
    for (let i = 0; i < 4; i += 1) {
      r(`autoClip${i}`, 1280 + i * 120, 723, 100, 28, A([C.brown, C.teal][i % 2], 210));
      fadeIn(`autoClip${i}`, 3.6 + i * 0.2, 0.3);
      an(`autoClip${i}`, 'translation_y', [[3.6 + i * 0.2, 20], [3.9 + i * 0.2, 0]], 'ease_out');
    }
  },

  localDevice(dur, ctx) {
    world('rings', dur);
    // a device outline with local engines inside, cloud crossed outside
    card3d('dev', 560, 150, 800, 620, { stroke: A(C.teal, 180) });
    t('devT', 'máy của bạn', 584, 164, 12, C.teal);
    const inside = [['cảnh động', C.purple], ['giọng đọc', C.brown], ['phụ đề', C.teal], ['dựng phim', C.blue]];
    inside.forEach(([label, color], i) => {
      const x = 620 + (i % 2) * 350, y = 250 + Math.floor(i / 2) * 240;
      card3d(`in${i}`, x, y, 300, 170, { head: false, stroke: A(color, 180) });
      t(`inT${i}`, label, x + 24, y + 122, 15, C.ink);
      e(`inD${i}`, x + 24, y + 24, 46, 46, A(color, 140));
      pop(`in${i}`, 0.3 + i * 0.22, 16);
    });
    // cloud, crossed, outside
    e('cloud', 1520, 300, 220, 110, A(C.mute, 60), { stroke: A(C.mute, 160), stroke_width: 2 });
    r('cloudX1', 1560, 336, 150, 6, C.red, { rotation_degrees: 35 });
    r('cloudX2', 1560, 354, 150, 6, C.red, { rotation_degrees: -35 });
    t('cloudT', 'không bắt buộc', 1556, 430, 12, C.dim);
    fadeIn('cloud', 1.6); fadeIn('cloudX1', 1.9); fadeIn('cloudX2', 1.9); fadeIn('cloudT', 2.0);
    // file stays on disk indicator
    r('disk', 620, 700, 300, 40, A(C.card2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
    t('diskT', 'dự án + tệp phim trên ổ đĩa', 636, 712, 12, C.mute);
    fadeIn('disk', 2.6);
  },

  freeLicence(dur, ctx) {
    world('grid', dur, '01');
    const badges = [['MIT', 'giấy phép phần mềm', C.teal], ['VieNeu', 'Apache-2.0', C.brown], ['Whisper', 'mô hình riêng', C.blue]];
    badges.forEach(([name, sub, color], i) => {
      const x = 240 + i * 520;
      card3d(`bd${i}`, x, 260, 440, 260, { stroke: A(color, 190) });
      t(`bdN${i}`, name, x + 28, 300, 34, C.ink);
      t(`bdS${i}`, sub, x + 28, 360, 14, color);
      pop(`bd${i}`, 0.3 + i * 0.35, 20);
    });
    // honest cost note as a separate flowing chip
    r('note', 420, 610, 1080, 90, A(C.gold, 18), { stroke: A(C.gold, 160), stroke_width: 1 });
    t('noteT', 'harness hoặc dịch vụ trả phí: chi phí do bên đó quyết định', 448, 638, 16, C.gold);
    fadeIn('note', 1.8); fadeIn('noteT', 1.9);
  },

  platforms(dur, ctx) {
    world('dots', dur);
    const rows = [['macOS', 0.92, C.teal], ['Windows', 0.86, C.blue], ['Linux', 0.8, C.purple], ['Android', 0.6, C.gold]];
    rows.forEach(([name, level, color], i) => {
      const y = 240 + i * 130;
      t(`pf${i}`, name, 260, y + 18, 18, C.ink);
      r(`pfB${i}`, 460, y, 900, 26, A(C.card2, 255));
      r(`pfF${i}`, 460, y, 0, 26, A(color, 200));
      an(`pfF${i}`, 'width', [[0.4 + i * 0.3, 0], [1.4 + i * 0.3, 900 * level]], 'ease_out');
      t(`pfN${i}`, i === 3 ? 'panel trợ lý cần tiến trình riêng' : 'đường phần cứng + dự phòng phần mềm', 1380, y + 4, 11, C.dim);
      fadeIn(`pfN${i}`, 1.2 + i * 0.3, 0.4);
    });
  },

  finale(dur, ctx) {
    world('rings', dur);
    // layers converge into one preview frame, then export progress
    const fx = 560, fy = 160, fw = 800, fh = 450;
    r('finFrame', fx, fy, fw, fh, A(C.bg2, 255), { stroke: A(C.edge, 220), stroke_width: 2 });
    const parts = [
      ['pvA', fx + 40, fy + 40, 340, 180, C.purple],
      ['pvB', fx + 420, fy + 60, 320, 140, C.blue],
      ['pvC', fx + 80, fy + 260, 640, 60, C.teal],
      ['pvD', fx + 80, fy + 340, 420, 60, C.brown],
    ];
    parts.forEach(([id, x, y, w, h, color], i) => {
      r(id, x + 260, y + 160, w, h, A(color, 120), { stroke: A(color, 200), stroke_width: 2 });
      an(id, 'translation_x', [[0.3 + i * 0.3, 260], [1.1 + i * 0.3, 0]], 'ease_in_out');
      an(id, 'translation_y', [[0.3 + i * 0.3, 160], [1.1 + i * 0.3, 0]], 'ease_in_out');
      fadeIn(id, 0.3 + i * 0.3, 0.5);
    });
    // export bar
    const ex = ctx.phase(dur, 0.6);
    r('exBar', 560, 680, 800, 16, A(C.card2, 255));
    r('exFill', 560, 680, 0, 16, C.gold);
    an('exFill', 'width', [[ex, 0], [ex + 1.8, 800]], 'ease_in_out');
    t('exT', 'xuất · Matroska / WebM', 560, 706, 13, C.mute);
    fadeIn('exT', ex);
    // final tick
    const done = ctx.phase(dur, 0.88);
    e('doneDot', 930, 640, 40, 40, C.teal);
    fadeIn('doneDot', done, 0.3);
  },

  close(dur, ctx) {
    world('grid', dur, '01');
    // a map of the workflow lights up in order
    const steps = [['ý tưởng', C.gold], ['cảnh động', C.purple], ['giọng đọc', C.brown], ['phụ đề', C.teal], ['dựng', C.blue], ['xuất', C.green]];
    steps.forEach(([label, color], i) => {
      const x = 200 + i * 270;
      e(`csDot${i}`, x, 380, 54, 54, A(color, 170));
      t(`csT${i}`, label, x - 6, 470, 15, C.mute);
      fadeIn(`csDot${i}`, 0.3 + i * 0.35, 0.4);
      fadeIn(`csT${i}`, 0.35 + i * 0.35, 0.4);
      if (i < steps.length - 1) {
        r(`csL${i}`, x + 60, 404, 0, 4, A(color, 140));
        an(`csL${i}`, 'width', [[0.7 + i * 0.35, 0], [1.0 + i * 0.35, 210]], 'ease_out');
      }
    });
    // one request line from the left
    r('reqLine', 160, 560, 0, 3, A(C.teal, 180));
    an('reqLine', 'width', [[0.4, 0], [1.4, 1600]], 'ease_out');
    t('closeT', 'một dự án · nhiều công cụ · cùng một chỗ', 560, 620, 17, C.dim);
    fadeIn('closeT', 2.4, 0.6);
  },
};

// --- beat grouping ----------------------------------------------------------
const BEATS = [
  { id: 'open', composer: 'open', camera: 'push', lines: ['mo-cau-hoi', 'mo-du-an-trong', 'mo-nam-phut'] },
  { id: 'chatLink', composer: 'chatLink', camera: 'pan', lines: ['chat-giong-mo-rong', 'chat-khong-phai-agent', 'chat-chon-workspace'] },
  { id: 'chatRun', composer: 'chatRun', camera: 'push', lines: ['chat-yeu-cau', 'chat-ket-qua', 'chat-tiep', 'chat-ngu-canh', 'chat-quyen', 'chat-lenh-that'] },
  { id: 'assembly', composer: 'assembly', camera: 'pull', lines: ['canh-tu-kich-ban', 'canh-khoi-lap-rap'] },
  { id: 'motionDetail', composer: 'motionDetail', camera: 'fly', lines: ['canh-tat-dinh', 'canh-xem-truoc', 'canh-anh', 'canh-mau', 'canh-chu'] },
  { id: 'motionLibrary', composer: 'motionLibrary', camera: 'pan', lines: ['canh-thu-vien', 'canh-camera', 'canh-chuyen-canh'] },
  { id: 'voiceGen', composer: 'voiceGen', camera: 'push', lines: ['giong-may-chu', 'giong-tung-dong'] },
  { id: 'voiceChoose', composer: 'voiceChoose', camera: 'pan', lines: ['giong-ba-nguon', 'giong-nhan-ban', 'giong-toc-do', 'giong-nghe-thu', 'giong-mau', 'giong-chi-phi'] },
  { id: 'whisperListen', composer: 'whisperListen', camera: 'push', lines: ['phu-de-whisper', 'phu-de-mo-hinh', 'phu-de-tung-tu'] },
  { id: 'captionSync', composer: 'captionSync', camera: 'pull', lines: ['phu-de-duyet', 'phu-de-kieu', 'phu-de-nhap-xuat', 'phu-de-sua'] },
  { id: 'editOps', composer: 'editOps', camera: 'fly', lines: ['dung-nhieu-lop', 'dung-cat-ghep', 'dung-toc-do', 'dung-nhom'] },
  { id: 'previewSync', composer: 'previewSync', camera: 'pan', lines: ['dung-xem-truoc', 'dung-phat'] },
  { id: 'layers', composer: 'layers', camera: 'pull', lines: ['dung-nhac', 'dung-marker', 'dung-media', 'dung-dinh-dang', 'dung-thu-vien', 'dung-am-luong'] },
  { id: 'automation', composer: 'automation', camera: 'pan', lines: ['auto-mot-yeu-cau', 'auto-nhieu-lenh', 'auto-doc-lai', 'auto-canh', 'auto-motion', 'auto-hoan-tac'] },
  { id: 'localDevice', composer: 'localDevice', camera: 'push', lines: ['local-tren-may', 'local-khong-tai-len', 'local-cache'] },
  { id: 'freeLicence', composer: 'freeLicence', camera: 'push', lines: ['local-ma-nguon-mo', 'local-giay-phep', 'local-chi-phi'] },
  { id: 'platforms', composer: 'platforms', camera: 'pan', lines: ['may-nen-tang', 'may-cau-hinh', 'may-xuat', 'may-o-cung', 'may-gpu', 'may-dien-thoai'] },
  { id: 'finale', composer: 'finale', camera: 'pull', lines: ['ket-hop-nhat', 'ket-xuat'] },
  { id: 'close', composer: 'close', camera: 'push', lines: ['ket-mo-lai', 'ket-khong-phai-chatbot', 'ket-thu-nghiem', 'ket-bat-dau-nhanh', 'ket-thu-ngay'] },
];

const CAMERA = {
  push: (dur) => [{ at: at(0), zoom: 1.0 }, { at: at(dur), zoom: 1.09, pan_x: -14 }],
  pull: (dur) => [{ at: at(0), zoom: 1.12, pan_x: 10 }, { at: at(dur), zoom: 1.0 }],
  pan: (dur) => [{ at: at(0), zoom: 1.06, pan_x: 50 }, { at: at(dur), zoom: 1.06, pan_x: -50 }],
  fly: (dur) => [{ at: at(0), zoom: 0.94 }, { at: at(dur * 0.55), zoom: 1.16, pan_x: -20 }, { at: at(dur), zoom: 1.02, pan_x: 18 }],
};

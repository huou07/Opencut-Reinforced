// S3 — 13–22 s. Chat Assistant is the focus: a request leaves the panel, an MCP
// link reaches outside harnesses, and the result comes back into the edit.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW, mark, since } from '../kit.mjs';

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 700, 200, 1400, 1000, A(C.violet, 12));
  ell('glowB', 120, 500, 900, 800, A(C.blue, 10));
  for (let i = 0; i < 11; i += 1) rect(`gx${i}`, 140 + i * 160, 0, 1, 1080, A(C.edge, 22));
  for (let i = 0; i < 7; i += 1) rect(`gy${i}`, 0, 120 + i * 140, 1920, 1, A(C.edge, 22));

  // ------------------------------------------------------------------ chat panel
  const P = { x: 190, y: 176, w: 620, h: 596 };
  const m0 = mark();
  rect('chatShadow', P.x + 14, P.y + 16, P.w, P.h, { red: 0, green: 0, blue: 0, alpha: 140 });
  rect('chat', P.x, P.y, P.w, P.h, C.panel, { stroke: A(C.edge2, 215), stroke_width: 1 });
  rect('chatBar', P.x, P.y, P.w, 44, C.panel2);
  ell('chatDot', P.x + 20, P.y + 18, 8, 8, A(C.violet, 240));
  txt('chatTitle', 'CHAT ASSISTANT', P.x + 38, P.y + 22, 14, A(C.ink, 230));
  txt('chatModel', 'HARNESS · MCP', P.x + P.w - 24 - textW('HARNESS · MCP', 12), P.y + 23, 12, A(C.dim, 220));
  rect('chatRule', P.x, P.y + 44, P.w, 1, A(C.edge2, 180));
  const ids = since(m0);
  ids.forEach((id) => {
    anim(id, 'translation_x', [[0, -40], [0.55, 0]], 'ease_out');
    anim(id, 'opacity', [[0, 0], [0.4, 1]], 'ease_out');
  });

  // user request
  rect('userBub', P.x + 150, P.y + 76, 440, 74, A(C.panel3, 255), { stroke: A(C.edge, 200), stroke_width: 1 });
  txt('userLabel', 'BẠN', P.x + 28, P.y + 84, 11, A(C.dim, 220));
  txt('userText', 'Tạo cảnh mở đầu 3 giây', P.x + 168, P.y + 104, 22, A(C.ink, 240));
  anim('userText', 'opacity', [[0.5, 0], [0.8, 0.35], [1.0, 0.7], [1.2, 1]], 'linear');
  rect('caret', P.x + 396, P.y + 94, 2, 30, A(C.violet, 230));
  anim('caret', 'opacity', [[1.2, 0], [1.3, 1], [1.4, 0], [1.5, 1], [1.6, 0]], 'linear');
  anim('caret', 'translation_x', [[0.7, 0], [0.75, 0]], 'linear');

  // assistant reply blocks
  rect('aiBub', P.x + 28, P.y + 178, 500, 60, A(C.panel2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
  txt('aiLabel', 'OR', P.x + 28, P.y + 188, 11, A(C.violet, 230));
  txt('aiText', 'Đang gọi harness qua MCP…', P.x + 44, P.y + 206, 18, A(C.mute, 235));
  anim('aiBub', 'opacity', [[1.5, 0], [1.8, 1]], 'ease_out');
  anim('aiText', 'opacity', [[1.6, 0], [1.9, 1]], 'ease_out');
  anim('aiLabel', 'opacity', [[1.5, 0], [1.8, 1]], 'ease_out');

  // typing dots while the harness works
  for (let i = 0; i < 3; i += 1) {
    ell(`typing${i}`, P.x + 44 + i * 16, P.y + 250, 7, 7, A(C.violet, 200));
    anim(`typing${i}`, 'opacity', [[2.4 + i * 0.16, 0.2], [2.6 + i * 0.16, 1], [2.8 + i * 0.16, 0.2], [3.0 + i * 0.16, 1], [3.2 + i * 0.16, 0.2]], 'linear');
  }

  // the scene the harness is building, inside the chat
  rect('genCard', P.x + 28, P.y + 380, 240, 136, A(C.bg2, 255), { stroke: A(C.edge2, 180), stroke_width: 1 });
  rect('genSky', P.x + 29, P.y + 381, 238, 80, { red: 16, green: 20, blue: 27, alpha: 255 });
  ell('genSun', P.x + 150, P.y + 400, 56, 56, A(C.amber, 230));
  rect('genGround', P.x + 29, P.y + 452, 238, 63, { red: 9, green: 11, blue: 14, alpha: 255 });
  rect('genBarBg', P.x + 28, P.y + 530, 240, 3, A(C.edge2, 200));
  rect('genBar', P.x + 28, P.y + 529, 20, 5, A(C.violet, 230));
  anim('genBar', 'width', [[3.2, 20], [5.0, 240]], 'linear');
  txt('genLabel', 'SCENE 01 · 3.0s', P.x + 284, P.y + 400, 14, A(C.mute, 220));
  txt('genLabel2', '1920×1080 · 30p', P.x + 284, P.y + 426, 12, A(C.dim, 200));
  ['genCard', 'genSky', 'genSun', 'genGround', 'genBarBg', 'genBar', 'genLabel', 'genLabel2'].forEach((id) => {
    anim(id, 'opacity', [[3.0, 0], [3.3, 1], [5.1, 1], [5.3, 0]], 'ease_in_out');
  });
  anim('genSun', 'translation_y', [[3.3, 14], [4.8, -6]], 'ease_in_out');

  // result line + the clip card that comes home
  const R = 6.6;
  rect('aiBub2', P.x + 28, P.y + 286, 470, 56, A(C.panel2, 255), { stroke: A(C.edge, 180), stroke_width: 1 });
  txt('aiText2', 'Xong. Đã thêm vào timeline.', P.x + 44, P.y + 312, 18, A(C.ink, 235));
  anim('aiBub2', 'opacity', [[R, 0], [R + 0.3, 1]], 'ease_out');
  anim('aiText2', 'opacity', [[R + 0.1, 0], [R + 0.4, 1]], 'ease_out');

  // ------------------------------------------------------------------ MCP hub
  const H = { cx: 1180, cy: 456 };
  rect('hubRing', H.cx - 92, H.cy - 92, 184, 184, A(C.violet, 0), { stroke: A(C.violet, 90), stroke_width: 1 });
  ell('hubGlow', H.cx - 130, H.cy - 130, 260, 260, A(C.violet, 22));
  const hub = rect('hub', H.cx - 42, H.cy - 42, 84, 84, C.panel3, { stroke: A(C.violet, 230), stroke_width: 2, rotation_degrees: 45 });
  txt('hubLabel', 'MCP', H.cx - textW('MCP', 20) / 2, H.cy + 6, 20, A(C.ink, 245));
  anim('hub', 'rotation', [[0, 0], [d, -8]], 'linear');
  anim('hubRing', 'rotation', [[0, 0], [d, 12]], 'linear');
  anim('hubGlow', 'scale', [[0, 0.9], [2.4, 1.15], [4.6, 1.0], [6, 1.1]], 'ease_in_out');
  ['hub', 'hubLabel', 'hubRing', 'hubGlow'].forEach((id) => anim(id, 'opacity', [[2.0, 0], [2.4, 1]], 'ease_out'));

  // harness chips on a rail
  const harnesses = [
    ['CLAUDE CODE', C.amber, C.amber],
    ['CODEX', C.ink, C.cyan],
    ['ANTIGRAVITY', C.cyan, C.cyan],
    ['DSH', C.violet, C.violet],
  ];
  harnesses.forEach(([name, color], i) => {
    const x = 1352, y = 236 + i * 116, w = 292, h = 76;
    const t0 = 2.4 + i * 0.22;
    rect(`ch${i}Shadow`, x + 8, y + 10, w, h, { red: 0, green: 0, blue: 0, alpha: 120 });
    rect(`ch${i}`, x, y, w, h, C.panel2, { stroke: A(C.edge2, 200), stroke_width: 1 });
    rect(`ch${i}Slot`, x - 12, y + h / 2 - 4, 12, 8, A(color, 220));
    ell(`ch${i}Dot`, x + 24, y + 22, 9, 9, A(color, 240));
    txt(`ch${i}T`, name, x + 44, y + 28, 16, A(C.ink, 235));
    txt(`ch${i}S`, 'HARNESS', x + 24, y + 54, 11, A(C.dim, 210));
    const lineX = x - 12, lineW = lineX - (H.cx + 40);
    rect(`link${i}`, H.cx + 40, y + h / 2, lineW, 1, A(C.edge2, 160));
    anim(`link${i}`, 'opacity', [[t0 - 0.1, 0], [t0 + 0.3, 1]], 'ease_out');
    // extension behaviour: the chip docks into the hub rail
    anim(`ch${i}`, 'translation_x', [[t0, 150], [t0 + 0.5, 0]], 'ease_out');
    anim(`ch${i}Shadow`, 'translation_x', [[t0, 150], [t0 + 0.5, 0]], 'ease_out');
    anim(`ch${i}Slot`, 'translation_x', [[t0 + 0.4, 0], [t0 + 0.62, -150], [t0 + 0.9, 0]], 'ease_out');
    anim(`ch${i}T`, 'translation_x', [[t0, 150], [t0 + 0.5, 0]], 'ease_out');
    anim(`ch${i}S`, 'translation_x', [[t0, 150], [t0 + 0.5, 0]], 'ease_out');
    anim(`ch${i}Dot`, 'translation_x', [[t0, 150], [t0 + 0.5, 0]], 'ease_out');
    ['', 'Slot', 'T', 'S', 'Dot', 'Shadow'].forEach((suffix) => anim(`ch${i}${suffix}`, 'opacity', [[t0, 0], [t0 + 0.16, 1]], 'ease_out'));
    // pulse when the link is live

  });

  // beam from the chat to the hub
  rect('beam', P.x + P.w, H.cy - 1, H.cx - 44 - (P.x + P.w), 2, A(C.violet, 90));
  anim('beam', 'width', [[2.0, 0], [2.6, H.cx - 44 - (P.x + P.w)]], 'ease_out');
  const beamStart = P.x + P.w, beamEnd = H.cx - 46;
  for (let i = 0; i < 10; i += 1) {
    const t0 = 2.4 + i * 0.18;
    ell(`spark${i}`, beamStart + 10, H.cy - 3, 6, 6, A(C.ink, 230));
    anim(`spark${i}`, 'translation_x', [[t0, 0], [t0 + 1.3, beamEnd - beamStart - 10]], 'linear');
    anim(`spark${i}`, 'opacity', [[t0, 0], [t0 + 0.12, 1], [t0 + 0.5, 0.7], [t0 + 0.66, 0]], 'linear');
    anim(`spark${i}`, 'scale', [[t0, 1], [t0 + 0.66, 0.4]], 'linear');
    anim(`spark${i}`, 'scale', [[4.6 + i * 0.1, 0.4], [4.72 + i * 0.1, 1.2], [5.3 + i * 0.1, 0.6]], 'linear');
    anim(`spark${i}`, 'translation_x', [[4.6 + i * 0.1, beamEnd - beamStart - 10], [5.3 + i * 0.1, 0]], 'linear');
    anim(`spark${i}`, 'opacity', [[4.6 + i * 0.1, 0], [4.72 + i * 0.1, 1], [5.2 + i * 0.1, 0.6], [5.3 + i * 0.1, 0]], 'linear');
  }

  // result card leaves the hub and lands on a mini timeline
    const rc = 6.6;
  rect('resThumb', 1180 - 30, 456 - 20, 60, 40, A(C.amber, 220));
  anim('resThumb', 'opacity', [[rc, 0], [rc + 0.1, 1]], 'linear');
  anim('resThumb', 'translation_x', [[rc, 0], [rc + 0.5, -360]], 'ease_in_out');
  anim('resThumb', 'translation_y', [[rc, 0], [rc + 0.5, 330]], 'ease_in_out');
  anim('resThumb', 'scale', [[rc, 1], [rc + 0.5, 1.5]], 'ease_out');
  anim('resThumb', 'opacity', [[rc + 0.42, 1], [rc + 0.52, 0]], 'ease_in');

  // --------------------------------------------------------------- mini timeline
  const T = { x: 190, y: 840, w: 1540 };
  rect('tlBg', T.x, T.y, T.w, 96, A(C.panel, 255), { stroke: A(C.edge2, 160), stroke_width: 1 });
  rect('tlV', T.x + 20, T.y + 16, 600, 26, A(C.amber, 180));
  rect('tlA', T.x + 20, T.y + 52, 600, 18, A(C.cyan, 160));
  rect('tlCap', T.x + 640, T.y + 52, 300, 18, A(C.ink, 120));
  anim('tlBg', 'opacity', [[2.6, 0], [3.0, 1]], 'ease_out');
  ['tlV', 'tlA', 'tlCap'].forEach((id) => anim(id, 'opacity', [[2.8, 0], [3.2, 1]], 'ease_out'));
  rect('tlNew', T.x + 636, T.y + 16, 210, 26, A(C.amber, 235));
  rect('tlNewCap', T.x + 636, T.y + 16, 210, 2, A(C.ink, 220));
  anim('tlNewCap', 'opacity', [[rc + 0.5, 0], [rc + 0.7, 1]], 'ease_out');
  anim('tlNew', 'opacity', [[rc + 0.5, 0], [rc + 0.7, 1]], 'ease_out');
  anim('tlNew', 'scale', [[rc + 0.5, 0.3], [rc + 0.8, 1]], 'ease_out');
  rect('tlPlay', T.x + 20, T.y + 10, 2, 76, A(C.red, 210));
  anim('tlPlay', 'translation_x', [[3.2, 0], [6.9, 1080]], 'linear');

  // a screen label naming the extension relationship, small and low
  txt('extLabel', 'MỞ RỘNG QUA MCP', 1352, 762, 14, A(C.dim, 220));
  anim('extLabel', 'opacity', [[3.2, 0], [3.5, 1]], 'ease_out');

  return finish({
    name: opts.name ?? 'Assistant + MCP',
    duration: d,
    camera: [
      { t: 0, zoom: 1.03, pan_x: 20 },
      { t: 1.6, zoom: 1.0 },
      { t: 2.4, zoom: 1.05, pan_x: -30 },
      { t: 3.6, zoom: 1.03, pan_x: -46 },
      { t: 4.8, zoom: 1.06, pan_x: -20, pan_y: 10 },
      { t: 6.2, zoom: 1.1, pan_x: 0, pan_y: 30 },
      { t: 9, zoom: 1.02, pan_y: 0 },
    ],
  });
}

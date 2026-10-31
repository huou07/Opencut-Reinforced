// S7 — 52–59 s. The close: mark, wordmark, what it is, and an invitation.
import { begin, finish, rect, ell, txt, centre, A, C, anim, particles, textW } from '../kit.mjs';

export function build(duration, opts = {}) {
  begin();
  const d = duration;

  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 640, 300, 1400, 1100, A(C.amber, 12));
  ell('glowB', 900, 300, 1100, 900, A(C.violet, 10));
  for (let i = 0; i < 4; i += 1) {
    const r = 200 + i * 130;
    ell(`ring${i}`, 960 - r, 468 - r, r * 2, r * 2, A(C.edge2, 0), { stroke: A(C.edge2, 34 - i * 6), stroke_width: 1 });
    anim(`ring${i}`, 'rotation', [[0, 0], [d, i % 2 ? -14 : 16]], 'linear');
  }
  // converging particles
  for (let i = 0; i < 40; i += 1) {
    const angle = (i / 40) * Math.PI * 2;
    const dist = 640 + (i % 5) * 60;
    const x = 960 + Math.cos(angle) * dist;
    const y = 460 + Math.sin(angle) * dist * 0.6;
    ell(`p${i}`, x, y, 3, 3, A(i % 3 === 0 ? C.amber : C.mute, 120));
    anim(`p${i}`, 'translation_x', [[0, 0], [1.1, Math.cos(angle) * -dist * 0.86]], 'ease_in');
    anim(`p${i}`, 'translation_y', [[0, 0], [1.1, Math.sin(angle) * -dist * 0.86 * 0.6]], 'ease_in');
    anim(`p${i}`, 'opacity', [[0, 0.9], [0.9, 0.9], [1.3, 0]], 'ease_in');
  }
  rect('hairTop', 960, 470, 0, 1, A(C.edge2, 160));
  anim('hairTop', 'width', [[0, 0], [0.5, 360]], 'ease_out');
  anim('hairTop', 'translation_x', [[0, -180], [0.5, 0]], 'ease_out');
  anim('hairTop', 'opacity', [[1.6, 1], [2.0, 0.35]], 'ease_in_out');

  // ------------------------------------------------------------------- the mark
  const mk = { cx: 960, cy: 262 };
  ell('markRing', mk.cx - 46, mk.cy - 46, 92, 92, A(C.amber, 0), { stroke: A(C.amber, 230), stroke_width: 3 });
  rect('markDia', mk.cx - 20, mk.cy - 20, 40, 40, A(C.amber, 220), { rotation_degrees: 45 });
  rect('markDiaIn', mk.cx - 10, mk.cy - 10, 20, 20, A(C.bg, 255), { rotation_degrees: 45 });
  rect('markBar', mk.cx - 64, mk.cy - 2, 18, 4, A(C.amber, 200));
  rect('markBarR', mk.cx + 46, mk.cy - 2, 18, 4, A(C.amber, 200));
  ['markRing', 'markDia', 'markDiaIn', 'markBar', 'markBarR'].forEach((id) => {
    anim(id, 'opacity', [[0.35, 0], [0.75, 1]], 'ease_out');
  });
  anim('markRing', 'scale', [[0.35, 0.7], [1.0, 1]], 'ease_out');
  anim('markDia', 'scale', [[0.35, 0.5], [0.9, 1]], 'ease_out');
  anim('markDiaIn', 'scale', [[0.35, 0.5], [0.9, 1]], 'ease_out');
  anim('markRing', 'rotation', [[0.35, -30], [2.2, 0]], 'ease_out');

  // --------------------------------------------------------------- the wordmark
  const word = 'OpenCut Reinforced';
  const wsize = 82;
  const total = textW(word, wsize);
  const wx = 960 - total / 2;
  txt('word', word, wx, 402, wsize, A(C.ink, 250));
  anim('word', 'opacity', [[1.0, 0], [1.5, 1]], 'ease_out');
  anim('word', 'translation_y', [[1.0, 22], [1.6, 0]], 'ease_out');
  rect('wordUnder', 960 - 150, 468, 300, 3, A(C.amber, 210));
  anim('wordUnder', 'width', [[1.4, 0], [2.0, 300]], 'ease_out');
  anim('wordUnder', 'translation_x', [[1.4, -150], [2.0, 0]], 'ease_out');

  // ------------------------------------------------------------------- tagline
  centre('tag', 'MIỄN PHÍ · XỬ LÝ LOCAL', 960, 500, 25, A(C.mute, 240));
  anim('tag', 'opacity', [[2.0, 0], [2.4, 1]], 'ease_in_out');
  anim('tag', 'translation_y', [[2.0, 12], [2.5, 0]], 'ease_out');
  rect('tagDot', 960 - textW('MIỄN PHÍ · XỬ LÝ LOCAL', 25) / 2 - 26, 494, 6, 6, A(C.green, 230));
  anim('tagDot', 'opacity', [[2.0, 0], [2.4, 1]], 'ease_in_out');

  // chips
  const chips = [['MIỄN PHÍ', C.green], ['LOCAL', C.cyan], ['AI HARNESS', C.violet]];
  let cx = 768;
  chips.forEach(([label, color], i) => {
    const w = textW(label, 16) + 34;
    rect(`chip${i}`, cx, 536, w, 38, A(color, 22), { stroke: A(color, 140), stroke_width: 1 });
    txt(`chipT${i}`, label, cx + 17, 555, 16, A(color, 235));
    const t0 = 2.7 + i * 0.16;
    anim(`chip${i}`, 'opacity', [[t0, 0], [t0 + 0.3, 1]], 'ease_out');
    anim(`chipT${i}`, 'opacity', [[t0, 0], [t0 + 0.3, 1]], 'ease_out');
    anim(`chip${i}`, 'width', [[t0, 6], [t0 + 0.4, w]], 'ease_out');
    anim(`chip${i}`, 'translation_y', [[t0, 8], [t0 + 0.35, 0]], 'ease_out');
    anim(`chipT${i}`, 'translation_y', [[t0, 8], [t0 + 0.35, 0]], 'ease_out');
    anim(`chip${i}`, 'translation_x', [[t0, w / 2 - 3], [t0 + 0.4, 0]], 'ease_out');
    cx += w + 16;
  });

  // fine print: the licence and the conditions, one short sentence
  const fine = 'Nhiều khả năng miễn phí khi phần cứng, model và dịch vụ đáp ứng điều kiện.';
  centre('fine', fine, 960, 630, 19, A(C.dim, 235));
  anim('fine', 'opacity', [[3.3, 0], [3.8, 1]], 'ease_in_out');

  // --------------------------------------------------------------------- CTA
  const ctaY = 726;
  rect('ctaBox', 960 - 210, ctaY - 30, 420, 60, A(C.amber, 0), { stroke: A(C.amber, 170), stroke_width: 1 });
  anim('ctaBox', 'opacity', [[4.6, 0], [5.0, 1]], 'ease_out');
  anim('ctaBox', 'width', [[4.6, 20], [5.1, 420]], 'ease_out');
  anim('ctaBox', 'translation_x', [[4.6, 200], [5.1, 0]], 'ease_out');
  txt('cta', 'Trải nghiệm ngay', 960 - textW('Trải nghiệm ngay', 27) / 2, ctaY + 2, 27, A(C.ink, 245));
  anim('cta', 'opacity', [[4.9, 0], [5.3, 1]], 'ease_out');
  rect('ctaArrow', 1196, ctaY - 1, 100, 2, A(C.amber, 230));
  anim('ctaArrow', 'width', [[4.9, 0], [5.4, 100]], 'ease_out');
  anim('ctaArrow', 'translation_x', [[4.9, -50], [5.4, 0]], 'ease_out');
  rect('ctaArrowTip', 1286, ctaY - 10, 20, 20, A(C.amber, 230), { rotation_degrees: 45 });
  anim('ctaArrowTip', 'opacity', [[5.2, 0], [5.4, 1]], 'ease_out');

  // exit crumbs
  txt('w1', 'OpenCut Reinforced', 64, 1010, 13, A(C.dim, 200));
  txt('w2', 'local-first · MIT', 1920 - 64 - textW('local-first · MIT', 13), 1010, 13, A(C.dim, 180));
  ['w1', 'w2'].forEach((id) => anim(id, 'opacity', [[5.2, 0], [5.6, 1]], 'ease_out'));

  // the mark keeps breathing so the last frame is never dead
  anim('markRing', 'scale', [[5.4, 1], [7, 1.04]], 'ease_in_out');
  anim('glowA', 'scale', [[0, 1], [7, 1.1]], 'ease_in_out');

  return finish({
    name: opts.name ?? 'Close',
    duration: d,
    camera: [
      { t: 0, zoom: 1.04 },
      { t: 1.6, zoom: 1.0 },
      { t: 3.6, zoom: 1.02 },
      { t: 7, zoom: 1.06 },
    ],
  });
}

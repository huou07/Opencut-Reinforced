// The composer set behind the 59 s OR showcase — this is the visual quality bar
// for every video built from this template.
//
// Every composer draws a moving picture: a camera move at all times, at least two
// ideas transforming into each other, and content that lives above SAFE_BOTTOM so
// the caption track never covers it. Nothing here is a title card: words appear
// only as labels, code, requests or the brand close.
//
// Each composer is tuned for a 5–13 s beat. For a longer beat, split it rather
// than stretching a composer past its timing.

import { begin, finish, rect, ell, centre, A, C, anim, particles } from './kit.mjs';

import * as deconstruct from './scenes/deconstruct.mjs';
import * as editor from './scenes/editor.mjs';
import * as assistant from './scenes/assistant.mjs';
import * as motionScene from './scenes/motion-scene.mjs';
import * as voiceWhisper from './scenes/voice-whisper.mjs';
import * as assembly from './scenes/assembly.mjs';
import * as close from './scenes/close.mjs';

// A flexible 4–7 s connector: rings, drifting particles and one short keyword.
// Use it between two dense beats so the film breathes without going static.
export function interlude(duration, opts = {}) {
  const d = duration;
  const word = opts.beat?.word ?? opts.word ?? 'OPEN CUT';
  begin();
  rect('bg', -900, -700, 3700, 2480, C.bg);
  ell('glowA', 520, 180, 1200, 900, A(C.blue, 14));
  ell('glowB', 1000, 300, 1100, 900, A(C.violet, 10));
  for (let i = 0; i < 5; i += 1) {
    const r = 180 + i * 120;
    ell(`ring${i}`, 960 - r, 470 - r, r * 2, r * 2, A(C.edge2, 0), { stroke: A(C.edge2, 40 - i * 6), stroke_width: 1 });
    anim(`ring${i}`, 'rotation', [[0, 0], [d, i % 2 ? 18 : -22]], 'linear');
  }
  particles('dust', 960, 470, 44, 520, C.mute, d, { size: 2, speed: 44, squash: 0.62 });
  centre('word', word, 960, 470, 64, A(C.ink, 240));
  anim('word', 'opacity', [[0, 0], [0.5, 0], [1.1, 1], [d - 0.5, 1], [d - 0.1, 0]], 'ease_in_out');
  anim('word', 'translation_y', [[0, 18], [1.1, 0], [d, -10]], 'ease_out');
  rect('rule', 960 - 120, 520, 240, 2, A(C.amber, 200));
  anim('rule', 'width', [[0.7, 0], [1.4, 240]], 'ease_out');
  anim('rule', 'translation_x', [[0.7, -120], [1.4, 0]], 'ease_out');
  return finish({
    name: opts.name ?? `Interlude · ${word}`,
    duration: d,
    camera: [{ t: 0, zoom: 1.06 }, { t: d * 0.5, zoom: 1.0 }, { t: d, zoom: 1.1 }],
  });
}

export const COMPOSERS = {
  deconstruct: deconstruct.build,
  editor: editor.build,
  assistant: assistant.build,
  motionScene: motionScene.build,
  voiceWhisper: voiceWhisper.build,
  assembly: assembly.build,
  close: close.build,
  interlude,
};

export const COMPOSER_NAMES = Object.keys(COMPOSERS);

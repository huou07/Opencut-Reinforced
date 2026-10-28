#!/usr/bin/env node
// Generates a royalty-free background music bed offline.
//
// The result is a deterministic, procedurally synthesised WAV: no model, no
// network, no sample library and no licence obligation beyond this project's
// own MIT licence. It is meant to sit under narration, so the default level is
// deliberately quiet; the clip gain in the inspector is the final control.

import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const SAMPLE_RATE = 44100;
const MAX_SECONDS = 900;
const MIN_SECONDS = 4;

const MOODS = {
  calm: {
    description: 'slow minor-seventh pads with a sparse arpeggio',
    chords: [
      [57, 60, 64, 67],
      [53, 57, 60, 64],
      [48, 52, 55, 59],
      [55, 59, 62, 64],
    ],
    arp: [0, 1, 2, 3, 2, 1, 3, 2],
    drums: 0.2,
    padGain: 0.055,
    arpGain: 0.05,
    brightness: 0.5,
  },
  warm: {
    description: 'major-seventh pads with a soft pulse',
    chords: [
      [48, 52, 55, 59],
      [55, 59, 62, 67],
      [57, 60, 64, 67],
      [53, 57, 60, 64],
    ],
    arp: [0, 2, 1, 3, 1, 2, 3, 2],
    drums: 0.35,
    padGain: 0.06,
    arpGain: 0.055,
    brightness: 0.6,
  },
  uplifting: {
    description: 'bright open chords with an eighth-note pulse',
    chords: [
      [48, 52, 55, 60],
      [55, 59, 62, 67],
      [57, 60, 64, 69],
      [53, 57, 60, 65],
    ],
    arp: [0, 1, 2, 3, 2, 1, 2, 3],
    drums: 0.55,
    padGain: 0.05,
    arpGain: 0.065,
    brightness: 0.8,
  },
  tense: {
    description: 'a low minor progression with an unsettled pulse',
    chords: [
      [50, 53, 57, 60],
      [46, 50, 53, 58],
      [43, 46, 50, 55],
      [45, 49, 52, 57],
    ],
    arp: [0, 2, 3, 1, 0, 3, 2, 1],
    drums: 0.4,
    padGain: 0.06,
    arpGain: 0.045,
    brightness: 0.35,
  },
  minimal: {
    description: 'two alternating chords and almost no percussion',
    chords: [
      [57, 60, 64, 69],
      [57, 60, 64, 69],
      [53, 57, 60, 65],
      [53, 57, 60, 65],
    ],
    arp: [0, 2, 1, 3, 2, 0, 1, 3],
    drums: 0.05,
    padGain: 0.07,
    arpGain: 0.035,
    brightness: 0.4,
  },
};

function usage() {
  return `Usage:
  node tools/music.mjs --out work/music/bed.wav --prompt "<request>" [--json]
  node tools/music.mjs --out work/music/bed.wav [--duration 60] [--mood calm] [--json]
  node tools/music.mjs --help

Options:
  --out PATH       Destination WAV file (required).
  --prompt TEXT    A free-form request, Vietnamese or English, read exactly like
                   the app's Generate background music dialog: character, tempo,
                   length and whether drums belong. Explicit flags below win.
  --variation N    Vary a prompt's result without changing its meaning (0+).
  --duration N     Seconds of music, ${MIN_SECONDS}–${MAX_SECONDS}; defaults to 60.
  --mood NAME      calm | warm | uplifting | tense | minimal; defaults to calm.
  --tempo BPM      40–180; defaults to 72.
  --seed N         Deterministic variation seed; defaults to the prompt's seed.
  --level-db N     Peak level, -30–0 dBFS; defaults to -9.
  --json           Print machine-readable JSON.
  --help           Print this text.

The bed is generated locally with simple synthesis and is royalty-free.`;
}

function fail(message) {
  console.error(`music: ${message}`);
  process.exit(2);
}

function parseArgs(argv) {
  const args = {
    out: null,
    prompt: null,
    duration: null,
    mood: null,
    tempo: null,
    seed: null,
    variation: 0,
    levelDb: -9,
    json: false,
  };
  const value = (name, next) => {
    if (next === undefined || next.startsWith('--')) fail(`${name} requires a value`);
    return next;
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    switch (argument) {
      case '--out':
        args.out = value(argument, argv[++index]);
        break;
      case '--prompt':
        args.prompt = value(argument, argv[++index]);
        break;
      case '--variation':
        args.variation = Number.parseInt(value(argument, argv[++index]), 10);
        break;
      case '--duration':
        args.duration = Number(value(argument, argv[++index]));
        break;
      case '--mood':
        args.mood = value(argument, argv[++index]);
        break;
      case '--tempo':
        args.tempo = Number(value(argument, argv[++index]));
        break;
      case '--seed':
        args.seed = Number.parseInt(value(argument, argv[++index]), 10);
        break;
      case '--level-db':
        args.levelDb = Number(value(argument, argv[++index]));
        break;
      case '--json':
        args.json = true;
        break;
      case '--help':
      case '-h':
        console.log(usage());
        process.exit(0);
        break;
      default:
        fail(`unknown argument ${argument}`);
    }
  }
  if (!args.out) fail('--out is required');
  if (args.mood !== null && !MOODS[args.mood]) fail(`--mood must be one of ${Object.keys(MOODS).join(', ')}`);
  if (
    args.duration !== null &&
    (!Number.isFinite(args.duration) || args.duration < MIN_SECONDS || args.duration > MAX_SECONDS)
  ) {
    fail(`--duration must be between ${MIN_SECONDS} and ${MAX_SECONDS} seconds`);
  }
  if (args.tempo !== null && (!Number.isFinite(args.tempo) || args.tempo < 40 || args.tempo > 180)) {
    fail('--tempo must be between 40 and 180');
  }
  if (args.seed !== null && !Number.isFinite(args.seed)) fail('--seed must be an integer');
  if (!Number.isFinite(args.variation) || args.variation < 0) fail('--variation must be a non-negative integer');
  if (!Number.isFinite(args.levelDb) || args.levelDb < -30 || args.levelDb > 0) {
    fail('--level-db must be between -30 and 0');
  }
  return args;
}

// Mirrors the app's dialog interpreter (`apps/or_app/lib/production/music_prompt.dart`)
// so an agent gets the same reading of a sentence the user typed into the app.
const FOLD_GROUPS = {
  a: 'àáạảãâầấậẩẫăằắặẳẵ',
  e: 'èéẹẻẽêềếệểễ',
  i: 'ìíịỉĩ',
  o: 'òóọỏõôồốộổỗơờớợởỡ',
  u: 'ùúụủũưừứựửữ',
  y: 'ỳýỵỷỹ',
  d: 'đ',
};

function fold(text) {
  let result = text.toLowerCase();
  for (const [plain, marks] of Object.entries(FOLD_GROUPS)) {
    for (const mark of marks) result = result.split(mark).join(plain);
  }
  return result;
}

const CALM_WORDS = [
  'calm', 'chill', 'relax', 'soft', 'gentle', 'quiet', 'ambient', 'lofi', 'lo-fi',
  'thu gian', 'nhe nhang', 'em diu', 'diu ', 'buon', 'sad', 'mua ', 'rain', 'chieu ',
  'evening', 'ngu ', 'sleep', 'thien', 'meditation', 'slow', 'cham',
];
const WARM_WORDS = ['warm', 'retro', 'vintage', 'soul', 'jazz', 'am ', 'hoai niem', 'xua', 'moc'];
const UPLIFTING_WORDS = [
  'uplifting', 'happy', 'joyful', 'bright', 'upbeat', 'energetic', 'vlog', 'travel', 'vui',
  'tuoi', 'soi dong', 'nang luong', 'nhiet huyet', 'du lich', 'hanh trinh', 'fast', 'nhanh', 'gap',
];
const TENSE_WORDS = [
  'tension', 'tense', 'thriller', 'dramatic', 'suspense', 'dark', 'cinematic', 'horror', 'cang',
  'kich tinh', 'hoi hop', 'toi ', 'den ', 'trailer', 'hanh dong', 'action',
];
const MINIMAL_WORDS = [
  'minimal', 'sparse', 'simple', 'toi gian', 'don gian', 'tinh te', 'thanh lich', 'elegant',
];
const SLOW_WORDS = [
  'slow', 'cham', 'nhe nhang', 'em diu', 'ballad', 'lang man', 'buon', 'sad', 'thu gian', 'relax',
];
const FAST_WORDS = ['fast', 'nhanh', 'soi dong', 'upbeat', 'energetic', 'gap', 'manh'];

function hasAny(text, words) {
  return words.some((word) => text.includes(word));
}

function promptSeed(folded, variation) {
  let hash = 0x811c9dc5;
  for (let index = 0; index < folded.length; index += 1) {
    hash ^= folded.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) & 0x7fffffff;
  }
  return (hash + variation * 7919) & 0x7fffffff;
}

/** Reads a music request the same way the app's prompt field does. */
function interpretPrompt(prompt, defaults, variation = 0) {
  const folded = ` ${fold(prompt).replace(/\s+/g, ' ').trim()} `;
  let mood = 'calm';
  if (hasAny(folded, TENSE_WORDS)) mood = 'tense';
  else if (hasAny(folded, UPLIFTING_WORDS)) mood = 'uplifting';
  else if (hasAny(folded, MINIMAL_WORDS)) mood = 'minimal';
  else if (hasAny(folded, WARM_WORDS)) mood = 'warm';
  else if (hasAny(folded, CALM_WORDS)) mood = 'calm';
  else mood = defaults.mood;

  let tempo = defaults.tempo;
  const explicitTempo = /(\d{2,3})\s*(bpm|nhip)/.exec(folded);
  if (explicitTempo) tempo = Number(explicitTempo[1]);
  else if (hasAny(folded, SLOW_WORDS)) tempo = 62;
  else if (hasAny(folded, FAST_WORDS)) tempo = 138;
  tempo = Math.min(180, Math.max(40, tempo));

  let seconds = defaults.seconds;
  const minutes = /(\d+)\s*(phut|min)/.exec(folded);
  const units = /(\d+)\s*(giay|sec|s )/.exec(folded);
  let total = 0;
  if (minutes) total += Number(minutes[1]) * 60;
  if (units) total += Number(units[1]);
  if (total > 0) seconds = Math.min(600, Math.max(4, total));

  let drums = 1;
  let pad = 1;
  let arp = 1;
  let brightness = 1;
  let octave = 0;
  let notes = [];

  const noDrums = [
    'khong trong', 'khong can trong', 'chi piano', 'chi dan ', 'piano only',
    'no drums', 'drumless', 'without drums',
  ].some((word) => folded.includes(word));
  const moreDrums = ['trong manh', 'co trong', 'punchy drums', 'with drums', 'strong drums', 'beat manh'].some(
    (word) => folded.includes(word),
  );
  if (noDrums) drums = 0;
  else if (moreDrums) drums = 1.2;

  if (['piano', 'dan ', 'acoustic', 'moc'].some((word) => folded.includes(word))) {
    arp = 1.25;
    pad = 0.85;
    if (noDrums) drums = 0;
  }
  if (['chuong', 'bell', 'bright', 'tuoi sang'].some((word) => folded.includes(word))) brightness = 1.2;
  if (['toi ', 'dark', 'u am', 'tram '].some((word) => folded.includes(word))) brightness = 0.75;
  if (['quang cao', 'high pitch', 'octave up'].some((word) => folded.includes(word))) octave = 12;
  if (['quang tram', 'octave down', 'bass sau'].some((word) => folded.includes(word))) {
    octave = -12;
    pad = 1.1;
  }

  if (prompt.trim() !== '' && notes.length === 0) {
    const understood = [];
    if (
      hasAny(folded, CALM_WORDS) || hasAny(folded, WARM_WORDS) || hasAny(folded, UPLIFTING_WORDS) ||
      hasAny(folded, TENSE_WORDS) || hasAny(folded, MINIMAL_WORDS)
    ) understood.push('mood');
    if (/\d+\s*(bpm|nhip)/.test(folded) || hasAny(folded, SLOW_WORDS) || hasAny(folded, FAST_WORDS)) {
      understood.push('tempo');
    }
    if (/\d+\s*(giay|phut|s |sec|min)/.test(folded)) understood.push('length');
    if (noDrums || moreDrums) understood.push('drums');
    if (understood.length === 0) {
      notes.push('No music words were recognised, so the controls below were kept.');
    }
  }

  const moodLabel = MOODS[mood] ? mood : defaults.mood;
  const drumsLabel = drums === 0 ? 'no drums' : drums > 1 ? 'strong drums' : 'light drums';
  return {
    moodId: moodLabel,
    tempo,
    seconds,
    drumsScale: drums,
    padScale: pad,
    arpScale: arp,
    brightnessScale: brightness,
    octaveShift: octave,
    seed: promptSeed(folded, variation),
    summary: `${moodLabel} · ${tempo} bpm · ${seconds} s · ${drumsLabel}`,
    notes,
  };
}

function mulberry32(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

function midiFrequency(note) {
  return 440 * 2 ** ((note - 69) / 12);
}

function addSine(left, right, start, length, frequency, amplitude, pan, envelope) {
  const rate = SAMPLE_RATE;
  for (let index = 0; index < length; index += 1) {
    const position = start + index;
    if (position >= left.length) break;
    const gain = envelope(index / rate, length / rate) * amplitude;
    if (gain <= 0) continue;
    const sample = Math.sin(2 * Math.PI * frequency * (index / rate)) * gain;
    left[position] += sample * (1 - pan);
    right[position] += sample * pan;
  }
}

function addHarmonicVoice(left, right, start, length, frequency, amplitude, pan) {
  const rate = SAMPLE_RATE;
  for (let index = 0; index < length; index += 1) {
    const position = start + index;
    if (position >= left.length) break;
    const time = index / rate;
    const decay = Math.exp(-time * 5.5);
    const shape =
      Math.sin(2 * Math.PI * frequency * time) * 0.7 +
      Math.sin(2 * Math.PI * frequency * 2 * time) * 0.22 +
      Math.sin(2 * Math.PI * frequency * 3 * time) * 0.08;
    const gain = shape * amplitude * decay;
    left[position] += gain * (1 - pan);
    right[position] += gain * pan;
  }
}

function padEnvelope(time, duration) {
  const attack = Math.min(0.9, duration * 0.35);
  const release = Math.min(1.1, duration * 0.35);
  if (time < attack) return time / attack;
  if (time > duration - release) return Math.max(0, (duration - time) / release);
  return 1;
}

function bassEnvelope(time, duration) {
  const attack = Math.min(0.08, duration * 0.1);
  const release = Math.min(0.35, duration * 0.25);
  if (time < attack) return time / attack;
  if (time > duration - release) return Math.max(0, (duration - time) / release);
  return 0.8;
}

function kick(left, right, start, amplitude) {
  const length = Math.floor(SAMPLE_RATE * 0.22);
  for (let index = 0; index < length; index += 1) {
    const position = start + index;
    if (position >= left.length) break;
    const time = index / SAMPLE_RATE;
    const frequency = 120 * Math.exp(-time * 38) + 42;
    const envelope = Math.exp(-time * 16);
    const sample = Math.sin(2 * Math.PI * frequency * time) * envelope * amplitude;
    left[position] += sample;
    right[position] += sample;
  }
}

function hat(left, right, start, amplitude, random) {
  const length = Math.floor(SAMPLE_RATE * 0.045);
  let previous = 0;
  for (let index = 0; index < length; index += 1) {
    const position = start + index;
    if (position >= left.length) break;
    const time = index / SAMPLE_RATE;
    const noise = random() * 2 - 1;
    const filtered = noise - previous * 0.85;
    previous = noise;
    const sample = filtered * Math.exp(-time * 70) * amplitude;
    left[position] += sample;
    right[position] += sample;
  }
}

function render({ duration, moodName, tempo, seed, levelDb, mix = {} }) {
  const scales = { drums: 1, pad: 1, arp: 1, brightness: 1, octave: 0, ...mix };
  const mood = MOODS[moodName];
  const frames = Math.ceil(duration * SAMPLE_RATE);
  const left = new Float32Array(frames);
  const right = new Float32Array(frames);
  const random = mulberry32(seed);
  const barSeconds = (60 / tempo) * 4;
  const barFrames = Math.floor(barSeconds * SAMPLE_RATE);
  const bars = Math.ceil(frames / barFrames);
  const delayFrames = Math.floor(SAMPLE_RATE * 0.375);

  for (let bar = 0; bar < bars; bar += 1) {
    const barStart = bar * barFrames;
    const chord = mood.chords[bar % mood.chords.length];
    const durationSeconds = barSeconds;

    // Pad: chord tones and their octave, gently detuned.
    for (const note of chord) {
      for (const octave of [0, 12]) {
        const frequency = midiFrequency(note + octave + scales.octave);
        const pan = 0.5 + (random() - 0.5) * 0.2;
        addSine(
          left,
          right,
          barStart,
          barFrames,
          frequency * (1 + (random() - 0.5) * 0.0012),
          (mood.padGain * scales.pad) / chord.length,
          pan,
          (time) => padEnvelope(time, durationSeconds),
        );
      }
    }

    // Bass: the chord root two octaves down, with a soft attack.
    addSine(
      left,
      right,
      barStart,
      barFrames,
      midiFrequency(chord[0] - 12 + scales.octave),
      0.11,
      0.5,
      (time) => bassEnvelope(time, durationSeconds),
    );

    // Arpeggio: eighth notes, alternating pan, with one delayed echo.
    const stepFrames = Math.floor(barFrames / mood.arp.length);
    for (let step = 0; step < mood.arp.length; step += 1) {
      const note = chord[mood.arp[step]] + 12 + scales.octave;
      const start = barStart + step * stepFrames;
      const pan = 0.35 + ((step % 2) * 0.3 + (random() - 0.5) * 0.1);
      const amplitude = mood.arpGain * scales.arp * (0.8 + random() * 0.4);
      const noteFrames = Math.min(Math.floor(SAMPLE_RATE * 0.5), barFrames);
      addHarmonicVoice(left, right, start, noteFrames, midiFrequency(note), amplitude, pan);
      addHarmonicVoice(
        left,
        right,
        start + delayFrames,
        noteFrames,
        midiFrequency(note),
        amplitude * 0.28,
        1 - pan,
      );
    }

    // Shimmer on the top chord tone, very quiet.
    addSine(
      left,
      right,
      barStart,
      barFrames,
      midiFrequency(chord[chord.length - 1] + 24),
      0.012 * mood.brightness * scales.brightness,
      0.5,
      (time) => padEnvelope(time, durationSeconds) * (0.6 + 0.4 * Math.sin(2 * Math.PI * 0.15 * time)),
    );

    // Percussion: soft kick on the first beat and a hat on the off-beats.
    if (mood.drums > 0 && scales.drums > 0) {
      kick(left, right, barStart, 0.2 * mood.drums * scales.drums);
      kick(left, right, barStart + Math.floor(barFrames / 2), 0.12 * mood.drums * scales.drums);
      for (const fraction of [0.25, 0.75]) {
        hat(left, right, barStart + Math.floor(barFrames * fraction), 0.05 * mood.drums * scales.drums, random);
      }
    }
  }

  // Peak normalise, then fade in and out so the bed never clicks.
  let peak = 0;
  for (let index = 0; index < frames; index += 1) {
    peak = Math.max(peak, Math.abs(left[index]), Math.abs(right[index]));
  }
  const target = 10 ** (levelDb / 20);
  const scale = peak > 0 ? target / peak : 1;
  const fadeFrames = Math.min(Math.floor(SAMPLE_RATE * 1.2), Math.floor(frames / 4));
  for (let index = 0; index < frames; index += 1) {
    let envelope = 1;
    if (index < fadeFrames) envelope = index / fadeFrames;
    const tail = frames - index;
    if (tail < fadeFrames) envelope = Math.min(envelope, tail / fadeFrames);
    left[index] = Math.max(-1, Math.min(1, left[index] * scale * envelope));
    right[index] = Math.max(-1, Math.min(1, right[index] * scale * envelope));
  }
  return { left, right, frames };
}

function wavBytes(left, right) {
  const frames = left.length;
  const dataBytes = frames * 4;
  const buffer = Buffer.alloc(44 + dataBytes);
  buffer.write('RIFF', 0, 'ascii');
  buffer.writeUInt32LE(36 + dataBytes, 4);
  buffer.write('WAVE', 8, 'ascii');
  buffer.write('fmt ', 12, 'ascii');
  buffer.writeUInt32LE(16, 16);
  buffer.writeUInt16LE(1, 20);
  buffer.writeUInt16LE(2, 22);
  buffer.writeUInt32LE(SAMPLE_RATE, 24);
  buffer.writeUInt32LE(SAMPLE_RATE * 4, 28);
  buffer.writeUInt16LE(4, 32);
  buffer.writeUInt16LE(16, 34);
  buffer.write('data', 36, 'ascii');
  buffer.writeUInt32LE(dataBytes, 40);
  let offset = 44;
  for (let index = 0; index < frames; index += 1) {
    buffer.writeInt16LE(Math.round(left[index] * 32767), offset);
    buffer.writeInt16LE(Math.round(right[index] * 32767), offset + 2);
    offset += 4;
  }
  return buffer;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const defaults = { mood: 'calm', tempo: 72, seconds: 60 };
  const plan = args.prompt !== null ? interpretPrompt(args.prompt, defaults, args.variation) : null;
  const mood = args.mood ?? plan?.moodId ?? 'calm';
  const tempo = args.tempo ?? plan?.tempo ?? 72;
  const duration = args.duration ?? plan?.seconds ?? 60;
  const seed = args.seed ?? plan?.seed ?? 1;
  const mix = plan
    ? {
        drums: plan.drumsScale,
        pad: plan.padScale,
        arp: plan.arpScale,
        brightness: plan.brightnessScale,
        octave: plan.octaveShift,
      }
    : {};
  const drumsLabel = (mix.drums ?? 1) === 0 ? 'no drums' : (mix.drums ?? 1) > 1 ? 'strong drums' : 'light drums';
  const summary = plan ? `${mood} · ${tempo} bpm · ${duration} s · ${drumsLabel}` : null;
  const output = path.resolve(args.out);
  const { left, right, frames } = render({
    duration,
    moodName: mood,
    tempo,
    seed,
    levelDb: args.levelDb,
    mix,
  });
  const bytes = wavBytes(left, right);
  await mkdir(path.dirname(output), { recursive: true });
  await writeFile(output, bytes);
  const result = {
    format: 'or-music-bed',
    schema_version: 1,
    out: output,
    duration_ms: Math.round((frames / SAMPLE_RATE) * 1000),
    sample_rate: SAMPLE_RATE,
    channels: 2,
    bits: 16,
    mood,
    tempo,
    seed,
    level_db: args.levelDb,
    bytes: bytes.length,
    ...(plan
      ? {
          prompt: args.prompt,
          variation: args.variation,
          summary,
          plan: {
            mood: plan.moodId,
            tempo: plan.tempo,
            seconds: plan.seconds,
            drums_scale: plan.drumsScale,
            pad_scale: plan.padScale,
            arp_scale: plan.arpScale,
            brightness_scale: plan.brightnessScale,
            octave_shift: plan.octaveShift,
            notes: plan.notes,
          },
        }
      : {}),
  };
  if (args.json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    if (plan) {
      console.log(`Read \u201c${args.prompt}\u201d as ${summary}.`);
      for (const note of plan.notes) console.log(`Note: ${note}`);
    }
    console.log(
      `Generated a ${mood} bed (${(frames / SAMPLE_RATE).toFixed(1)} s, ${tempo} bpm, peak ${args.levelDb} dBFS) at ${output}.`,
    );
    console.log('Import it with tools/or media add, place it on an audio track, then save.');
  }
}

main().catch((error) => fail(error?.message ?? String(error)));

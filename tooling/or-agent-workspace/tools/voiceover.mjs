#!/usr/bin/env node
// Turns an OR assistant script into offline narration, exact timings and SRT
// captions, then optionally imports and places the result in the open project.
//
// The audio comes from the macOS `say` voice, so nothing leaves the machine and
// no model is downloaded. Captions use the measured TTS duration, which is
// exact for the narration itself; existing recorded audio uses
// `tools/or captions transcribe` instead.

import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const TOOL_ROOT = path.dirname(fileURLToPath(import.meta.url));
const SAMPLE_RATE = 44100;
const MAX_SCENES = 300;
const MAX_NARRATION_BYTES = 2000;
const DEFAULT_VOICE = 'Linh';
const DEFAULT_RATE = 180;
const DEFAULT_GAP_MS = 250;

function usage() {
  return `Usage:
  node tools/voiceover.mjs --script work/scripts/demo.json --plan [--json]
  node tools/voiceover.mjs --script work/scripts/demo.json --out work/voiceover/demo
  node tools/voiceover.mjs --script work/scripts/demo.json --out work/voiceover/demo --apply [--attach project.json]
  node tools/voiceover.mjs --help

Options:
  --script PATH   The or-assistant-script v1 file to narrate (required).
  --out DIR       Output directory; defaults to work/voiceover/<script slug>.
  --voice NAME    macOS voice for say; defaults to the script voice or "${DEFAULT_VOICE}".
  --rate N        Words per minute for say; defaults to the script rate or ${DEFAULT_RATE}.
  --gap-ms N      Silence between narration lines; defaults to the script gap_ms or ${DEFAULT_GAP_MS}.
  --plan          Validate and print the planned timings without speaking.
  --apply         Import the generated WAVs, place them on an audio track, and
                  import the generated SRT as caption clips. Requires tools/or.
  --attach PATH   Session pointer used with --apply; defaults to project.json.
  --or PATH       The or CLI wrapper; defaults to tools/or beside this tool.
  --json          Print machine-readable JSON.
  --help          Print this text.

The macOS command 'say -v ?' lists the installed voices. The default Linh voice is vi_VN.`;
}

function fail(message) {
  console.error(`voiceover: ${message}`);
  process.exit(2);
}

function parseArgs(argv) {
  const args = {
    script: null,
    out: null,
    voice: null,
    rate: null,
    gapMs: null,
    plan: false,
    apply: false,
    attach: 'project.json',
    or: null,
    json: false,
  };
  const value = (name, next) => {
    if (next === undefined || next.startsWith('--')) fail(`${name} requires a value`);
    return next;
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    switch (argument) {
      case '--script':
        args.script = value(argument, argv[++index]);
        break;
      case '--out':
        args.out = value(argument, argv[++index]);
        break;
      case '--voice':
        args.voice = value(argument, argv[++index]);
        break;
      case '--rate':
        args.rate = Number(value(argument, argv[++index]));
        break;
      case '--gap-ms':
        args.gapMs = Number(value(argument, argv[++index]));
        break;
      case '--attach':
        args.attach = value(argument, argv[++index]);
        break;
      case '--or':
        args.or = value(argument, argv[++index]);
        break;
      case '--plan':
        args.plan = true;
        break;
      case '--apply':
        args.apply = true;
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
  if (!args.script) fail('--script is required');
  if (args.rate !== null && (!Number.isFinite(args.rate) || args.rate < 90 || args.rate > 400)) {
    fail('--rate must be a number between 90 and 400');
  }
  if (
    args.gapMs !== null &&
    (!Number.isFinite(args.gapMs) || args.gapMs < 0 || args.gapMs > 10000)
  ) {
    fail('--gap-ms must be between 0 and 10000');
  }
  return args;
}

function validateScript(document) {
  if (document === null || typeof document !== 'object' || Array.isArray(document)) {
    fail('the script must be a JSON object');
  }
  if (document.format !== 'or-assistant-script' || document.schema_version !== 1) {
    fail('the script must declare format "or-assistant-script" and schema_version 1');
  }
  if (!Array.isArray(document.scenes) || document.scenes.length === 0) {
    fail('the script must contain at least one scene');
  }
  if (document.scenes.length > MAX_SCENES) {
    fail(`the script may contain at most ${MAX_SCENES} scenes`);
  }
  const ids = new Set();
  if (Number.isFinite(document.rate) && (document.rate < 90 || document.rate > 400)) {
    fail('the script rate must be between 90 and 400');
  }
  if (Number.isFinite(document.gap_ms) && (document.gap_ms < 0 || document.gap_ms > 10000)) {
    fail('the script gap_ms must be between 0 and 10000');
  }
  const scenes = document.scenes.map((scene, index) => {
    if (scene === null || typeof scene !== 'object' || Array.isArray(scene)) {
      fail(`scene ${index + 1} must be an object`);
    }
    const id = String(scene.id ?? `scene-${index + 1}`);
    if (!/^[a-z0-9][a-z0-9-]{0,63}$/.test(id)) {
      fail(`scene ${index + 1} needs a lowercase kebab-case id`);
    }
    if (ids.has(id)) fail(`scene id ${id} is duplicated`);
    ids.add(id);
    const narration = typeof scene.narration === 'string' ? scene.narration.trim() : '';
    if (!narration) fail(`scene ${id} has no narration`);
    if (Buffer.byteLength(narration) > MAX_NARRATION_BYTES) {
      fail(`scene ${id} narration exceeds ${MAX_NARRATION_BYTES} bytes`);
    }
    const caption =
      typeof scene.caption === 'string' && scene.caption.trim()
        ? scene.caption.trim()
        : narration;
    const marker = typeof scene.marker === 'string' && scene.marker.trim() ? scene.marker.trim() : null;
    const durationMs =
      Number.isFinite(scene.duration_ms) && scene.duration_ms > 0 ? Math.round(scene.duration_ms) : null;
    return { id, narration, caption, marker, durationMs };
  });
  return {
    title: typeof document.title === 'string' && document.title.trim() ? document.title.trim() : null,
    language: typeof document.language === 'string' ? document.language.trim() : null,
    voice:
      typeof document.voice === 'string' && document.voice.trim() ? document.voice.trim() : DEFAULT_VOICE,
    rate: Number.isFinite(document.rate) ? Number(document.rate) : DEFAULT_RATE,
    gapMs: Number.isFinite(document.gap_ms) ? Number(document.gap_ms) : DEFAULT_GAP_MS,
    scenes,
  };
}

function slugify(value) {
  return (
    String(value ?? 'script')
      .toLowerCase()
      .normalize('NFD')
      .replace(/[\u0300-\u036f]/g, '')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 60) || 'script'
  );
}

function say(text, outPath, voice, rate) {
  const phrase = text.startsWith('-') ? ` ${text}` : text;
  const voiceArgs = voice === 'default' ? [] : ['-v', voice];
  const result = spawnSync(
    'say',
    [...voiceArgs, '-r', String(rate), '-o', outPath, '--data-format=LEI16@44100', phrase],
    { encoding: 'utf8' },
  );
  if (result.error) {
    fail(
      `macOS say is unavailable (${result.error.message}); this voiceover path is macOS-only`,
    );
  }
  if (result.status !== 0) {
    const detail = (result.stderr ?? '').trim() || `exit ${result.status}`;
    fail(`say failed for ${path.basename(outPath)}: ${detail}`);
  }
}

function inspectWav(buffer, file) {
  if (
    buffer.length < 44 ||
    buffer.toString('ascii', 0, 4) !== 'RIFF' ||
    buffer.toString('ascii', 8, 12) !== 'WAVE'
  ) {
    fail(`${file} is not a RIFF/WAVE file`);
  }
  let offset = 12;
  let format = null;
  let dataBytes = null;
  while (offset + 8 <= buffer.length) {
    const id = buffer.toString('ascii', offset, offset + 4);
    const size = buffer.readUInt32LE(offset + 4);
    const body = offset + 8;
    if (id === 'fmt ' && size >= 16) {
      format = {
        audioFormat: buffer.readUInt16LE(body),
        channels: buffer.readUInt16LE(body + 2),
        sampleRate: buffer.readUInt32LE(body + 4),
        bits: buffer.readUInt16LE(body + 14),
      };
    } else if (id === 'data') {
      dataBytes = Math.min(size, buffer.length - body);
      break;
    }
    offset = body + size + (size & 1);
  }
  if (!format || dataBytes === null) fail(`${file} has no PCM format or data chunk`);
  if (format.audioFormat !== 1 || format.bits !== 16) {
    fail(`${file} is not 16-bit PCM; regenerate it with say`);
  }
  const frameBytes = format.channels * 2;
  return {
    durationMs: Math.floor(((dataBytes / frameBytes) * 1000) / format.sampleRate),
    sampleRate: format.sampleRate,
    channels: format.channels,
  };
}

function srtTime(milliseconds) {
  const hours = Math.floor(milliseconds / 3_600_000);
  const minutes = Math.floor((milliseconds % 3_600_000) / 60_000);
  const seconds = Math.floor((milliseconds % 60_000) / 1000);
  const millis = milliseconds % 1000;
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(
    seconds,
  ).padStart(2, '0')},${String(millis).padStart(3, '0')}`;
}

function toSrt(lines) {
  return `${lines
    .map(
      (line, index) =>
        `${index + 1}\n${srtTime(line.start_ms)} --> ${srtTime(line.end_ms)}\n${line.caption}\n`,
    )
    .join('\n')}\n`;
}

function estimateDurationMs(narration, rate) {
  const words = narration.split(/\s+/u).filter(Boolean).length;
  return Math.max(1200, Math.round((words / rate) * 60_000) + 500);
}

function runOr(orPath, args, attach) {
  const result = spawnSync(orPath, [...args, '--attach', attach, '--json'], {
    encoding: 'utf8',
  });
  if (result.error) fail(`could not run ${orPath}: ${result.error.message}`);
  if (result.status !== 0) {
    const stdout = (result.stdout ?? '').trim();
    const stderr = (result.stderr ?? '').trim();
    let detail = stderr || stdout;
    try {
      const parsed = JSON.parse(stdout);
      detail = parsed?.error?.message ?? detail;
    } catch {
      // Not JSON; keep the raw text.
    }
    fail(`or ${args.join(' ')} failed: ${detail || `exit ${result.status}`}`);
  }
  try {
    return JSON.parse(result.stdout);
  } catch {
    fail(`or ${args.join(' ')} did not return JSON`);
  }
}

function applyProject({ orPath, attach, lines, srtPath }) {
  const tracks = runOr(orPath, ['timeline', 'tracks'], attach)?.timeline_tracks_v2 ?? [];
  let track = tracks.find(
    (candidate) =>
      candidate.kind === 'audio' &&
      candidate.state?.locked !== true &&
      candidate.clip_count === 0,
  );
  if (!track) {
    const added = runOr(orPath, ['timeline', 'add-track', '--kind', 'audio'], attach);
    track = { track_id: added.track_id, kind: 'audio' };
  }
  const mediaIds = [];
  for (const line of lines) {
    const imported = runOr(orPath, ['media', 'add', '--source', line.file], attach);
    const mediaId = imported?.media?.id;
    if (!mediaId) fail(`the app did not return a media id for ${line.file}`);
    mediaIds.push(mediaId);
    runOr(
      orPath,
      [
        'timeline',
        'insert-clip',
        '--track',
        track.track_id,
        '--media',
        mediaId,
        '--at',
        `${line.start_ms}/1000`,
        '--source-start',
        '0/1',
        '--duration',
        `${line.duration_ms}/1000`,
      ],
      attach,
    );
  }
  const imported = runOr(orPath, ['timeline', 'import-captions', '--file', srtPath], attach);
  return {
    track_id: track.track_id,
    media_ids: mediaIds,
    captions: imported?.imported ?? null,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const scriptPath = path.resolve(args.script);
  let raw;
  try {
    raw = JSON.parse(await readFile(scriptPath, 'utf8'));
  } catch (error) {
    fail(`could not read ${args.script}: ${error.message}`);
  }
  const script = validateScript(raw);
  const voice = args.voice ?? script.voice;
  const rate = args.rate ?? script.rate;
  const gapMs = args.gapMs ?? script.gapMs;
  const outDirectory = path.resolve(
    args.out ?? path.join('work', 'voiceover', slugify(script.title ?? path.basename(scriptPath, '.json'))),
  );

  const planned = [];
  let cursor = 0;
  for (const scene of script.scenes) {
    const durationMs = args.plan
      ? (scene.durationMs ?? estimateDurationMs(scene.narration, rate))
      : 0;
    const startMs = cursor;
    const endMs = startMs + durationMs;
    planned.push({ ...scene, startMs, endMs, durationMs, estimated: scene.durationMs === null });
    cursor = endMs + gapMs;
  }

  if (args.plan) {
    const plan = {
      format: 'or-voiceover-plan',
      schema_version: 1,
      script: scriptPath,
      title: script.title,
      language: script.language,
      voice,
      rate,
      gap_ms: gapMs,
      out: outDirectory,
      total_ms: planned.at(-1)?.endMs ?? 0,
      lines: planned.map((line) => ({
        scene_id: line.id,
        marker: line.marker,
        caption: line.caption,
        narration: line.narration,
        start_ms: line.startMs,
        end_ms: line.endMs,
        duration_ms: line.durationMs,
        estimated: line.estimated,
      })),
    };
    console.log(args.json ? JSON.stringify(plan, null, 2) : renderPlan(plan));
    return;
  }

  if (process.platform !== 'darwin') {
    fail('offline voiceover uses the macOS say command; this platform has no provider');
  }

  await mkdir(outDirectory, { recursive: true });
  const lines = [];
  let startMs = 0;
  for (const scene of script.scenes) {
    const wavPath = path.join(outDirectory, `${scene.id}.wav`);
    say(scene.narration, wavPath, voice, rate);
    const audio = inspectWav(await readFile(wavPath), wavPath);
    const endMs = startMs + audio.durationMs;
    lines.push({
      scene_id: scene.id,
      marker: scene.marker,
      narration: scene.narration,
      caption: scene.caption,
      file: wavPath,
      file_name: path.basename(wavPath),
      start_ms: startMs,
      end_ms: endMs,
      duration_ms: audio.durationMs,
    });
    startMs = endMs + gapMs;
  }
  const srtPath = path.join(outDirectory, 'captions.srt');
  await writeFile(srtPath, toSrt(lines), 'utf8');
  const manifestPath = path.join(outDirectory, 'manifest.json');
  const manifest = {
    format: 'or-voiceover-manifest',
    schema_version: 1,
    script: scriptPath,
    title: script.title,
    language: script.language,
    voice,
    rate,
    gap_ms: gapMs,
    sample_rate: SAMPLE_RATE,
    channels: 1,
    captions_srt: srtPath,
    total_ms: lines.at(-1)?.end_ms ?? 0,
    lines,
  };
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

  let applied = null;
  if (args.apply) {
    const orPath = path.resolve(args.or ?? path.join(TOOL_ROOT, 'or'));
    const attach = path.resolve(args.attach);
    applied = applyProject({ orPath, attach, lines, srtPath });
  }
  const result = { manifest: manifestPath, captions: srtPath, total_ms: manifest.total_ms, applied };
  if (args.json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log(renderResult(manifest, manifestPath, srtPath, applied));
  }
}

function renderPlan(plan) {
  const lines = [
    `Script: ${plan.title ?? path.basename(plan.script)} (${plan.language ?? 'unspecified language'})`,
    `Voice: ${plan.voice} at ${plan.rate} wpm · gap ${plan.gap_ms} ms · ${plan.lines.length} lines`,
    `Output would be: ${plan.out}`,
  ];
  for (const line of plan.lines) {
    lines.push(
      `  ${srtTime(line.start_ms)} – ${srtTime(line.end_ms)}  ${line.scene_id}  ${line.narration}`,
    );
  }
  lines.push('Run again without --plan to speak the lines.');
  return lines.join('\n');
}

function renderResult(manifest, manifestPath, srtPath, applied) {
  const lines = [
    `Narrated ${manifest.lines.length} lines with ${manifest.voice} (${manifest.total_ms} ms).`,
    `Manifest: ${manifestPath}`,
    `Captions: ${srtPath}`,
  ];
  for (const line of manifest.lines) {
    lines.push(
      `  ${srtTime(line.start_ms)} – ${srtTime(line.end_ms)}  ${line.scene_id}  ${line.caption}`,
    );
  }
  if (applied) {
    lines.push(
      `Placed ${applied.media_ids.length} clips on audio track ${applied.track_id} and imported ${applied.captions ?? 0} captions.`,
      'The project is unsaved: review the timeline, then run tools/or project save.',
    );
  } else {
    lines.push('Run with --apply to import, place and caption these lines in the open project.');
  }
  return lines.join('\n');
}

main().catch((error) => fail(error?.message ?? String(error)));

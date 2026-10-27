import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const run = promisify(execFile);
const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const musicTool = path.join(root, 'tools', 'music.mjs');
const voiceoverTool = path.join(root, 'tools', 'voiceover.mjs');

async function temporaryDirectory(prefix) {
  return mkdtemp(path.join(os.tmpdir(), prefix));
}

function script(overrides = {}) {
  return {
    format: 'or-assistant-script',
    schema_version: 1,
    title: 'Tool test',
    language: 'en',
    voice: 'default',
    rate: 180,
    gap_ms: 250,
    scenes: [
      { id: 'first', narration: 'Hello from OR.', caption: 'Hello from OR.' },
      { id: 'second', narration: 'This is the second line.' },
    ],
    ...overrides,
  };
}

async function runTool(tool, args) {
  const { stdout } = await run(process.execPath, [tool, ...args], { cwd: root });
  return stdout;
}

test('music writes a valid quiet stereo WAV bed', async () => {
  const directory = await temporaryDirectory('or-music-');
  try {
    const output = path.join(directory, 'bed.wav');
    const result = JSON.parse(
      await runTool(musicTool, [
        '--out',
        output,
        '--duration',
        '4',
        '--mood',
        'warm',
        '--tempo',
        '88',
        '--seed',
        '7',
        '--json',
      ]),
    );
    assert.equal(result.format, 'or-music-bed');
    assert.equal(result.channels, 2);
    assert.equal(result.sample_rate, 44100);
    assert.equal(result.duration_ms, 4000);

    const buffer = await readFile(output);
    assert.equal(buffer.toString('ascii', 0, 4), 'RIFF');
    assert.equal(buffer.toString('ascii', 8, 12), 'WAVE');
    assert.equal(buffer.readUInt16LE(22), 2);
    assert.equal(buffer.readUInt32LE(24), 44100);
    assert.equal(buffer.readUInt16LE(34), 16);
    const dataBytes = buffer.readUInt32LE(40);
    assert.equal(dataBytes, 4 * 44100 * 4);
    assert.equal(buffer.length, 44 + dataBytes);

    let nonzero = 0;
    let peak = 0;
    for (let offset = 44; offset < 44 + 44100; offset += 2) {
      const sample = Math.abs(buffer.readInt16LE(offset));
      if (sample !== 0) nonzero += 1;
      peak = Math.max(peak, sample);
    }
    assert.ok(nonzero > 1000, 'the bed is not silent');
    assert.ok(peak <= 32767, 'samples stay inside 16-bit range');
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('music refuses an unknown mood without writing a file', async () => {
  const directory = await temporaryDirectory('or-music-bad-');
  try {
    const output = path.join(directory, 'bed.wav');
    await assert.rejects(
      runTool(musicTool, ['--out', output, '--mood', 'nonsense', '--json']),
      /--mood must be one of/,
    );
    await assert.rejects(stat(output), /ENOENT/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('voiceover plan validates the script and writes nothing', async () => {
  const directory = await temporaryDirectory('or-voice-plan-');
  try {
    const scriptPath = path.join(directory, 'script.json');
    const output = path.join(directory, 'voiceover');
    await writeFile(scriptPath, JSON.stringify(script()));
    const plan = JSON.parse(
      await runTool(voiceoverTool, [
        '--script',
        scriptPath,
        '--out',
        output,
        '--plan',
        '--json',
      ]),
    );
    assert.equal(plan.format, 'or-voiceover-plan');
    assert.equal(plan.lines.length, 2);
    assert.equal(plan.lines[0].start_ms, 0);
    assert.equal(plan.lines[1].start_ms, plan.lines[0].end_ms + plan.gap_ms);
    assert.equal(plan.total_ms, plan.lines[1].end_ms);
    await assert.rejects(stat(output), /ENOENT/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('voiceover refuses a duplicate scene id', async () => {
  const directory = await temporaryDirectory('or-voice-bad-');
  try {
    const scriptPath = path.join(directory, 'script.json');
    await writeFile(
      scriptPath,
      JSON.stringify(
        script({
          scenes: [
            { id: 'same', narration: 'One.' },
            { id: 'same', narration: 'Two.' },
          ],
        }),
      ),
    );
    await assert.rejects(
      runTool(voiceoverTool, ['--script', scriptPath, '--plan', '--json']),
      /duplicated/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test(
  'voiceover speaks a script and derives exact caption timings',
  { skip: process.platform !== 'darwin' ? 'the say provider is macOS-only' : false },
  async () => {
    const directory = await temporaryDirectory('or-voice-say-');
    try {
      const scriptPath = path.join(directory, 'script.json');
      const output = path.join(directory, 'voiceover');
      await writeFile(
        scriptPath,
        JSON.stringify(script({ scenes: [{ id: 'hello', narration: 'Hello from OR.' }] })),
      );
      const result = JSON.parse(
        await runTool(voiceoverTool, [
          '--script',
          scriptPath,
          '--out',
          output,
          '--voice',
          'default',
          '--json',
        ]),
      );
      const manifest = JSON.parse(await readFile(result.manifest, 'utf8'));
      assert.equal(manifest.format, 'or-voiceover-manifest');
      assert.equal(manifest.lines.length, 1);
      assert.ok(manifest.lines[0].duration_ms > 500, 'speech has a measurable duration');
      assert.equal(manifest.lines[0].end_ms, manifest.lines[0].duration_ms);
      const captions = await readFile(result.captions, 'utf8');
      assert.match(captions, /Hello from OR\./);
      assert.match(captions, /00:00:00,000 --> 00:/);
      const wav = await readFile(manifest.lines[0].file);
      assert.equal(wav.toString('ascii', 0, 4), 'RIFF');
      assert.equal(wav.toString('ascii', 8, 12), 'WAVE');
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);

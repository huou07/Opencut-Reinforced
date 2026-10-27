#!/usr/bin/env node
// Writes a minimal, valid V1 MotionScene the agent can edit.
//
// The envelope and node fields match `crates/or_motion`; keep this file boring.
// It writes JSON only — no render, no network, no project access.

import { randomUUID } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const USAGE =
  'usage: node tools/motion-scaffold.mjs [--name TITLE] [--out PATH] [--width N] [--height N] [--seconds N]';

function fail(message) {
  process.stderr.write(`${message}\n${USAGE}\n`);
  process.exit(2);
}

function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith('--')) fail(`unexpected argument: ${token}`);
    const key = token.slice(2);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith('--')) {
      fail(`missing value for --${key}`);
    }
    values.set(key, value);
    index += 1;
  }
  return values;
}

function positiveInteger(values, key, fallback) {
  if (!values.has(key)) return fallback;
  const value = Number(values.get(key));
  if (!Number.isInteger(value) || value <= 0) {
    fail(`--${key} must be a positive integer`);
  }
  return value;
}

const values = parseArgs(process.argv.slice(2));
const name = (values.get('name') ?? 'Untitled scene').trim();
if (name.length === 0) fail('--name must not be empty');

const out = values.get('out') ?? 'scenes/scene.ormotion.json';
const width = positiveInteger(values, 'width', 1920);
const height = positiveInteger(values, 'height', 1080);
const seconds = positiveInteger(values, 'seconds', 4);

// One second is written as 1/1 seconds so the duration stays exact.
const duration = { numerator: seconds, denominator: 1 };
const fadeEnd = { numerator: 1, denominator: 2 };

const scene = {
  format: 'opencut-reinforced-motion-scene',
  schema_version: 1,
  scene_id: randomUUID(),
  name,
  duration,
  canvas: { width, height },
  assets: [],
  nodes: [
    {
      node_id: 'background',
      kind: 'rectangle',
      position: { x: 0, y: 0 },
      size: { width, height },
      fill: { red: 24, green: 26, blue: 31, alpha: 255 },
    },
    {
      node_id: 'title',
      kind: 'text',
      text: name,
      position: { x: Math.round(width * 0.06), y: Math.round(height * 0.16) },
      font_size: Math.round(height * 0.09),
      fill: { red: 236, green: 238, blue: 242, alpha: 255 },
    },
  ],
  animations: [
    {
      node_id: 'title',
      property: 'opacity',
      keyframes: [
        { at: { numerator: 0, denominator: 1 }, value: 0 },
        { at: fadeEnd, value: 1 },
      ],
      easing: 'ease_out',
    },
  ],
};

const target = path.resolve(out);
await mkdir(path.dirname(target), { recursive: true });
await writeFile(target, `${JSON.stringify(scene, null, 2)}\n`, 'utf8');
process.stdout.write(`${target}\n`);

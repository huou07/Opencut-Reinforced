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

// A flat, continuously-moving starter: three colours, a shape that travels for
// the whole scene, and type that rises rather than appearing all at once. The
// renderer auto-fits a long title to the canvas, but keep titles short anyway.
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
      fill: { red: 17, green: 20, blue: 26, alpha: 255 },
    },
    {
      node_id: 'accent-bar',
      kind: 'rectangle',
      position: { x: Math.round(width * 0.06), y: Math.round(height * 0.72) },
      size: { width: Math.round(width * 0.18), height: Math.max(6, Math.round(height * 0.008)) },
      fill: { red: 242, green: 193, blue: 78, alpha: 255 },
    },
    {
      node_id: 'orb',
      kind: 'ellipse',
      position: { x: Math.round(width * 0.72), y: Math.round(height * 0.18) },
      size: { width: Math.round(height * 0.34), height: Math.round(height * 0.34) },
      fill: { red: 63, green: 199, blue: 255, alpha: 46 },
    },
    {
      node_id: 'title',
      kind: 'text',
      text: name,
      position: { x: Math.round(width * 0.06), y: Math.round(height * 0.36) },
      font_size: Math.round(height * 0.1),
      fill: { red: 245, green: 245, blue: 245, alpha: 255 },
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
    {
      node_id: 'title',
      property: 'translation_y',
      keyframes: [
        { at: { numerator: 0, denominator: 1 }, value: Math.round(height * 0.03) },
        { at: fadeEnd, value: 0 },
      ],
      easing: 'ease_out',
    },
    {
      node_id: 'accent-bar',
      property: 'scale',
      keyframes: [
        { at: { numerator: 0, denominator: 1 }, value: 0.05 },
        { at: { numerator: 3, denominator: 4 }, value: 1 },
      ],
      easing: 'ease_out',
    },
    {
      // The orb keeps moving for the whole scene, so no frame is a dead slide.
      node_id: 'orb',
      property: 'translation_x',
      keyframes: [
        { at: { numerator: 0, denominator: 1 }, value: 0 },
        { at: duration, value: Math.round(width * -0.08) },
      ],
      easing: 'linear',
    },
  ],
  // A slow push-in keeps the whole scene alive without a cut. Zoom stays at or
  // above 1 so a flat background never reveals the canvas edge.
  camera: {
    keyframes: [
      {
        at: { numerator: 0, denominator: 1 },
        pan_x: 0,
        pan_y: 0,
        zoom: 1,
        rotation_degrees: -0.4,
      },
      {
        at: duration,
        pan_x: Math.round(width * -0.012),
        pan_y: Math.round(height * -0.008),
        zoom: 1.08,
        rotation_degrees: 0.4,
      },
    ],
    easing: 'ease_in_out',
  },
};

const target = path.resolve(out);
await mkdir(path.dirname(target), { recursive: true });
await writeFile(target, `${JSON.stringify(scene, null, 2)}\n`, 'utf8');
process.stdout.write(`${target}\n`);

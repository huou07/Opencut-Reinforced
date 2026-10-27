#!/usr/bin/env node
// Packs the workspace source tree into the Flutter asset the app installs.
//
// The source tree is the truth; the checked-in bundle is a build product. Run
// `node build.mjs` after editing and commit both. `npm test` fails when they
// drift apart.

import { readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const BUNDLE_VERSION = 3;

const root = path.dirname(fileURLToPath(import.meta.url));
const output = path.resolve(
  root,
  '..',
  '..',
  'apps',
  'or_app',
  'assets',
  'agent_workspace',
  'bundle.json',
);

const EXCLUDED_AT_ROOT = new Set([
  'build.mjs',
  'package.json',
  'README.md',
  'node_modules',
  'test',
]);

async function collect(directory, prefix = '') {
  const files = {};
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of entries) {
    if (prefix.length === 0 && EXCLUDED_AT_ROOT.has(entry.name)) continue;
    if (entry.name.startsWith('.') && entry.name !== '.agents') {
      // Hidden entries are container metadata, not workspace content. The
      // exception is `.agents`, which carries the skills.
      continue;
    }
    const relative = prefix.length === 0 ? entry.name : `${prefix}/${entry.name}`;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      Object.assign(files, await collect(absolute, relative));
      continue;
    }
    if (!entry.isFile()) continue;
    const content = await readFile(absolute, 'utf8');
    if (content.includes('\u0000')) continue;
    files[relative] = content;
  }
  return files;
}

export async function buildBundle() {
  const files = await collect(root);
  for (const key of Object.keys(files)) {
    if (key.startsWith('/') || key.split('/').includes('..')) {
      throw new Error(`unsafe bundle path: ${key}`);
    }
  }
  return { version: BUNDLE_VERSION, files };
}

export async function writeBundle() {
  const bundle = await buildBundle();
  await writeFile(output, `${JSON.stringify(bundle, null, 2)}\n`, 'utf8');
  return { bundle, output };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const { bundle, output: written } = await writeBundle();
  process.stdout.write(
    `wrote ${Object.keys(bundle.files).length} files to ${written}\n`,
  );
}

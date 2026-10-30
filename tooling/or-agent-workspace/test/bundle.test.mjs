import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { buildBundle } from '../build.mjs';

const run = promisify(execFile);
const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const bundlePath = path.resolve(
  root,
  '..',
  '..',
  'apps',
  'or_app',
  'assets',
  'agent_workspace',
  'bundle.json',
);

test('the checked-in bundle matches the workspace source', async () => {
  const built = await buildBundle();
  const checkedIn = JSON.parse(await readFile(bundlePath, 'utf8'));
  assert.deepEqual(checkedIn, built);
});

test('the bundle carries the index, guides, skills and tools', async () => {
  const bundle = await buildBundle();
  for (const required of [
    'AGENTS.md',
    'docs/timeline-editing.md',
    'docs/media-and-import.md',
    'docs/motion-scenes.md',
    'docs/project-safety.md',
    'docs/script-and-voice.md',
    'docs/audio-prep.md',
    'docs/troubleshooting.md',
    'tools/README.md',
    'tools/motion-scaffold.mjs',
    'tools/voiceover.mjs',
    'tools/music.mjs',
    '.agents/skills/or-timeline-edit/SKILL.md',
    '.agents/skills/or-motion-scene/SKILL.md',
    '.agents/skills/or-project-safety/SKILL.md',
    '.agents/skills/or-script-production/SKILL.md',
  ]) {
    assert.ok(bundle.files[required], `${required} is missing`);
  }
});

test('AGENTS.md routes every guide and skill', async () => {
  const bundle = await buildBundle();
  const index = bundle.files['AGENTS.md'];
  for (const file of Object.keys(bundle.files)) {
    if (file.startsWith('docs/')) {
      assert.ok(index.includes(file), `${file} is not routed from AGENTS.md`);
    }
    if (file.startsWith('.agents/skills/') && file.endsWith('/SKILL.md')) {
      const name = file.split('/')[2];
      assert.ok(index.includes(`\`${name}\``), `${name} is not routed`);
    }
  }
});

test('every skill declares a kebab-case name equal to its directory', async () => {
  const bundle = await buildBundle();
  const skills = Object.entries(bundle.files).filter(([file]) =>
    file.endsWith('/SKILL.md'),
  );
  assert.ok(skills.length >= 3);
  for (const [file, content] of skills) {
    const directory = file.split('/')[2];
    const match = /^---\n([\s\S]*?)\n---\n/.exec(content);
    assert.ok(match, `${file} has no frontmatter`);
    const name = /^name:\s*(.+)$/m.exec(match[1]);
    assert.ok(name, `${file} has no name`);
    assert.equal(name[1].trim(), directory);
    assert.ok(/^description:\s*\S/m.test(match[1]), `${file} has no description`);
  }
});

test('the scaffold writes a scene the V1 envelope accepts', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'or-scaffold-'));
  try {
    const out = path.join(directory, 'scenes', 'title.ormotion.json');
    await run(process.execPath, [
      path.join(root, 'tools', 'motion-scaffold.mjs'),
      '--name',
      'Demo',
      '--out',
      out,
      '--seconds',
      '3',
    ]);
    const scene = JSON.parse(await readFile(out, 'utf8'));
    assert.equal(scene.format, 'opencut-reinforced-motion-scene');
    assert.equal(scene.schema_version, 1);
    assert.match(
      scene.scene_id,
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
    assert.deepEqual(scene.duration, { numerator: 3, denominator: 1 });
    const ids = new Set(scene.nodes.map((node) => node.node_id));
    for (const animation of scene.animations) {
      assert.ok(ids.has(animation.node_id));
      assert.ok(animation.keyframes.length > 0);
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('no hidden container metadata leaks into the bundle', async () => {
  const bundle = await buildBundle();
  const directory = await readdir(root, { withFileTypes: true });
  const hidden = directory
    .filter((entry) => entry.name.startsWith('.'))
    .map((entry) => entry.name)
    .filter((name) => name !== '.agents');
  for (const name of hidden) {
    assert.ok(
      !Object.keys(bundle.files).some((file) => file.startsWith(`${name}/`)),
      `${name} leaked into the bundle`,
    );
  }
});

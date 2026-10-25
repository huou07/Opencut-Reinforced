/**
 * Hermetic checks for the dsh-or-skills provider. No host process required:
 * these assert the documented SkillProvider contract directly.
 */
import assert from 'node:assert/strict'
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'
import test from 'node:test'
import { createOrSkillProvider, parseSkillDocument } from '../lib/provider.js'

const SKILL_DIR = fileURLToPath(new URL('../skills/', import.meta.url))
const EXPECTED_SKILLS = [
  'or-claim-evidence',
  'or-dev-loop',
  'or-edit-feature',
  'or-media-render',
  'or-motionscene',
  'or-release',
  'or-ui-ux',
]
const SKILL_NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

const makeProvider = (skillDir = SKILL_DIR) =>
  createOrSkillProvider({ skillDir, providerName: 'dsh-or-skills', rank: 20 })

test('frontmatter parses quoted descriptions and rejects a broken file', () => {
  const doc = parseSkillDocument('---\nname: demo-skill\ndescription: "a: b — c"\n---\n\n# Body\n', '/x/SKILL.md')
  assert.equal(doc.name, 'demo-skill')
  assert.equal(doc.description, 'a: b — c')
  assert.equal(doc.body, '# Body')
  assert.throws(() => parseSkillDocument('no frontmatter', '/x/SKILL.md'), /frontmatter/)
  assert.throws(() => parseSkillDocument('---\nname: Bad_Name\ndescription: d\n---\n', '/x'), /invalid skill name/)
  assert.throws(() => parseSkillDocument('---\nname: ok-name\ndescription: ""\n---\n', '/x'), /description/)
})

test('the repository skill set is complete and contract-shaped', async () => {
  const provider = makeProvider()
  const observation = await provider.list()
  const candidates = observation.candidates
  assert.equal(observation.complete, true)
  assert.deepEqual(
    candidates.map(candidate => candidate.name).sort(),
    [...EXPECTED_SKILLS].sort(),
  )
  for (const candidate of candidates) {
    assert.match(candidate.name, SKILL_NAME)
    assert.ok(candidate.description.length > 0, `${candidate.name} description`)
    assert.equal(candidate.provider, 'dsh-or-skills')
    assert.equal(candidate.source, 'bundled')
    assert.equal(candidate.rank, 20)
    assert.equal(typeof candidate.path, 'string')
    assert.equal(candidate.resourceBase.kind, 'directory')
    assert.ok(candidate.locator.path.endsWith('/SKILL.md'))
    // The catalog assembles skill.invocation.modelInvocable unconditionally, so an
    // omitted policy crashes the turn; every candidate must carry one.
    assert.deepEqual(candidate.invocation, { modelInvocable: true, userInvocable: true })
  }
})

test('every candidate resolves to a definition with content and resource base', async () => {
  const provider = makeProvider()
  const { candidates } = await provider.list()
  for (const candidate of candidates) {
    const definition = await provider.get(candidate)
    assert.ok(definition, `${candidate.name} definition`)
    assert.equal(definition.name, candidate.name)
    assert.equal(definition.provider, 'dsh-or-skills')
    assert.ok(definition.content.length > 40, `${candidate.name} content`)
    assert.ok(definition.resourceBase.path.endsWith(candidate.name))
  }
})

test('zh language selects description_zh when present', async () => {
  const provider = makeProvider()
  const [english] = (await provider.list({ language: 'en' })).candidates
  const [chinese] = (await provider.list({ language: 'zh' })).candidates
  assert.equal(english.name, chinese.name)
  assert.equal(typeof chinese.description, 'string')
})

test('broken files are skipped, duplicates rejected, missing files tolerated', async () => {
  const base = await mkdtemp(join(tmpdir(), 'or-skills-'))
  try {
    await mkdir(join(base, 'good-one'))
    await mkdir(join(base, 'broken'))
    await mkdir(join(base, 'duplicate'))
    await mkdir(join(base, 'empty'))
    await writeFile(join(base, 'good-one', 'SKILL.md'), '---\nname: good-one\ndescription: d\n---\nbody one')
    await writeFile(join(base, 'broken', 'SKILL.md'), '---\nname: nope\ndescription: ""\n---\nbody')
    await writeFile(join(base, 'duplicate', 'SKILL.md'), '---\nname: good-one\ndescription: dup\n---\nbody')
    const provider = createOrSkillProvider({ skillDir: base, providerName: 'p', rank: 5 })
    const { candidates } = await provider.list()
    assert.deepEqual(candidates.map(candidate => candidate.name), ['good-one'])
    assert.equal(candidates[0].rank, 5)
    const definition = await provider.get({ ...candidates[0], name: 'other-name' })
    assert.equal(definition, undefined)
    assert.equal(await provider.get({ name: 'gone', locator: { path: join(base, 'gone', 'SKILL.md') } }), undefined)
  } finally {
    await rm(base, { recursive: true, force: true })
  }
})

test('apply() registers one provider through the skills seam', async () => {
  const { apply } = await import('../index.js')
  const registrations = []
  const injected = []
  const ctx = {
    logger: { info() {}, warn() {}, debug() {} },
    inject(services, callback) {
      injected.push(services)
      callback({ skills: { registerProvider(create) { registrations.push(create) } } })
    },
  }
  apply(ctx, {})
  assert.deepEqual(injected, [['skills']])
  assert.equal(registrations.length, 1)
  const provider = registrations[0]()
  assert.equal(provider.name, 'dsh-or-skills')
  const { candidates } = await provider.list()
  assert.equal(candidates.length, EXPECTED_SKILLS.length)
})

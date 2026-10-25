/**
 * provider.js — a dependency-free DSH SkillProvider over a local skills/ directory.
 *
 * Contract (from @deepseek-ai/dsh-skill):
 *   list(options)  -> Candidate[] | { candidates, complete }
 *   get(candidate) -> Definition | undefined
 * A Candidate needs name / description / source / provider / rank / locator / path.
 * A Definition additionally needs content.
 *
 * `invocation` MUST be resolved here even though validation tolerates omission:
 * the catalog assembly calls `skill.invocation.modelInvocable` unconditionally,
 * so an omitted policy crashes the turn with
 * "Cannot read properties of undefined (reading 'modelInvocable')".
 * The derivation mirrors @deepseek-ai/dsh-skill-filesystem:modelInvocable is on
 * unless `disable-model-invocation: true`; userInvocable is on unless
 * `user-invocable: false`.
 */
import { readdir, readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'

const SKILL_NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

/** Minimal YAML subset: `key: value`, `key: "value"`, and `key: |` blocks. */
function parseFrontmatter(raw) {
  const data = {}
  const lines = raw.split(/\r?\n/)
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index]
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue
    const match = /^([A-Za-z0-9_-]+)\s*:\s*(.*)$/.exec(line)
    if (!match) continue
    const [, key, rawValue] = match
    const value = rawValue.trim()
    if (value === '|' || value === '>' || value === '|-' || value === '>-') {
      const block = []
      while (index + 1 < lines.length && /^\s+\S/.test(lines[index + 1])) {
        index += 1
        block.push(lines[index].trim())
      }
      data[key] = value.startsWith('>') ? block.join(' ') : block.join('\n')
      continue
    }
    data[key] = value.replace(/^(['"])([\s\S]*)\1$/, '$2')
  }
  return data
}

/** Frontmatter boolean, matching the host's accepted spellings. */
function frontmatterBoolean(data, key) {
  if (!Object.hasOwn(data, key)) return undefined
  const value = data[key]
  if (typeof value === 'boolean') return value
  if (value === 1 || value === '1') return true
  if (value === 0 || value === '0') return false
  if (typeof value === 'string') {
    switch (value.trim().toLowerCase()) {
      case 'true':
      case 'yes':
      case 'on':
        return true
      case 'false':
      case 'no':
      case 'off':
        return false
      default:
        return undefined
    }
  }
  return undefined
}

/** Parse one SKILL.md into the fields this plugin needs. Throws on a broken file. */
export function parseSkillDocument(text, path) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---[ \t]*\r?\n?([\s\S]*)$/.exec(text)
  if (!match) throw new Error('missing YAML frontmatter')
  const data = parseFrontmatter(match[1])
  const name = typeof data.name === 'string' ? data.name.trim() : ''
  if (!SKILL_NAME.test(name)) throw new Error(`invalid skill name "${name}" (must be kebab-case)`)
  const description = typeof data.description === 'string' ? data.description.trim() : ''
  if (description === '') throw new Error('missing description')
  const whenToUse = typeof data.whenToUse === 'string' && data.whenToUse.trim() !== '' ? data.whenToUse.trim() : undefined
  const invocation = {
    modelInvocable: frontmatterBoolean(data, 'disable-model-invocation') !== true,
    userInvocable: frontmatterBoolean(data, 'user-invocable') !== false,
  }
  return {
    name,
    description,
    descriptionZh: typeof data.description_zh === 'string' ? data.description_zh.trim() : undefined,
    whenToUse,
    invocation,
    body: (match[2] ?? '').trim(),
    path,
  }
}

/**
 * Build the provider. Discovery is re-read on every list() call: the directory
 * holds a handful of small files, and always-fresh state removes the whole class
 * of stale-cache bugs while an operator edits skill bodies.
 */
export function createOrSkillProvider({ skillDir, providerName, rank, logger }) {
  if (typeof skillDir !== 'string' || skillDir === '') throw new TypeError('skillDir is required')
  if (typeof providerName !== 'string' || providerName === '') throw new TypeError('providerName is required')
  if (typeof rank !== 'number' || !Number.isFinite(rank)) throw new TypeError('rank must be a finite number')

  const toCandidate = (doc, language) => ({
    name: doc.name,
    description: language === 'zh' && doc.descriptionZh !== undefined ? doc.descriptionZh : doc.description,
    ...(doc.whenToUse === undefined ? {} : { whenToUse: doc.whenToUse }),
    invocation: doc.invocation,
    source: 'bundled',
    provider: providerName,
    rank,
    locator: { path: doc.path, directory: dirname(doc.path) },
    resourceBase: { kind: 'directory', path: dirname(doc.path) },
    path: doc.path,
  })

  const toDefinition = (doc, language) => ({
    name: doc.name,
    description: language === 'zh' && doc.descriptionZh !== undefined ? doc.descriptionZh : doc.description,
    ...(doc.whenToUse === undefined ? {} : { whenToUse: doc.whenToUse }),
    invocation: doc.invocation,
    source: 'bundled',
    provider: providerName,
    resourceBase: { kind: 'directory', path: dirname(doc.path) },
    path: doc.path,
    content: doc.body,
  })

  const language = options => (options?.language === 'zh' ? 'zh' : 'en')

  async function scan() {
    const dirents = await readdir(skillDir, { withFileTypes: true })
    const directories = dirents
      .filter(entry => entry.isDirectory() && !entry.name.startsWith('.'))
      .map(entry => entry.name)
      .sort((left, right) => (left < right ? -1 : left > right ? 1 : 0))
    const documents = []
    const seen = new Set()
    for (const directory of directories) {
      const path = join(skillDir, directory, 'SKILL.md')
      let doc
      try {
        doc = parseSkillDocument(await readFile(path, 'utf8'), path)
      } catch (error) {
        const code = error?.code
        if (code === 'ENOENT' || code === 'ENOTDIR') continue
        logger?.warn?.(`[dsh-or-skills] skipping ${path}: ${error?.message ?? String(error)}`)
        continue
      }
      if (seen.has(doc.name)) {
        logger?.warn?.(`[dsh-or-skills] skipping ${path}: duplicate skill name "${doc.name}"`)
        continue
      }
      seen.add(doc.name)
      documents.push(doc)
    }
    return documents
  }

  return {
    name: providerName,
    async list(options = {}) {
      const documents = await scan()
      const languageTag = language(options)
      return { candidates: documents.map(doc => toCandidate(doc, languageTag)), complete: true }
    },
    async get(candidate, options = {}) {
      const path = candidate?.locator?.path
      if (typeof path !== 'string' || path === '') return undefined
      let text
      try {
        text = await readFile(path, 'utf8')
      } catch (error) {
        if (error?.code === 'ENOENT' || error?.code === 'ENOTDIR') return undefined
        throw error
      }
      const doc = parseSkillDocument(text, path)
      if (doc.name !== candidate.name) {
        logger?.warn?.(`[dsh-or-skills] ${path} declares "${doc.name}" but was requested as "${candidate.name}"`)
        return undefined
      }
      return toDefinition(doc, language(options))
    },
  }
}

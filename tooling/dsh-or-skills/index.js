/**
 * dsh-or-skills — Opencut Reinforced repo contracts as a DSH SkillProvider.
 *
 * Registers the skills under ./skills so any OR session gets the repository
 * contracts from a plugin instead of a project-local .agents/skills tree.
 * No @deepseek-ai imports: the provider only needs the documented
 * list()/get() contract, which keeps this plugin loadable across hosts.
 */
import { fileURLToPath } from 'node:url'
import { createOrSkillProvider } from './lib/provider.js'

export const name = 'or-skills'

const DEFAULT_SKILL_DIR = fileURLToPath(new URL('./skills/', import.meta.url))

/**
 * @param {object} ctx cordis plugin context
 * @param {object} [config] { providerName?, rank?, skillDir? }
 */
export function apply(ctx, config = {}) {
  const providerName =
    typeof config.providerName === 'string' && config.providerName !== '' ? config.providerName : 'dsh-or-skills'
  const rank = Number.isFinite(Number(config.rank)) ? Number(config.rank) : 20
  const skillDir = typeof config.skillDir === 'string' && config.skillDir !== '' ? config.skillDir : DEFAULT_SKILL_DIR

  ctx.inject(['skills'], skillContext => {
    ctx.logger?.info?.(`[dsh-or-skills] registering provider "${providerName}" from ${skillDir}`)
    skillContext.skills.registerProvider(() =>
      createOrSkillProvider({ skillDir, providerName, rank, logger: ctx.logger }),
    )
  })
}

# dsh-or-skills

A local DeepSeek Harness plugin that serves the Opencut Reinforced repo contracts
(`or-edit-feature`, `or-ui-ux`, `or-media-render`, `or-motionscene`,
`or-dev-loop`, `or-claim-evidence`, `or-release`) as harness skills, so no
session depends on project-local `.agents/skills` copies.

It is deliberately tiny and dependency-free: `lib/provider.js` reads
`skills/*/SKILL.md` on every listing and returns host-shaped candidates and
definitions. There is no cache, so editing a skill body takes effect on the next
listing after an application restart.

## Install into a profile

```sh
dsh plugin --profile desktop add "file:$PWD/tooling/dsh-or-skills"
```

The plugin registers one `SkillProvider` named `dsh-or-skills` at rank 20, which
wins over filesystem roots (100–500) and bundled skills (600). Bundles compose at
boot, so restart the application to see changes. Remove with:

```sh
dsh plugin --profile desktop remove dsh-or-skills
```

## Contract notes

The host tolerates an omitted `invocation` policy when validating a candidate but
dereferences `skill.invocation.modelInvocable` while assembling the catalog, so
the provider always emits one. Derivation matches the host filesystem provider:
`modelInvocable` is on unless the frontmatter sets
`disable-model-invocation: true`; `userInvocable` is on unless it sets
`user-invocable: false`.

## Edit a skill

Change `skills/<name>/SKILL.md`, keep the frontmatter `name` equal to the
directory name, then refresh the installed copy (the install hard-links files,
so an in-place edit does not propagate by itself):

```sh
dsh plugin --profile desktop remove dsh-or-skills
dsh plugin --profile desktop add "file:$PWD/tooling/dsh-or-skills"
```

## Test

```sh
cd tooling/dsh-or-skills && node --test test/provider.test.mjs
```

The suite covers frontmatter parsing, the complete repo skill set, the
candidate→definition contract, the mandatory invocation policy, language
selection, skip behaviour for broken or duplicate files, and registration through
the `skills` seam.

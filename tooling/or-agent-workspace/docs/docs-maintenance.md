# Docs maintenance

Scope: how this index and `docs/` stay navigable. Read this before adding,
moving, renaming or reviewing a guide.

## The contract

`AGENTS.md` carries exactly three rules and is an index, not a handbook. Every
entry has the same three lines:

```
- **Name** — one line on what it is for.
  When: the situation that makes this the right thing to read.
  Path: `exact/path/from/workspace/root.md`
```

- The path is relative to the workspace root, backticked, and exact. An agent
  must be able to open it without searching.
- One entry per file. Every guide under `docs/`, every skill under
  `.agents/skills/` and every template entry point must be routed, and every
  path must exist.
- Sections are `Guides`, `Skills`, `Tools`, `Templates`. Add a section only when
  a new class of artifact appears, not for one file.
- No procedure, tip, bound or example belongs in `AGENTS.md`; it belongs in the
  guide the entry points to.

## Where content goes

| Content | Home |
|---|---|
| Which file to read for a task | `AGENTS.md` |
| Operating procedure, rules, failure handling | `docs/<concern>.md` |
| A repeatable task recipe with checks | `.agents/skills/<name>/SKILL.md` |
| Runnable machinery for a production | `templates/<name>/` |
| Portable style method | `templates/styles/<name>/` |
| A helper script with no project state | `tools/<name>.mjs` |

A guide may link to another guide; it must not copy it. One concern per file.

## Adding a guide

1. Create `docs/<concern>.md` with a `# Title`, a `Scope:` paragraph and links to
   the neighbouring guides.
2. Add the entry in `Guides`, in the order a task would meet it.
3. Add a reference from the skill that owns the task, if any.
4. Rebuild and test:

```sh
cd tooling/or-agent-workspace
node build.mjs
node --test test/bundle.test.mjs test/tools.test.mjs
```

5. Commit the source and the rebuilt `apps/or_app/assets/agent_workspace/bundle.json`
   together. A bundle that drifts from source fails the test.

## Moving or renaming

Update the entry, every inbound link, every skill reference and the required-file
list in `test/bundle.test.mjs`. The bundle has no redirects; a stale path is a
broken route.

## Review checklist

- The three rules are unchanged and no fourth rule was added.
- Every `docs/*.md`, skill and template entry point is routed exactly once.
- Every entry is three lines and its description is one line.
- No guide duplicates another; links replace repetition.
- English only, no story text, no project-specific content.
- `node --test` passes and the bundle is rebuilt.

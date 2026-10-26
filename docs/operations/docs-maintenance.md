# Documentation Maintenance

Last reviewed: 2026-10-26

Scope: the shape of this documentation set and the rules for changing it. This is
the only place that defines the routing template; AGENTS.md applies it and adds
no other rule.

## Shape

| Directory | Holds |
|---|---|
| `AGENTS.md` | one-sentence project intro, the routing table, one rule |
| `docs/architecture/` | how the system is built; no feature behaviour |
| `docs/features/` | one document per user capability, with its contract and gaps |
| `docs/design/` | visual language and enforced UI invariants |
| `docs/operations/` | how to work: workflow, verification, host commands, git, permissions, docs, skills |
| `docs/reference/` | product inventory, roadmap, quality, testing, tooling, security, release |
| `docs/adr/` | recorded architecture decisions |
| `docs/archive/` | history: superseded plans, delivery logs, dated receipts |

Rules:

- One concern per file. If a document needs a "miscellaneous" section, it is two
  documents.
- Current documents describe current truth only. Dated run logs, delivery
  records and superseded plans belong in `docs/archive/`.
- No duplication: state a rule once, in its owning document, and link to it.
- Every current document starts with a title and `Last reviewed: YYYY-MM-DD`.

## Routing entry template

Every routable item (document, tool, skill) gets exactly three lines in the
AGENTS.md table:

```
- **Name** — one short sentence on what it covers.
  When: the task situation that makes this the right thing to read.
  Path: `relative/path/to/the/file`
```

- The name is the concept, not the filename.
- `When` states a task trigger, not a summary of the content.
- `Path` points at the exact file an agent should open, so no search is needed.

## When you change documentation

1. Edit or add the file in the correct directory above.
2. Add or update its routing entry in `AGENTS.md` using the template.
3. Add it to `in_scope` in `scripts/check_doc_freshness.sh` when it makes current
   claims, and set its `Last reviewed` date.
4. Add it to `required_files` in `scripts/check-repo.sh` when it is a core
   document.
5. Fix every relative link that pointed at a moved file.
6. Run `bash scripts/check-repo.sh` (it runs the link and freshness gates).

Do not add a second rule to `AGENTS.md`, and do not restate operational rules
there — put them in the owning operations document and route to it.

## Gates

| Gate | What it proves |
|---|---|
| `scripts/check_doc_links.py` | every relative link in tracked Markdown resolves |
| `scripts/check_doc_freshness.sh` | every current-claim document carries a recent review date |
| `scripts/check_claims.py` | claim records are well formed and resolve to a real commit |

Archived material is excluded from link and freshness checks: history keeps its
original paths and dates.

## Language

Framework artifacts are written in English; conversation follows the owner's
language.

# Skill routing

Purpose: select one procedure without importing a second project framework.

Authority: navigation. [AGENTS.md](../AGENTS.md), [AGENT_POLICY.md](AGENT_POLICY.md)
and [AGENT-AUTHORITY.md](AGENT-AUTHORITY.md) govern execution.

Last reviewed: 2026-10-18.

## Project-local skills

These are OR-specific procedures that no shared skill covers. Their canonical
bodies live in [.agents/skills/](../.agents/skills/) as one `SKILL.md` per
directory. They are loaded only by an agent session whose working project is this
repository; a file on disk does not by itself prove that a session discovered it.

| Skill | Trigger | Neighbour / exclusion |
|---|---|---|
| `or-edit-feature` | Add or extend timeline, clip, media, marker, text, caption, undo or project-lifecycle behavior; a command or query and its GUI/CLI surface | Generic planning (task-planning); a failing gate (verify-debug); AI/provider or codec work (`or-media-ai`) |
| `or-ui-ux` | Design, change or judge a user-visible surface — layout, navigation, interaction, typography, colour, feedback, accessibility, desktop-versus-compact parity — and measure the interaction performance that makes it acceptable | Backend- or domain-only work (`or-edit-feature`); a failing build or gate (verify-debug); the generic evidence-class method, which stays in the shared ui-ux-review skill |
| `or-dev-loop` | Local build, test, benchmark or preflight on this host; a lane reported `BLOCKED`; deciding whether a local result is admissible | Release publication (`or-release`); claim records (`or-claim-evidence`) |
| `or-media-ai` | AI providers, model assets, transcription/captions, noise reduction or audio cleanup, code-to-illustration MotionScene | Plain editing with no AI component (`or-edit-feature`); credential, dependency or licence review (security-review) |
| `or-claim-evidence` | Recording or auditing a completion claim; binding an exact SHA to a hosted run and job; deciding an evidence class | Running local lanes (`or-dev-loop`); publishing (`or-release`) |
| `or-release` | Packaging, tagging, publishing, release readiness, or changing a release workflow | Local packaged builds (`or-dev-loop`); claim bookkeeping (`or-claim-evidence`) |

## Shared procedures this repository defers to

Use the skills the current session exposes; the names below are the intent, not a
guarantee that a given harness has installed them.

| Request | Shared procedure |
|---|---|
| Locate core/UI command flow and callers | code-investigate |
| Diagnose a failing authorized source or hosted check | verify-debug |
| Sustain an authorized outcome across many turns, handoffs or quota pressure | sustained-repair |
| Review auth, dependencies, CI or publication risk | security-review |
| Turn an outcome into a bounded, sequenced plan | task-planning |
| Write or improve a handoff prompt artifact | prompt-assist |
| Report roadmap progress, next outcome or readiness | project-steward |
| Maintain instruction or skill contracts | skill-maintainer |
| Review a user-visible interface for usability and accessibility | ui-ux-review |

For UI work, [DESIGN.md](../DESIGN.md) and [UX_ACCEPTANCE.md](UX_ACCEPTANCE.md)
remain authoritative; a generic skill never authorizes a new design system or a
new local native runtime. The shared `ui-ux-review` supplies the generic
evidence-class method, journey checklist and review-versus-fix rule;
`or-ui-ux` binds that method to OR's design language, thresholds, shell slots,
evidence classes and local lanes. Load `or-ui-ux` for OR work and keep its
content here rather than duplicating it into the shared procedure.

## Policy

- Do not copy a shared skill into this repository. A project procedure that
  duplicates a shared skill is retired here and archived as project history,
  never maintained as a parallel activatable copy. The single skill this
  happened to, `dont-giveup-soon`, was promoted to the shared library as
  `sustained-repair` on 2026-10-14; its original text is preserved as archival
  data in
  [evidence/dont-giveup-soon-archived-skill.md](evidence/dont-giveup-soon-archived-skill.md).
- A project skill must be OR-specific: it must name real OR paths, commands and
  invariants, and it must state its exclusions, or it does not earn a slot here.
- Skill packages are instruction artifacts, not current-status documents, so they
  are not on the `scripts/check_doc_freshness.sh` list; a skill that restates a
  status claim is wrong by construction.
- Disk-only plugin packages and earlier session catalogs grant no current tools.

## Control workspace

This repository is registered in the host control workspace (AEH) under the slug
`or`. From the AEH workspace root:

```sh
python3 scripts/hub.py status or    # read-only: Git and recorded verification
python3 scripts/hub.py verify or    # explicit: runs scripts/check-repo.sh
python3 scripts/hub.py where or     # absolute path
```

`status` never executes project code. AEH holds no product or task state for OR;
the authoritative progress source is [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md), and
a verified claim is a record under [evidence/claims/](evidence/claims/README.md).
The gate ladder itself is in [QUALITY.md](QUALITY.md): `check-repo.sh` is the
source gate, `acceptance.sh` adds the Rust and Flutter lanes and the local
performance budget, and GitHub Actions is the only authority for native, packaged
and cross-platform acceptance.

Keep framework artifacts in English; conversation follows the owner's language.

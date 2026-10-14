# OR pause and Debian migration handoff

> **Append-only historical record.** Each section below describes the state at
> the moment it was written and is not updated afterwards. Several sections
> disagree with each other and with current HEAD, because that is what an
> append-only pause log is. Read the section header for its date; do not read
> the last section as current state.
>
> Current state: `git log --oneline -5` and [PRODUCT_ROADMAP.md](PRODUCT_ROADMAP.md).
>
> **Superseded facts.** Two statements below are now wrong; they are corrected
> here rather than edited in place:
>
> - The final section says the `dont-giveup-soon` skill was copied to
>   `.agents/skills/dont-giveup-soon/SKILL.md`. It was later promoted to the
>   shared library as `sustained-repair` and retired from this repository in
>   commit `64a0af1`. No project-local skill remains; see [skills.md](skills.md).
> - The preserved artifacts it describes (`migration-preserved/`) are now
>   gitignored. They remain on disk, untracked, so the migration can still be
>   audited once.

Captured 2026-10-13 (initial migration inventory). For the latest pause state, use the final `Pause state refresh` section below. This is a relocation and pause record, not a new roadmap or checkpoint authority. No product implementation, commit, release, merge, or CI run was started for this migration.

## Pause and active product direction

Development is paused at the owner's request while the workspace moves to Debian. Resume from `docs/PRODUCT_ROADMAP.md`; do not restart the historical checkpoint sequence in `docs/execution/PLAN.json` and `STATE.json`. Keep legacy plans and evidence as history and traceability.

The active roadmap's current priority is first-install and real-user feedback: make the normal first-project journey clearer and remove measured install, media, recovery, responsiveness, and export friction. Pending gaps include clean-machine install/upgrade/uninstall, ordinary-user dogfood feedback, large-library cache behavior, broader offline-media/bulk-relink workflows, Android transcription, and physical-device responsiveness evidence. The four debug Developer Preview packages from run `38369979125` expire 2026-10-20; they are not stable installers.

## Git source, destination, branches, and worktrees

- Source repository: `https://github.com/huou07/Opencut-Reinforced.git`
- Debian checkout: `/home/huou07/Projects/OR`
- Active branch and exact HEAD on both source and destination: `feat/folder-import` — `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`.
- `origin/main`: `e953aadb4437bae2210f8e317846c399494c4bf3`.
- PR #7: open, `feat/local-transcription-proposals` at `c329d8e26c994fb24af5f53d3f4c50e31087d1cd`, based on `main`.
- PR #8: open, `feat/folder-import` at `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`, based on PR #7 at `c329d8e26c994fb24af5f53d3f4c50e31087d1cd`.
- Other open PRs at inventory: #2 rusqlite `1100f3c67cf6fbedb09713a3423b753632feff39`; #3 rust-toolchain `76ad3eb6fc552e26e989162712296bdfd9f72cc1`; #4 wgpu `defde0c0257cdd330b2f9d18be212aac1421a60b`; #5 cpal `83617819a6349d9e1e11066b5992444f5fdffeee`; #6 uuid `392fb2d54cc644f9f76b330a4c8dbdbe58bcb783`. No PR was merged or retargeted.
- All 23 source local branch heads and 24 tags are present under their normal local refs. Exact SHAs are in `migration-preserved/mac-ref-manifest.txt`. The source ref snapshot, including remote-tracking and Codex checkpoint refs, is also retained under `refs/migration-source/*` and in `migration-preserved/mac-full-refs.bundle` (SHA-256 `54440733ac62252a2112b537749aaa29352923e178708831a640abf445cf9e7a`). The active checkout is a full, non-shallow GitHub clone; the Mac `.git` directory was not copied over it.
- The one branch with a commit absent from `origin/*` is `control/model-orchestrator-v2-m0-r2-unpublished-5acda49` at `5acda49945da141dbb64b1edab9e3a91835f7d1a`; it is preserved as a branch and in the bundle.
- Other inspected worktrees were clean: `checkpoint/9b-verify` at `0e4b4465b4a9f691f2fd780efc9cfb8aae28517f`, `control/model-orchestrator-v2-complete` at `325137c556018d1fe80650c616ffc1b438214696`, and `control/model-orchestrator-v2-roadmap-bounded` at `546b94e08f15d3c513923e59411d589301715e47`. The stale `/private/tmp/opencut-android-process-recovery` worktree contained no files; its branch `codex/android-process-recovery` at `b07296e1d12a144d148727d40db142b46ab67275` is preserved with the other refs.
- The source worktree had no staged, unstaged, or non-ignored untracked source changes. The external worktrees were clean. No active local product build/test process was found. The OR-scoped CodeGraph server was stopped with SIGTERM; its disposable `.codegraph` cache was not copied.

## GitHub Actions at pause

- PR #8 Repository Hygiene run `38383545957` for exact SHA `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`: completed successfully.
- PR #8 Platform Verification run `38383545793` for the same SHA: completed with overall **failure**. Flutter, descriptor-boundary, Linux, macOS, Rust, and Windows jobs passed. Android failed while fetching pinned libvpx from Chromium Gitiles (HTTP 503); subsequent Android build and journey steps were skipped. No run was cancelled. At the final inventory, no repository Actions runs were queued or in progress. Do not treat this run as complete platform acceptance.

## Local-only files and exclusions

The Mac source was clean for source code, but had ignored local-only state. The following was transferred separately and is deliberately uncommitted:

- `.opencode/checkpoint.local.md` plus `.opencode/.gitignore`, `package.json`, and `package-lock.json`.
- The distinct 9B worktree checkpoint at `migration-preserved/worktrees/9b-diagnostic/.opencode/checkpoint.local.md`.
- `full-roadmap.log` at `migration-preserved/logs/full-roadmap.log` (30,135,004 bytes).
- `migration-preserved/mac-ref-manifest.txt` and the complete Git bundle above.

The two checkpoint files differ; both are retained. Their SHA-256 values, respectively, are `56df715e066a7e647d1f1940cb5d9a9e3c0dfa5ed2bf965dfa4ef60bbd3a1a2d` and `5a88444011032f687b94ca043680e29163ae3b5d2d9219ad055c89deb3406496`. The log SHA-256 is `1546d4c7ef7940653c099faeba021fa0e068c7639d2a71b66ce39c17e194e78e`.

Excluded as generated, machine-specific, or disposable: `.codegraph` index, `.opencode/node_modules`, Flutter `.dart_tool` and build/ephemeral outputs, Cargo `target` directories, FFmpeg helper build output, IDE state, generated Android Java, `android/local.properties`, `.DS_Store`, and other platform-generated metadata. No personal credentials, provider tokens, login state, SSH keys, unrelated home configuration, VM, or media cache was copied. No project-specific `SKILL.md` exists in the active tree or the inspected Git refs. OR's versioned `.opencode/agents/*.md` prompts remain on their original branches. The Debian shared skill library at `/srv/agent-workbench/skills` was left canonical and untouched; `agents-worker` belongs to its `agent-skills` group.

## AEH, permissions, and verification

OR was registered through the existing AEH command (`scripts/hub.py add`) as slug `or`; AEH resolves `/home/huou07/Projects/OR`. The registry entry identifies a Git repository, active branch `feat/folder-import`, and HEAD `79b239b`.

The destination existed but was empty before cloning. Its existing ownership/ACL policy was retained: workspace root is owned by `agents-worker` (UID 987); explicit and default ACL entries grant `agents-worker` `rwx`. `AGENTS.md`, `docs/PRODUCT_ROADMAP.md`, and Git metadata are readable/writable through that ACL. No Debian global AGENTS file or shared skill library was changed. The Hub reports a clean repository before this local pause note and preservation directory were added; these local-only files will make Git status intentionally show untracked migration state.

Lightweight migration checks completed: source and destination HEADs and named GitHub refs matched; branch/tag SHAs were compared with the bundled source refs; GitHub remote URL, current branch, branches, tags, project/doc inventory, and AEH `where`, `list`, and `status` were checked. The transferred bundle/archive checksums were verified. No Debian build, emulator, VM, or native compilation was run. The full source report is in `migration-preserved/mac-ref-manifest.txt`.

## Resume from Debian

Use the existing AEH project entry (`or`) and `/home/huou07/Projects/OR`. Start with `git status --short --branch`, `git remote -v`, and `git branch -avv`. Continue product work from `docs/PRODUCT_ROADMAP.md`, first using real user feedback and the install/first-project gaps above. Keep PR #7 and #8 stacked as recorded, do not infer completion from the failed Android run, and use GitHub Actions for platform verification. Preserve the local pause note and migration archive until their contents are no longer needed.

## Pause state refresh — 2026-10-13 17:30 +07:00

The Debian workspace is now the active preserved checkout. Its current branch is `feat/linux-debian-preview-stacked`, HEAD `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`, based on PR #8 without changing that PR. The clean Mac checkout remains at `feat/folder-import` on the same SHA. `origin/main`, PR #7, and PR #8 refs were rechecked against GitHub and still resolve to `e953aadb4437bae2210f8e317846c399494c4bf3`, `c329d8e26c994fb24af5f53d3f4c50e31087d1cd`, and `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6` respectively. PR #7 and #8 remain open and stacked (#8 targets #7).

One in-flight, staged but uncommitted WIP is preserved on the Debian branch: Linux `.deb` packaging, install smoke coverage, workflow integration, and install/release/tooling documentation (the eight staged paths shown by `git status`). This work is not accepted or verified and must not be described as shipped. Independent review found unresolved issues: include OR's MIT copyright/license, make the install test use an isolated user-data directory, and substantiate the claimed distro range with pinned/tested distro images. No local implementation process was running when checked. No commit, push, PR, or new CI run was created for this WIP.

GitHub Actions run `38383545793` (Platform verification, exact SHA `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`) remains in progress on its Android APK build; the earlier Android Gitiles HTTP 503 is being retried. Other platform jobs completed successfully. Run `38383545957` (Repository hygiene, same SHA) passed. No other OR Actions run was in progress at the latest query. Leave the existing run alone and do not monitor it during the pause.

AEH slug `or` resolves to `/home/huou07/Projects/OR`. Its status command recognizes Git state when invoked with a command-scoped `safe.directory` setting; without it, Git's ownership guard makes Hub status report no repository because the checkout is owned by `agents-worker`. No shared Git configuration was changed. A direct `sudo -u agents-worker` check confirmed worker UID 987 can read `AGENTS.md` and `docs/PRODUCT_ROADMAP.md`, create a temporary file in the workspace, and remove it. The migration bundle verifies complete history and its recorded SHA-256 remains `54440733ac62252a2112b537749aaa29352923e178708831a640abf445cf9e7a`.

The OR-local `.opencode/` files and migration archive remain present. No project-local `SKILL.md` exists; no global skills or credentials were copied or modified. Product development is paused here. Resume later from Debian, preserve the staged WIP for review/repair before any commit, and continue from `docs/PRODUCT_ROADMAP.md` rather than the historic checkpoint runner.

The staged Debian package WIP also has a byte-preserving patch backup at `migration-preserved/linux-deb-preview-staged.patch` (SHA-256 `425f706ec329687ec5992b874c338340034f23c51c0a321245b8edb997561038`); the staged working-tree copy remains untouched.

## Product work resumed — 2026-10-13

The owner later resumed the broader product mission. The preserved Debian-package WIP was reviewed, repaired, and committed as `63a2a6b25f11e489fda1eb41b77f056a9f0bda2c` (`feat(packaging): add verified Debian preview installer`), pushed on `feat/linux-debian-preview-stacked`, and opened as PR #9 stacked on PR #8. Repository Hygiene run `38394866774` passed. Platform Verification run `38394866775` is live for the exact commit; its Linux job now includes package install/launch/uninstall checks on the Ubuntu runner and clean Ubuntu 24.04 and Debian 13 containers. Do not treat these runtime claims as accepted until that run completes successfully.

The bounded mission control remains `milestone:full-roadmap`, with legacy sequencing paused. Its read-only preflight on a clean `main` worktree accepted explicit selection `9E`; a local dirty-worktree refusal was expected and no state was changed. `scripts/test_execution_infra.py` passed all 152 tests. The prior PR #8 Platform Verification rerun `38383545793` has since completed successfully for SHA `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`.

## Final pause refresh — 2026-10-13 19:14 +07:00

The owner requested OR development pause and workspace relocation. No new OR implementation, local build, release, merge, PR retarget, or CI run was initiated for the migration. Development must remain paused after this handoff.

- Debian source of truth: `/home/huou07/Projects/OR`; origin is `https://github.com/huou07/Opencut-Reinforced.git`. The clone is full (`--is-shallow-repository=false`) with 24 tags and all recorded local/remote branches. Only the active checkout is mounted as a worktree. `agents-worker` (UID 987) can read `AGENTS.md` and write within the workspace. No platform builds, emulators, VMs, or native compilation were run here.
- Current development branch: `feat/linux-debian-preview-stacked`, exact HEAD `63a2a6b25f11e489fda1eb41b77f056a9f0bda2c`; its upstream branch matches. `origin/main` is `e953aadb4437bae2210f8e317846c399494c4bf3`.
- Open stacked PRs: #7 `feat/local-transcription-proposals` at `c329d8e26c994fb24af5f53d3f4c50e31087d1cd` → `main`; #8 `feat/folder-import` at `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6` → #7; #9 `feat/linux-debian-preview-stacked` at `63a2a6b25f11e489fda1eb41b77f056a9f0bda2c` → #8. All remain open; no target changed and none was merged.
- At the check time, #9 Repository Hygiene run `38394866774` passed on the exact HEAD. Platform Verification run `38394866775` remains in progress on the same exact HEAD. Its Android APK job `115287397496` failed during Android SDK/NDK/emulator setup before product acceptance steps; Windows, Rust, macOS, and Linux jobs were still active, with Flutter and the descriptor-boundary job passed. Do not cancel the run, claim acceptance, or start a rerun. PR #8 Platform Verification run `38383545793` subsequently passed on exact SHA `79b239b4a8a16ea0d46c39dd69aefc55840dc6d6`. This refresh is the final observation; do not continue monitoring.
- Product status and next priorities remain those in `docs/PRODUCT_ROADMAP.md`: the recent packaged editor journey is verified, but ordinary-user dogfood, clean install/upgrade/uninstall, broader offline media/bulk relink, large-library cache behavior, Android transcription, and physical-device responsiveness remain unverified or incomplete. PR #9 adds the Debian package candidate but its current platform acceptance is pending. Do not mark these gaps done from local checks or the active partial CI run.
- Local-only work is retained separately and uncommitted: `docs/PAUSE_MIGRATION_HANDOFF.md`, `migration-preserved/` (including full-ref manifest/bundle, WIP patch, and source-only historical receipts), `.opencode/checkpoint.local.md` and local package metadata, plus generated `scripts/__pycache__/`. The source checkout had no user source edits to transfer beyond the separately preserved Debian packaging WIP, now committed as PR #9. No local source changes are staged; the listed migration records are intentionally not mixed into PR #9. The preserved unpublished control branch remains `control/model-orchestrator-v2-m0-r2-unpublished-5acda49` at `5acda49945da141dbb64b1edab9e3a91835f7d1a`; the migration bundle and manifest retain its provenance and other source refs.
- The active checkout contains `AGENTS.md`, docs/execution and product/architecture/UX acceptance records, scripts, and GitHub workflows. OR-local `.opencode` configuration/checkpoint files were retained. No project-local `SKILL.md` was found in the active tree; do not treat a missing project-local skill as a reason to duplicate or alter the Debian global skill library. The global/shared skill library and global AGENTS were not changed; credentials and login state were excluded.
- AEH slug `or` is registered and resolves to `/home/huou07/Projects/OR`; the registry addition is present in `/home/huou07/dsh projects/projects/registry.json` and remains uncommitted there. AEH can report the correct branch and HEAD when Git's `safe.directory` setting is scoped to both the Hub and OR paths for that invocation; no shared Git configuration was changed. A prior single-path invocation showed Git's ownership guard and was not evidence that OR lacks a Git repo.
- One unrelated process was visible during inventory: a root-owned shell running a Rust toolchain investigation under `/tmp/vce-422d0ae-linux-build/native-offline`. It is outside OR and was left untouched. No active OR-local build or test process was found.

Resume only when the owner explicitly resumes OR development. Use AEH slug `or` and this Debian checkout. First read this pause record and `docs/PRODUCT_ROADMAP.md`, inspect the current GitHub/CI state, and continue from the exact remote state; keep legacy execution documents as historical traceability rather than restarting their checkpoint loop.

## Pause checkpoint — resumed product work stopped on owner request (2026-10-13)

The owner clarified that work should pause after the current commit batch and that the immediate goal is transfer of project/progress to Debian. The current commit batch is complete and pushed; no further OR implementation, commit, build, release, rerun, merge, or CI dispatch was started. The Debian checkout is already the active workspace.

- Current branch `feat/linux-debian-preview-stacked`, HEAD and upstream `63a2a6b25f11e489fda1eb41b77f056a9f0bda2c` (`feat(packaging): add verified Debian preview installer`). PR #9 remains open against PR #8 at that exact SHA. No staged or modified source files are present. The only untracked state remains this local pause handoff, `migration-preserved/`, and generated `scripts/__pycache__/`.
- Platform Verification run `38394866775` is still in progress for that exact HEAD and was left uncancelled. Windows, Rust, Flutter, and granted-descriptor checks passed. Linux failed at `Verify Debian package installation and user-data safety`; Android failed at `Install Android platform, Rust NDK, and x86_64 emulator` before product acceptance. macOS is still running `Run native Flutter project lifecycle tests`. The detailed logs were not available until aggregate workflow completion at the time of pause. Do not infer the Linux failure cause or claim the package candidate accepted. Do not keep polling while paused; inspect the terminal run/logs when the owner resumes work.
- No Developer Preview was dispatched. No release or merge occurred. The previously recorded product roadmap priorities and remaining limitations are unchanged.

The workspace transfer and AEH registration remain complete as recorded above. Resume only on explicit owner instruction, beginning with current GitHub/Actions state and this exact Debian checkout.

## Project-local persistence skill added — 2026-10-14

At the owner's request, the Mac skill `/Users/meomeo/.agents/skills/dont-giveup-soon/SKILL.md` was copied byte-for-byte to `.agents/skills/dont-giveup-soon/SKILL.md` in this Debian project. SHA-256 matched on both copies (`b4a3676bda3543aa56ff879adf8995b4fc13dd337b66269e94366871857f1c07`), and `agents-worker` can read and write the new file. The shared Debian skill library was not changed.

The copy is committed locally as `7ab59780a238b1880c8c2a2ab8e278d75991a8f7` (`docs(agents): add project dont-giveup-soon skill`) on `feat/semantics-journey-guards`, whose parent is `27db1f26dfa6bcde7f8139c79e514c329e03eeb4`. That branch has no matching `origin` branch, so the skill commit remains Debian-local and was not pushed. No product work or CI was started. Preserve this commit and branch; inspect remote state before resuming or publishing.

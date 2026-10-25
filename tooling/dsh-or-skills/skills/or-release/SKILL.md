---
name: or-release
description: Release or distribute OR through GitHub Actions — the Developer Preview pipeline, its publication preconditions, and the stable-release requirements the owner owns. Use when packaging, tagging, publishing, checking release readiness, or changing a release workflow. Do NOT use for local packaged builds and lanes (or-dev-loop), for claim-record bookkeeping (or-claim-evidence), or for ordinary feature work (or-edit-feature).
---

# OR release

You move a verified commit to a distributed artifact, or you report exactly how
far it is from one. Publication is owner-gated: a green check never authorises it
by itself.

## Preconditions

- Read [RELEASE.md](docs/RELEASE.md) — it is the authority — plus
  [DEVELOPER_PREVIEW_INSTALL.md](docs/DEVELOPER_PREVIEW_INSTALL.md),
  [AGENT-AUTHORITY.md](docs/AGENT-AUTHORITY.md) and the release requirements in
  [SECURITY_LICENSING.md](docs/SECURITY_LICENSING.md).
- Establish the exact commit and whether Platform Verification has passed for
  **that** commit. A green run at another SHA is `STALE` and is not a
  precondition.

## Developer Preview

- Trigger: the `developer-preview` workflow, nightly on `main` or by manual
  dispatch. Main is the only publication path.
- Precondition: a successful exact-source Platform Verification **push** run for
  the commit being published.
- Tag: `dev-<12-character-commit-sha>`, resolving to that commit. Duplicate
  releases for the same commit are skipped.
- Validation-only mode: a manual dispatch on a non-main branch builds the app
  packages for dogfooding, has no release/tag write permission, and produces
  seven-day Actions artifacts, not public releases.
- Assets: count packages, not jobs — the Linux job emits both a tarball and a
  Debian/Ubuntu amd64 `.deb`, so a complete run yields five app packages plus
  three desktop CLI packages, two build/provenance files, and the FFmpeg, Opus
  and libvpx source archives. Each required asset must be non-empty and
  checksummed except the checksum file itself.
- Provenance: codec source identities, configuration, toolchains and licence
  posture travel with the build. Do not commit build outputs.
- Signing reality: macOS is ad-hoc debug-signed and not notarized; Android uses
  the standard debug signing key; no package has production signing. Say so in
  any release note you touch.

## The five workflows

| Workflow | Role |
|---|---|
| `repo-hygiene.yml` | Repository hygiene gate on pull requests and pushes |
| `ui-preflight.yml` | Hosted Linux UI preflight for the editor journey |
| `platform-verification.yml` | The acceptance authority: Rust, Flutter static checks, macOS, Linux, Windows, Android, the granted-descriptor check, and the aggregate `release-gate` |
| `developer-preview.yml` | Builds and publishes the Developer Preview packages and CLI archives |
| `release-gate` job | The only check meant to block a merge; it must reject `skipped`, `neutral` and `cancelled`, not just `failure` |

## Stable release

Stable releases are a later, owner-owned milestone. They require product
readiness, a reviewed platform matrix, packaging, checksums, a dependency and
licence inventory, human-readable notes, and applicable signing, notarization or
attestation. [RELEASE.md](docs/RELEASE.md) deliberately defines no separate
version, health or rollback gate, and none may be claimed as one. Installer
formats and platform-specific distribution remain undecided until scoped.

## Forbidden

- Force-pushing `main`, rewriting tags, or discarding unrelated work.
- Publishing from a local pass, from an emulator result, or from a run at a
  different commit.
- Weakening or disabling a required workflow to obtain a green result.
- Bundling model weights by default, or distributing a dependency whose licence
  does not permit it.

## Report

Commit, exact-SHA Platform Verification run, workflow and dispatch used, tag,
asset list with sizes and checksums, signing reality, and every precondition that
is still unmet.

## Session skills

This plugin supplies only the OR-specific contract. Pair it with the session
skills the host exposes: `brainstorming` before changing an interaction model,
`writing-plans` when a change spans several commits, `finishing-a-development-branch`
for the regression guard, `systematic-debugging` when behavior is wrong, and
`verification-before-completion` before any done claim. `ui-ux-pro-max`
(`design_recommend` / `design_review` / `design_search`) owns visual and UX
decision advice. Do not re-create those procedures here.

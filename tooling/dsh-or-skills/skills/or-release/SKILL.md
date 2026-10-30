---
name: or-release
description: Release or distribute OR through GitHub Actions — the Developer Preview pipeline, its publication preconditions, and the stable-release requirements the owner owns. Use when packaging, tagging, publishing, checking release readiness, or changing a release workflow. Do NOT use for local packaged builds and lanes (or-dev-loop), for claim-record bookkeeping (or-claim-evidence), or for ordinary feature work (or-edit-feature).
---

# OR release

Move a verified commit to a distributed artifact, or report exactly how far it is
from one. Publication is owner-gated: a green check never authorises it.

## Preconditions

- Read `docs/reference/release.md` (authority),
  `docs/reference/developer-preview-install.md`, `docs/operations/permissions.md`
  and the release requirements in `docs/reference/security-licensing.md`.
- Establish the exact commit and whether Platform Verification passed for **that**
  commit. A green run at another SHA is `STALE`.

## Developer Preview

- Trigger: `developer-preview` workflow, nightly on `main` or manual dispatch.
  Main is the only publication path.
- Precondition: successful exact-source Platform Verification push run for the
  commit being published.
- Tag: `dev-<12-character-commit-sha>` resolving to that commit; duplicates are
  skipped.
- Validation-only dispatch on a non-main branch has no release/tag permission and
  produces seven-day Actions artifacts, not public releases.
- Assets: count packages, not jobs — five app packages (macOS, Windows, Linux
  tarball, Debian/Ubuntu amd64 `.deb`, Android APK), three desktop CLI packages,
  two build/provenance files, and the FFmpeg, Opus and libvpx source archives;
  each required asset non-empty and checksummed except the checksum file.
- Signing reality: macOS ad-hoc debug-signed and not notarised, Android debug
  key, no production signing. Say so in any release note you touch.

## Workflows

| Workflow | Role |
|---|---|
| `repo-hygiene.yml` | repository hygiene gate |
| `ui-preflight.yml` | hosted Linux UI preflight |
| `platform-verification.yml` | acceptance authority for all native platforms |
| `developer-preview.yml` | builds and publishes preview packages |
| `release-gate` job | must reject `skipped`, `neutral` and `cancelled`, not just `failure` |

## Forbidden

Force-pushing `main`, rewriting tags, discarding unrelated work, publishing from a
local pass or an emulator result, weakening a required workflow to get green,
bundling model weights by default, distributing a dependency whose licence does
not permit it.

## Report

Commit, exact-SHA Platform Verification run, workflow and dispatch, tag, asset
list with sizes and checksums, signing reality, and every unmet precondition.

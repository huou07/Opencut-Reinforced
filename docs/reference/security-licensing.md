# Security, Dependencies and Licensing

Last reviewed: 2026-10-26

Scope: adoption rules for dependencies, media, fonts and models; the FFmpeg
posture; and how secrets and licences are kept out of trouble. Architecture trust
boundaries are in [architecture/security-boundaries.md](../architecture/security-boundaries.md).

This is product guidance, not legal advice. OR is pre-MVP: recheck exact versions,
build options, assets and model artifacts before any distribution.

## Repository licence

MIT. Third-party dependencies, media, fonts, templates, plugins and model
artifacts keep their own terms and need individual review.

## Adoption rules

Before adding a dependency or asset, inspect its source, maintenance, integration
cost and licence. Do not introduce GPL, AGPL, SSPL, non-commercial or
source-available-only components into the distributed product without explicit
owner approval and a recorded licensing decision.

Do not bundle model weights by default; any future inclusion needs a size,
licence and distribution strategy. Keep secrets, model weights, generated media,
caches and build output out of tracked source.

## Current dependency posture

| Dependency | Purpose | Licence |
|---|---|---|
| `serde`, `serde_json` | DTO serialization, strict project codec | MIT OR Apache-2.0 |
| `uuid`, `url`, `sha2` | typed IDs, file-URI validation, cache keys | Apache-2.0 OR MIT |
| `rusqlite` (bundled) | private disposable cache index only | MIT |
| `windows-sys` | Windows atomic replace, named pipes, DACLs | MIT OR Apache-2.0 |
| `cosmic-text` | bundled-font shaping and rasterisation in `or_render` | MIT OR Apache-2.0 |
| `subtitler` (srt, vtt) | bounded SRT/WebVTT codec | Apache-2.0 |
| FFmpeg 8.1.3 | media runtime, LGPL-only build profile | LGPL (per configured components) |
| Inter (bundled, `or_render/assets/fonts/inter`) | default text face | SIL OFL 1.1 |
| Be Vietnam Pro (bundled, `or_render/assets/fonts/bevietnampro`) | Vietnamese display text face | SIL OFL 1.1 |

Cargo and pub lockfiles record the resolved graph. Recheck transitive licences
before distribution.

## FFmpeg and codecs

FFmpeg is built from the official source archive under an LGPL-only profile and
ships with the app bundles together with its notices. H.264/AAC and VP9/Opus
carry separate patent and distribution obligations that must be reviewed for each
market before relying on a delivery profile. Bundled FFmpeg is a
media-interchange build (limited muxers and filters); do not depend on
statistics-printing filters in shipped builds.

## Secrets and scanning

- Provider credentials live in OS secure storage and never reach CLI or agents.
- `gitleaks detect` runs in the acceptance lane when installed and is pinned in
  CI. `scripts/check-repo.sh` alone does not scan for secrets.
- Secret scanning and push protection are enabled on GitHub as a hosting control;
  that is not a local proof.

## Claim records

Hosted claim records live in `docs/evidence/claims/` and must resolve to a real
commit. Never fabricate or edit one to represent local evidence; see
[testing.md](testing.md) and [operations/verification.md](../operations/verification.md).

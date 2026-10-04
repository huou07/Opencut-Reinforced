# Application Release Plan

## Release evidence

Release availability, package contents, capability maturity and runtime versions
must be obtained from [release artifacts](https://github.com/huou07/Opencut-Reinforced/releases),
their exact implementation SHA and supervisor evidence. Current checkpoint state
belongs only to [STATE](execution/STATE.json). This plan maintains no availability
or capability inventory. Query STATE → locked specification → supervisor evidence
→ implementation SHA/source for implementation truth.

## Developer Preview

Developer Preview publication must follow the pinned workflow at the source SHA
and the required package/evidence contract in [EVIDENCE_POLICY](execution/EVIDENCE_POLICY.json).
A successful build alone does not prove publication or product readiness. Tags,
package identities, checksums, provenance, license notices and signing properties
must be verified against the exact published artifacts; do not infer them from
a manually maintained list in this plan.

The supervisor independently verifies required hosted runs/jobs and previews
before advancing a checkpoint. A runner cannot grant completion, publication,
operational adoption or promotion authority. Missing required evidence leaves
execution state unchanged. Build outputs and credentials never enter Git.

## Future Stable Release

A stable release is a later product milestone, not a Developer Preview. Stable releases will use deliberate semantic versions and require product readiness plus a reviewed platform matrix, packaging, checksums, dependency and license inventory, human-readable notes, and applicable signing, notarization, or attestation. Installer formats and platform-specific distribution remain undecided until that work is scoped.

## Intended platforms

- macOS
- Windows
- Linux
- Android

The product does not currently target iOS or web. Do not bundle model weights by default. Any future inclusion needs a clear size, license, and distribution strategy. FFmpeg configuration and redistribution implications are recorded per target in the Developer Preview provenance asset and must be rechecked for any codec or packaging change.

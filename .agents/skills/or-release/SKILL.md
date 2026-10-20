---
name: or-release
description: Verify OR release readiness or change the hosted packaging/release workflow under explicit scope. Publish/tag only when separately authorized. Exclude local packaged diagnostic lanes (or-dev-loop), ordinary completion claims, UI/backend implementation and source-only checks.
---

# OR release verification

[RELEASE.md](../../../docs/RELEASE.md) is the release authority; read applicable
requirements and [SECURITY_LICENSING.md](../../../docs/SECURITY_LICENSING.md).
[DEVELOPER_PREVIEW_INSTALL.md](../../../docs/DEVELOPER_PREVIEW_INSTALL.md) owns
installation/signing reality.

1. Establish exact source and required platform-verification push run/jobs.
   Another SHA is STALE; skipped/neutral/cancelled required jobs block approval.
2. Check the actual shipped entrypoint, packaged dependencies and permissions,
   supported platform journeys, independent output decoding/player, provenance,
   asset checksums and licence notices. A debug/widget run is insufficient.
3. Preserve the aggregate release-gate and main-only publication safeguards.
   Non-main validation-only artifacts are not releases or release authorization.
4. Report signing/notarization limits accurately. Stable-release criteria remain
   owner-owned; do not invent a version/health/rollback gate.
5. Run only the verification requested. Readiness/report-only requests authorize
   no edits, builds, dispatches or publication; explicit verification authorizes
   its checks. Publishing, tagging, merging and deploying need separate authority.

Stop with the readiness verdict and concrete missing requirements, or at the
authorized publication boundary. Report source, run/jobs, package identity,
executed journeys, assets/provenance, failures and UNVERIFIED targets.

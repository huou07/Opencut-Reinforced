---
name: or-media-ai
description: Implement or verify an authorized OR media compatibility, import/export, audio DSP, AI provider or declarative MotionScene change. Use for container/codec and decoded-content acceptance as well as model boundaries. Exclude plain timeline editing (or-edit-feature), UI-only interactions (or-ui-ux), host lane mechanics, generic planning and security-only reviews.
---

# OR media and provider boundaries

Read the affected media contract in
[TECHNICAL_PLAN.md](../../../docs/TECHNICAL_PLAN.md) and the applicable
[SECURITY_LICENSING.md](../../../docs/SECURITY_LICENSING.md) section. Provider
work additionally reads ADR 0005; MotionScene work reads ADR 0007. Plain media
compatibility does not require AI/provider context.

1. Identify supported platform, real input, expected streams/content and current
   capability limits. Reuse packaged FFmpeg/media components and fixture recipes.
2. Import through the supported picker/provider for native import claims; check
   usable canonical references, missing/corrupt/unsupported recovery and save/reopen.
3. For preview require changing decoded/presented frames, seek and supported audio;
   an open surface/thumbnail/transport clock alone never establishes playback.
4. Export through the app, then independently probe preset duration/dimensions/
   streams/codecs, decode frames/audio and open with an independent player.
   Reuse the packaged journey and media_checks; retain errors as failures.
5. Preserve revision-checked apply, cancellation, source provenance and licences.
   DSP stays in audio/media, never Flutter or unbounded realtime callbacks.
   AI returns proposals, never instructions; credentials remain inaccessible.
   Rendering makes no provider calls. MotionScene remains non-executable.

No bundled/downloaded models without the required confirmation. Stop on a
licence, unsupported-platform or trust-boundary conflict; do not replace the
engine or silently widen support. Report inputs/checksums, source/build,
actual actions and decoded observations, evidence, outcome and remaining gaps.

# Journey acceptance infrastructure — 2026-10-20

Status: instrument controls and real diagnostic detection PASS for the scopes below.
Product acceptance remains FAIL/UNVERIFIED. No product defect is declared fixed.
Final local admission and follow-up hosted checks are recorded in PR #19.

## Source and scope

Base: `8abb41365ca0b2a89852fd60a6df64ab94a76195`. The original registered
checkout was on main at `21bfb3aa6891cf68d5158946935dc40f86c569fd`, 145 commits
behind remote main, with unrelated changes in seven files and an untracked
toolchain manifest. Those were preserved. This task uses an isolated
`chore/journey-acceptance-system` worktree. Initial infrastructure commit:
`aae462cfba27a6611580c73ebe97cdd41339e016`;
[review PR #19](https://github.com/huou07/Opencut-Reinforced/pull/19).
No merge, tag, release, deployment or product feature changes.

AEH's prerequisite selfcheck was rerun before OR changes: 175 tests, structural
checks and doctor PASS, exit 0. AEH's existing uncommitted repair set was untouched.
Registration remains at the original checkout. One writer; no delegated agents,
new framework, new dependency installation or host-service changes.

Local real-app receipts name the first infrastructure commit plus their dirty
diff and task-source hashes. Production application code remains the base source;
only the integration entrypoint/infrastructure changed. The early failure
reporter lost build mode and gave a misleading pub-cache warning as the top reason;
its raw split transcript retains the actual failure. The repaired reporter
preserves runner exit, build identity, failed step and unavailable remaining steps.

## Changes and authorities

- AGENTS is an operating map; AGENT_POLICY owns scope/safety; INDEX routes
  ownership; DEVELOPMENT_WORKFLOW owns the user-journey loop. Existing product,
  architecture, format, licensing and release authorities remain.
- All six OR skills were narrowed: triggers, exclusions, stops and observable
  outcomes. Shared skills are referenced, not copied or recursively activated.
  Ten reviewed routing cases cover UI, Rust, media, performance, release and
  neighboring non-triggers. These are contract checks, not live model-selection
  or token-cost measurements.
- QUALITY and UX_ACCEPTANCE require actual interaction, canonical state, changing
  preview content, independently validated output and independent human usability.
  TESTING owns check selection, fixtures and the ordinary-user procedure;
  TOOLING owns invocation and host constraints. Historical evidence remains.
- Existing or_ui/Flutter facilities gained semantic relative dragging, assertions
  over the production Rust session, viewer-only preview probes, profile invocation,
  bounded Flutter build/raster timings and app RSS, source/build/artifact hashes.
  Missing/contradictory evidence cannot inherit a runner's PASS.
- Profile invocation needed two runner repairs before it could be observed: debug-only
  implicit IME registration, then the implicit physical-key lookup that reads
  `debugName`, which profile builds strip. The runner now registers Flutter's
  supported test input, asserts that an entered field actually retains the text
  and supplies explicit physical keys. Both earlier attempts are retained as FAIL
  receipts rather than retried away.
- Independent export checks cover preset bounds, full bounded video/audio decode
  and optional ffplay. Existing packaged verification also rejects excessive
  duration and decoder errors even when the process exits 0.
- Seven original deterministic smoke recipes supplement existing tiny H.264/AAC
  and representative 30-second 1080p package workloads. They cover audio/video,
  silent video, audio-only, multiple sizes/fps/codecs/containers and bad input.
- The existing Linux UI workflow keeps its editor journey and adds the real split
  diagnostic. Instrument PASS and product FAIL are printed separately. No release
  safeguard or acceptance authority was weakened.

Preflight --exec was found to bypass supervision. The follow-up reuses its
existing supervisor, 600-second budget, 2 GiB runtime floor, one compiler worker,
owned-process cleanup and manifest restoration. The 6 GiB initial floor remains.

## Real application observations

[Receipt index](journey-acceptance-2026-10-20/index.json) records exact original
and retained hashes. JSON host paths were normalized to $REPO/$HOME; PNG bytes
were preserved. Reports, supplied scenarios, transcripts, canonical observations
and selected screenshots/semantic trees are retained. Screenshots were captured,
but this agent did not visually adjudicate them.

All local runs below use native Flutter/Rust under Linux Xvfb/software GL with
a substituted file picker. They are not installed-package, actual file-dialog,
physical-device, audible-output or human usability acceptance.

| Journey | Actual result and interpretation |
|---|---|
| [Direct trim](journey-acceptance-2026-10-20/direct-trim-diagnostic/report.json) | PASS: accessible end-handle drag changed canonical duration 1/1 → 1/4; keyboard undo restored 1/1. Nineteen actions/assertions completed. The reported missing trim control was not reproduced here. |
| [Split at playhead](journey-acceptance-2026-10-20/split-at-playhead-diagnostic/split-at-playhead-diagnostic.json) | FAIL: after real import/insert/seek, Split opens a required empty exact-time field. Step 20 observes timeline-split-time still present; canonical precondition is one clip. The instrument correctly retains this existing defect. |
| [Initial playback](journey-acceptance-2026-10-20/preview-playback-diagnostic-repaired/report.json) | FAIL: first-flow setup displays Set rate; attempted Play stays playing=false, sequence 1, time 0/1, unchanged viewer pixels. This does not establish a frozen active decoder. |
| [Configured playback](journey-acceptance-2026-10-20/preview-configured-playback-diagnostic/report.json) | FAIL acceptance observation: after selecting 24 fps/enabled Play, decoded sequences 7,14,22 and presented times 1/4,2/3,1/1 advance, but viewer-only pixels do not. Presentation failure versus native texture readback limitation requires independent target capture. No preview PASS. |
| [Video-only export](journey-acceptance-2026-10-20/export-configured-diagnostic/report.json) | Runner PASS, overall FAIL: independent decode detects silent audio. Scenario inserted video only; this is invalid A/V setup, not a proven export-engine defect. |
| [Unaligned A/V export](journey-acceptance-2026-10-20/export-audio-track-diagnostic/report.json) | Runner PASS, overall FAIL: independent duration is 4.042 s rather than 2 s. Canonical starts 0/1 and 251/125 prove audio was appended at global content end. |
| [Aligned A/V export](journey-acceptance-2026-10-20/export-aligned-audio-diagnostic/report.json) | PASS scoped media correctness: explicitly insert A1 and align start 0/1; output 2.042 s, 160×90, VP9/Opus WebM, 49 distinct decoded frames, 16,064 PCM samples, RMS 1991.55. ffplay completes with explicit dummy audio. Numeric alignment and sequence setup prevent a beginner-UX acceptance claim. |
| [Direct trim in a profile build](journey-acceptance-2026-10-20/profile-direct-trim-key-repaired/report.json) | PASS: the same end-handle drag changed canonical duration 1/1 → 1/4 and keyboard undo restored 1/1 across nineteen actions; build mode `profile`, Flutter build/raster timings and current/peak RSS 380.8/398.6 MB were retained. Both earlier profile attempts are kept as [setup](journey-acceptance-2026-10-20/profile-direct-trim/report.json) and [key](journey-acceptance-2026-10-20/profile-direct-trim-input-repaired/report.json) FAIL receipts. This is a software-path sample on this host, not a device baseline. |

The [actual exported video](journey-acceptance-2026-10-20/export-aligned-audio-diagnostic/export.webm)
has SHA-256 `3c41f1a0b89cc710ba39dd3b6cfa1d7cc14a7bb9b586be84690737426b1ca1f0`.
Fixture WebM checksum:
`1920ea7bb6c59ff862c4b0e1e25a342d89ea06fc53fac7069c170eb9fd40aff3`.
The prioritized, confidence-graded backlog is in
[PRODUCT_ROADMAP](../PRODUCT_ROADMAP.md#acceptance-defect-backlog-2026-10-20).
No speculative codec or missing-control defect was promoted to confirmed.

The two rejected profile attempts are the honest record of that lane: the first
reported a real timeout because the entered project name never reached the field,
and the second stopped on a null IME/client lookup. Neither was relabelled PASS;
the runner defect was repaired and the journey rerun. The host, Flutter/Rust and
media versions for these runs are recorded in
[environment.json](journey-acceptance-2026-10-20/environment.json), and the
fixture checksums there are valid only for that generator version.

## Instrument checks and CI

The source gate passed at the first candidate (482 tracked files).
Follow-up focused checks pass: twelve evidence-grading tests, six harness tests,
two skill-contract tests, and twenty-one preflight tests; revised Dart analysis
and formatting passed. Media controls regenerated seven identical checksums,
decoded positive content and rejected corrupt/unsupported/frozen inputs.
The positive generated source decoded 24 distinct frames and nonzero audio.
Initial ffplay with real audio failed for lack of ALSA sink; the explicit dummy
run proves player reopening/decoding only, not perceived audio or sync.

At the first commit, hosted
[Linux UI 39367547247](https://github.com/huou07/Opencut-Reinforced/actions/runs/39367547247)
passed the editor and diagnostic lanes, retaining PRODUCT_JOURNEY=FAIL and
INFRASTRUCTURE_CHECK=PASS for split. Hosted
[hygiene 39367547280](https://github.com/huou07/Opencut-Reinforced/actions/runs/39367547280)
failed because an invalid-media unit test assumed ffprobe was installed.
That infrastructure defect was corrected: reject an invalid WebM signature
before probing, test with empty PATH, and separately require missing tools to
report BLOCKED. Valid-looking files still require independent probing/decoding.
[Platform verification 39367547124](https://github.com/huou07/Opencut-Reinforced/actions/runs/39367547124)
and follow-up exact-source conclusions are linked in the PR. Earlier runs do not
become evidence for later HEAD automatically.

The scoped initial gitleaks patch scan (208,505 bytes) found no leaks.
Automatic review rejected blanket untracked-file hashing; source provenance uses
named task files instead. A public-cache staging attempt hit a denied template
certificate and did not retry that file or inspect credentials. Dart used an
isolated analyzer state; the tracked lockfile was unchanged.

## Previous and revised process

[Same-corpus measurements](journey-acceptance-2026-10-20/process-comparison.json)
cover ten authorities and six skills: 4,206 → 3,760 lines and 42,129 → 39,233
whitespace-delimited words. AGENTS: 145 → 62 lines; policy: 436 → 133;
skills total: 530 → 178. These are context-size counts, not tokenizer counts or
a semantic-duplication score. Some testing/tooling guidance grew to cover missing
acceptance. New diagnostic receipts/backlog are outside that comparison corpus.

Previously duplicated priorities, procedure and host mechanics now route to
distinct authorities. Tool availability improved within the existing harness;
new Python checks use the standard library and installed media tools.
Expensive native/media/platform work is selected by the claim; ordinary prose
does not acquire a new mandatory platform run. Existing GitHub workflow triggers
still determine hosted jobs.

[Controlled report replay](journey-acceptance-2026-10-20/report-replay.json)
fed the same eight synthetic transcripts through the base and revised finish
functions: base classified 1/8 correctly, revised 8/8. Seven selected false-PASS
cases (missing control/step/screenshot, wrong scenario, canonical mismatch,
frozen preview, framework error) are rejected while the positive control passes.
This finite instrument corpus is not a general false-claim rate or product test.

Reports now make source, fixture, build, actions, observations and unavailable
claims inspectable. Overall execution-cost improvement is UNVERIFIED; model
usage/currency cost is unavailable. More validation has a cost, and no end-to-end
overhead percentage is claimed. Debug/software smoke timings cannot be compared
with physical release performance or the representative packaged workload.

## Remaining acceptance

Independent ordinary-user evaluation, actual OS picker behavior in these new
diagnostics, physical-device and GPU baselines and targets (the local profile
build now executes the journey under software GL, which is not such a baseline),
real GPU frame presentation, audible output/A/V sync and expanded media/platform
compatibility remain UNVERIFIED. The existing hosted packaged and Android SAF
lanes retain authority; this task does not replace them with synthetic scenarios.

The framework has demonstrated genuine failure capture and rejects the requested
negative controls. That scoped instrument PASS does not make the product journey
PASS, establish live model skill selection, or authorize release.

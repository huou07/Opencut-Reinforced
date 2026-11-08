# External Tools and Companion Programs

Last reviewed: 2026-11-09

Scope: how OR detects, explains, launches and records the external programs a
project can use, and what a result has to satisfy before it counts as part of a
project. Install policy and licence posture are in
[security-licensing.md](../reference/security-licensing.md); packaging is in
[platforms.md](../architecture/platforms.md).

## Principle

An external tool stays outside OR. OR calls its CLI, opens its companion window
or exchanges a file with it; it does not copy the tool's source, runtime or
renderer into the core. The value OR adds is the integration: one place that
answers what the tool is for, whether this machine can run it, what it costs in
space and licence, how to install it, how to remove it, and how its output
becomes an editable part of a project.

## Catalog

`or_core::external_tools` is the single catalog. It is data plus pure policy:
no process is spawned there, so the app, the CLI and an agent answer with the
same facts. Each entry carries:

- identity: `id`, `name`, `category`, `summary`, `benefit`;
- cost: `license`, `license_note`, `size_hint`, `runtime`;
- install: `homepage`, `install_page`, one `install_command` per platform with
  a note, and `update_hint`;
- invocation: `probe` (a version flag), optional `gui_program`, `outputs`;
- scope: `project_scoped`, `platforms`, `removable`, `remove_hint`.

The catalog is curated, not a filesystem scan. Opening Settings runs one short
version probe per supported tool; nothing else is touched.

## States

| State | Meaning |
|---|---|
| `ready` | The probe ran, exited zero and printed a version. OR may launch it. |
| `missing` | The probe could not start the program. |
| `failed` | The program started but did not answer a version. |
| `unsupported` | The catalog knows it, but not on this platform. |
| `project_scoped` | It installs into a project; a machine-wide probe cannot answer. |

A launch requires `ready`. A broken install reports why instead of flashing a
window and dying.

## Surfaces

- **Settings → Resources → Companion tools** lists every entry with its state,
  version, licence, size, runtime, outputs, the exact install command, the
  official page and the update/removal steps. Expanding a row is read-only;
  nothing downloads or installs from the screen. `Check all` re-probes.
- **`or tools [list|check <id>|launch <id>] [--json]`** prints the same states.
  `or tools check <id>` exits non-zero for an unknown id, not for a missing
  tool.
- **`or_tools`** is the only crate that spawns a tool. Probes are bounded
  (`PROBE_TIMEOUT`, 4 s); a probe that overruns is killed and reported.

## What counts as a result

A result is usable when it can be brought into the project without pretending
it is more than it is:

- video, image or audio → an ordinary media item, inserted as a clip;
- subtitle → an ordinary caption import;
- MotionScene → the native scene path;
- an image sequence or a scene that only a companion can render → the rendered
  media plus the source project, manifest and the command that produced it.

A flat render is never described as an editable composition. The source stays
beside the output so it can be reopened in the companion and re-rendered; the
project keeps the reference and the provenance.

## Boundaries

- No automatic download, install, media upload or permission grant. The screen
  shows the command and the official page; the person runs it.
- A companion that lives outside a Flatpak sandbox is named and documented; OR
  does not assume a host tool can be executed inside the sandbox.
- Project-scoped tools are checked in the project that contains them, never
  globally.

## Gaps

- The provenance record for a rendered result and its "update from source"
  action are not implemented yet.
- Lottie/dotLottie and glTF have no importer; both need a renderer before their
  output can enter a timeline, and that decision is recorded in
  [technology-review.md](../reference/technology-review.md).
- There is no per-tool job log or cancellation yet; a launch reports its
  outcome and nothing more.

# Agent-Facing UI Harness

Last reviewed: 2026-10-26

Scope: how to drive the real application by semantics instead of coordinates, and
how to read the result. Prefer this over screenshots, MCP or vision.

## Entry point

`python3 scripts/or_ui.py` drives the real app by widget key, semantics label
(exact or regex), visible text, role or dialog-scoped control, and writes a
step-by-step transcript with action, selector, result, elapsed ms, screenshot and
semantics snapshot.

```sh
python3 scripts/or_ui.py screens                       # inventory screens
python3 scripts/or_ui.py tree --name editor            # semantics tree + dump
python3 scripts/or_ui.py shot --name editor            # screenshot at viewport
python3 scripts/or_ui.py act --label "Import Media" --role button
python3 scripts/or_ui.py run scripts/ui_scenarios/editor-journey.json
python3 scripts/or_ui.py --viewport 390x844 run scripts/ui_scenarios/compact-editor-journey.json
```

On this Mac the default target is `macos`: it builds and launches the real
desktop app through the Flutter integration test and reads artifacts from the
host filesystem. A 1440x900 window launches in about twenty seconds.

`python3 scripts/or_tree_outline.py <tree-*.json>` prints one line per node with
its global rect and actions — the fast way to read a screen.

## Scenario format

A step is one JSON object. Notable ops: `tap`, `secondary_tap` (opens clip
context menus), `drag` (selector + `dx`/`dy` + optional `steps`), `resize`
(`width`, `height`), and `fixtures` (including `copy:<path>`, which imports a real
file through the real picker).

Right-click a clip by its own label rather than a key regex with
`allow_multiple`: the first clip in the lane is rarely the one under test.

## Artifacts

`apps/or_app/build/ui-harness/<scenario>/` holds `scenario.json`, `run.log`,
`report.json`, and `artifacts/` with the transcript, screenshots and semantics
JSON. These are gitignored runtime state, not evidence records.

## Exit codes

`2` bad invocation/scenario, `3` BLOCKED (no guest, no Xvfb, another lane holding
the lock — the message names the settling command), `1` a step failed.

## Related

- [local-macos.md](local-macos.md)
- [verification.md](verification.md)
- [design/ux-invariants.md](../design/ux-invariants.md)

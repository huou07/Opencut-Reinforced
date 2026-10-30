# OR Focused Monochrome — Design Language

Last reviewed: 2026-10-26

Authority for application visual design. A creative work tool, not a marketing
surface: work, content, clarity and predictable interaction outrank decoration.

## Core principle

Black-first, workspace-first, content-first, distraction-free, compact but
ordered, modern, native-feeling, cross-platform, accessible, agent-readable.

## Hard rules

Application UI must not contain decorative marketing elements. Do not add hero
banners, scenic backgrounds, decorative photography, slogans, promotional quotes,
fake statistics, purposeless illustrations, neon glow, excessive gradients or
glassmorphism, gradient card borders, decorative animation, AI sparkles, nested
cards, or unrelated imagery.

The chrome is monochrome; user media may be full colour. The UI consists of logo,
navigation, tabs, panels, cards, lists, tables, controls, timelines, inspectors,
media previews, text and status indicators.

## Tokens

```
--or-bg:             #090909
--or-bg-raised:      #0F0F10
--or-surface:        #151516
--or-surface-hover:  #1C1C1E
--or-border:         #2A2A2D
--or-border-strong:  #3A3A3E
--or-text:           #F5F5F5
--or-text-secondary: #A6A6AB
--or-text-muted:     #717176
--or-primary:        #F5F5F5
--or-primary-text:   #090909
```

Selection cyan `#3FC7FF`; violet is reserved for explicit AI state; semantic
warning/error colours only when required. No new accent colours without review.

## Type, geometry, spacing

- Type scale is carried by `ThemeData`; no UI text style below 11 px.
- Minimum contrast 4.5:1 for text against background, raised and surface; focus
  indicators at least 3:1.
- 4/8 px spacing rhythm; restrained radii; borders instead of heavy shadows.
- Desktop trim handles keep a hit area of at least 24 px, 44 px in compact layout,
  while the painted bar stays thin.

## Shell and screens

- Stable shell: top bar, left rail, panel, centre workspace, inspector, timeline,
  status area. Do not redesign the shell per tab.
- Home is a project/workspace dashboard (new, open, recent, pinned tools,
  templates), never a landing page.
- The editor prioritises media, preview, timeline, inspector, commands and export.
- Timeline colour stays restrained: neutral clip surfaces, thumbnails, cyan for
  selection and playhead, neutral waveforms, no rainbow-by-media-type.

## Motion and copy

Motion is functional and short (<200 ms), reduced-motion aware; no decorative
animation. Copy is plain, specific and terse: name the action and its result; no
marketing voice, no exclamation marks, no invented statistics.

## Anti-slop and consistency

If an element does not help the user finish the task, remove it. Every screen
uses the same tokens, spacing, control shapes and terminology; one concept has
one name everywhere (project, clip, track, marker, export).

## Accessibility

Keyboard navigation, visible focus, usable touch targets, and status not
communicated by colour alone are requirements, not enhancements.

## Related

- [ux-invariants.md](ux-invariants.md) — enforced non-regression list
- [features/workspace-ui.md](../features/workspace-ui.md) — where controls live

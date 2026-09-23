
# Opencut Reinforced Design Language

## Status

This document is the source of truth for the Opencut Reinforced application design language.

Design name:

**OR Focused Monochrome**

## Core principle

Opencut Reinforced is a creative work tool, not a marketing surface.

The application UI must prioritize work, content, clarity, and predictable interaction over decoration.

The interface should feel like a precise modern creative workstation:

- black-first
- workspace-first
- content-first
- distraction-free
- compact but ordered
- modern
- native-feeling
- cross-platform
- accessible
- agent-readable

## Hard visual rules

Application UI must not contain decorative marketing elements.

Do not add:

- hero marketing banners
- scenic or landscape backgrounds
- decorative background photography
- inspirational slogans
- promotional quotes
- fake community statistics
- decorative illustrations with no functional purpose
- neon glow
- excessive gradients
- excessive glassmorphism
- gradient borders on ordinary cards
- animated visual decoration with no functional purpose
- decorative AI sparkles everywhere
- unnecessary nested cards
- unrelated imagery inserted only to make a screen look impressive

The application should primarily consist of:

- logo
- navigation
- tabs
- panels
- cards
- lists
- tables
- controls
- timelines
- inspectors
- media previews
- text
- status indicators

Media belonging to the user's project may of course contain full color.

The application chrome itself remains predominantly monochrome.

## Brand

Inside the application use the monochrome OR mark by default.

Preferred application treatment:

- white OR logo
- white/neutral wordmark
- no glow
- no drop-shadow decoration
- no gradient decoration

A colored brand mark may later be used outside the workspace for website, installer, app icon, or other branding contexts, but it must not drive the application UI.

## Palette

Base colors:

    --or-bg:             #090909
    --or-bg-raised:      #0F0F10
    --or-surface:        #151516
    --or-surface-hover:  #1C1C1E
    --or-border:         #2A2A2D
    --or-border-strong:  #3A3A3E

    --or-text:           #F5F5F5
    --or-text-secondary: #A6A6AB
    --or-text-muted:     #717176

Primary action:

    --or-primary:        #F5F5F5
    --or-primary-text:   #090909

Semantic accents:

    --or-selection:      #3FC7FF
    --or-ai:             #9B7CFF
    --or-success:        #47B881
    --or-warning:        #D8A542
    --or-danger:         #E05A5A

Accent colors are semantic, not decorative.

Use them sparingly.

Examples:

- cyan: selection, playhead, active editing state
- violet: AI-specific state or generated-content marker
- green: success/connected
- amber: warning/missing media
- red: destructive/error

Do not turn the interface into a cyan/violet theme.

## Media and color

Most color should come from the user's actual content:

- video preview
- image preview
- media thumbnails
- template previews
- generated media

Application panels, controls, navigation, and workspace chrome remain neutral.

Do not artificially desaturate user media as part of the UI design.

## Typography

Use a clean sans-serif UI typeface.

Preferred candidates:

- Geist
- Inter

Use a monospace font such as JetBrains Mono for CLI/code/terminal surfaces.

Suggested hierarchy:

- Display: 36–48 px, semibold
- Page title: 28–32 px, semibold
- Section title: 18–20 px, semibold
- Control: 14–15 px, medium
- Body: 14–15 px, regular
- Metadata: 12–13 px
- Tiny/status: 11–12 px

Avoid:

- decorative handwritten fonts
- excessive uppercase text
- oversized marketing headlines
- decorative typography inside the working UI

## Geometry

Use restrained rounded geometry.

Suggested radii:

- small controls: 6 px
- buttons and inputs: 8 px
- cards: 10 px
- large panels: 12 px
- modal/mobile sheet: 14 px

Avoid extremely rounded pill-heavy UI unless the component genuinely requires it.

## Spacing

Use a 4 px base grid.

Preferred spacing scale:

4, 8, 12, 16, 20, 24, 32, 40, 48

Dense editor surfaces may use 8–12 px spacing.

General panels should normally use 16–20 px padding.

## Borders and elevation

Prefer:

- subtle surface differences
- thin 1 px borders
- clear hierarchy

over heavy shadows.

Use strong shadows only where elevation is functionally important:

- modal
- dropdown
- floating toolbar
- mobile bottom sheet

## Icons

Use one coherent outline icon family.

Preferred baseline style:

- simple outline
- consistent stroke
- rounded joins
- standard size grid

Do not mix unrelated icon styles such as filled, 3D, emoji, neon, duotone, and outline icons.

## Buttons

Primary:

- light/white surface
- dark text
- reserved for the main action

Secondary:

- dark surface
- thin border
- light text

Ghost:

- transparent/subtle
- used for tertiary actions

Destructive:

- red semantic treatment only for genuinely destructive operations

AI buttons must not become special glowing gradient buttons.

A small AI icon or semantic marker is sufficient.

## Desktop application shell

The desktop application should keep a stable structural shell.

Typical editor layout:

    global top bar
    left tool/navigation rail
    media/tool panel
    central preview/workspace
    right contextual inspector
    timeline/task area where appropriate
    bottom status where useful

Do not redesign the entire shell for each tab.

Home may use a wider workspace but still follows the same system.

## Home

Home is a project/workspace dashboard, not a landing page.

Recommended content:

- new project
- open project
- recent projects
- pinned/quick tools
- templates
- recent AI jobs
- model/download state where useful

Do not place decorative hero images or marketing copy on Home.

## Video editor

The video editor prioritizes:

- media
- preview
- timeline
- inspector
- editing commands
- export

No decorative content should compete with the timeline or preview.

## Timeline

Timeline colors must remain restrained.

Use:

- neutral clip surfaces
- thumbnails where useful
- cyan for selected clip/playhead
- neutral waveform colors
- violet only for explicit AI state
- semantic warning/error colors when required

Do not create a rainbow timeline where every media type has an unrelated saturated color.

## AI Studio

AI Studio uses the same design system as the rest of the application.

Typical structure:

- task tabs
- prompt
- model selector
- media/reference inputs
- generation parameters
- generate action
- queue/history/results

AI is a capability, not a visual theme.

No neon AI dashboard aesthetic.

## Subtitles and translation

Favor a functional editing workspace:

- preview
- transcript/subtitle rows
- source/target language
- model/provider
- timing
- style controls
- apply/export actions

Tables and editable rows are preferred over decorative cards when the content is tabular.

## Model manager

Model management is utility UI.

Prioritize:

- model name
- task/type
- version
- size
- license
- hardware requirement
- installed/download status
- location
- update/remove controls

No promotional model cards.

## CLI and Agent Hub

Agent tooling should feel like a developer tool.

Use:

- neutral dark surfaces
- monospace terminal output
- restrained connection/status colors
- task queue
- AGENTS.md/context view
- clear agent capability/permission state

Do not rely on colorful vendor logos to organize the interface.

## Mobile

Mobile is not a scaled-down desktop layout.

Reuse the same entities, terminology, icons, tokens, and interaction concepts while adapting layout natively.

Typical mobile editor order:

    top bar
    preview
    transport controls
    timeline
    contextual editing toolbar
    persistent navigation where appropriate

Desktop inspectors typically become mobile sheets/panels.

## Motion

Motion is functional and restrained.

Typical durations:

- hover: 100–120 ms
- panel: 160–180 ms
- modal: 180–220 ms
- mobile sheet: around 220 ms

Avoid decorative looping motion, pulsing glow, or bouncing AI indicators.

## Copywriting

Application copy is short, literal, and task-oriented.

Preferred:

- New Project
- Import Media
- Auto Captions
- Translate Subtitles
- Generate Video
- Install Model
- Connect Agent
- Export

Avoid product-marketing language inside the workspace.

## Anti-slop rule

When choosing between:

1. adding another decorative visual element, or
2. removing it while keeping the interface understandable,

prefer removal.

Every visible UI element should answer at least one of:

- What can I do?
- What is selected?
- What is happening?
- What changed?
- What is the result?
- What requires my attention?

If it answers none of these, it probably does not belong in the application.

## Accessibility

Minimalism must never reduce accessibility.

Maintain:

- sufficient contrast
- visible keyboard focus
- usable touch targets
- semantic labels
- keyboard navigation on desktop
- screen-reader-friendly controls
- status information that does not depend only on color

## Consistency rule

Desktop and mobile must look like members of one product family.

Do not invent a new visual language per screen.

When a new component is needed, first check whether an existing OR component can solve the problem.


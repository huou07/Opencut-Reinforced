# Templates

Last reviewed: 2026-11-08

Three kinds of template, one Library surface. They are deliberately distinct:
a project format decides a canvas, an edit style decides how a cut is paced, a
creative direction decides how a video should feel and be built. Mixing them was
the old problem — a "template" meant only a starter project, so the library could
not help someone who had footage and wanted a cut.

## Where they live

`Library → Templates` renders three sections in this order:

1. **Project formats** — Landscape 1080p, Vertical short, Square, Tutorial.
2. **Edit styles** — Cinematic documentary, Fast-paced educational, Personal vlog,
   Product showcase, Interview storytelling.
3. **Creative directions** — Cinematic documentary, Modern tech explainer,
   Energetic social, Minimalist storytelling, Warm personal narrative.

A fourth section, **My directions**, appears once a person keeps a copy.

The Library is a top-level destination (`Home → Library`, `or` workspace sidebar,
or ⌘K → `Go to a screen → Library`). The editor hides the sidebar, so from inside
a project the palette is the route; the style and direction cards stay usable
because they read the open project.

## Project formats

A format writes a canvas, a frame rate and starter titles, and nothing else.
Copy on the card says so: "No media is added and nothing is pre-cut." Creating
one is unchanged from the earlier implementation (`ProjectTemplate` in
`apps/or_app/lib/library/project_templates.dart`, applied through
`_createProjectFromTemplate`).

## Edit styles

An edit style is a recipe for pacing: shot length, hook length, how many shots,
whether captions and a title belong, how the material is ordered, and where the
sound sits. The five bundled styles are declared in
`apps/or_app/lib/library/edit_styles.dart`; each describes a way of cutting, not a
subject.

### Applying one

1. Open a project that has media. Without one every card says "Open a project
   first" and explains why.
2. Press **Build the cut** on a style card.
3. A review dialog lists every shot with its own reason line, the plan summary,
   what the style had to compromise on, and the title clip it will add. Nothing
   has been written yet.
4. **Build the cut** applies it.

The plan is computed by the pure `planEditStyleCut({style, sources, frameRate})`:
non-video sources and clips shorter than a quarter second are skipped, material is
ordered by the style, the first shot uses the hook length and the rest the shot
length, and every shot duration is a whole number of frames at the sequence rate.
The review dialog renders that same plan object, so what a person approves is what
runs.

### What it writes

Ordinary timeline commands, in this order:

- a video track when the project has none (`addTimelineTrack`),
- one `insertTimelineClip` per planned shot, appended after the existing content
  end, with frame-aligned `timelineStart`, `sourceStart` and `duration`,
- one three-second `insertTimelineTextClip` on a text track when the style asks
  for a title. The title text is the project name, or the style's own line when
  the project has no name.

Nothing writes to the media library and no source file is touched. Each insert is
its own history entry, so Undo removes the cut one clip at a time; the review
dialog states this instead of pretending it is a single undo.

### When there is no AI provider

The plan is deterministic and local: it uses the media's probed durations and the
style's declared pacing. No transcript, scene detection or model call is required,
so the feature works offline and never uploads media. A future assistant-assisted
plan can replace the planner behind the same review dialog; until then the styles
are honest about being a structural first cut, not an automatic edit decision
list derived from content.

## Creative directions

A creative direction is a brief, not a script. Each of the five bundled entries
(`apps/or_app/lib/library/creative_directions.dart`) answers seven questions in
prose: voice and narration, script shape, storytelling, graphics, captions,
editing, audio — plus a summary and three kinds of video it fits. No entry names a
subject, a company or a story, so the same direction carries a product launch, a
travel film or a lesson.

**Read the brief** opens the direction as readable sections. **Copy brief** puts
`toBrief()` on the clipboard — a markdown brief in the order a video is decided —
for pasting into the assistant panel or an external agent. **Keep a copy** stores
it in the person's own library.

### Stored copies

`CreativeDirectionStore` (`apps/or_app/lib/library/creative_direction_store.dart`)
persists copies to `creative-directions.v1.json` in the application support
directory as `{version: 1, directions: [...]}`. Loading tolerates a missing file, a
hand-edited file and a filesystem error by returning an empty list rather than
failing the Library. Ids are made unique with `copiedDirectionId`, which appends
`-mine`, `-mine-2`, and so on. A stored copy loads with `isMine: true` and shows a
`Mine` badge.

## Tests

- `apps/or_app/test/edit_styles_test.dart` — planner rules, ordering per style,
  frame alignment, the shot cap and its note, skipped material, a zero frame rate,
  the title rule, and that every bundled brief answers all seven questions.
- `apps/or_app/test/edit_style_dialog_test.dart` — the review dialog's singular
  and plural copy, the note lines, both buttons, and the seven brief sections.
- `apps/or_app/test/creative_direction_store_test.dart` — round-trip, upsert,
  removal, a corrupt file and id collisions.
- `apps/or_app/test/library_screen_test.dart` — the three sections stay apart, a
  style builds only with a project open, and a direction opens as a brief.

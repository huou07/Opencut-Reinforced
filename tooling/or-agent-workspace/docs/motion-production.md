# Motion production

Scope: producing more than one scene — splitting a sequence into independent
streams, rendering them in parallel, and assembling the result. Single-scene
authoring is [motion-scenes.md](motion-scenes.md); subtitles are
[captions.md](captions.md); the full pipeline is
[templates/video-pipeline/README.md](../templates/video-pipeline/README.md).

## Split before rendering

- One scene is one beat: 4–8 seconds, one idea, one headline. A ten-beat video
  is ten scenes, not one long scene.
- Group beats into acts or streams. A stream is a directory of scenes that share
  palette, type scale and camera behaviour, so it can be reviewed on its own.
- Fix the canvas and frame rate once (`1920×1080`, `30/1` unless asked) and use
  them for every scene; a mixed set will not assemble cleanly.
- Write the beat sheet and the voiceover timing first. Scene duration should come
  from the measured narration, not from a guess.

## Render in parallel

Materialisation writes files and never touches the project revision, so it is the
safe parallel step. `motion publish` is a project mutation and is serial.

```sh
mkdir -p work/renders/act1
for scene in scenes/act1/*.ormotion.json; do
  name=$(basename "$scene" .ormotion.json)
  tools/or motion materialize --attach project.json --scene "$scene" \
    --output "$PWD/work/renders/act1/$name.mkv" --frame-rate 30/1 --json &
done
wait
```

- Use 3–4 concurrent renders on this machine. More workers mostly add encoder
  contention; the pipeline default is 3.
- One background job per stream when the streams are independent; `wait` for all
  of them before assembling.
- A failed scene does not block the others. Collect failures, fix the scene,
  render only that one.
- Do not re-render a file that already exists and is complete; check its size or
  the pipeline's skip logic first.

## Read the composition, not a screenshot

`tools/or motion layout --file scenes/act1/03-proof.ormotion.json --at 1/2 --json`
returns every node's evaluated box at one exact time: position, size, rotation,
opacity, text and animated flag. Use it to check overflow, overlap and empty
frames. `motion inspect` summarises nodes and animations; `motion evaluate` shows
the resolved paint instructions. A PNG render is the last resort, not the first.

## Assemble sequentially

After all renders exist, import and place them one at a time. Each insert is a
project mutation and advances the revision:

```sh
for file in work/renders/act1/*.mkv; do
  tools/or media add --attach project.json --source "$PWD/$file" --json
done
```

Then insert the clips in order, or concatenate the files with the pipeline's
assembly step and import the single result. Either way, verify the track after the
last insert and save once.

## Failure handling

- A render timeout is not proof of failure. Check the output file and the
  timeline before retrying.
- Never start a second `publish` for the same scene while one is running; the
  first may still insert a clip.
- A scene that fails validation is fixed and re-validated, not rendered again.
- Report rendered count, failed count, elapsed time and output directory.

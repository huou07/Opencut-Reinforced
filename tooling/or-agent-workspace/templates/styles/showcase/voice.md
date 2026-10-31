# Voice

## Settings

- Rate 180 words per minute; 250 ms between lines.
- Voice order: the script's `voice`, then the app's saved default, then the
  first installed voice for the language. Never hardcode a voice name.
- One voice for the whole film; do not switch mid-piece.
- Mono 44.1 kHz WAV; the workspace voiceover tool writes this already.

## Delivery

- Short sentences carry emphasis; do not use capitals or punctuation tricks.
- Put the stressed word first or last in the sentence.
- Read the product name the same way every time.
- Leave the pause after a claim; do not fill it with a breath word.

## Before applying

Run the plan first and listen:

```sh
node tools/voiceover.mjs --script work/scripts/demo.json --plan --json
```

Fix wording until the read is clean, then run with `--apply`. A voiceover that
sounds rushed is a script problem, not a rate problem; shorten the line before
raising the speed.

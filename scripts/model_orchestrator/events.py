"""Shared parser for `opencode run --format json` NDJSON event output.

Real format (installed OpenCode 1.18.31, verified by live probe):
  {"type":"step_start",...,"part":{"type":"step-start",...}}
  {"type":"text",...,"part":{"type":"text","text":"<payload>",...}}
  {"type":"step_finish",...,"part":{"type":"step-finish",...}}

The final assistant textual payload is the concatenation of every
part with type "text", in stream order. Used by the reviewer and the
Jev advisory adapter. Never duplicated ad-hoc.
"""
from __future__ import annotations

import json


def extract_assistant_text(output: str) -> str:
    """Return concatenated text parts. Raises ValueError when unusable."""
    texts: list[str] = []
    seen_event = False
    for line in output.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except ValueError as error:
            raise ValueError(f"malformed OpenCode event line: {line[:120]!r}") from error
        if not isinstance(event, dict):
            raise ValueError("OpenCode event is not an object")
        seen_event = True
        part = event.get("part")
        if isinstance(part, dict) and part.get("type") == "text":
            text = part.get("text", "")
            if isinstance(text, str):
                texts.append(text)
    if not seen_event:
        raise ValueError("empty OpenCode output")
    return "".join(texts)


def extract_label(output: str, allowed: tuple[str, ...]) -> str:
    """Exactly one allowed router label from assistant text, else ValueError."""
    label = extract_assistant_text(output).strip().strip('"')
    if label not in allowed:
        raise ValueError(f"invalid label: {label!r}")
    return label


def extract_json_object(output: str) -> dict:
    """Assistant text must be a single JSON object, else ValueError."""
    text = extract_assistant_text(output).strip()
    try:
        parsed = json.loads(text)
    except ValueError as error:
        raise ValueError(f"assistant payload is not JSON: {text[:120]!r}") from error
    if not isinstance(parsed, dict):
        raise ValueError("assistant payload is not an object")
    return parsed

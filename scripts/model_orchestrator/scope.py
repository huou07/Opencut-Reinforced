"""Deterministic path-scope matching for task packets.

Pattern semantics (documented, no ambiguity):
  exact:     `crates/or_media/src/decoder.rs` matches only that POSIX path.
  recursive: `crates/or_media/**` matches that subtree (prefix + "/").
  broad:     `**` matches every path (explicit opt-in for wide scope).

All git paths are normalized to POSIX form (backslashes -> "/",
leading "./" stripped). An empty allowed_paths list means NO files are in
scope — never "all files". FORBIDDEN always wins over ALLOWED.
"""
from __future__ import annotations


def normalize(path: str) -> str:
    normalized = path.replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def pattern_matches(pattern: str, path: str) -> bool:
    pattern = normalize(pattern)
    path = normalize(path)
    if pattern == "**":
        return True
    if pattern.endswith("/**"):
        prefix = pattern[: -len("/**")]
        return path == prefix or path.startswith(prefix + "/")
    return path == pattern


def scope_violations(
    changed: list[str],
    allowed: list[str],
    forbidden: list[str],
) -> dict:
    """Return {'out_of_scope': [...], 'forbidden': [...]}. Empty means in scope."""
    out_of_scope = []
    forbidden_hits = []
    for raw in changed:
        path = normalize(raw)
        if any(pattern_matches(pattern, path) for pattern in forbidden):
            forbidden_hits.append(path)
            continue
        if not any(pattern_matches(pattern, path) for pattern in allowed):
            out_of_scope.append(path)
    return {"out_of_scope": sorted(out_of_scope), "forbidden": sorted(forbidden_hits)}

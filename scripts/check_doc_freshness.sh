#!/usr/bin/env bash
# Documentation freshness check.
#
# A document that states a current status is a claim. This checks that the
# documents which make current claims carry a review date and that it is
# recent, so a stale document is noticed while it is still cheap to fix.
#
#   bash scripts/check_doc_freshness.sh [max_days]
#
# Exit 0 when every in-scope document is reviewed within the window.
set -uo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir/.." rev-parse --show-toplevel)" || exit 1
cd "$repo_root" || exit 1

max_days="${1:-120}"
failed=0

# Documents that assert current product, architecture or process truth. Archival
# material is deliberately excluded: history is not supposed to become fresh.
# A document that claims a current status must be here, or its staleness is
# invisible to every gate.
in_scope=(
  README.md
  AGENTS.md
  docs/design/design-language.md
  docs/design/ux-invariants.md
  docs/architecture/system-overview.md
  docs/architecture/control-plane.md
  docs/architecture/realtime-plane.md
  docs/architecture/media-pipeline.md
  docs/architecture/project-format.md
  docs/architecture/platforms.md
  docs/architecture/security-boundaries.md
  docs/features/project-lifecycle.md
  docs/features/editing.md
  docs/features/media-library.md
  docs/features/preview-playback.md
  docs/features/text-captions.md
  docs/features/effects-audio.md
  docs/features/export.md
  docs/features/motionscene.md
  docs/features/motion-assets.md
  docs/features/workspace-ui.md
  docs/features/assistant.md
  docs/features/agent-workspace.md
  docs/operations/workflow.md
  docs/operations/verification.md
  docs/operations/local-macos.md
  docs/operations/ui-harness.md
  docs/operations/git.md
  docs/operations/permissions.md
  docs/operations/docs-maintenance.md
  docs/operations/skills.md
  docs/reference/product.md
  docs/reference/roadmap.md
  docs/reference/quality.md
  docs/reference/testing.md
  docs/reference/tooling.md
  docs/reference/security-licensing.md
  docs/reference/release.md
  docs/reference/developer-preview-install.md
  docs/evidence/claims/README.md
)

date_to_epoch() {
  local reviewed="$1"
  local epoch

  # macOS/BSD date uses -j -f; GNU date uses -d.
  if epoch="$(date -u -j -f "%Y-%m-%d" "$reviewed" +%s 2>/dev/null)"; then
    printf '%s\n' "$epoch"
    return 0
  fi
  if epoch="$(date -u -d "$reviewed" +%s 2>/dev/null)"; then
    printf '%s\n' "$epoch"
    return 0
  fi
  return 1
}

today_epoch="$(date -u +%s)"

for path in "${in_scope[@]}"; do
  if [[ ! -f "$path" ]]; then
    printf 'FAIL: in-scope document is missing: %s\n' "$path" >&2
    failed=1
    continue
  fi
  reviewed="$(grep -m1 -iE '^(last reviewed|reviewed|last-reviewed):' "$path" \
    | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' | head -n1)"
  if [[ -z "$reviewed" ]]; then
    printf 'FAIL: %s makes current claims but has no review date\n' "$path" >&2
    printf '      add a line: "Last reviewed: YYYY-MM-DD"\n' >&2
    failed=1
    continue
  fi
  reviewed_epoch="$(date_to_epoch "$reviewed" 2>/dev/null)" || reviewed_epoch=""
  if [[ -z "$reviewed_epoch" ]]; then
    printf 'FAIL: %s has an unparseable review date: %s\n' "$path" "$reviewed" >&2
    failed=1
    continue
  fi
  age_days=$(( (today_epoch - reviewed_epoch) / 86400 ))
  if (( age_days > max_days )); then
    printf 'FAIL: %s was last reviewed %s (%d days ago, limit %d)\n' \
      "$path" "$reviewed" "$age_days" "$max_days" >&2
    failed=1
  else
    printf '  ok   %s (reviewed %s, %d days ago)\n' "$path" "$reviewed" "$age_days"
  fi
done

if (( failed )); then
  printf 'Doc freshness: FAIL\n' >&2
  exit 1
fi
printf 'Doc freshness: PASS (window %d days)\n' "$max_days"

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
repo_root="$(git -C "$script_dir/.." rev-parse --show-toplevel)"
cd "$repo_root"

max_days="${1:-120}"
failed=0

# Documents that assert current product, architecture or process truth. Archival
# material is deliberately excluded: history is not supposed to become fresh.
in_scope=(
  AGENTS.md
  docs/AGENT_POLICY.md
  docs/ARCHITECTURE.md
  docs/DEVELOPMENT_WORKFLOW.md
  docs/INDEX.md
  docs/PRODUCT.md
  docs/PRODUCT_ROADMAP.md
  docs/SECURITY_LICENSING.md
  docs/TECHNICAL_PLAN.md
  docs/TESTING.md
  docs/skills.md
  docs/UX_ACCEPTANCE.md
)

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
  reviewed_epoch="$(date -u -d "$reviewed" +%s 2>/dev/null)" || reviewed_epoch=""
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

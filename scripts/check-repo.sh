#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir/.." rev-parse --show-toplevel)"
cd "$repo_root"

failed=0
tracked_count=0
max_bytes=$((25 * 1024 * 1024))

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  failed=1
}

required_files=(
  README.md
  LICENSE
  AGENTS.md
  DESIGN.md
  CONTRIBUTING.md
  SECURITY.md
  docs/INDEX.md
  docs/PRODUCT.md
  docs/PROJECT_FORMAT.md
  scripts/stage_codec_licences.sh
  docs/ARCHITECTURE.md
  docs/TECHNICAL_PLAN.md
  docs/DEVELOPMENT_WORKFLOW.md
  docs/ROADMAP.md
  docs/TESTING.md
  docs/SECURITY_LICENSING.md
  docs/RELEASE.md
  docs/TOOLING.md
  docs/UX_ACCEPTANCE.md
  docs/QUALITY.md
  docs/AGENT-AUTHORITY.md
  docs/PAUSE_MIGRATION_HANDOFF.md
  docs/evidence/PRODUCT_ROADMAP_HISTORY.md
  docs/prompts/CONTINUE-OR.md
  docs/prompts/FINISH-OR.md
  docs/prompts/NEXT-OR.md
  docs/skills.md
  docs/evidence/claims/README.md
  docs/adr/README.md
  docs/adr/0001-control-and-realtime-planes.md
  docs/adr/0002-wgpu-render-spine-and-native-interop.md
  docs/adr/0003-frame-memory-domain-and-ownership.md
  docs/adr/0004-runtime-capabilities-and-provider-selection.md
  docs/adr/0005-ai-provider-boundary.md
  docs/adr/0006-autonomous-agent-execution-contract.md
  docs/adr/0007-declarative-motion-scenes-and-procedural-isolation.md
  docs/adr/0008-product-acceptance-and-execution-quality.md
)

for path in "${required_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    fail "required file is missing: $path"
  fi
done

if ! grep -Fq 'MIT License' LICENSE 2>/dev/null; then
  fail 'LICENSE does not contain the expected MIT License text'
fi

check_whitespace() {
  local revision="$1"
  local label="$2"
  local path="$3"
  local output
  local status

  if output="$(git diff --no-index --check -- /dev/null <(git show "${revision}:${path}") 2>&1)"; then
    status=0
  else
    status=$?
  fi

  if [[ -n "$output" ]]; then
    printf 'Whitespace issue in %s content for %s:\n%s\n' "$label" "$path" "$output" >&2
    failed=1
  elif (( status > 1 )); then
    fail "could not check whitespace in $label content for $path (git exit $status)"
  fi
}

while IFS= read -r -d '' path; do
  tracked_count=$((tracked_count + 1))

  if git cat-file -e ":$path" 2>/dev/null; then
    check_whitespace '' 'index' "$path"
  else
    fail "could not read the index version of tracked file: $path"
  fi

  if [[ -e "$path" || -L "$path" ]]; then
    if output="$(git diff --no-index --check -- /dev/null "$path" 2>&1)"; then
      status=0
    else
      status=$?
    fi
    if [[ -n "$output" ]]; then
      printf 'Whitespace issue in working-tree content for %s:\n%s\n' "$path" "$output" >&2
      failed=1
    elif (( status > 1 )); then
      fail "could not check working-tree whitespace for $path (git exit $status)"
    fi
  else
    fail "tracked file is missing from the working tree: $path"
  fi

  case "/$path/" in
    */.codegraph/*|*/.DS_Store/*|*/node_modules/*|*/target/*|*/build/*|*/dist/*|*/.dart_tool/*|*/.gradle/*|*/coverage/*)
      fail "generated or local path is tracked: $path"
      ;;
  esac

  case "$path" in
    .env.example)
      # An explicitly named root template is allowed; do not put secrets in it.
      ;;
    .env|.env.*|*/.env|*/.env.*)
      fail "environment file is tracked: $path"
      ;;
  esac

  basename="${path##*/}"
  case "$basename" in
    id_rsa|id_ed25519|*.p12|*.P12|*.pfx|*.PFX)
      fail "private-key or certificate file type is tracked: $path"
      ;;
  esac

  if size="$(git cat-file -s ":$path" 2>/dev/null)"; then
    if (( size > max_bytes )); then
      printf 'FAIL: tracked file exceeds 25 MiB: %s (%s bytes)\n' "$path" "$size" >&2
      failed=1
    fi
  else
    fail "could not determine tracked file size: $path"
  fi
done < <(git ls-files -z)



# Documentation gates. These replaced the archived execution-plan validator,
# which only checked a paused ledger. These check the surfaces that are still
# live: broken routing links, stale current-claim documents, and claim records.
if ! python3 scripts/check_doc_links.py; then
  failed=1
fi

if ! bash scripts/check_doc_freshness.sh; then
  failed=1
fi

if ! python3 scripts/check_claims.py; then
  failed=1
fi

# The claim validator now resolves each `implementation_sha` in Git, so its
# reject-a-fabricated-receipt behaviour needs its own regression guard. It also
# needs full history, which is why repo-hygiene.yml checks out with
# `fetch-depth: 0` instead of the default shallow clone.
if ! python3 scripts/test_check_claims.py; then
  fail "the claim-evidence validator regression checks failed"
fi

# `or_core` owns canonical project state, so GPU, media, audio, text-shaping,
# scripting and AI-runtime layers must not be reachable from it. This replaces
# the boundary check that only ran as a side effect of the paused execution
# plan validator.
if ! python3 scripts/check_or_core_deps.py; then
  failed=1
fi

# The bounded Linux preflight depends on cache preparation that no compiler
# exercises: staging project-local Cargo/pub caches, probing write access, and
# mirroring a read-only Flutter SDK. A silent break there only shows up as a
# confusing permission error minutes into a native run, so its hermetic unit
# tests run with the other gates.
if ! python3 scripts/test_linux_ui_preflight.py; then
  fail "the bounded Linux preflight preparation checks failed"
fi

# Two hermetic Android report/runner suites existed without a runner, which made
# them dead weight: nothing could fail if they rotted. They need no device or
# network. `test_verify_android_saf_recovery.py` imports the verifier as
# `scripts.verify_android_saf_recovery`, so it runs as a module from the root.
if ! python3 -m scripts.test_verify_android_saf_recovery; then
  fail "the Android SAF recovery report checks failed"
fi

if ! python3 scripts/test_android_preview_runner.py; then
  fail "the Android preview runner control-flow checks failed"
fi

# The aggregate release gate is the only check meant to block a merge, so its
# logic must reject `skipped`/`neutral`/`cancelled` as well as `failure`. If this
# logic rots, the gate silently becomes a rubber stamp.
if ! python3 scripts/test_release_gate.py; then
  fail "the aggregate release-gate logic checks failed"
fi

# Lint the repository's own shell gates. A gate that fails to parse is worse
# than no gate: it looks like a passing check.
if command -v shellcheck >/dev/null 2>&1; then
  # Every shell script the repository ships, not a hand-picked list. A gate that
  # lints only the files someone remembered to add is how an unlinted script
  # reaches CI and fails there instead.
  while IFS= read -r shell_script; do
    if ! shellcheck -S warning "$shell_script"; then
      fail "shellcheck reported problems in $shell_script"
    fi
  done < <(find scripts -name '*.sh' -not -path 'scripts/archive/*' | sort)
else
  printf 'WARN: shellcheck not available; shell gates were not linted\n' >&2
fi

if (( failed != 0 )); then
  printf 'Repository hygiene checks failed.\n' >&2
  exit 1
fi

printf 'Repository hygiene checks passed (%s tracked files; whitespace, paths, size, license, key-file, doc-link, doc-freshness, claim-source-identity, preflight-preparation, Android-report and release-gate-logic checks).\n' "$tracked_count"

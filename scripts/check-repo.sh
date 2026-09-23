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
  docs/ARCHITECTURE.md
  docs/TOOLING.md
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

if (( failed != 0 )); then
  printf 'Repository hygiene checks failed.\n' >&2
  exit 1
fi

printf 'Repository hygiene checks passed (%s tracked files; whitespace, paths, size, license, and key-file checks).\n' "$tracked_count"

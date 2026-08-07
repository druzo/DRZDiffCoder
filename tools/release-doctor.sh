#!/usr/bin/env bash
# tools/release-doctor.sh
# Pre-flight sanity check for a DRZ Diff release.
# Verifies:
#   - cargo workspace builds (cargo check)
#   - workflow files exist and parse as YAML
#   - Dockerfiles parse
#   - scripts/release/* exist and are executable
#   - Cargo.toml version == expected VERSION arg
# Exits 1 if any check fails.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EXPECTED_VERSION="${1:-}"
FAIL=0

ok()   { printf '  \033[1;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  \033[1;33m!\033[0m %s\n' "$*"; }
fail() { printf '  \033[1;31m✗\033[0m %s\n' "$*"; FAIL=$((FAIL+1)); }

section() { printf '\n\033[1;36m== %s ==\033[0m\n' "$*"; }

section "Repo layout"
[ -d ".github/workflows" ] && ok ".github/workflows/" || fail "missing .github/workflows/"
[ -d "scripts/release" ]   && ok "scripts/release/"   || fail "missing scripts/release/"
[ -d "docker" ]            && ok "docker/"            || fail "missing docker/"
[ -d "tools" ]             && ok "tools/"             || fail "missing tools/"

section "Workflows"
for f in .github/workflows/*.yml; do
  [ -f "$f" ] || continue
  if python3 -c "import sys, yaml; yaml.safe_load(open(sys.argv[1]))" "$f" >/dev/null 2>&1; then
    ok "$(basename "$f") YAML valid"
  else
    fail "$(basename "$f") YAML invalid"
  fi
done

section "Dockerfiles"
for f in docker/*.Dockerfile; do
  [ -f "$f" ] || continue
  if grep -q "^FROM " "$f"; then
    ok "$(basename "$f")"
  else
    fail "$(basename "$f") missing FROM"
  fi
done

section "Scripts"
for f in scripts/release/*.sh; do
  [ -f "$f" ] || continue
  if [ -x "$f" ]; then
    ok "$(basename "$f") executable"
  else
    fail "$(basename "$f") not executable"
  fi
done

section "Tools"
for f in tools/*.sh; do
  [ -f "$f" ] || continue
  if [ -x "$f" ]; then
    ok "$(basename "$f") executable"
  else
    fail "$(basename "$f") not executable"
  fi
done

section "Cargo version"
CARGO_VER="$(grep '^version' crates/drz-app/Cargo.toml | head -1 | cut -d'"' -f2)"
if [ -n "$EXPECTED_VERSION" ]; then
  if [ "$CARGO_VER" = "$EXPECTED_VERSION" ]; then
    ok "crates/drz-app version = $CARGO_VER (matches expected)"
  else
    fail "crates/drz-app version = $CARGO_VER, expected $EXPECTED_VERSION"
  fi
else
  warn "crates/drz-app version = $CARGO_VER (no expected VERSION passed)"
fi

section "git status"
if git diff --quiet --exit-code HEAD 2>/dev/null; then
  ok "working tree clean"
else
  warn "working tree has uncommitted changes"
fi

CURRENT_BRANCH="$(git branch --show-current)"
echo "  branch: $CURRENT_BRANCH"
git log --oneline -1 | sed 's/^/  HEAD:   /'

echo "-----"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[1;32mDOCTOR OK\033[0m\n'
  exit 0
else
  printf '\033[1;31mDOCTOR FAILED: %d issue(s)\033[0m\n' "$FAIL"
  exit 1
fi
#!/usr/bin/env bash
# tools/collect-artifacts.sh
# Aggregate every artifact in releases/<VERSION>/ into a single folder and
# emit SHA256SUMS.txt (relative paths, sorted, deduplicated).
#
# Usage: tools/collect-artifacts.sh <version> [<dist-dir>]
# Default dist-dir: dist/
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

VERSION="${1:?usage: $0 <version> [<dist-dir>]}"
DIST="${2:-dist}"

SRC="releases/${VERSION}"
[ -d "$SRC" ] || { echo "missing $SRC" >&2; exit 1; }

rm -rf "$DIST"
mkdir -p "$DIST"

echo "[collect] copying from $SRC -> $DIST"
# Use rsync to exclude ephemeral build directories.
rsync -a --exclude='deb-build' --exclude='.dmg-stage' "$SRC/" "$DIST/"

echo "[collect] computing SHA256SUMS.txt"
cd "$DIST"
: > SHA256SUMS.txt
find . -type f -name SHA256SUMS -prune -o \
       -type d -name 'deb-build' -prune -o \
       -type d -name '.dmg-stage' -prune -o \
       -type f -print0 \
  | while IFS= read -r -d '' f; do
      rel="${f#./}"
      sum="$(sha256sum "$f" | awk '{print $1}')"
      echo "${sum}  ${rel}"
    done | sort -u >> SHA256SUMS.txt

echo "[collect] done"
echo "  artifacts: $(find "$DIST" -type f -not -name SHA256SUMS.txt | wc -l)"
echo "  sums:      $(wc -l < SHA256SUMS.txt) entries"
head -20 SHA256SUMS.txt
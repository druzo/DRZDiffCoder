#!/usr/bin/env bash
# scripts/release.sh
# Top-level release orchestrator. Reads VERSION, generates icons,
# invokes per-platform builders, stages artifacts under releases/$VERSION/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# 1. Resolve VERSION --------------------------------------------------------
VERSION="${VERSION:-}"
if [ -z "$VERSION" ]; then
  if git describe --tags --abbrev=0 >/dev/null 2>&1; then
    VERSION="$(git describe --tags --abbrev=0 | sed 's/^v//')"
  else
    VERSION="$(git rev-parse --short HEAD)"
  fi
fi
echo "[release] version = $VERSION"

# Cross-check Cargo.toml version (informational only)
CARGO_VER="$(grep '^version' crates/drz-app/Cargo.toml | head -1 | cut -d'"' -f2)"
if [ -n "$CARGO_VER" ] && [ "$CARGO_VER" != "$VERSION" ]; then
  echo "[release] WARN: crates/drz-app/Cargo.toml version = $CARGO_VER, release = $VERSION"
fi
export VERSION

# 2. Pre-flight -------------------------------------------------------------
echo "[release] prereq check"
PLATFORMS_LIST="${PLATFORMS:-windows linux darwin-x86_64 darwin-arm64}"
CHECK_ARGS=""
for p in $PLATFORMS_LIST; do
  CHECK_ARGS="$CHECK_ARGS $p"
done
"$(dirname "$0")/release/check.sh" $CHECK_ARGS

# 3. Icons ------------------------------------------------------------------
echo "[release] icons"
"$(dirname "$0")/release/make-icons.sh"

# 4. Build per-platform -----------------------------------------------------
mkdir -p "releases/${VERSION}"

PLATFORMS="${PLATFORMS:-windows linux-x86_64 linux-arm64 darwin-x86_64 darwin-arm64}"
for p in $PLATFORMS; do
  case "$p" in
    windows)
      echo "[release] === WINDOWS ==="
      "$(dirname "$0")/release/build-windows.sh"
      ;;
    linux|linux-x86_64)
      echo "[release] === LINUX x86_64 ==="
      LINUX_ONLY="x86_64" "$(dirname "$0")/release/build-linux.sh"
      ;;
    linux-arm64)
      echo "[release] === LINUX arm64 ==="
      LINUX_ONLY="arm64" "$(dirname "$0")/release/build-linux.sh"
      ;;
    darwin-x86_64)
      echo "[release] === MACOS x86_64 ==="
      "$(dirname "$0")/release/build-macos.sh" x86_64 || \
        echo "[release] macos x86_64 failed (or skipped)"
      ;;
    darwin-arm64)
      echo "[release] === MACOS arm64 ==="
      "$(dirname "$0")/release/build-macos.sh" arm64 || \
        echo "[release] macos arm64 failed (or skipped)"
      ;;
    *)
      echo "WARN: unknown platform '$p'" >&2
      ;;
  esac
done

# 5. Top-level index --------------------------------------------------------
INDEX="releases/${VERSION}/INDEX.md"
cat > "$INDEX" <<EOF
# DRZ Diff ${VERSION}

Build date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Commit:     $(git rev-parse --short HEAD)

## Platforms

| Platform | Arch | Files |
|---|---|---|
EOF
for d in releases/${VERSION}/*/; do
  [ -d "$d" ] || continue
  platform=$(basename "$d")
  files=$(ls "$d" | grep -v '^SHA256SUMS$' | grep -v '^INDEX.md$' | tr '\n' ' ')
  echo "| \`${platform}\` | | ${files} |" >> "$INDEX"
done

cat >> "$INDEX" <<EOF

## Install

\`\`\`bash
# Windows
cd windows-x86_64 && ./install.bat

# Linux (Debian/Ubuntu)
cd linux-x86_64 && ./install.sh

# Linux (ARM64 / Asahi, Pi 5, etc.)
cd linux-arm64 && ./install.sh

# macOS — Intel
cd darwin-x86_64 && bash install.sh

# macOS — Apple Silicon
cd darwin-arm64 && bash install.sh
\`\`\`

## Verify

\`\`\`bash
sha256sum -c SHA256SUMS
\`\`\`
EOF

echo "[release] all done"
echo "  output: releases/${VERSION}/"
ls -R "releases/${VERSION}/" | head -80

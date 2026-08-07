#!/bin/sh
# docker/entrypoint.sh
# Container entrypoint. Resolves VERSION from env, tag, or git describe,
# then invokes scripts/release.sh with sane defaults for in-container builds.
set -eu

REPO_ROOT="${REPO_ROOT:-/src}"
cd "$REPO_ROOT" || exit 1

if [ -z "${VERSION:-}" ]; then
  if [ -n "${GIT_TAG:-}" ]; then
    VERSION="${GIT_TAG#v}"
  elif git describe --tags --abbrev=0 >/dev/null 2>&1; then
    VERSION="$(git describe --tags --abbrev=0 | sed 's/^v//')"
  else
    VERSION="0.0.0"
  fi
fi
export VERSION

PLATFORMS="${PLATFORMS:-linux-x86_64 linux-arm64}"
echo "[docker-entrypoint] VERSION=$VERSION PLATFORMS=$PLATFORMS"

if [ "$#" -eq 0 ]; then
  set -- ./scripts/release.sh
fi

exec "$@"
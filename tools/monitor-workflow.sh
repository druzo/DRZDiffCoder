#!/usr/bin/env bash
# tools/monitor-workflow.sh
# Poll a GitHub Actions run until it completes.
# On success: exit 0.
# On failure: print failed job names + first ~40 lines of each failed log,
#             then exit 1.
#
# Usage: tools/monitor-workflow.sh <run-id-or-url>
#        tools/monitor-workflow.sh --workflow release.yml [--branch release/v0.1.2]
#
# Requires: gh CLI authenticated against druzo/DRZDiffCoder.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not found — install: https://cli.github.com/" >&2
  exit 1
fi

RUN_ID=""
WORKFLOW=""
BRANCH=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --workflow) WORKFLOW="$2"; shift 2 ;;
    --branch)   BRANCH="$2";   shift 2 ;;
    --run)      RUN_ID="$2";   shift 2 ;;
    *)          RUN_ID="$1";   shift   ;;
  esac
done

if [ -z "$RUN_ID" ] && [ -n "$WORKFLOW" ]; then
  BRANCH="${BRANCH:-$(git branch --show-current)}"
  echo "[monitor] resolving latest run for $WORKFLOW on $BRANCH"
  RUN_ID="$(gh run list --workflow "$WORKFLOW" --branch "$BRANCH" --limit 1 \
              --json databaseId -q '.[0].databaseId' 2>/dev/null || true)"
fi

if [ -z "$RUN_ID" ]; then
  echo "usage: $0 <run-id> | --workflow <name> [--branch <branch>]" >&2
  exit 2
fi

echo "[monitor] run = $RUN_ID"
INTERVAL="${INTERVAL:-15}"

while true; do
  STATUS="$(gh run view "$RUN_ID" --json status -q '.status' 2>/dev/null || echo unknown)"
  CONCL="$(gh run view "$RUN_ID" --json conclusion -q '.conclusion' 2>/dev/null || echo '')"
  printf '[monitor] %s status=%s conclusion=%s\n' "$(date +%H:%M:%S)" "$STATUS" "$CONCL"

  case "$STATUS" in
    completed) break ;;
    *)         sleep "$INTERVAL" ;;
  esac
done

echo "----- final status: $CONCL -----"

if [ "$CONCL" = "success" ]; then
  exit 0
fi

echo "[monitor] failures detected — fetching logs"
gh run view "$RUN_ID" --json jobs \
  -q '.jobs[] | select(.conclusion == "failure") | .databaseId' \
  | while read -r JOB_ID; do
      [ -n "$JOB_ID" ] || continue
      echo "===== failed job $JOB_ID ====="
      gh run view --job "$JOB_ID" --log-failed 2>/dev/null | head -60
      echo ""
    done

exit 1
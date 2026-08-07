# tools/

Helper scripts used during release preparation, monitoring, and validation.

| Script | Purpose |
|---|---|
| `release-doctor.sh` | Pre-flight sanity check: workflow YAML, Dockerfile, script permissions, version alignment. Run before tagging. |
| `monitor-workflow.sh` | Poll a GitHub Actions run via `gh` until completion; on failure print failed job logs. Used by the self-heal loop. |
| `collect-artifacts.sh` | Aggregate every artifact in `releases/<VERSION>/` into `dist/` and emit `SHA256SUMS.txt`. Mirrors what the CI `release` job does. |

All scripts are idempotent and exit non-zero on any failure. They require
`gh` (authenticated), `python3` (YAML validation), and standard Unix tools.
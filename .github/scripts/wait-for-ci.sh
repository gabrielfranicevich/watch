#!/usr/bin/env bash
set -euo pipefail

required_workflows=(
  ".github/workflows/rust.yml"
  ".github/workflows/markdown.yml"
)

deadline=$((SECONDS + 3600))

for workflow_path in "${required_workflows[@]}"; do
  printf 'Waiting for %s on commit %s\n' "$workflow_path" "$GITHUB_SHA"

  while true; do
    if (( SECONDS >= deadline )); then
      printf 'Timed out waiting for %s\n' "$workflow_path" >&2
      exit 1
    fi

    run_info="$(
      gh api "repos/$GITHUB_REPOSITORY/actions/runs?head_sha=$GITHUB_SHA&per_page=100" \
        --jq "[.workflow_runs[] | select(.path == \"$workflow_path\" and .head_sha == \"$GITHUB_SHA\" and .event == \"push\")] | sort_by(.created_at) | last // empty | [.id, .status, (.conclusion // \"\")] | @tsv"
    )"

    if [[ -z "$run_info" ]]; then
      sleep 15
      continue
    fi

    IFS=$'\t' read -r run_id status conclusion <<< "$run_info"

    if [[ "$status" == "completed" ]]; then
      if [[ "$conclusion" == "success" ]]; then
        printf '%s passed (run %s)\n' "$workflow_path" "$run_id"
        break
      fi

      printf '%s finished with conclusion: %s (run %s)\n' "$workflow_path" "$conclusion" "$run_id" >&2
      exit 1
    fi

    sleep 15
  done
done

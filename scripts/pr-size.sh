#!/usr/bin/env bash
# Pull request size check (CI job: pr-size; ~/.claude/CLAUDE.md "Speed and
# reliability"). Counts lines added between BASE and HEAD, leaving out lock
# files, fixtures and generated files, and fails over the limit unless the
# pull request description (env PR_BODY) has a "## Size" section with text.
# Usage: PR_BODY="..." scripts/pr-size.sh <base-sha> <head-sha>
set -euo pipefail

base=${1:?base sha}
head=${2:?head sha}
limit=${PR_SIZE_LIMIT:-800}

added=$(git diff --numstat "$base...$head" -- . \
  ':(exclude,glob)**/*.lock' \
  ':(exclude,glob)**/Package.resolved' \
  ':(exclude,glob)**/fixtures/**' \
  ':(exclude,glob)**/*.snap' \
  ':(exclude,glob)**/generated/**' \
  | awk '$1 != "-" { n += $1 } END { print n + 0 }')

echo "Added lines (lock files, fixtures and generated files left out): $added (limit $limit)"
[ "$added" -le "$limit" ] && exit 0

# Text under "## Size": strip CR and HTML comments, stop at the next heading.
reason=$(printf '%s\n' "${PR_BODY:-}" | tr -d '\r' | perl -0pe 's/<!--.*?-->//gs' \
  | awk '/^## /{ on = ($0 ~ /^## Size[[:space:]]*$/); next } on' | tr -d '[:space:]')

if [ -n "$reason" ]; then
  echo "Over the limit, with a Size section in the description: allowed."
  exit 0
fi
echo "::error::This pull request adds $added lines, over the $limit limit. Split it, or add a \"## Size\" section to the description saying why it cannot be split."
exit 1

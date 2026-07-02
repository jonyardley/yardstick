#!/usr/bin/env bash
# Repo invariants enforced in CI (job: guardrails). Each check prints what
# failed and the script exits non-zero if any check fails.
set -uo pipefail

cd "$(git rev-parse --show-toplevel)"
fail=0
err() { echo "FAIL: $1"; fail=1; }

# 1. CLAUDE.md hard cap — rules must stay small enough to be followed.
max_lines=120
if [ -f CLAUDE.md ]; then
  lines=$(wc -l < CLAUDE.md | tr -d ' ')
  [ "$lines" -le "$max_lines" ] || err "CLAUDE.md is $lines lines (cap: $max_lines). Remove something before adding."
else
  err "CLAUDE.md is missing."
fi

# 2. Spec and plan filename conventions.
for f in docs/superpowers/specs/*; do
  [ -e "$f" ] || continue
  basename "$f" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}-[a-z0-9-]+-design\.md$' \
    || err "spec name breaks convention YYYY-MM-DD-<topic>-design.md: $f"
done
for f in docs/superpowers/plans/*; do
  [ -e "$f" ] || continue
  basename "$f" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}-[a-z0-9-]+\.md$' \
    || err "plan name breaks convention YYYY-MM-DD-<name>.md: $f"
done

# 3. .claude/settings.json must be valid JSON (a broken file silently
#    disables every hook in it).
if [ -f .claude/settings.json ]; then
  jq empty .claude/settings.json 2>/dev/null || err ".claude/settings.json is not valid JSON."
fi

# 4. No merge-conflict markers anywhere.
if git grep -nE '^(<{7}|={7}|>{7})( |$)' -- ':!scripts/guardrails.sh' >/dev/null 2>&1; then
  git grep -nE '^(<{7}|={7}|>{7})( |$)' -- ':!scripts/guardrails.sh'
  err "merge conflict markers found."
fi

# 5. No TODO/FIXME/XXX in source code (docs are allowed to discuss them).
src_dirs=""
for d in shared store mcp runtime apple/Daily; do [ -d "$d" ] && src_dirs="$src_dirs $d"; done
if [ -n "$src_dirs" ]; then
  # shellcheck disable=SC2086
  if git grep -nE '\b(TODO|FIXME|XXX)\b' -- $src_dirs >/dev/null 2>&1; then
    git grep -nE '\b(TODO|FIXME|XXX)\b' -- $src_dirs
    err "TODO/FIXME/XXX in source. Finish it or file it as a plan task."
  fi
fi

# 6. apple/generated must never be tracked.
if git ls-files apple/generated | grep -q .; then
  err "apple/generated/ is tracked in git — it must stay generated-only."
fi

[ "$fail" -eq 0 ] && echo "guardrails: all checks passed."
exit "$fail"

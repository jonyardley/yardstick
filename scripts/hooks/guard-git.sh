#!/usr/bin/env bash
# Claude Code PreToolUse hook (matcher: Bash). Mechanically enforces the
# SDLC's branch discipline: no commits/merges/rebases on main, no pushes
# to main, no force pushes — from any Claude session in this repo.
# Input: hook JSON on stdin. Output: permissionDecision JSON to deny.
set -uo pipefail

cmd=$(jq -r '.tool_input.command // empty' 2>/dev/null)
[ -z "$cmd" ] && exit 0

# Fast path: not a git mutation → allow without further work.
printf '%s' "$cmd" | grep -qE '(^|[;&|[:space:]])git[[:space:]]' || exit 0
printf '%s' "$cmd" | grep -qE '[[:space:]](commit|merge|rebase|push)([[:space:]]|$)' || exit 0

deny() {
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"%s"}}' "$1"
  exit 0
}

# Force pushes are never allowed (docs/SDLC.md §2).
if printf '%s' "$cmd" | grep -qE 'git[[:space:]]+push[^;&|]*[[:space:]](--force(-with-lease)?|-f)([[:space:]]|$)'; then
  deny "Force-push is blocked by the SDLC (docs/SDLC.md §2)."
fi

# Pushes that name main as the destination are never allowed.
if printf '%s' "$cmd" | grep -qE 'git[[:space:]]+push[^;&|]*[[:space:]]([^ :]+:)?(refs/heads/)?main([[:space:]]|$)'; then
  deny "Pushing to main is blocked — open a PR instead (docs/SDLC.md §2)."
fi

# symbolic-ref (not rev-parse) so unborn branches (fresh repo, no commits)
# still report "main" correctly.
branch=$(git -C "${CLAUDE_PROJECT_DIR:-.}" symbolic-ref --short -q HEAD 2>/dev/null || echo "")
[ "$branch" = "main" ] || exit 0

# On main: block history mutations and bare pushes.
if printf '%s' "$cmd" | grep -qE '(^|[;&|[:space:]])git[[:space:]]+(commit|merge|rebase)([[:space:]]|$)'; then
  deny "You are on main — create a task branch first (docs/SDLC.md §2, e.g. p0/t3-store)."
fi
if printf '%s' "$cmd" | grep -qE '(^|[;&|[:space:]])git[[:space:]]+push([[:space:]]|$)'; then
  deny "You are on main — pushes from main are blocked; work on a branch and open a PR (docs/SDLC.md §2)."
fi

exit 0

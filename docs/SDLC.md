# Yardstick SDLC

How software gets built in this repository. **Every rule here is enforced by a machine** (CI, git hooks, branch protection, or Claude Code hooks) or it names the human gate that owns it. Rules that can't be enforced don't go in this file — they go in a review conversation.

This process is deliberately shaped by what went wrong in `jonyardley/intrada`: architecture churn (three UI shells in five months), a 49KB agent-instructions file nobody could follow, process invented per-PR, and paused code kept on life support. The counters here: one committed architecture, a hard-capped rule file, one lifecycle for all work, and delete-don't-pause.

## 1. Lifecycle

Every change follows the same pipeline. No stage is skipped, whoever (human or agent) does the work.

```
Idea ──► Spec ──► Plan ──► Task branches + PRs ──► Phase gate ──► next phase
         (docs/superpowers/specs)   (docs/superpowers/plans)      (Jon uses the build)
```

| Stage | Artifact | Gate to pass |
|---|---|---|
| **Spec** | `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md` — decisions + rationale, approaches considered, open questions | Jon approves the spec (PR review) |
| **Plan** | `docs/superpowers/plans/YYYY-MM-DD-<name>.md` — bite-sized TDD tasks, complete code, exact commands | Jon approves the plan (PR review) |
| **Implementation** | One branch + PR per plan task (or small coherent task group) | CI green + PR review; plan checkboxes ticked in the same PR |
| **Phase gate** | Working build; `git tag phase-N` on completion | Jon uses the build; feedback becomes the next spec/plan revision — **before** new scope |

**Scope rule:** work not described by an approved plan task does not get implemented. If reality diverges from the plan (an API changed, a better approach emerged), the PR that deviates must update the plan file in the same PR and say why in its description. Architecture changes (new crate, new dependency, new process/topology, version pin changes) additionally require a spec amendment PR *first*.

## 2. Branching & merging

- **Trunk-based.** `main` is always releasable-quality; it is protected: no direct pushes, no force pushes, PRs only, required status checks, enforced for admins.
- **Branch naming:** `p<phase>/t<task>-<slug>` for plan tasks (e.g. `p0/t3-store`), `chore/<slug>`, `fix/<slug>`, `docs/<slug>` otherwise. Branches live hours-to-a-day, not weeks.
- **Squash merge only** (repo setting). PR title becomes the commit subject → history on `main` is one conventional commit per PR.
- **Conventional commits** for PR titles and branch commits: `feat|fix|docs|chore|refactor|test|ci(scope): summary`. CI lints the PR title.
- Merged branches are deleted automatically (repo setting).

## 3. Pull requests

- **Small.** One plan task per PR is the default. A PR that touches more than one concern gets split.
- Every PR uses the template: what/why, link to the plan task, TDD evidence (the failing-test output and the passing run), Definition of Done checklist.
- **CI must be green** — no "will fix in a follow-up" merges.
- **Review:** Jon reviews every PR (CODEOWNERS auto-assigns). Agent-authored PRs are never self-merged; they wait for Jon. Trivial-doc exception: none — the process is the same for everything, that's the point.

## 4. Definition of Done (per task)

1. Failing test written first and observed failing; minimal implementation; test observed passing (evidence in PR).
2. `just test` passes locally and in CI; `cargo clippy -- -D warnings` clean; no new warnings.
3. No `TODO`/`FIXME`/placeholder code, no commented-out code, no dead code behind flags "for later" (CI-checked).
4. Plan checkbox(es) ticked in the same PR; docs touched by the change updated in the same PR.
5. Generated code (`apple/generated/`) never committed, never hand-edited.

## 5. Quality gates (CI)

| Job | Runs | Blocks merge |
|---|---|---|
| `guardrails` | repo invariants: CLAUDE.md line cap, spec/plan naming, valid `.claude/settings.json`, no TODO/FIXME in source, no conflict markers | yes |
| `pr-title` | conventional-commit lint on the PR title | yes |
| `rust` | `cargo nextest run --locked --workspace`, `cargo clippy --locked -D warnings`, `cargo fmt --check` (added in Phase 0 Task 1) | yes |
| `apple` | typegen + BoltFFI pack + `just app-test` (Swift unit tests via `xcodebuild test`, added in Phase 1 Task 9) | yes |

New required checks are added to branch protection in the same PR that introduces the job.

## 6. Determinism rules

- **Pinned toolchain.** Versions live in one place each (`rust-toolchain.toml`, workspace `Cargo.toml`, spec §2 table). Exact pins (`facet =0.44`, `boltffi =0.25.2`) change only via spec amendment. `Cargo.lock` is committed.
- **One entry point:** `just`. If a command matters, it is a `just` target; CI runs the same targets developers run. No bespoke command lines living only in someone's shell history or an agent's context.
- **One architecture.** The spec's topology (single app process; Crux core; EffectRouter; embedded MCP) is the only one being built. No parallel shells, no "experiment" crates on `main`. Superseded code is **deleted in the PR that supersedes it** — never paused, never kept building "just in case" (the intrada lesson).
- **Agent behavior** is constrained by `CLAUDE.md` (hard-capped at 120 lines, CI-enforced) plus Claude Code hooks in `.claude/settings.json` that mechanically block commits/pushes to `main` and force pushes. If a rule for agents matters, it becomes a hook or a CI check, not a paragraph.

## 7. Change control for the process itself

- Changes to this file, `CLAUDE.md`, `.claude/settings.json`, CI workflows, or branch protection go through their own PR with a rationale.
- `CLAUDE.md` may only grow if something else in it shrinks (the cap is the budget). Prefer replacing prose with an enforcement mechanism.
- If the same mistake happens twice, the fix is a new check, not a new sentence.

## 8. Roles

- **Jon** — product owner and sole human reviewer. Approves specs, plans, and every PR; decides phase gates; owns the open questions in each spec.
- **Claude Code** — implementer. Works only from approved plan tasks, on task branches, via PRs. May propose spec/plan amendments; never merges its own work.

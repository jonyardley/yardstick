# Yardstick — Claude Code Rules

Daily: a calm todo + daily-notes macOS app. Crux (Rust) core, SwiftUI shell, Rust-side SQLite, embedded MCP server.

This file is hard-capped at 120 lines (CI-enforced). Process detail lives in [docs/SDLC.md](docs/SDLC.md) — read it once per session before doing any work.

## Sources of truth (in precedence order)

1. Approved spec: `docs/superpowers/specs/` (architecture, decisions, pins)
2. Approved plan: `docs/superpowers/plans/` (the current plan = the newest one; work = its unchecked tasks)
3. Design reference: `docs/design/reference/` (pixel/behavior acceptance criteria); `docs/design/handoff/README.md` (product principles)
4. Research context: `docs/research/`

## The workflow (no exceptions)

1. **Only implement approved plan tasks.** Anything else — including "quick fixes" Jon asks for in passing — first gets a plan (or plan-amendment) PR. If a plan step doesn't survive contact with reality, update the plan file in the same PR and explain in the PR description.
2. **Branch per task:** `p<phase>/t<task>-<slug>` (e.g. `p0/t3-store`); `chore/`, `fix/`, `docs/` otherwise. Never commit, merge, rebase, or push on `main` (hook-blocked + branch protection). Never force-push (hook-blocked).
3. **TDD, strictly:** failing test → run it, observe the failure → minimal implementation → run it, observe the pass → commit. Paste both outputs into the PR. Code without a driving test does not get written.
4. **Verify before claiming done:** run `just test` (and `just app-test` if Swift changed) and read the output. Report results faithfully — failing is a status, not a secret.
5. **PR per task**, conventional-commit title (`feat|fix|docs|chore|refactor|test|ci(scope): summary`), template filled in, plan checkboxes ticked in the same PR. Open it, report, stop — **never merge your own PR**; Jon reviews and merges.

## Hard prohibitions

- No architecture changes (new crate/dependency/process/topology, version-pin bumps) without a spec amendment PR first. Exact pins that never float silently: `facet = "=0.44"`, `boltffi = "=0.25.2"`.
- No TODO/FIXME/placeholder/commented-out code in commits (CI-blocked).
- No editing or committing `apple/generated/` (generated code is disposable).
- No parallel implementations or "experimental" alternatives kept alongside the real one — superseded code is deleted in the superseding PR.
- No I/O, clocks, randomness, or tokio in the `shared` crate; IDs are generated in `store`.
- No new instructions added here without removing something — the cap is the budget; prefer a hook or CI check over a sentence.

## Commands

- `just test` — all Rust tests (cargo nextest)
- `just generate` — typegen + BoltFFI Swift packages
- `just app-test` — Swift unit tests + app build
- Crate DAG (never violate): `shared → crux_core` only; `store → shared`; `mcp → shared, store`; `runtime → shared, store, mcp`.

## When stuck

APIs here are young (EffectRouter is RFC-stage, BoltFFI and rmcp 2.x are new). When a documented signature doesn't exist, mirror the canonical example (`crux` repo: `examples/counter-routing`, `examples/counter`, `examples/weather`; `rust-sdk`: `examples/servers`), note the deviation in the PR, and update the plan file. Do not invent APIs from training data.

# Daily (codename Yardstick)

A calm todo + daily-notes app for macOS. Crux (Rust) core, SwiftUI shell,
SQLite via Rust-side effect handling, embedded MCP server for AI agents.

- Product/design spec: `docs/design/handoff/README.md`
- Architecture decisions: `docs/superpowers/specs/2026-07-02-daily-app-design.md`
- Current plan: `docs/superpowers/plans/2026-07-04-phase-1-shell-and-notes.md`

## Prerequisites

- Rust 1.90 (`rustup`; pinned in `rust-toolchain.toml`)
- `just`
- `cargo-nextest` — `cargo install cargo-nextest`. On rustc 1.90 the latest
  nextest release may refuse to build (it wants rustc 1.95+); if so, pin an
  older release: `cargo install cargo-nextest --version 0.9.128 --locked`.
  CI is unaffected — it installs a prebuilt binary via `taiki-e/install-action`.
- `boltffi_cli` **=0.25.2** (`cargo install boltffi_cli --version '=0.25.2' --locked`)
- XcodeGen (`brew install xcodegen`), Xcode 16+

## Dev loop

    just test       # all Rust tests (cargo nextest)
    just app-test   # Swift unit tests (builds the app as test host)
    just generate   # typegen + BoltFFI Swift packages
    just app        # build the macOS app
    cd apple && just run

## MCP

The app serves MCP (streamable HTTP) on 127.0.0.1:52111.
Token: `~/Library/Application Support/Daily/mcp-token`.

    claude mcp add --transport http daily http://127.0.0.1:52111/mcp \
      --header "Authorization: Bearer $(cat ~/Library/Application\ Support/Daily/mcp-token)"

default: test

test:
    cargo nextest run --workspace --no-tests=pass

# Generate the Swift "App" package (value types + bincode serializers).
typegen:
    rm -rf apple/generated/App
    cargo run -p shared --bin codegen --features codegen -- --output-dir apple/generated

# Build + package the "Shared" Swift package (BoltFFI bindings + static lib).
# boltffi reads runtime/boltffi.toml and scans runtime/src for exports.
# MACOSX_DEPLOYMENT_TARGET pins the staticlib's stamped SDK version to the
# app's deployment target (apple/project.yml); without it, cargo's C build
# scripts (e.g. the vendored sqlite3.c) stamp the host SDK version instead,
# which trips a "built for newer macOS" linker warning at app-link time.
# boltffi.toml has no first-class setting for this (checked: no per-target
# key under [targets.apple] for deployment target / min-OS as of 0.25.2).
package:
    rm -rf apple/generated/Shared
    cd runtime && MACOSX_DEPLOYMENT_TARGET=15.0 boltffi pack apple

# Both generated Swift packages. (apple/Justfile adds the xcodegen step.)
generate: typegen package

# Build the macOS app (regenerates packages + Xcode project first).
app:
    cd apple && just build

# Build and launch the macOS app.
app-run:
    cd apple && just run

# Run the Swift unit tests (headless XCTest via xcodebuild).
app-test:
    cd apple && just test

# Run the XCUITest journeys (takes over the desktop while running).
app-ui-test:
    cd apple && just ui-test

# Fast local checks to pass before any code review (CI's guardrails and rust
# lanes), plus the Swift tests unless every file changed against origin/main
# (committed, uncommitted or untracked) is docs, CI or scripts only. Components
# are added here, not in rust-toolchain.toml, so CI jobs that never lint do not
# download them.
pre-review:
    #!/usr/bin/env bash
    set -euo pipefail
    rustup component add clippy rustfmt
    bash scripts/guardrails.sh
    cargo fmt --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    just test
    base=$(git merge-base origin/main HEAD) || { echo "No merge base with origin/main: run git fetch origin" >&2; exit 1; }
    changed=$(git diff --name-only "$base"; git ls-files --others --exclude-standard)
    if grep -qvE '^$|^(docs/|\.github/|scripts/)|\.md$' <<< "$changed"; then
      just app-test
    else
      echo "Only docs, CI or scripts changed: app-test skipped"
    fi

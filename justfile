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

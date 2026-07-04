default: test

test:
    cargo nextest run --workspace --no-tests=pass

# Generate the Swift "App" package (value types + bincode serializers).
typegen:
    rm -rf apple/generated/App
    cargo run -p shared --bin codegen --features codegen -- --output-dir apple/generated

# Build + package the "Shared" Swift package (BoltFFI bindings + static lib).
# boltffi reads runtime/boltffi.toml and scans runtime/src for exports.
package:
    rm -rf apple/generated/Shared
    cd runtime && boltffi pack apple

# Both generated Swift packages. (apple/Justfile adds the xcodegen step.)
generate: typegen package

# Build the macOS app (regenerates packages + Xcode project first).
app:
    cd apple && just build

default: test

test:
    cargo nextest run --workspace --no-tests=pass

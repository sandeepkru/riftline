# Tasks for building and testing the Riftline workspace

# Default task
default: ci

# Format the code using rustfmt
fmt:
    cargo fmt --all -- --check

# Run clippy lints and deny warnings
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Build all targets in the workspace
build:
    cargo build --workspace --all-targets

# Run tests for the entire workspace
test:
    cargo test --workspace --all-targets

# Convenience task for CI: format, lint, build, and test
ci: fmt lint build test

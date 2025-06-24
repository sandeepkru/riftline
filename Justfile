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

# Generate a coverage report and fail if below threshold
coverage:
    cargo tarpaulin --workspace --all-targets --timeout 120 --fail-under 0

# Convenience task for CI: format, lint, build, test and coverage
ci: fmt lint build test coverage

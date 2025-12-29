# Default recipe - show available commands
default:
    @just --list

# ============================================================================
# Development Commands
# ============================================================================

# Run all tests across workspace
test:
    cargo test -p konnekt-session-core

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run BDD tests specifically
test-bdd:
    @cd konnekt-session-tests && just test

# Run BDD tests with JSON output
test-bdd-json:
    @cd konnekt-session-tests && just test-json

# Run BDD tests with JUnit XML output
test-bdd-junit:
    @cd konnekt-session-tests && just test-junit

# Run tests for a specific package
test-package package:
    cargo test -p {{ package }}

# ============================================================================
# Code Quality
# ============================================================================

# Run clippy linter
lint:
    cargo clippy --workspace -- -D warnings

# Run clippy with fixes
lint-fix:
    cargo clippy --workspace --fix --allow-dirty --allow-staged

# Format code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run all quality checks (lint + format check + test)
check: fmt-check lint test

# ============================================================================
# Build Commands
# ============================================================================

# Build all workspace packages
build:
    cargo build --workspace

# Build in release mode
build-release:
    cargo build --workspace --release

# List all BDD scenarios
list-scenarios:
    @cd konnekt-session-tests && just list-scenarios

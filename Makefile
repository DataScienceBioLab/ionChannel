# ionChannel Makefile
# Convenience targets for development

.PHONY: help build test lint fmt clean ci check coverage docs

# Default target
help:
	@echo "ionChannel Development Commands"
	@echo ""
	@echo "  make build     - Build all crates"
	@echo "  make test      - Run all tests"
	@echo "  make lint      - Run clippy linter"
	@echo "  make fmt       - Format code"
	@echo "  make check     - Run fmt + lint + test"
	@echo "  make ci        - Full CI check (build + check + docs)"
	@echo "  make coverage  - Run tests with coverage"
	@echo "  make docs      - Build documentation"
	@echo "  make clean     - Clean build artifacts"
	@echo ""
	@echo "Quick commands:"
	@echo "  make t         - Alias for test"
	@echo "  make c         - Alias for check"

# Build
build:
	cargo build --workspace --all-targets

# Test
test t:
	cargo test --workspace

# Lint
lint:
	cargo clippy --workspace --all-targets -- -D warnings

# Format
fmt:
	cargo fmt --all

# Format check (no modify)
fmt-check:
	cargo fmt --all -- --check

# Quick check (fmt + lint + test)
check c: fmt-check lint test

# Full CI simulation
ci: build check docs
	@echo ""
	@echo "✅ CI passed!"

# Coverage
coverage:
	cargo tarpaulin --workspace --out html --skip-clean
	@echo ""
	@echo "Coverage report: tarpaulin-report.html"

# Documentation
docs:
	cargo doc --workspace --no-deps --open

# Clean
clean:
	cargo clean

# Run specific test suites
test-unit:
	cargo test --workspace --lib

test-e2e:
	cargo test --package ion-test-substrate --test e2e_demonstration

test-chaos:
	cargo test --package ion-test-substrate --test chaos_tests

test-security:
	cargo test --package ion-test-substrate --test security_tests

# Platform check
platform-check:
	cargo run --package ion-compositor --bin capability-check

# Count tests
test-count:
	@echo "Test counts:"
	@cargo test --workspace 2>&1 | grep "test result" | awk '{sum += $$3} END {print "Total: " sum " tests"}'

# Show crate structure
tree:
	@echo "ionChannel Crate Structure:"
	@echo ""
	@ls -la crates/

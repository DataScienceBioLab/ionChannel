# ionChannel Makefile
# Common development tasks

.PHONY: all build test check clippy fmt doc clean examples demo setup-upstream help

# Default target
all: check test

# Build all crates
build:
	cargo build --workspace

# Build in release mode
release:
	cargo build --workspace --release

# Run all tests
test:
	cargo test --workspace

# Run tests with output
test-verbose:
	cargo test --workspace -- --nocapture

# Check compilation without building
check:
	cargo check --workspace --all-targets

# Run clippy lints
clippy:
	cargo clippy --workspace --all-targets

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Build documentation
doc:
	cargo doc --workspace --no-deps --open

# Clean build artifacts
clean:
	cargo clean

# Build examples
examples:
	cargo build --examples

# Run the Smithay integration demo
demo:
	cargo run --example smithay_integration

# Run full stack demo
demo-full:
	cargo run --example full_stack_demo

# Run portal test client (check mode)
portal-check:
	cargo run --bin portal-test -- check

# Setup upstream repositories
setup-upstream:
	./scripts/setup-upstream.sh

# Full CI check (what GitHub Actions runs)
ci: fmt-check clippy test
	@echo "✓ CI checks passed"

# Pre-commit hook
pre-commit: fmt clippy test
	@echo "✓ Ready to commit"

# Watch for changes and run tests
watch:
	cargo watch -x 'test --workspace'

# Count lines of code
loc:
	@echo "=== Lines of Code ==="
	@find . -name "*.rs" -not -path "./target/*" -not -path "./upstream/*" | xargs wc -l | tail -1

# Show project structure
tree:
	@find . -type f \( -name "*.rs" -o -name "*.toml" -o -name "*.md" \) \
		-not -path "./target/*" -not -path "./upstream/*" | sort

# Help
help:
	@echo "ionChannel Development Commands"
	@echo ""
	@echo "Build:"
	@echo "  make build        - Build all crates (debug)"
	@echo "  make release      - Build all crates (release)"
	@echo "  make examples     - Build examples"
	@echo ""
	@echo "Test:"
	@echo "  make test         - Run all tests"
	@echo "  make test-verbose - Run tests with output"
	@echo ""
	@echo "Quality:"
	@echo "  make check        - Check compilation"
	@echo "  make clippy       - Run clippy lints"
	@echo "  make fmt          - Format code"
	@echo "  make fmt-check    - Check formatting"
	@echo "  make ci           - Full CI check"
	@echo ""
	@echo "Demo:"
	@echo "  make demo         - Run Smithay integration demo"
	@echo "  make demo-full    - Run full stack demo"
	@echo "  make portal-check - Check portal availability"
	@echo ""
	@echo "Setup:"
	@echo "  make setup-upstream - Clone upstream repos"
	@echo "  make doc            - Build and open docs"
	@echo "  make clean          - Clean build artifacts"
	@echo ""
	@echo "Utilities:"
	@echo "  make loc          - Count lines of code"
	@echo "  make tree         - Show project structure"
	@echo "  make watch        - Watch for changes"


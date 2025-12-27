#!/bin/bash
# Test script to verify all ionChannel E2E components
# Run this to ensure everything is working before demo

set -e

echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                      ║"
echo "║          🧪 ionChannel E2E Validation Test Suite                    ║"
echo "║                                                                      ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
PASSED=0
FAILED=0
WARNINGS=0

test_step() {
    echo ""
    echo "═══════════════════════════════════════════════════════════════════════"
    echo "  $1"
    echo "═══════════════════════════════════════════════════════════════════════"
    echo ""
}

check_ok() {
    echo -e "${GREEN}✅ $1${NC}"
    ((PASSED++))
}

check_fail() {
    echo -e "${RED}❌ $1${NC}"
    ((FAILED++))
}

check_warn() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    ((WARNINGS++))
}

# Test 1: Prerequisites
test_step "1. Checking Prerequisites"

if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    check_ok "Rust installed: $RUST_VERSION"
else
    check_fail "Rust not found"
fi

if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    check_ok "Cargo installed: $CARGO_VERSION"
else
    check_fail "Cargo not found"
fi

if command -v virsh &> /dev/null; then
    VIRSH_VERSION=$(virsh --version)
    check_ok "libvirt installed: $VIRSH_VERSION"
else
    check_fail "libvirt not found (required for VM demos)"
fi

# Test 2: libvirt connectivity
test_step "2. Testing libvirt Connectivity"

if virsh -c qemu:///system list &> /dev/null; then
    check_ok "Can connect to libvirt (qemu:///system)"
    
    VM_COUNT=$(virsh -c qemu:///system list --all | tail -n +3 | wc -l)
    echo "   Found $VM_COUNT existing VM(s)"
else
    check_fail "Cannot connect to libvirt"
    echo "   Try: sudo usermod -aG libvirt \$USER && newgrp libvirt"
fi

# Test 3: Workspace build
test_step "3. Building Workspace"

if cargo build --workspace --all-features 2>&1 | tail -1 | grep -q "Finished"; then
    check_ok "Workspace builds successfully"
else
    check_fail "Workspace build failed"
fi

# Test 4: Unit tests
test_step "4. Running Unit Tests"

if cargo test --workspace --lib 2>&1 | grep -q "test result: ok"; then
    TEST_RESULTS=$(cargo test --workspace --lib 2>&1 | grep "test result:" | tail -1)
    check_ok "Unit tests pass: $TEST_RESULTS"
else
    check_fail "Unit tests failed"
fi

# Test 5: ion-validation with libvirt feature
test_step "5. Building ion-validation (libvirt feature)"

if cargo build -p ion-validation --features libvirt 2>&1 | tail -1 | grep -q "Finished"; then
    check_ok "ion-validation builds with libvirt feature"
else
    check_fail "ion-validation build failed"
fi

# Test 6: Examples compilation
test_step "6. Checking Demo Examples"

EXAMPLES=(
    "full_e2e_demo"
    "discover_and_provision"
    "create_working_vm"
    "provision_and_connect"
    "autonomous_rustdesk_id"
)

for example in "${EXAMPLES[@]}"; do
    if cargo check -p ion-validation --example "$example" --features libvirt &> /dev/null; then
        check_ok "Example '$example' compiles"
    else
        check_fail "Example '$example' compilation failed"
    fi
done

# Test 7: Linting
test_step "7. Running Linters"

CLIPPY_OUTPUT=$(cargo clippy --workspace --all-targets --all-features 2>&1)
if echo "$CLIPPY_OUTPUT" | grep -q "error:"; then
    check_fail "Clippy errors found"
    echo "$CLIPPY_OUTPUT" | grep "error:" | head -5
else
    WARNING_COUNT=$(echo "$CLIPPY_OUTPUT" | grep -c "warning:" || true)
    if [ "$WARNING_COUNT" -gt 0 ]; then
        check_warn "Clippy: $WARNING_COUNT warning(s)"
    else
        check_ok "Clippy: no issues"
    fi
fi

# Test 8: Formatting
test_step "8. Checking Code Formatting"

if cargo fmt --all -- --check &> /dev/null; then
    check_ok "Code is properly formatted"
else
    check_warn "Code formatting issues found (run: cargo fmt --all)"
fi

# Test 9: Documentation
test_step "9. Verifying Documentation"

DOCS=(
    "README.md"
    "STATUS.md"
    "DEMO_GUIDE.md"
    "E2E_COMPLETE.md"
    "READY_FOR_DEMO.md"
    "CAPABILITY_BASED_VM_DISCOVERY.md"
    "BENCHSCALE_INTEGRATION.md"
)

for doc in "${DOCS[@]}"; do
    if [ -f "$doc" ]; then
        check_ok "Documentation exists: $doc"
    else
        check_fail "Missing documentation: $doc"
    fi
done

# Test 10: Demo launcher
test_step "10. Checking Demo Launcher"

if [ -f "RUN_DEMO.sh" ]; then
    if [ -x "RUN_DEMO.sh" ]; then
        check_ok "Demo launcher exists and is executable"
    else
        check_warn "Demo launcher exists but not executable (run: chmod +x RUN_DEMO.sh)"
    fi
else
    check_fail "Demo launcher (RUN_DEMO.sh) not found"
fi

# Summary
echo ""
echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                        TEST SUMMARY                                  ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${GREEN}✅ Passed:   $PASSED${NC}"
echo -e "${YELLOW}⚠️  Warnings: $WARNINGS${NC}"
echo -e "${RED}❌ Failed:   $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "╔══════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                      ║"
    echo "║              ✅ ALL TESTS PASSED - READY FOR DEMO! 🚀                ║"
    echo "║                                                                      ║"
    echo "╚══════════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "🎯 Next Steps:"
    echo ""
    echo "   Run the E2E demo:"
    echo "     ./RUN_DEMO.sh"
    echo ""
    echo "   Or run directly:"
    echo "     cargo run -p ion-validation --example full_e2e_demo --features libvirt"
    echo ""
    exit 0
else
    echo "╔══════════════════════════════════════════════════════════════════════╗"
    echo "║                                                                      ║"
    echo "║              ⚠️  SOME TESTS FAILED - REVIEW ABOVE                    ║"
    echo "║                                                                      ║"
    echo "╚══════════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "📋 Common Issues:"
    echo ""
    echo "   1. libvirt not accessible:"
    echo "      sudo usermod -aG libvirt \$USER"
    echo "      newgrp libvirt"
    echo ""
    echo "   2. Build failures:"
    echo "      cargo clean && cargo build --workspace --all-features"
    echo ""
    echo "   3. Missing documentation:"
    echo "      Review commit history or re-generate docs"
    echo ""
    exit 1
fi


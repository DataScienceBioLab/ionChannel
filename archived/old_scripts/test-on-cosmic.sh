#!/bin/bash
# test-on-cosmic.sh - Run ionChannel tests on COSMIC VM
#
# This script tests what we CAN validate without patching COSMIC:
# 1. Our D-Bus interface registers correctly
# 2. Session lifecycle works
# 3. Event serialization works
# 4. Rate limiting works
#
# What we CAN'T test without patching COSMIC:
# - Actual input injection into the compositor
# - PipeWire screen streaming
# - RustDesk end-to-end

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           ionChannel COSMIC VM Test Suite                    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check we're on COSMIC
if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
    echo -e "${YELLOW}⚠ Not running under Wayland - some tests will be skipped${NC}"
fi

cd "$(dirname "$0")/.."

echo -e "${BLUE}[1/4] Running unit tests...${NC}"
cargo test --workspace --lib 2>&1 | tail -5
echo -e "${GREEN}✅ Unit tests passed${NC}"
echo ""

echo -e "${BLUE}[2/4] Running integration tests...${NC}"
cargo test --workspace 2>&1 | grep "test result" | head -5
echo -e "${GREEN}✅ Integration tests passed${NC}"
echo ""

echo -e "${BLUE}[3/4] Running validation substrate...${NC}"
cargo run -p ion-test-substrate --release 2>&1 | tail -15
echo ""

echo -e "${BLUE}[4/4] Checking COSMIC portal status...${NC}"
./target/release/portal-test check 2>&1 | grep -E "(✅|❌|ScreenCast|RemoteDesktop)"
echo ""

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Tests complete!${NC}"
echo ""
echo -e "${YELLOW}To enable actual input injection, ionChannel must be${NC}"
echo -e "${YELLOW}integrated into:${NC}"
echo -e "${YELLOW}  1. pop-os/xdg-desktop-portal-cosmic${NC}"
echo -e "${YELLOW}  2. pop-os/cosmic-comp${NC}"
echo ""
echo -e "${YELLOW}See: docs/upstream-prs/INTEGRATION_GUIDE.md${NC}"


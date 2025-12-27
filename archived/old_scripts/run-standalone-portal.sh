#!/bin/bash
# run-standalone-portal.sh - Run ionChannel portal standalone
#
# This registers our RemoteDesktop implementation on D-Bus
# so we can test the interface, even though events won't
# actually reach the compositor.
#
# Usage:
#   ./scripts/run-standalone-portal.sh &
#   # Then in another terminal:
#   busctl --user introspect org.freedesktop.impl.portal.desktop.ionChannel /org/freedesktop/portal/desktop

set -euo pipefail

cd "$(dirname "$0")/.."

echo "Starting ionChannel standalone portal..."
echo ""
echo "This will register our RemoteDesktop interface on D-Bus."
echo "Events will be logged but NOT injected into COSMIC."
echo ""
echo "Press Ctrl+C to stop."
echo ""

# Build if needed
cargo build --release -p ion-test-substrate 2>/dev/null

# Run the validator which starts a test portal
RUST_LOG=info cargo run -p ion-test-substrate --release


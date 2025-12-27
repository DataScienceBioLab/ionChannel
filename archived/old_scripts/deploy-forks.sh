#!/bin/bash
# Deploy ionChannel forks to system

set -euo pipefail

echo "🚀 Deploying ionChannel Forks"
echo "============================"
echo ""
echo "⚠️  WARNING: This will replace system portal and compositor!"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
fi

# Build portal
echo ""
echo "🔨 Building portal..."
cd ~/Development/syntheticChemistry/cosmic-portal-fork
cargo build --release

# Build compositor
echo ""
echo "🔨 Building compositor..."
cd ~/Development/syntheticChemistry/cosmic-comp-fork
cargo build --release --features remote-desktop

# Backup originals
echo ""
echo "💾 Backing up originals..."
sudo cp /usr/libexec/xdg-desktop-portal-cosmic /usr/libexec/xdg-desktop-portal-cosmic.bak || true
sudo cp /usr/bin/cosmic-comp /usr/bin/cosmic-comp.bak || true

# Install forks
echo ""
echo "📥 Installing forks..."
sudo cp ~/Development/syntheticChemistry/cosmic-portal-fork/target/release/xdg-desktop-portal-cosmic /usr/libexec/
sudo cp ~/Development/syntheticChemistry/cosmic-comp-fork/target/release/cosmic-comp /usr/bin/

# Update portal config
sudo cp ~/Development/syntheticChemistry/cosmic-portal-fork/data/cosmic.portal /usr/share/xdg-desktop-portal/portals/

echo ""
echo "✅ Deployment complete!"
echo ""
echo "Next steps:"
echo "  1. Log out of COSMIC"
echo "  2. Log back in (will use new compositor)"
echo "  3. Run: busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop"
echo "  4. Look for RemoteDesktop interface"

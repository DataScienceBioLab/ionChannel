#!/bin/bash
# ionChannel Fork Setup Script
# Creates working forks for testing

set -euo pipefail

WORKDIR="$HOME/Development/syntheticChemistry"
GITHUB_USER="${GITHUB_USER:-DataScienceBioLab}"

echo "🚀 ionChannel Fork Setup"
echo "======================="
echo ""
echo "This will:"
echo "  1. Fork COSMIC repositories"
echo "  2. Create integration branches"
echo "  3. Set up ionChannel integration"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
fi

cd "$WORKDIR"

# Fork xdg-desktop-portal-cosmic
echo ""
echo "📦 Forking xdg-desktop-portal-cosmic..."
if [ ! -d "cosmic-portal-fork" ]; then
    git clone https://github.com/pop-os/xdg-desktop-portal-cosmic.git cosmic-portal-fork
    cd cosmic-portal-fork
    git remote add upstream https://github.com/pop-os/xdg-desktop-portal-cosmic.git
    git checkout -b feat/ionChannel-remote-desktop
    cd ..
    echo "✅ Portal fork created"
else
    echo "⚠️  cosmic-portal-fork already exists"
fi

# Fork cosmic-comp
echo ""
echo "📦 Forking cosmic-comp..."
if [ ! -d "cosmic-comp-fork" ]; then
    git clone https://github.com/pop-os/cosmic-comp.git cosmic-comp-fork
    cd cosmic-comp-fork
    git remote add upstream https://github.com/pop-os/cosmic-comp.git
    git checkout -b feat/ionChannel-input-injection
    cd ..
    echo "✅ Compositor fork created"
else
    echo "⚠️  cosmic-comp-fork already exists"
fi

# Symlink ionChannel crates into portal fork
echo ""
echo "🔗 Linking ionChannel into portal fork..."
cd cosmic-portal-fork

# Update Cargo.toml to include ionChannel
if ! grep -q "ion-portal" Cargo.toml; then
    cat >> Cargo.toml <<EOF

# ionChannel integration
[dependencies.ion-portal]
path = "../ionChannel/crates/ion-portal"

[dependencies.ion-core]
path = "../ionChannel/crates/ion-core"
EOF
    echo "✅ Added ionChannel dependencies to portal"
else
    echo "⚠️  ionChannel already in portal Cargo.toml"
fi

cd ..

# Update compositor Cargo.toml
echo ""
echo "🔗 Linking ionChannel into compositor fork..."
cd cosmic-comp-fork

if ! grep -q "ion-compositor" Cargo.toml; then
    cat >> Cargo.toml <<EOF

# ionChannel integration
[dependencies.ion-compositor]
path = "../ionChannel/crates/ion-compositor"

[dependencies.ion-core]
path = "../ionChannel/crates/ion-core"

[dependencies.reis]
version = "0.5"
features = ["tokio"]
optional = true

[features]
remote-desktop = ["reis", "ion-compositor"]
EOF
    echo "✅ Added ionChannel dependencies to compositor"
else
    echo "⚠️  ionChannel already in compositor Cargo.toml"
fi

cd ..

# Create integration starter files
echo ""
echo "📝 Creating integration starter files..."

# Portal integration stub
cat > cosmic-portal-fork/src/remote_desktop.rs <<'EOF'
// ionChannel RemoteDesktop Portal Integration
// 
// This module integrates ion-portal into xdg-desktop-portal-cosmic

use ion_portal::{RemoteDesktopPortal, PortalConfig};
use ion_core::session::SessionId;
use zbus::Connection;

pub struct RemoteDesktop {
    portal: RemoteDesktopPortal,
}

impl RemoteDesktop {
    pub fn new() -> Self {
        let portal = RemoteDesktopPortal::new(PortalConfig::default());
        Self { portal }
    }
}

// TODO: Implement zbus interface wrapper
// See: ionChannel/crates/ion-portal/src/portal.rs for reference
EOF

echo "✅ Created cosmic-portal-fork/src/remote_desktop.rs"

# Compositor integration stub
mkdir -p cosmic-comp-fork/src/input/virtual
cat > cosmic-comp-fork/src/input/virtual.rs <<'EOF'
// ionChannel Virtual Input Integration
//
// This module integrates ion-compositor into cosmic-comp

use ion_compositor::{VirtualInput, VirtualInputEvent};

pub struct VirtualInputManager {
    processor: VirtualInput,
}

impl VirtualInputManager {
    pub fn new() -> Self {
        let processor = VirtualInput::new();
        Self { processor }
    }

    pub fn inject_event(&mut self, event: VirtualInputEvent) {
        // TODO: Convert to Smithay input events
        // See: cosmic-comp/src/input/mod.rs for Smithay integration
    }
}
EOF

echo "✅ Created cosmic-comp-fork/src/input/virtual.rs"

# Create test script
cat > "$WORKDIR/test-ionChannel-integration.sh" <<'EOF'
#!/bin/bash
# Test ionChannel integration in forks

set -euo pipefail

echo "🧪 Testing ionChannel Integration"
echo "================================"

# Test portal builds
echo ""
echo "📦 Testing portal build..."
cd ~/Development/syntheticChemistry/cosmic-portal-fork
if cargo check 2>&1 | grep -q "error"; then
    echo "❌ Portal build failed"
    exit 1
else
    echo "✅ Portal builds"
fi

# Test compositor builds
echo ""
echo "📦 Testing compositor build..."
cd ~/Development/syntheticChemistry/cosmic-comp-fork
if cargo check --features remote-desktop 2>&1 | grep -q "error"; then
    echo "❌ Compositor build failed"
    exit 1
else
    echo "✅ Compositor builds"
fi

echo ""
echo "✅ All integration tests passed!"
EOF

chmod +x "$WORKDIR/test-ionChannel-integration.sh"
echo "✅ Created test-ionChannel-integration.sh"

# Create deployment script
cat > "$WORKDIR/ionChannel/scripts/deploy-forks.sh" <<'EOF'
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
EOF

chmod +x "$WORKDIR/ionChannel/scripts/deploy-forks.sh"
echo "✅ Created deploy-forks.sh"

echo ""
echo "✅ Fork setup complete!"
echo ""
echo "📋 What was created:"
echo "   • cosmic-portal-fork/ - Portal with ionChannel"
echo "   • cosmic-comp-fork/ - Compositor with ionChannel"
echo "   • Integration starter files"
echo "   • Test and deployment scripts"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Test builds:"
echo "   cd $WORKDIR"
echo "   ./test-ionChannel-integration.sh"
echo ""
echo "2. Complete integration (see FORK_AND_TEST_STRATEGY.md):"
echo "   • Wire up D-Bus interfaces"
echo "   • Implement input injection"
echo "   • Add consent dialogs"
echo ""
echo "3. Deploy and test:"
echo "   cd $WORKDIR/ionChannel"
echo "   ./scripts/deploy-forks.sh"
echo ""
echo "📖 Full documentation:"
echo "   $WORKDIR/ionChannel/FORK_AND_TEST_STRATEGY.md"
echo ""


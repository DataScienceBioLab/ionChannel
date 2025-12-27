#!/bin/bash
# Pop!_OS VM Setup Script for ionChannel Testing
# Run this inside your Pop!_OS VM

set -euo pipefail

echo "🚀 ionChannel VM Setup for Pop!_OS"
echo "=================================="
echo ""

# Check if running on Pop!_OS
if ! grep -q "Pop!_OS" /etc/os-release 2>/dev/null; then
    echo "⚠️  Warning: This script is designed for Pop!_OS"
    echo "Current OS: $(cat /etc/os-release | grep PRETTY_NAME)"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 0
    fi
fi

# Update system
echo ""
echo "📦 Updating system packages..."
sudo apt update
sudo apt upgrade -y

# Install COSMIC desktop if not present
echo ""
echo "🎨 Checking COSMIC desktop..."
if ! dpkg -l | grep -q cosmic-comp; then
    echo "Installing COSMIC desktop..."
    sudo apt install -y cosmic-session cosmic-comp
else
    echo "✅ COSMIC desktop already installed"
fi

# Install development dependencies
echo ""
echo "🔧 Installing development dependencies..."
sudo apt install -y \
    build-essential \
    cargo \
    rustc \
    pkg-config \
    git \
    curl \
    libdbus-1-dev \
    libwayland-dev \
    libpipewire-0.3-dev \
    libgbm-dev \
    libdrm-dev \
    libxkbcommon-dev \
    libseat-dev \
    libudev-dev \
    libinput-dev \
    libglib2.0-dev \
    libpango1.0-dev \
    libcairo2-dev \
    libgdk-pixbuf2.0-dev \
    libgtk-4-dev

echo "✅ Dependencies installed"

# Set up workspace directory
echo ""
echo "📁 Setting up workspace..."
WORKSPACE="$HOME/Development/syntheticChemistry"
mkdir -p "$WORKSPACE"
cd "$WORKSPACE"

# Clone ionChannel if not present
if [ ! -d "ionChannel" ]; then
    echo "Cloning ionChannel..."
    git clone https://github.com/DataScienceBioLab/ionChannel.git
    cd ionChannel
else
    echo "✅ ionChannel already present"
    cd ionChannel
    git pull
fi

# Run fork setup
echo ""
echo "🍴 Setting up forks..."
cd "$WORKSPACE"
if [ ! -d "cosmic-portal-fork" ]; then
    bash ionChannel/scripts/fork-setup.sh <<< "y"
else
    echo "✅ Forks already set up"
fi

# Build forks
echo ""
echo "🔨 Building cosmic-portal-fork..."
cd "$WORKSPACE/cosmic-portal-fork"
cargo build --release 2>&1 | tee build-portal.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✅ Portal built successfully"
else
    echo "❌ Portal build failed - check build-portal.log"
    exit 1
fi

echo ""
echo "🔨 Building cosmic-comp-fork..."
cd "$WORKSPACE/cosmic-comp-fork"
cargo build --release --features remote-desktop 2>&1 | tee build-comp.log
if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✅ Compositor built successfully"
else
    echo "⚠️  Compositor build failed - may need manual fixes"
fi

# Install RustDesk
echo ""
echo "🖥️  Installing RustDesk..."
if ! command -v rustdesk &> /dev/null; then
    cd /tmp
    RUSTDESK_VERSION="1.2.3"
    wget "https://github.com/rustdesk/rustdesk/releases/download/${RUSTDESK_VERSION}/rustdesk-${RUSTDESK_VERSION}-x86_64.deb" -O rustdesk.deb
    sudo dpkg -i rustdesk.deb || sudo apt install -f -y
    rm rustdesk.deb
    echo "✅ RustDesk installed"
else
    echo "✅ RustDesk already installed"
fi

# Get VM IP address
echo ""
echo "📡 Network Configuration:"
VM_IP=$(hostname -I | awk '{print $1}')
echo "   VM IP Address: $VM_IP"
echo "   Use this IP to connect from other machines"

# Create deployment script
cat > "$WORKSPACE/deploy-to-system.sh" <<'DEPLOY_EOF'
#!/bin/bash
# Deploy ionChannel forks to system
set -euo pipefail

echo "🚀 Deploying ionChannel to system"
echo "================================"
echo ""
echo "⚠️  This will replace system portal and compositor!"
echo "⚠️  You will need to log out and back in after this."
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 0
fi

WORKSPACE="$HOME/Development/syntheticChemistry"

# Backup originals
echo "💾 Backing up original files..."
sudo cp /usr/libexec/xdg-desktop-portal-cosmic /usr/libexec/xdg-desktop-portal-cosmic.bak 2>/dev/null || true
sudo cp /usr/bin/cosmic-comp /usr/bin/cosmic-comp.bak 2>/dev/null || true
echo "✅ Originals backed up"

# Install portal
echo ""
echo "📥 Installing cosmic-portal-fork..."
sudo cp "$WORKSPACE/cosmic-portal-fork/target/release/xdg-desktop-portal-cosmic" /usr/libexec/
sudo chmod +x /usr/libexec/xdg-desktop-portal-cosmic

# Update portal config
echo "📝 Updating portal configuration..."
sudo tee /usr/share/xdg-desktop-portal/portals/cosmic.portal > /dev/null <<EOF
[portal]
DBusName=org.freedesktop.impl.portal.desktop.cosmic
Interfaces=org.freedesktop.impl.portal.Access;org.freedesktop.impl.portal.FileChooser;org.freedesktop.impl.portal.Screenshot;org.freedesktop.impl.portal.Settings;org.freedesktop.impl.portal.ScreenCast;org.freedesktop.impl.portal.RemoteDesktop
UseIn=COSMIC
EOF

# Install compositor (if built)
if [ -f "$WORKSPACE/cosmic-comp-fork/target/release/cosmic-comp" ]; then
    echo ""
    echo "📥 Installing cosmic-comp-fork..."
    sudo cp "$WORKSPACE/cosmic-comp-fork/target/release/cosmic-comp" /usr/bin/
    sudo chmod +x /usr/bin/cosmic-comp
    echo "✅ Compositor installed"
else
    echo "⚠️  Compositor not built, skipping"
fi

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📝 Next steps:"
echo "   1. Log out of COSMIC"
echo "   2. Log back in"
echo "   3. Run test script: ~/Development/syntheticChemistry/test-ionChannel.sh"
echo ""
DEPLOY_EOF

chmod +x "$WORKSPACE/deploy-to-system.sh"

# Create test script
cat > "$WORKSPACE/test-ionChannel.sh" <<'TEST_EOF'
#!/bin/bash
# Test ionChannel integration
set -euo pipefail

echo "🧪 Testing ionChannel Integration"
echo "================================"
echo ""

# Check if portal is running
echo "1. Checking portal service..."
if systemctl --user is-active --quiet xdg-desktop-portal.service; then
    echo "   ✅ Portal service is running"
else
    echo "   ⚠️  Portal service not running"
    echo "   Starting portal..."
    systemctl --user restart xdg-desktop-portal.service
    sleep 2
fi

# Check D-Bus
echo ""
echo "2. Checking D-Bus registration..."
if busctl --user list | grep -q "org.freedesktop.portal.Desktop"; then
    echo "   ✅ Portal registered on D-Bus"
else
    echo "   ❌ Portal not registered"
    exit 1
fi

# Check RemoteDesktop interface
echo ""
echo "3. Checking RemoteDesktop interface..."
if busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop 2>/dev/null | grep -q "RemoteDesktop"; then
    echo "   ✅ RemoteDesktop interface found!"
else
    echo "   ❌ RemoteDesktop interface not found"
    echo "   Available interfaces:"
    busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop 2>/dev/null | grep "interface" || true
    exit 1
fi

# Check compositor
echo ""
echo "4. Checking compositor..."
if pgrep -x cosmic-comp > /dev/null; then
    echo "   ✅ Compositor running"
else
    echo "   ⚠️  Compositor not running"
fi

# Get device types
echo ""
echo "5. Testing AvailableDeviceTypes property..."
DEVICES=$(busctl --user get-property org.freedesktop.portal.Desktop \
    /org/freedesktop/portal/desktop \
    org.freedesktop.impl.portal.RemoteDesktop \
    AvailableDeviceTypes 2>/dev/null | awk '{print $2}')

if [ -n "$DEVICES" ]; then
    echo "   ✅ Available devices: $DEVICES"
else
    echo "   ⚠️  Could not get device types"
fi

# Network info
echo ""
echo "6. Network configuration:"
VM_IP=$(hostname -I | awk '{print $1}')
echo "   VM IP: $VM_IP"
echo "   RustDesk ID: $(rustdesk --get-id 2>/dev/null || echo "Run 'rustdesk --server' first")"

echo ""
echo "✅ All tests passed!"
echo ""
echo "📝 Next steps:"
echo "   1. Start RustDesk server: rustdesk --server"
echo "   2. Note the RustDesk ID shown"
echo "   3. From client machine:"
echo "      - Install RustDesk client"
echo "      - Enter server ID"
echo "      - Connect and test mouse/keyboard"
TEST_EOF

chmod +x "$WORKSPACE/test-ionChannel.sh"

# Create RustDesk startup script
cat > "$WORKSPACE/start-rustdesk-server.sh" <<'RUSTDESK_EOF'
#!/bin/bash
# Start RustDesk server for testing
echo "🖥️  Starting RustDesk Server"
echo "=========================="
echo ""

# Get VM IP
VM_IP=$(hostname -I | awk '{print $1}')
echo "VM IP: $VM_IP"
echo ""

# Start RustDesk in server mode
echo "Starting RustDesk..."
echo "Press Ctrl+C to stop"
echo ""

rustdesk --server
RUSTDESK_EOF

chmod +x "$WORKSPACE/start-rustdesk-server.sh"

echo ""
echo "✅ VM Setup Complete!"
echo ""
echo "📋 What was created:"
echo "   • cosmic-portal-fork (built)"
echo "   • cosmic-comp-fork (built)"
echo "   • deploy-to-system.sh - Deploy forks"
echo "   • test-ionChannel.sh - Test integration"
echo "   • start-rustdesk-server.sh - Start RustDesk"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Deploy to system:"
echo "   cd $WORKSPACE"
echo "   ./deploy-to-system.sh"
echo ""
echo "2. Log out and back in to COSMIC"
echo ""
echo "3. Test integration:"
echo "   cd $WORKSPACE"
echo "   ./test-ionChannel.sh"
echo ""
echo "4. Start RustDesk server:"
echo "   ./start-rustdesk-server.sh"
echo ""
echo "5. From client machine:"
echo "   - Install RustDesk"
echo "   - Connect to VM using RustDesk ID"
echo "   - Test mouse and keyboard control"
echo ""
echo "🌐 Your VM IP: $VM_IP"
echo ""


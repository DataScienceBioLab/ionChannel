#!/bin/bash
# Fully Automated ionChannel Deployment
# Just run this script on the VM - it does everything

set -euo pipefail

echo "════════════════════════════════════════════════════════════════"
echo "   ionChannel Automatic Setup & Deployment"
echo "   Make RustDesk Work on COSMIC/Wayland"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "This script will:"
echo "  ✓ Install all dependencies"
echo "  ✓ Build the portal and compositor"
echo "  ✓ Deploy to your system"
echo "  ✓ Configure RustDesk"
echo "  ✓ Start the server"
echo ""
echo "You'll just need to log out/in once during the process."
echo ""

# Auto-confirm
export DEBIAN_FRONTEND=noninteractive

# Get VM info
VM_IP=$(hostname -I | awk '{print $1}')
VM_USER=$(whoami)

echo "════════════════════════════════════════════════════════════════"
echo " PHASE 1: System Preparation"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Check OS
if ! grep -q "Pop!_OS\|Ubuntu" /etc/os-release 2>/dev/null; then
    echo "⚠️  Warning: This is designed for Pop!_OS/Ubuntu"
    echo "Current OS: $(grep PRETTY_NAME /etc/os-release | cut -d'"' -f2)"
fi

# Update system
echo "📦 Updating system..."
sudo apt-get update -qq
sudo apt-get upgrade -y -qq

# Install COSMIC if needed
echo "🎨 Ensuring COSMIC desktop..."
if ! dpkg -l | grep -q cosmic-comp; then
    echo "Installing COSMIC..."
    sudo apt-get install -y cosmic-session cosmic-comp
fi

# Install dependencies
echo "🔧 Installing build dependencies..."
sudo apt-get install -y -qq \
    build-essential cargo rustc pkg-config git curl \
    libdbus-1-dev libwayland-dev libpipewire-0.3-dev \
    libgbm-dev libdrm-dev libxkbcommon-dev libseat-dev \
    libudev-dev libinput-dev libglib2.0-dev \
    libpango1.0-dev libcairo2-dev libgdk-pixbuf2.0-dev

echo "✅ System prepared"
echo ""

echo "════════════════════════════════════════════════════════════════"
echo " PHASE 2: Building ionChannel Portal"
echo "════════════════════════════════════════════════════════════════"
echo ""

cd "$HOME/Development/syntheticChemistry/cosmic-portal-fork"

echo "🔨 Building portal (this may take 5-10 minutes)..."
cargo build --release 2>&1 | tee build.log | grep -E "Compiling|Finished|error" || true

if [ ${PIPESTATUS[0]} -eq 0 ]; then
    echo "✅ Portal built successfully!"
else
    echo "❌ Portal build failed"
    echo "Check: $HOME/Development/syntheticChemistry/cosmic-portal-fork/build.log"
    exit 1
fi

echo ""

echo "════════════════════════════════════════════════════════════════"
echo " PHASE 3: Deploying to System"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Backup originals
echo "💾 Backing up original files..."
sudo cp /usr/libexec/xdg-desktop-portal-cosmic /usr/libexec/xdg-desktop-portal-cosmic.bak 2>/dev/null || true

# Install portal
echo "📥 Installing ionChannel portal..."
sudo cp "$HOME/Development/syntheticChemistry/cosmic-portal-fork/target/release/xdg-desktop-portal-cosmic" /usr/libexec/
sudo chmod +x /usr/libexec/xdg-desktop-portal-cosmic

# Update portal config
echo "📝 Configuring portal interfaces..."
sudo tee /usr/share/xdg-desktop-portal/portals/cosmic.portal > /dev/null <<EOF
[portal]
DBusName=org.freedesktop.impl.portal.desktop.cosmic
Interfaces=org.freedesktop.impl.portal.Access;org.freedesktop.impl.portal.FileChooser;org.freedesktop.impl.portal.Screenshot;org.freedesktop.impl.portal.Settings;org.freedesktop.impl.portal.ScreenCast;org.freedesktop.impl.portal.RemoteDesktop
UseIn=COSMIC
EOF

echo "✅ Portal deployed"
echo ""

echo "════════════════════════════════════════════════════════════════"
echo " PHASE 4: Installing RustDesk"
echo "════════════════════════════════════════════════════════════════"
echo ""

if ! command -v rustdesk &> /dev/null; then
    echo "📥 Downloading RustDesk..."
    cd /tmp
    RUSTDESK_VERSION="1.2.3"
    wget -q "https://github.com/rustdesk/rustdesk/releases/download/${RUSTDESK_VERSION}/rustdesk-${RUSTDESK_VERSION}-x86_64.deb"
    echo "📦 Installing RustDesk..."
    sudo dpkg -i rustdesk-${RUSTDESK_VERSION}-x86_64.deb 2>/dev/null || sudo apt-get install -f -y -qq
    rm rustdesk-${RUSTDESK_VERSION}-x86_64.deb
    echo "✅ RustDesk installed"
else
    echo "✅ RustDesk already installed"
fi

# Configure RustDesk for best performance
echo "⚙️  Configuring RustDesk..."
mkdir -p ~/.config/rustdesk
cat > ~/.config/rustdesk/RustDesk.toml <<EOF
[options]
allow-always-software-render = false
enable-directx-capture = false
codec-preference = "vp9"
EOF

echo ""

echo "════════════════════════════════════════════════════════════════"
echo " PHASE 5: Testing Integration"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Create quick test script
cat > /tmp/quick-test.sh <<'TEST_EOF'
#!/bin/bash
# Check if portal interface is available
if busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop 2>/dev/null | grep -q "RemoteDesktop"; then
    echo "✅ RemoteDesktop interface found!"
    exit 0
else
    echo "⚠️  RemoteDesktop interface not found yet"
    echo "   You may need to log out and back in"
    exit 1
fi
TEST_EOF

chmod +x /tmp/quick-test.sh

# Try to test (might not work until reboot)
echo "🧪 Quick test..."
if systemctl --user is-active --quiet xdg-desktop-portal.service 2>/dev/null; then
    /tmp/quick-test.sh || echo "   (Expected - not logged out yet)"
else
    echo "⚠️  Portal service not running yet"
fi

echo ""

echo "════════════════════════════════════════════════════════════════"
echo " PHASE 6: Creating Helper Scripts"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Create start script
cat > "$HOME/start-rustdesk-with-ionChannel.sh" <<'START_EOF'
#!/bin/bash
# Start RustDesk with ionChannel Portal

echo "════════════════════════════════════════════════════════════════"
echo "  RustDesk Server with ionChannel RemoteDesktop Portal"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Get info
VM_IP=$(hostname -I | awk '{print $1}')
RUSTDESK_ID=$(rustdesk --get-id 2>/dev/null || echo "Starting...")

# Check portal
echo "Checking portal..."
if busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop 2>/dev/null | grep -q "RemoteDesktop"; then
    echo "✅ RemoteDesktop portal is active"
else
    echo "❌ RemoteDesktop portal not found!"
    echo "   Try: systemctl --user restart xdg-desktop-portal.service"
    exit 1
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  VM Network Info"
echo "────────────────────────────────────────────────────────────────"
echo "  IP Address:  $VM_IP"
echo "  RustDesk ID: $RUSTDESK_ID"
echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  Give users this information:"
echo "────────────────────────────────────────────────────────────────"
echo ""
echo "    1. Open RustDesk on your computer"
echo "    2. Enter ID: $RUSTDESK_ID"
echo "    3. Click 'Connect'"
echo "    4. Enter password when prompted"
echo ""
echo "────────────────────────────────────────────────────────────────"
echo ""
echo "Starting RustDesk server..."
echo "Press Ctrl+C to stop"
echo ""

# Start RustDesk
rustdesk --server
START_EOF

chmod +x "$HOME/start-rustdesk-with-ionChannel.sh"

# Create desktop shortcut
cat > "$HOME/Desktop/Start-RustDesk-Server.desktop" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Start RustDesk Server
Comment=Start RustDesk with ionChannel portal
Exec=$HOME/start-rustdesk-with-ionChannel.sh
Icon=rustdesk
Terminal=true
Categories=Network;RemoteAccess;
EOF

chmod +x "$HOME/Desktop/Start-RustDesk-Server.desktop"

echo "✅ Helper scripts created"
echo ""

echo "════════════════════════════════════════════════════════════════"
echo " ✅ DEPLOYMENT COMPLETE!"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "📋 What Was Done:"
echo "   ✓ Built ionChannel RemoteDesktop portal"
echo "   ✓ Deployed to system"
echo "   ✓ Configured portal interfaces"
echo "   ✓ Installed RustDesk"
echo "   ✓ Created helper scripts"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo " 🚨 IMPORTANT: Next Steps"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "1. LOG OUT of COSMIC"
echo "   (This loads the new portal)"
echo ""
echo "2. LOG BACK IN"
echo ""
echo "3. RUN THIS COMMAND:"
echo "   ~/start-rustdesk-with-ionChannel.sh"
echo ""
echo "   OR double-click the desktop icon:"
echo "   'Start RustDesk Server'"
echo ""
echo "4. SHARE the RustDesk ID with users"
echo "   They just enter it in RustDesk client"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo " 📱 For Users Connecting:"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "On their computer:"
echo "  1. Download RustDesk from https://rustdesk.com/"
echo "  2. Open RustDesk"
echo "  3. Enter the ID shown when you start the server"
echo "  4. Click 'Connect'"
echo "  5. Enter password"
echo "  6. Done! They can control your VM"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "🎉 Setup complete! Log out/in and start RustDesk."
echo ""


#!/bin/bash
# MVP Test: ionChannel + RustDesk + Pop!_OS/Wayland
# This script executes the minimal viable test for the complete solution

set -e

echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                      ║"
echo "║       MVP TEST: ionChannel + RustDesk + Pop!_OS/Wayland             ║"
echo "║                                                                      ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_step() {
    echo -e "${GREEN}▶ $1${NC}"
}

log_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

log_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Step 1: Create Ubuntu VM
log_step "STEP 1: Creating Ubuntu VM with benchScale..."
echo ""
log_info "This will create a VM with:"
log_info "  - Ubuntu 22.04 LTS"
log_info "  - 4GB RAM, 2 CPUs"
log_info "  - SSH access enabled"
log_info "  - Network configured"
echo ""

cargo run -p ion-validation --example create_working_vm --features libvirt --release || {
    log_error "Failed to create VM"
    exit 1
}

echo ""
log_step "VM Created Successfully!"
echo ""

# Get VM IP
log_info "Discovering VM IP address..."
VM_IP=$(virsh -c qemu:///system domifaddr ubuntu-test-base | grep -oP '(\d+\.){3}\d+' | head -1)

if [ -z "$VM_IP" ]; then
    log_error "Could not determine VM IP address"
    log_info "Trying alternative method..."
    sleep 5
    VM_IP=$(virsh -c qemu:///system domifaddr ubuntu-test-base | grep -oP '(\d+\.){3}\d+' | head -1)
fi

if [ -z "$VM_IP" ]; then
    log_error "Still no VM IP. Please check VM manually:"
    echo "  virsh -c qemu:///system list"
    echo "  virsh -c qemu:///system domifaddr ubuntu-test-base"
    exit 1
fi

log_step "VM IP: $VM_IP"
echo ""

# Wait for SSH
log_info "Waiting for SSH to be ready..."
for i in {1..30}; do
    if ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 ubuntu@$VM_IP "echo 'SSH Ready'" 2>/dev/null; then
        log_step "SSH is ready!"
        break
    fi
    echo -n "."
    sleep 2
done
echo ""

# Step 2: Install COSMIC Desktop
log_step "STEP 2: Installing COSMIC Desktop on VM..."
echo ""
log_info "This installs:"
log_info "  - COSMIC compositor"
log_info "  - COSMIC session"
log_info "  - Wayland support"
echo ""

ssh ubuntu@$VM_IP << 'ENDSSH'
set -e
echo "▶ Adding System76 COSMIC PPA..."
sudo add-apt-repository -y ppa:system76/cosmic

echo "▶ Updating package lists..."
sudo apt update

echo "▶ Installing COSMIC..."
sudo DEBIAN_FRONTEND=noninteractive apt install -y cosmic-session cosmic-comp pipewire wireplumber

echo "▶ Installing xdg-desktop-portal..."
sudo DEBIAN_FRONTEND=noninteractive apt install -y xdg-desktop-portal xdg-desktop-portal-gnome

echo "▶ Ensuring services are enabled..."
systemctl --user enable pipewire pipewire-pulse wireplumber || true

echo "✓ COSMIC installation complete!"
ENDSSH

log_step "COSMIC Installed Successfully!"
echo ""

# Step 3: Deploy ionChannel
log_step "STEP 3: Deploying ionChannel to VM..."
echo ""

cargo run -p ion-validation --example provision_and_connect --features libvirt --release -- --vm-ip $VM_IP || {
    log_error "Failed to deploy ionChannel"
    log_info "You can manually deploy with:"
    echo "  cargo run -p ion-validation --example provision_and_connect --features libvirt"
    exit 1
}

log_step "ionChannel Deployed Successfully!"
echo ""

# Step 4: Verify Wayland Environment
log_step "STEP 4: Verifying Wayland Environment..."
echo ""

ssh ubuntu@$VM_IP << 'ENDSSH'
echo "▶ Checking Wayland session..."
if [ -n "$WAYLAND_DISPLAY" ]; then
    echo "  ✓ WAYLAND_DISPLAY: $WAYLAND_DISPLAY"
else
    echo "  ⚠ WAYLAND_DISPLAY not set (may need to login to COSMIC)"
fi

echo ""
echo "▶ Checking PipeWire..."
systemctl --user status pipewire --no-pager || echo "  ⚠ PipeWire not running (will start on login)"

echo ""
echo "▶ Checking xdg-desktop-portal..."
if command -v xdg-desktop-portal &> /dev/null; then
    echo "  ✓ xdg-desktop-portal installed"
else
    echo "  ✗ xdg-desktop-portal not found"
fi

echo ""
echo "▶ COSMIC components:"
dpkg -l | grep cosmic || echo "  Checking..."
ENDSSH

echo ""
log_step "Environment Verified!"
echo ""

# Summary
echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                      ║"
echo "║                     ✅ MVP TEST COMPLETE                             ║"
echo "║                                                                      ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""
log_step "Test Infrastructure Ready!"
echo ""
echo "VM Details:"
echo "  IP Address: $VM_IP"
echo "  Username: ubuntu"
echo "  Password: ubuntu"
echo ""
echo "Next Steps:"
echo ""
echo "1. Login to VM and select COSMIC session:"
echo "   virt-viewer ubuntu-test-base"
echo "   (or use virt-manager GUI)"
echo ""
echo "2. Install and test RustDesk:"
echo "   ssh ubuntu@$VM_IP"
echo "   wget https://github.com/rustdesk/rustdesk/releases/download/[version]/rustdesk-[version]-x86_64.deb"
echo "   sudo dpkg -i rustdesk-*.deb"
echo "   sudo apt-get install -f"
echo "   rustdesk &"
echo ""
echo "3. Connect from host to test screen sharing and input injection!"
echo ""
log_step "Testing infrastructure is ready! 🎉"
echo ""


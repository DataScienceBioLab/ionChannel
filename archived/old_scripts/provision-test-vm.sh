#!/bin/bash
# Provision a test VM with ionChannel pre-installed
# User can then connect to it via RustDesk

set -euo pipefail

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

VM_NAME="${1:-ionChannel-test}"
VM_MEMORY="${2:-4096}"  # 4GB
VM_CPUS="${3:-2}"
VM_DISK="${4:-20G}"

echo "════════════════════════════════════════════════════════════════"
echo "  🤖 ionChannel VM Provisioning"
echo "  Creating test VM with ionChannel pre-installed"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  VM Name:   $VM_NAME"
echo "  Memory:    ${VM_MEMORY}MB"
echo "  CPUs:      $VM_CPUS"
echo "  Disk:      $VM_DISK"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo ""

# Check dependencies
echo -e "${BLUE}[1/8]${NC} Checking dependencies..."

MISSING_DEPS=()

# Check libvirt daemon
if ! systemctl is-active --quiet libvirtd; then
    echo -e "${YELLOW}⚠️  libvirtd not running${NC}"
    echo "  Starting libvirtd..."
    sudo systemctl start libvirtd
fi

if ! command -v virt-install &> /dev/null; then
    MISSING_DEPS+=("virt-install")
fi

if ! command -v virsh &> /dev/null; then
    MISSING_DEPS+=("libvirt")
fi

if ! command -v qemu-system-x86_64 &> /dev/null; then
    MISSING_DEPS+=("qemu")
fi

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Missing dependencies: ${MISSING_DEPS[*]}${NC}"
    echo ""
    echo "Install with:"
    echo "  sudo apt install -y qemu-kvm libvirt-daemon-system libvirt-clients bridge-utils virt-manager virtinst"
    echo ""
    exit 1
fi

echo -e "${GREEN}✓${NC} All dependencies installed"
echo ""

# Check/create default network
echo "Checking libvirt network..."
if ! virsh net-list --all | grep -q "default"; then
    echo "  → Creating default network..."
    cat > /tmp/default-network.xml << 'NETXML'
<network>
  <name>default</name>
  <forward mode='nat'/>
  <bridge name='virbr0' stp='on' delay='0'/>
  <ip address='192.168.122.1' netmask='255.255.255.0'>
    <dhcp>
      <range start='192.168.122.2' end='192.168.122.254'/>
    </dhcp>
  </ip>
</network>
NETXML
    virsh net-define /tmp/default-network.xml
    rm /tmp/default-network.xml
fi

if ! virsh net-list | grep -q "default.*active"; then
    echo "  → Starting default network..."
    virsh net-start default
    virsh net-autostart default
fi

echo -e "${GREEN}✓${NC} Network ready"
echo ""

# Check if VM already exists
if virsh list --all | grep -q "$VM_NAME"; then
    echo -e "${YELLOW}⚠️  VM '$VM_NAME' already exists${NC}"
    echo ""
    read -p "Delete and recreate? [y/N]: " recreate
    if [[ $recreate =~ ^[Yy]$ ]]; then
        echo "  → Destroying existing VM..."
        virsh destroy "$VM_NAME" 2>/dev/null || true
        virsh undefine "$VM_NAME" --remove-all-storage 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Old VM removed"
    else
        echo "Using existing VM"
        VM_EXISTS=true
    fi
fi

if [ "${VM_EXISTS:-false}" = "false" ]; then
    echo -e "${BLUE}[2/8]${NC} Checking for Pop!_OS ISO..."
    
    ISO_DIR="$HOME/Downloads"
    ISO_PATH=""
    
    # Look for Pop!_OS ISO
    for iso in "$ISO_DIR"/pop-os*.iso "$ISO_DIR"/Pop_OS*.iso; do
        if [ -f "$iso" ]; then
            ISO_PATH="$iso"
            break
        fi
    done
    
    if [ -z "$ISO_PATH" ]; then
        echo -e "${YELLOW}⚠️  No Pop!_OS ISO found in ~/Downloads${NC}"
        echo ""
        echo "Please download Pop!_OS:"
        echo "  https://pop.system76.com/"
        echo ""
        echo "Or use cloud-init for faster provisioning:"
        echo "  ./provision-test-vm.sh --cloud-init"
        echo ""
        exit 1
    fi
    
    echo -e "${GREEN}✓${NC} Found ISO: $(basename "$ISO_PATH")"
    echo ""
    
    echo -e "${BLUE}[3/8]${NC} Creating VM..."
    
    virt-install \
        --name "$VM_NAME" \
        --memory "$VM_MEMORY" \
        --vcpus "$VM_CPUS" \
        --disk size=${VM_DISK%G} \
        --cdrom "$ISO_PATH" \
        --os-variant ubuntu22.04 \
        --network network=default \
        --graphics spice \
        --noautoconsole \
        --wait -1 &
    
    VIRT_PID=$!
    
    echo -e "${GREEN}✓${NC} VM creation started (PID: $VIRT_PID)"
    echo ""
    echo "══════════════════════════════════════════════════════════════"
    echo -e "${YELLOW}  ⚠️  MANUAL INSTALLATION REQUIRED${NC}"
    echo "══════════════════════════════════════════════════════════════"
    echo ""
    echo "  1. Open virt-manager:"
    echo "     virt-manager"
    echo ""
    echo "  2. Connect to '$VM_NAME' console"
    echo ""
    echo "  3. Install Pop!_OS with these settings:"
    echo "     - Username: iontest"
    echo "     - Hostname: ionChannel-vm"
    echo "     - Install SSH server"
    echo ""
    echo "  4. After installation, SSH is enabled automatically"
    echo ""
    echo "══════════════════════════════════════════════════════════════"
    echo ""
    echo "Press Enter when installation is complete..."
    read
    
    # Wait for VM to be running
    echo "Waiting for VM to start..."
    for i in {1..30}; do
        if virsh list | grep -q "$VM_NAME.*running"; then
            break
        fi
        sleep 2
    done
fi

echo ""
echo -e "${BLUE}[4/8]${NC} Getting VM IP address..."

VM_IP=""
for i in {1..30}; do
    VM_IP=$(virsh domifaddr "$VM_NAME" 2>/dev/null | grep -oP '(\d+\.){3}\d+' | head -1)
    if [ -n "$VM_IP" ]; then
        break
    fi
    echo "  Waiting for network... (attempt $i/30)"
    sleep 2
done

if [ -z "$VM_IP" ]; then
    echo -e "${YELLOW}⚠️  Could not get VM IP automatically${NC}"
    echo ""
    read -p "Enter VM IP manually: " VM_IP
fi

echo -e "${GREEN}✓${NC} VM IP: $VM_IP"
echo ""

echo -e "${BLUE}[5/8]${NC} Waiting for SSH to be available..."

SSH_USER="${SSH_USER:-iontest}"

for i in {1..60}; do
    if ssh -o ConnectTimeout=2 -o StrictHostKeyChecking=no "$SSH_USER@$VM_IP" "exit" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} SSH is ready"
        break
    fi
    echo "  Waiting for SSH... (attempt $i/60)"
    sleep 5
done

echo ""
echo -e "${BLUE}[6/8]${NC} Installing dependencies on VM..."

ssh "$SSH_USER@$VM_IP" "sudo apt update && sudo apt install -y \
    build-essential \
    pkg-config \
    libdbus-1-dev \
    libwayland-dev \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libei-dev \
    libeis-dev \
    rustdesk \
    curl \
    git" 2>&1 | tail -20

echo -e "${GREEN}✓${NC} Dependencies installed"
echo ""

echo -e "${BLUE}[7/8]${NC} Installing Rust and deploying ionChannel..."

# Install Rust
ssh "$SSH_USER@$VM_IP" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y" 2>&1 | tail -5

# Create directory
ssh "$SSH_USER@$VM_IP" "mkdir -p ~/Development/syntheticChemistry"

# Transfer ionChannel
echo "  → Transferring ionChannel..."
rsync -az --info=progress2 \
    "$(dirname "$0")/../" \
    "$SSH_USER@$VM_IP:~/Development/syntheticChemistry/ionChannel/" 2>&1 | grep -v "sending incremental" | head -10

# Build ionChannel
echo "  → Building ionChannel..."
ssh "$SSH_USER@$VM_IP" "source ~/.cargo/env && cd ~/Development/syntheticChemistry/ionChannel && cargo build --release" 2>&1 | tail -20

echo -e "${GREEN}✓${NC} ionChannel deployed"
echo ""

echo -e "${BLUE}[8/8]${NC} Starting RustDesk and getting connection ID..."

# Start RustDesk
ssh "$SSH_USER@$VM_IP" "DISPLAY=:0 rustdesk > /tmp/rustdesk.log 2>&1 &"
sleep 3

# Get RustDesk ID
RUSTDESK_ID=$(ssh "$SSH_USER@$VM_IP" "rustdesk --get-id 2>/dev/null" | tr -d '[:space:]')

echo -e "${GREEN}✓${NC} RustDesk started"
echo ""

echo "════════════════════════════════════════════════════════════════"
echo -e " ${GREEN}🎉 VM PROVISIONED SUCCESSFULLY!${NC}"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  VM Name:     $VM_NAME"
echo "  VM IP:       $VM_IP"
echo "  SSH User:    $SSH_USER"
echo "  RustDesk ID: ${RUSTDESK_ID:-[Starting...]}"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo " 👥 Connect from Your Remote/LAN Tower:"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  1. Install RustDesk on your remote tower"
echo "  2. Open RustDesk"
echo "  3. Enter ID: $RUSTDESK_ID"
echo "  4. Click 'Connect'"
echo "  5. Enter password"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo " 🔧 Management Commands:"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  SSH to VM:"
echo "    ssh $SSH_USER@$VM_IP"
echo ""
echo "  Console (GUI):"
echo "    virt-manager # Connect to '$VM_NAME'"
echo ""
echo "  Stop VM:"
echo "    virsh shutdown $VM_NAME"
echo ""
echo "  Start VM:"
echo "    virsh start $VM_NAME"
echo ""
echo "  Delete VM:"
echo "    virsh destroy $VM_NAME"
echo "    virsh undefine $VM_NAME --remove-all-storage"
echo ""
echo "════════════════════════════════════════════════════════════════"

# Save connection info
cat > "$HOME/ionChannel-vm-connection.txt" <<INFO
ionChannel Test VM - Connection Information
Generated: $(date)

VM Details:
  Name:     $VM_NAME
  IP:       $VM_IP
  User:     $SSH_USER
  Password: [set during installation]

RustDesk Connection:
  ID: $RUSTDESK_ID

To Connect from Remote/LAN:
  1. Install RustDesk: https://rustdesk.com/
  2. Open RustDesk
  3. Enter ID: $RUSTDESK_ID
  4. Click "Connect"
  5. Enter password

VM Management:
  SSH:      ssh $SSH_USER@$VM_IP
  Console:  virt-manager (connect to '$VM_NAME')
  Stop:     virsh shutdown $VM_NAME
  Start:    virsh start $VM_NAME
  Delete:   virsh destroy $VM_NAME && virsh undefine $VM_NAME --remove-all-storage
INFO

echo ""
echo "Connection info saved to: ~/ionChannel-vm-connection.txt"
echo ""
echo -e "${GREEN}✓ Ready to test! Connect from your remote tower with RustDesk ID: $RUSTDESK_ID${NC}"
echo ""


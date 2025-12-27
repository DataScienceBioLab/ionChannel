#!/bin/bash
# Smart VM Discovery and Auto-Configuration
# Agents guide, users just confirm

set -euo pipefail

echo "════════════════════════════════════════════════════════════════"
echo "  🤖 ionChannel Smart VM Discovery"
echo "  Agent-Guided Deployment"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Colors for better UX
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration file
CONFIG_FILE="$HOME/.ionChannel-vm-config"

# Load last used VM if exists
if [ -f "$CONFIG_FILE" ]; then
    source "$CONFIG_FILE"
    LAST_VM_IP="${LAST_VM_IP:-}"
    LAST_VM_USER="${LAST_VM_USER:-}"
    LAST_VM_NAME="${LAST_VM_NAME:-}"
fi

echo "🔍 Discovering VMs..."
echo ""

# Method 1: Check libvirt/virsh (local VMs)
discover_libvirt_vms() {
    if command -v virsh &> /dev/null; then
        echo -e "${BLUE}[*]${NC} Checking libvirt VMs..."
        
        local vms=()
        while IFS= read -r line; do
            if [[ $line =~ ^[0-9]+ ]]; then
                local id=$(echo "$line" | awk '{print $1}')
                local name=$(echo "$line" | awk '{print $2}')
                local state=$(echo "$line" | awk '{print $3}')
                
                if [ "$state" = "running" ]; then
                    # Try to get IP
                    local ip=$(virsh domifaddr "$name" 2>/dev/null | grep -oP '(\d+\.){3}\d+' | head -1)
                    if [ -n "$ip" ]; then
                        vms+=("libvirt:$name:$ip")
                        echo -e "  ${GREEN}✓${NC} Found: $name ($ip)"
                    fi
                fi
            fi
        done < <(virsh list 2>/dev/null)
        
        echo "${vms[@]}"
    fi
}

# Method 2: Check QEMU/KVM via process list
discover_qemu_vms() {
    echo -e "${BLUE}[*]${NC} Checking QEMU/KVM processes..."
    
    local vms=()
    while IFS= read -r line; do
        if [[ $line =~ -name\ ([^\ ]+) ]]; then
            local name="${BASH_REMATCH[1]}"
            echo -e "  ${GREEN}✓${NC} Found QEMU VM: $name"
            vms+=("qemu:$name:unknown")
        fi
    done < <(ps aux | grep qemu-system 2>/dev/null)
    
    echo "${vms[@]}"
}

# Method 3: Scan local network (mDNS/Avahi)
discover_network_vms() {
    echo -e "${BLUE}[*]${NC} Scanning local network..."
    
    local vms=()
    
    # Try avahi-browse for mDNS
    if command -v avahi-browse &> /dev/null; then
        while IFS= read -r line; do
            if [[ $line =~ hostname\ =\ \[([^\]]+)\] ]]; then
                local hostname="${BASH_REMATCH[1]}"
                # Try to resolve
                local ip=$(getent hosts "$hostname" 2>/dev/null | awk '{print $1}')
                if [ -n "$ip" ] && [[ $hostname =~ (vm|virtual|pop|cosmic) ]]; then
                    vms+=("network:$hostname:$ip")
                    echo -e "  ${GREEN}✓${NC} Found: $hostname ($ip)"
                fi
            fi
        done < <(timeout 3 avahi-browse -apt 2>/dev/null || true)
    fi
    
    echo "${vms[@]}"
}

# Method 4: Check SSH config for known VMs
discover_ssh_config_vms() {
    echo -e "${BLUE}[*]${NC} Checking SSH config..."
    
    local vms=()
    if [ -f "$HOME/.ssh/config" ]; then
        while IFS= read -r line; do
            if [[ $line =~ ^Host\ +([^\ ]+) ]]; then
                local host="${BASH_REMATCH[1]}"
                if [[ $host =~ (vm|virtual|pop|cosmic) ]]; then
                    # Try to get IP
                    local ip=$(ssh -G "$host" 2>/dev/null | grep "^hostname " | awk '{print $2}')
                    if [ -n "$ip" ]; then
                        vms+=("ssh:$host:$ip")
                        echo -e "  ${GREEN}✓${NC} Found: $host ($ip)"
                    fi
                fi
            fi
        done < "$HOME/.ssh/config"
    fi
    
    echo "${vms[@]}"
}

# Method 5: Ping sweep on local subnet (fast)
discover_subnet_vms() {
    echo -e "${BLUE}[*]${NC} Quick subnet scan..."
    
    local vms=()
    local my_ip=$(hostname -I | awk '{print $1}')
    
    if [ -n "$my_ip" ]; then
        local subnet=$(echo "$my_ip" | cut -d. -f1-3)
        
        # Parallel ping sweep (only common VM IPs)
        for last_octet in {100..110} {150..160} {200..210}; do
            {
                local ip="$subnet.$last_octet"
                if ping -c 1 -W 0.3 "$ip" &>/dev/null; then
                    # Try to detect if it's a VM
                    local hostname=$(getent hosts "$ip" 2>/dev/null | awk '{print $2}')
                    if [ -n "$hostname" ] && [[ $hostname =~ (vm|virtual|pop|cosmic) ]]; then
                        vms+=("scan:$hostname:$ip")
                        echo -e "  ${GREEN}✓${NC} Found: $hostname ($ip)"
                    elif ssh -o ConnectTimeout=1 -o StrictHostKeyChecking=no "$ip" "grep -q 'Pop\|COSMIC' /etc/os-release 2>/dev/null" 2>/dev/null; then
                        vms+=("scan:unknown:$ip")
                        echo -e "  ${GREEN}✓${NC} Found: Pop!_OS at $ip"
                    fi
                fi
            } &
        done
        wait
    fi
    
    echo "${vms[@]}"
}

# Collect all discovered VMs
echo ""
echo "Running discovery methods..."
echo "─────────────────────────────────────────────────────────────────"

ALL_VMS=()

# Run all discovery methods
if VMS=$(discover_libvirt_vms); then
    ALL_VMS+=($VMS)
fi

if VMS=$(discover_qemu_vms); then
    ALL_VMS+=($VMS)
fi

if VMS=$(discover_network_vms); then
    ALL_VMS+=($VMS)
fi

if VMS=$(discover_ssh_config_vms); then
    ALL_VMS+=($VMS)
fi

if VMS=$(discover_subnet_vms); then
    ALL_VMS+=($VMS)
fi

echo "─────────────────────────────────────────────────────────────────"
echo ""

# Deduplicate VMs by IP
declare -A UNIQUE_VMS
if [ ${#ALL_VMS[@]} -gt 0 ]; then
    for vm in "${ALL_VMS[@]}"; do
        IFS=':' read -r method name ip <<< "$vm"
        if [ -n "$ip" ] && [ "$ip" != "unknown" ]; then
            UNIQUE_VMS["$ip"]="$name:$method"
        fi
    done
fi

# Present choices
if [ ! -v UNIQUE_VMS ] || [ ${#UNIQUE_VMS[@]} -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No VMs auto-discovered${NC}"
    echo ""
    echo "Manual entry required:"
    read -p "VM IP Address: " VM_IP
    read -p "VM Username: " VM_USER
    VM_NAME="manual"
elif [ ${#UNIQUE_VMS[@]} -gt 0 ]; then
    echo -e "${GREEN}✓ Found ${#UNIQUE_VMS[@]} VM(s)${NC}"
    echo ""
    
    # Show last used if available
    if [ -n "${LAST_VM_IP:-}" ]; then
        echo -e "${BLUE}Last used:${NC} $LAST_VM_NAME ($LAST_VM_IP)"
        echo ""
        read -p "Use last VM? [Y/n]: " use_last
        if [[ $use_last =~ ^[Yy]?$ ]]; then
            VM_IP="$LAST_VM_IP"
            VM_USER="$LAST_VM_USER"
            VM_NAME="$LAST_VM_NAME"
            echo -e "${GREEN}✓${NC} Using $VM_NAME ($VM_IP)"
        fi
    fi
    
    # If not using last, show menu
    if [ -z "${VM_IP:-}" ]; then
        echo "Select VM:"
        echo ""
        
        local i=1
        declare -A VM_MENU
        for ip in "${!UNIQUE_VMS[@]}"; do
            IFS=':' read -r name method <<< "${UNIQUE_VMS[$ip]}"
            echo "  $i) $name - $ip (via $method)"
            VM_MENU[$i]="$ip:$name"
            ((i++))
        done
        
        echo "  m) Manual entry"
        echo ""
        read -p "Choice [1]: " choice
        choice=${choice:-1}
        
        if [ "$choice" = "m" ]; then
            read -p "VM IP Address: " VM_IP
            read -p "VM Username: " VM_USER
            VM_NAME="manual"
        else
            IFS=':' read -r VM_IP VM_NAME <<< "${VM_MENU[$choice]}"
            echo ""
            echo -e "${GREEN}Selected:${NC} $VM_NAME ($VM_IP)"
        fi
    fi
fi

# Auto-detect username if not set
if [ -z "${VM_USER:-}" ]; then
    echo ""
    echo "🔍 Detecting username..."
    
    # Try common usernames
    for test_user in "$USER" "$(whoami)" "ubuntu" "pop" "cosmic"; do
        if ssh -o ConnectTimeout=2 -o StrictHostKeyChecking=no "$test_user@$VM_IP" "exit" 2>/dev/null; then
            VM_USER="$test_user"
            echo -e "${GREEN}✓${NC} Detected username: $VM_USER"
            break
        fi
    done
    
    # If still not found, ask
    if [ -z "$VM_USER" ]; then
        read -p "Username for $VM_IP: " VM_USER
    fi
fi

# Test connection
echo ""
echo "🔌 Testing connection to $VM_USER@$VM_IP..."
if ssh -o ConnectTimeout=3 -o StrictHostKeyChecking=no "$VM_USER@$VM_IP" "exit" 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Connection successful!"
else
    echo -e "${YELLOW}⚠️  Cannot connect${NC}"
    read -p "Continue anyway? [y/N]: " continue_anyway
    if [[ ! $continue_anyway =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Save configuration
echo ""
echo "💾 Saving configuration..."
cat > "$CONFIG_FILE" <<EOF
LAST_VM_IP="$VM_IP"
LAST_VM_USER="$VM_USER"
LAST_VM_NAME="$VM_NAME"
LAST_USED=$(date +%s)
EOF

echo -e "${GREEN}✓${NC} Configuration saved"

# Export for other scripts
export IONCHANNEL_VM_IP="$VM_IP"
export IONCHANNEL_VM_USER="$VM_USER"
export IONCHANNEL_VM_NAME="$VM_NAME"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo -e " ${GREEN}✓ VM Discovered!${NC}"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  VM Name: $VM_NAME"
echo "  IP:      $VM_IP"
echo "  User:    $VM_USER"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo ""

# Offer to proceed with transfer
read -p "Transfer ionChannel to this VM? [Y/n]: " do_transfer
if [[ $do_transfer =~ ^[Yy]?$ ]]; then
    echo ""
    echo "🚀 Starting transfer..."
    echo ""
    
    cd "$(dirname "$0")/.."
    
    # Ensure target directory exists
    ssh "$VM_USER@$VM_IP" "mkdir -p ~/Development/syntheticChemistry"
    
    # Transfer with progress
    rsync -avz --progress \
        ionChannel cosmic-portal-fork cosmic-comp-fork \
        "$VM_USER@$VM_IP:~/Development/syntheticChemistry/"
    
    if [ $? -eq 0 ]; then
        echo ""
        echo -e "${GREEN}✓ Transfer complete!${NC}"
        echo ""
        echo "Next steps:"
        echo "  1. SSH to VM:"
        echo "     ssh $VM_USER@$VM_IP"
        echo ""
        echo "  2. Run deployment:"
        echo "     cd ~/Development/syntheticChemistry/ionChannel"
        echo "     ./AUTO_DEPLOY.sh"
    else
        echo -e "${YELLOW}⚠️  Transfer had errors${NC}"
        exit 1
    fi
else
    echo ""
    echo "Connection info saved to: $CONFIG_FILE"
    echo "You can transfer manually with:"
    echo "  scp -r ionChannel cosmic-portal-fork cosmic-comp-fork \\"
    echo "    $VM_USER@$VM_IP:~/Development/syntheticChemistry/"
fi


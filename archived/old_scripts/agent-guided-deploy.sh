#!/bin/bash
# Fully Agent-Guided Deployment
# User just confirms, agent does everything

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "════════════════════════════════════════════════════════════════"
echo "  🤖 ionChannel Agent-Guided Deployment"
echo "  You confirm, we do the rest"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Phase 1: Discover VM
echo -e "${BLUE}[Phase 1/4]${NC} Discovering VMs..."
echo ""

cd "$WORKSPACE_DIR"
source "$SCRIPT_DIR/smart-vm-discover.sh"

if [ -z "${IONCHANNEL_VM_IP:-}" ]; then
    echo -e "${YELLOW}⚠️  No VM configured${NC}"
    exit 1
fi

VM_IP="$IONCHANNEL_VM_IP"
VM_USER="$IONCHANNEL_VM_USER"
VM_NAME="$IONCHANNEL_VM_NAME"

echo ""
echo -e "${GREEN}✓ Phase 1 Complete${NC}"
echo ""

# Phase 2: Deploy on VM (remote execution)
echo "════════════════════════════════════════════════════════════════"
echo -e "${BLUE}[Phase 2/4]${NC} Deploying on VM..."
echo "════════════════════════════════════════════════════════════════"
echo ""

echo "Running AUTO_DEPLOY.sh on $VM_NAME..."
echo ""

# Execute deployment remotely
ssh -t "$VM_USER@$VM_IP" "cd ~/Development/syntheticChemistry/ionChannel && ./AUTO_DEPLOY.sh"

if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ Phase 2 Complete${NC}"
else
    echo -e "${YELLOW}⚠️  Deployment had errors${NC}"
    echo ""
    read -p "Continue anyway? [y/N]: " continue_deploy
    if [[ ! $continue_deploy =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Phase 3: Reboot prompt
echo ""
echo "════════════════════════════════════════════════════════════════"
echo -e "${BLUE}[Phase 3/4]${NC} COSMIC Restart Required"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "The VM needs to log out and back into COSMIC to load the new portal."
echo ""
echo "Options:"
echo "  1) I'll log out/in manually on the VM console"
echo "  2) Restart display manager (faster)"
echo "  3) Skip for now"
echo ""
read -p "Choice [1]: " restart_choice
restart_choice=${restart_choice:-1}

case $restart_choice in
    2)
        echo "Restarting display manager..."
        ssh "$VM_USER@$VM_IP" "sudo systemctl restart display-manager" || true
        echo "Waiting for system to come back up..."
        sleep 10
        
        # Wait for SSH to be available again
        for i in {1..30}; do
            if ssh -o ConnectTimeout=1 "$VM_USER@$VM_IP" "exit" 2>/dev/null; then
                echo -e "${GREEN}✓ System is back up${NC}"
                break
            fi
            sleep 2
        done
        ;;
    3)
        echo "⚠️  Remember to log out/in before testing!"
        ;;
    *)
        echo ""
        echo "Please log out and back into COSMIC on the VM now."
        echo ""
        read -p "Press Enter when you've logged back in..."
        ;;
esac

echo ""
echo -e "${GREEN}✓ Phase 3 Complete${NC}"

# Phase 4: Start RustDesk and display connection info
echo ""
echo "════════════════════════════════════════════════════════════════"
echo -e "${BLUE}[Phase 4/4]${NC} Starting RustDesk Server"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Check if portal is running
echo "Verifying portal..."
if ssh "$VM_USER@$VM_IP" "busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop 2>/dev/null | grep -q RemoteDesktop" 2>/dev/null; then
    echo -e "${GREEN}✓ RemoteDesktop portal is active${NC}"
else
    echo -e "${YELLOW}⚠️  Portal not detected yet${NC}"
    echo "   It may need a moment to start"
fi

echo ""
echo "Getting RustDesk connection info..."
RUSTDESK_ID=$(ssh "$VM_USER@$VM_IP" "rustdesk --get-id 2>/dev/null" | tr -d '[:space:]')

if [ -z "$RUSTDESK_ID" ]; then
    echo "Starting RustDesk..."
    # Start RustDesk in background
    ssh "$VM_USER@$VM_IP" "nohup rustdesk --server > /tmp/rustdesk.log 2>&1 &" 
    sleep 3
    RUSTDESK_ID=$(ssh "$VM_USER@$VM_IP" "rustdesk --get-id 2>/dev/null" | tr -d '[:space:]')
fi

echo ""
echo -e "${GREEN}✓ Phase 4 Complete${NC}"
echo ""

# Final success screen
echo "════════════════════════════════════════════════════════════════"
echo -e " ${GREEN}🎉 DEPLOYMENT COMPLETE!${NC}"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  VM Name:     $VM_NAME"
echo "  IP Address:  $VM_IP"
echo "  RustDesk ID: ${RUSTDESK_ID:-Starting...}"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo " 👥 For Users to Connect:"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  1. Install RustDesk: https://rustdesk.com/"
echo "  2. Open RustDesk"
echo "  3. Enter ID: $RUSTDESK_ID"
echo "  4. Click 'Connect'"
echo "  5. Enter password when prompted"
echo ""
echo "════════════════════════════════════════════════════════════════"
echo ""

# Save connection info to file
cat > "$WORKSPACE_DIR/VM_CONNECTION_INFO.txt" <<EOF
ionChannel VM Connection Information
Generated: $(date)

VM Details:
  Name:     $VM_NAME  
  IP:       $VM_IP
  User:     $VM_USER

RustDesk Connection:
  ID:       $RUSTDESK_ID
  
For Users:
  1. Download RustDesk from https://rustdesk.com/
  2. Open RustDesk
  3. Enter ID: $RUSTDESK_ID
  4. Click "Connect"
  5. Enter the password
  6. You're connected!

Management Commands:
  SSH to VM:
    ssh $VM_USER@$VM_IP
  
  Check portal status:
    ssh $VM_USER@$VM_IP "busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop | grep RemoteDesktop"
  
  View portal logs:
    ssh $VM_USER@$VM_IP "journalctl --user -u xdg-desktop-portal.service -f"
  
  Restart RustDesk:
    ssh $VM_USER@$VM_IP "~/start-rustdesk-with-ionChannel.sh"

Testing:
  From local machine:
    rustdesk
    # Enter ID: $RUSTDESK_ID
  
  From LAN:
    Same process - just enter the ID!
EOF

echo "Connection info saved to: VM_CONNECTION_INFO.txt"
echo ""

# Create simple reconnect script
cat > "$WORKSPACE_DIR/reconnect-to-vm.sh" <<EOF
#!/bin/bash
# Quick reconnect to configured VM
ssh $VM_USER@$VM_IP "\$@"
EOF
chmod +x "$WORKSPACE_DIR/reconnect-to-vm.sh"

echo "Quick reconnect script: ./reconnect-to-vm.sh"
echo ""

# Offer to monitor logs
read -p "Monitor portal logs now? [y/N]: " monitor_logs
if [[ $monitor_logs =~ ^[Yy]$ ]]; then
    echo ""
    echo "Monitoring portal logs (Ctrl+C to exit)..."
    echo ""
    ssh "$VM_USER@$VM_IP" "journalctl --user -u xdg-desktop-portal.service -f"
fi

echo ""
echo -e "${GREEN}✓ All done! Users can now connect with RustDesk ID: $RUSTDESK_ID${NC}"
echo ""


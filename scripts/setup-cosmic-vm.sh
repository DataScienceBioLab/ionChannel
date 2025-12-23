#!/bin/bash
# ionChannel COSMIC VM Setup Script
# Run this AFTER enabling VT-x in BIOS

set -e

echo "=== ionChannel COSMIC VM Setup ==="
echo ""

# Check for VT-x
if ! grep -qE "(vmx|svm)" /proc/cpuinfo; then
    echo "❌ ERROR: VT-x/AMD-V not detected!"
    echo ""
    echo "Please enable virtualization in BIOS:"
    echo "  1. Reboot and enter BIOS (DEL/F2/F12)"
    echo "  2. Find 'Intel Virtualization Technology' or 'VT-x'"
    echo "  3. Set to ENABLED"
    echo "  4. Save and reboot"
    exit 1
fi

echo "✅ VT-x/AMD-V detected"
echo ""

# Install QEMU/KVM and virt-manager
echo "Installing virtualization packages..."
sudo apt update
sudo apt install -y \
    qemu-kvm \
    libvirt-daemon-system \
    libvirt-clients \
    bridge-utils \
    virt-manager \
    virtinst

# Add user to libvirt and kvm groups
echo "Adding $USER to virtualization groups..."
sudo usermod -aG libvirt "$USER"
sudo usermod -aG kvm "$USER"

# Start libvirt service
echo "Starting libvirt service..."
sudo systemctl enable --now libvirtd

# Verify KVM
echo ""
echo "=== Verification ==="
if [ -e /dev/kvm ]; then
    echo "✅ KVM is available"
else
    echo "❌ KVM device not found (may need reboot)"
fi

echo ""
echo "=== Next Steps ==="
echo ""
echo "1. LOG OUT AND BACK IN (for group membership)"
echo ""
echo "2. Download Pop!_OS COSMIC ISO:"
echo "   https://pop.system76.com/"
echo "   (Look for 'COSMIC' or 'Alpha/Beta' version)"
echo ""
echo "3. Create the VM:"
echo "   virt-manager  # GUI tool"
echo ""
echo "   Or via command line:"
echo "   virt-install \\"
echo "     --name cosmic-test \\"
echo "     --ram 8192 \\"
echo "     --vcpus 4 \\"
echo "     --disk size=50 \\"
echo "     --cdrom /path/to/pop-os-cosmic.iso \\"
echo "     --os-variant ubuntu24.04"
echo ""
echo "4. VM Settings for COSMIC:"
echo "   - RAM: 8GB minimum (you have 32GB, so 8-16GB is fine)"
echo "   - CPUs: 4 cores"
echo "   - Disk: 50GB"
echo "   - Display: Spice (for clipboard/input)"
echo ""
echo "=== Done ==="


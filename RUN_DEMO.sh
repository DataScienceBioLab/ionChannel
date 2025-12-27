#!/bin/bash
# Quick demo launcher for ionChannel E2E validation
# 
# This script runs the comprehensive E2E demo showing:
# - Capability-based VM discovery
# - VM provisioning via benchScale
# - RustDesk installation
# - ionChannel portal deployment
# - Full event streaming

set -e

echo "🚀 ionChannel E2E Demo Launcher"
echo "════════════════════════════════════════════════════════════"
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."

if ! command -v virsh &> /dev/null; then
    echo "❌ libvirt not found. Please install:"
    echo "   sudo apt install libvirt-daemon-system libvirt-clients"
    exit 1
fi

if ! virsh -c qemu:///system list &> /dev/null; then
    echo "❌ Cannot connect to libvirt. Check permissions:"
    echo "   sudo usermod -aG libvirt \$USER"
    echo "   newgrp libvirt"
    exit 1
fi

echo "✅ Prerequisites OK"
echo ""

# Set default environment variables if not set
export VM_SSH_USER="${VM_SSH_USER:-ubuntu}"
export VM_SSH_PASSWORD="${VM_SSH_PASSWORD:-ubuntu}"
export BENCHSCALE_LIBVIRT_URI="${BENCHSCALE_LIBVIRT_URI:-qemu:///system}"

echo "⚙️  Configuration:"
echo "   VM_SSH_USER=$VM_SSH_USER"
echo "   VM_SSH_PASSWORD=$VM_SSH_PASSWORD"
echo "   BENCHSCALE_LIBVIRT_URI=$BENCHSCALE_LIBVIRT_URI"
echo ""

echo "▶️  Running E2E demo..."
echo "════════════════════════════════════════════════════════════"
echo ""

cargo run -p ion-validation --example full_e2e_demo --features libvirt

echo ""
echo "════════════════════════════════════════════════════════════"
echo "✅ Demo complete!"
echo ""
echo "📚 For more information, see:"
echo "   - DEMO_GUIDE.md (complete guide)"
echo "   - E2E_COMPLETE.md (implementation details)"
echo "   - CAPABILITY_BASED_VM_DISCOVERY.md (architecture)"

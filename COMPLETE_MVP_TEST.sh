#!/bin/bash
# Complete MVP Test - Automated cloud-init approach

set -e

echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                      ║"
echo "║    COMPLETE MVP TEST: Cloud-Init Enabled Ubuntu VM                  ║"
echo "║                                                                      ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""

# Check if we have Ubuntu cloud image
CLOUD_IMG="/var/lib/libvirt/images/ubuntu-22.04-server-cloudimg-amd64.img"

if [ ! -f "$CLOUD_IMG" ]; then
    echo "⚠️  Ubuntu cloud image not found. Downloading..."
    echo ""
    echo "You can download it with:"
    echo "  cd /var/lib/libvirt/images"
    echo "  sudo wget https://cloud-images.ubuntu.com/releases/22.04/release/ubuntu-22.04-server-cloudimg-amd64.img"
    echo ""
    echo "Or use the existing VM and configure SSH via console:"
    echo "  virt-manager → ubuntu-test-base → Console"
    echo "  Login and run: sudo systemctl enable ssh && sudo systemctl start ssh"
    echo ""
    exit 1
fi

# Create cloud-init config
echo "📝 Creating cloud-init configuration..."
cat > /tmp/user-data <<'USERDATA'
#cloud-config
users:
  - name: ubuntu
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    lock_passwd: false
    passwd: $6$rounds=4096$saltsalt$L2NnhvMdVnJ4U7h.jLWyJ9rXZgZHyYZH8YzMlYmSJ2MeJ9rXZgZH8YzMlYmSJ2MeL2NnhvMdVnJ4U7h.jL
    ssh_authorized_keys:
      - ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQC... (your key here)

packages:
  - openssh-server
  - python3
  - python3-pip

runcmd:
  - systemctl enable ssh
  - systemctl start ssh
  - echo "SSH enabled" > /var/log/cloud-init-complete.log
USERDATA

cat > /tmp/meta-data <<'METADATA'
instance-id: ionchannel-test-vm
local-hostname: ionchannel-test
METADATA

echo "✓ Cloud-init config created"
echo ""

echo "📋 Next Steps:"
echo ""
echo "1. Create VM with cloud-init:"
echo "   virt-install \\"
echo "     --name ionchannel-test \\"
echo "     --ram 4096 \\"
echo "     --vcpus 2 \\"
echo "     --disk path=/var/lib/libvirt/images/ionchannel-test.qcow2,size=20,format=qcow2,backing_store=$CLOUD_IMG \\"
echo "     --cloud-init user-data=/tmp/user-data,meta-data=/tmp/meta-data \\"
echo "     --network network=default \\"
echo "     --graphics vnc \\"
echo "     --os-variant ubuntu22.04 \\"
echo "     --import"
echo ""
echo "2. Wait for VM to boot (30 seconds)"
echo ""
echo "3. Get VM IP and continue test"
echo ""


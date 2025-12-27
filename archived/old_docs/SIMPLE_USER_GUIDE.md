# Connect to Your VM with RustDesk - Simple Guide

**For End Users:** Just like connecting to any RustDesk computer

---

## 🎯 What This Is

You have a Pop!_OS VM running COSMIC desktop. After setup, you can connect to it from any computer on your network using RustDesk - **exactly like connecting to any other RustDesk computer**.

**No technical knowledge required.**

---

## 🚀 One-Time Setup (For You - The Admin)

### Step 1: Transfer Files to VM

```bash
# On your development machine
cd /home/nestgate/Development/syntheticChemistry

# Your VM details (change these)
VM_IP="192.168.1.100"      # Your VM's IP address
VM_USER="yourname"          # Your VM username

# Transfer (one command)
scp -r ionChannel cosmic-portal-fork cosmic-comp-fork \
    "${VM_USER}@${VM_IP}:~/Development/syntheticChemistry/"
```

### Step 2: Run Automated Setup on VM

```bash
# SSH to VM
ssh ${VM_USER}@${VM_IP}

# Run automated setup (just press 'y' when prompted)
cd ~/Development/syntheticChemistry/ionChannel
./scripts/vm-setup-popos.sh

# Deploy (just press 'y' when prompted)
cd ~/Development/syntheticChemistry
./deploy-to-system.sh
```

**Then log out and back into COSMIC on the VM.**

### Step 3: Start RustDesk Server

```bash
# On VM (after logging back in)
cd ~/Development/syntheticChemistry
./start-rustdesk-server.sh
```

You'll see:
```
🖥️  Starting RustDesk Server
==========================

VM IP: 192.168.1.100

RustDesk ID: 123-456-789
Password: abc123xyz

Server running...
```

**Write down the RustDesk ID and Password!**

---

## 👤 For End Users: How to Connect

### On Your Local Computer

1. **Download RustDesk** (if not installed)
   - Go to https://rustdesk.com/
   - Download for your OS (Windows/Mac/Linux)
   - Install it

2. **Launch RustDesk**
   - Double-click the RustDesk icon
   - You'll see the main window

3. **Enter the VM's RustDesk ID**
   ```
   Remote ID: 123-456-789  ← (the ID from the VM)
   ```

4. **Click "Connect"**

5. **Enter Password When Prompted**
   ```
   Password: abc123xyz  ← (the password from the VM)
   ```

6. **You're Connected!**
   - You'll see the VM's desktop
   - Use mouse and keyboard normally
   - Everything "just works"

### From Another Computer on LAN

**Exactly the same steps!** RustDesk works over your local network automatically.

---

## 🎮 Using the Connection

Once connected:

### ✅ What Works
- **Mouse** - Move, click, scroll
- **Keyboard** - Type anything
- **Copy/Paste** - Between computers (if enabled)
- **File Transfer** - Drag and drop (if enabled)

### 🖱️ Controlling the VM
- Move your mouse → VM cursor moves
- Click → Clicks on VM
- Type → Text appears on VM
- Keyboard shortcuts → Work on VM

### 📋 Toolbar (Bottom of Window)
- **Full Screen** - Fill your screen
- **Settings** - Quality, display options
- **Exit** - Disconnect

---

## 🔍 Troubleshooting

### Can't Connect?

**Check VM is Running**
```
- Is the VM powered on?
- Is RustDesk server running on VM?
```

**Check Network**
```
- Are both computers on same network?
- Can you ping the VM?
  ping 192.168.1.100
```

**Restart RustDesk Server**
```
# On VM:
cd ~/Development/syntheticChemistry
./start-rustdesk-server.sh
```

### Connection Slow?

**On RustDesk client:**
- Click Settings (gear icon)
- Reduce quality if needed
- Try different codec

### Mouse/Keyboard Not Working?

**This means input injection needs work - BUT:**
- If you can see the desktop, **the portal works!**
- This is Phase 2 (compositor integration)
- Screen sharing is working ✅
- Input will be fixed in next update

**For now:**
- You can still see the screen
- Useful for monitoring
- Demonstrates portal is functional

---

## 📱 Multiple Connections

You can connect from multiple computers:

### Local Machine
```
1. Launch RustDesk
2. Enter VM ID: 123-456-789
3. Enter Password
4. Connected!
```

### LAN Tower
```
1. Launch RustDesk  
2. Enter VM ID: 123-456-789
3. Enter Password
4. Connected!
```

### Laptop
```
Same process!
```

**All connections work the same way.**

---

## 🔐 Security

### How It's Secure

1. **Password Required** - Can't connect without it
2. **Encryption** - All traffic is encrypted
3. **Local Network** - Stays on your LAN
4. **Consent System** - User approval (in portal)

### Changing Password

On VM:
```bash
rustdesk --password
# Enter new password when prompted
```

---

## 💡 Tips for Best Experience

### For Smooth Performance
- Use wired connection (not WiFi) when possible
- Close other apps on client machine
- Adjust quality in RustDesk settings

### For Multiple Users
- Each gets their own session
- No conflicts
- Can view simultaneously

### For Support/Admin
- You can connect to help users
- See exactly what they see
- Control their VM to fix issues

---

## 📊 What Success Looks Like

### ✅ Everything Working
```
1. Open RustDesk on client
2. Enter VM ID and password
3. See VM desktop immediately
4. Mouse cursor moves smoothly
5. Keyboard input works
6. No lag or stuttering
```

### ⚠️ Partial Success
```
1. Can connect ✅
2. Can see desktop ✅
3. Mouse doesn't work ⚠️
4. Keyboard doesn't work ⚠️

This means:
- Portal is working ✅
- Screen sharing works ✅
- Input injection needs more work 🔧
- Still useful for monitoring!
```

---

## 🎯 Quick Reference Card

```
╔════════════════════════════════════════╗
║  RustDesk Connection Info              ║
╠════════════════════════════════════════╣
║  VM IP:      192.168.1.100             ║
║  RustDesk ID: 123-456-789              ║
║  Password:    abc123xyz                ║
╠════════════════════════════════════════╣
║  To Connect:                           ║
║  1. Open RustDesk                      ║
║  2. Enter ID                           ║
║  3. Click "Connect"                    ║
║  4. Enter Password                     ║
║  5. Done!                              ║
╚════════════════════════════════════════╝
```

**Print this and give it to users!**

---

## 🆘 Help

### For Users
- "It won't connect" → Check VM is on, RustDesk running
- "It's slow" → Reduce quality in settings
- "I see desktop but can't control" → Input coming soon, you can still watch

### For Admin
- Check logs: `journalctl --user -u xdg-desktop-portal.service`
- Restart portal: `systemctl --user restart xdg-desktop-portal.service`
- Test portal: `cd ~/Development/syntheticChemistry && ./test-ionChannel.sh`

---

## 🎉 That's It!

**It's literally that simple:**
1. Admin runs setup once
2. Users just enter ID and password
3. Everything works like normal RustDesk

**No bridge knowledge needed. No technical expertise required. Just connect and use.**

---

*Simple User Guide - ionChannel + RustDesk*
*December 26, 2025*


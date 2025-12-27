# ionChannel VM Testing Guide

**Goal:** Test RemoteDesktop portal end-to-end with RustDesk

---

## 🎯 Test Scenarios

### Scenario 1: Local Machine → VM
Connect from your local development machine to the VM

### Scenario 2: LAN Tower → VM  
Connect from another machine on your LAN to the VM

Both should work with mouse and keyboard control.

---

## 🚀 Quick Start

### On Your Development Machine (Host)

1. **Transfer files to VM:**
   ```bash
   # From syntheticChemistry directory
   cd /home/nestgate/Development/syntheticChemistry
   
   # Copy to VM (replace VM_IP with your VM's IP)
   VM_IP="192.168.x.x"
   VM_USER="your-vm-username"
   
   scp -r ionChannel cosmic-portal-fork cosmic-comp-fork "${VM_USER}@${VM_IP}:~/Development/syntheticChemistry/"
   ```

2. **SSH into VM:**
   ```bash
   ssh ${VM_USER}@${VM_IP}
   ```

### On the VM (Pop!_OS)

3. **Run setup script:**
   ```bash
   cd ~/Development/syntheticChemistry/ionChannel
   ./scripts/vm-setup-popos.sh
   ```
   
   This will:
   - Install dependencies
   - Build portal and compositor forks
   - Install RustDesk
   - Create deployment and test scripts

4. **Deploy forks:**
   ```bash
   cd ~/Development/syntheticChemistry
   ./deploy-to-system.sh
   ```

5. **Log out and back in to COSMIC**
   This loads the new portal and compositor

6. **Test integration:**
   ```bash
   cd ~/Development/syntheticChemistry
   ./test-ionChannel.sh
   ```
   
   Should show:
   ```
   ✅ Portal service is running
   ✅ Portal registered on D-Bus
   ✅ RemoteDesktop interface found!
   ✅ Compositor running
   ✅ Available devices: 7 (keyboard + mouse + touchscreen)
   ```

7. **Start RustDesk server:**
   ```bash
   ./start-rustdesk-server.sh
   ```
   
   Note the **RustDesk ID** displayed (e.g., `123-456-789`)

---

## 🖥️ Client Setup

### On Local Machine (Scenario 1)

1. **Install RustDesk client:**
   ```bash
   # Linux
   wget https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb
   sudo dpkg -i rustdesk-1.2.3-x86_64.deb
   
   # Or download from: https://rustdesk.com/
   ```

2. **Launch RustDesk:**
   ```bash
   rustdesk
   ```

3. **Connect:**
   - Enter the RustDesk ID from the VM
   - Click "Connect"
   - When prompted for permission, you should see the COSMIC consent dialog on VM
   - Approve the connection
   - Test mouse and keyboard control

### On LAN Tower (Scenario 2)

Same steps as above - RustDesk works over LAN automatically.

---

## 🧪 Testing Checklist

### Pre-Flight Checks ✅

- [ ] VM is running Pop!_OS with COSMIC
- [ ] VM has network connectivity
- [ ] Can SSH into VM
- [ ] VM IP is known (run `hostname -I` on VM)

### Build & Deploy ✅

- [ ] `vm-setup-popos.sh` ran successfully
- [ ] Portal built without errors
- [ ] Compositor built (or skipped with warning)
- [ ] Forks deployed to system
- [ ] Logged out and back into COSMIC

### Integration Tests ✅

- [ ] `test-ionChannel.sh` passes all checks
- [ ] Portal service is running
- [ ] RemoteDesktop interface appears on D-Bus
- [ ] `AvailableDeviceTypes` returns non-zero value

### RustDesk Tests ✅

#### From Local Machine:
- [ ] RustDesk server running on VM
- [ ] RustDesk ID is displayed
- [ ] Client can connect from local machine
- [ ] Can see VM desktop
- [ ] Mouse cursor moves on VM
- [ ] Keyboard input works on VM
- [ ] No lag or crashes

#### From LAN Tower:
- [ ] Client can connect from tower
- [ ] Can see VM desktop
- [ ] Mouse control works
- [ ] Keyboard control works

---

## 🔍 Troubleshooting

### Issue: Portal build fails

**Error:** `Cannot find libraries: libpipewire-0.3`

**Solution:**
```bash
sudo apt install -y libpipewire-0.3-dev libwayland-dev libdbus-1-dev
cd ~/Development/syntheticChemistry/cosmic-portal-fork
cargo clean
cargo build --release
```

### Issue: RemoteDesktop interface not found

**Check D-Bus registration:**
```bash
busctl --user list | grep portal
busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop
```

**Restart portal:**
```bash
systemctl --user restart xdg-desktop-portal.service
sleep 2
busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop
```

**Check logs:**
```bash
journalctl --user -u xdg-desktop-portal.service -f
```

### Issue: RustDesk can't connect

**Check firewall:**
```bash
# On VM
sudo ufw allow 21115/tcp
sudo ufw allow 21116/tcp
sudo ufw allow 21117/tcp
sudo ufw allow 21118/tcp
sudo ufw allow 21119/tcp
```

**Check RustDesk is running:**
```bash
ps aux | grep rustdesk
```

**Test local portal first:**
```bash
cd ~/Development/syntheticChemistry/ionChannel
cargo run --package portal-test-client check
```

### Issue: Mouse/keyboard don't work

**This means input injection is not reaching compositor.**

**Check compositor logs:**
```bash
journalctl --user -u cosmic-comp.service -f
```

**Verify compositor has input injection:**
```bash
# Should show virtual input code
strings /usr/bin/cosmic-comp | grep -i "virtual\|inject\|remote"
```

**Fallback:** Compositor integration may need more work. The portal itself is working if RustDesk connects.

### Issue: Consent dialog doesn't appear

**Current setup uses auto-approve for testing.**

Check portal logs:
```bash
journalctl --user -u xdg-desktop-portal.service | grep -i consent
```

Should see:
```
Consent auto-approved for session /test/session
```

---

## 📊 Success Criteria

### Minimum Success (Portal Working) ✅
- [ ] RustDesk connects to VM
- [ ] Can see VM desktop
- [ ] Portal logs show session creation
- [ ] Portal logs show input events

### Full Success (Input Working) ✅
- [ ] Mouse cursor moves on VM when moved on client
- [ ] Clicking works
- [ ] Keyboard input appears on VM
- [ ] No noticeable lag (< 100ms)
- [ ] No crashes or freezes

### Bonus Success ⭐
- [ ] Works from multiple clients simultaneously
- [ ] Works over WiFi and Ethernet
- [ ] Clipboard sync (if RustDesk supports it)
- [ ] Consent dialog appears (after replacing auto-approve)

---

## 🐛 Debug Mode

### Enable verbose logging:

**Portal:**
```bash
# Edit service file
systemctl --user edit xdg-desktop-portal.service

# Add:
[Service]
Environment="RUST_LOG=debug"
Environment="G_MESSAGES_DEBUG=all"

# Restart
systemctl --user daemon-reload
systemctl --user restart xdg-desktop-portal.service
```

**Compositor:**
```bash
systemctl --user edit cosmic-comp.service

# Add:
[Service]
Environment="RUST_LOG=debug"

# Restart
systemctl --user daemon-reload
systemctl --user restart cosmic-comp.service
```

**Watch logs:**
```bash
# Terminal 1: Portal logs
journalctl --user -u xdg-desktop-portal.service -f

# Terminal 2: Compositor logs
journalctl --user -u cosmic-comp.service -f

# Terminal 3: RustDesk
./start-rustdesk-server.sh
```

### Manual D-Bus testing:

```bash
# Create session
dbus-send --session --print-reply \
  --dest=org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop \
  org.freedesktop.impl.portal.RemoteDesktop.CreateSession \
  objpath:/org/freedesktop/portal/test/handle \
  objpath:/org/freedesktop/portal/test/session \
  string:test-app \
  dict:string:string:

# Check available devices
dbus-send --session --print-reply \
  --dest=org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop \
  org.freedesktop.DBus.Properties.Get \
  string:org.freedesktop.impl.portal.RemoteDesktop \
  string:AvailableDeviceTypes
```

---

## 📝 Test Log Template

Use this to document your testing:

```markdown
## Test Run: [Date/Time]

### Environment
- VM OS: Pop!_OS [version]
- VM IP: [IP address]
- COSMIC version: [version]
- RustDesk ID: [ID]

### Client 1: Local Machine
- OS: [OS]
- RustDesk version: [version]
- Connection: [ ] Success / [ ] Failed
- Desktop visible: [ ] Yes / [ ] No
- Mouse control: [ ] Yes / [ ] No
- Keyboard control: [ ] Yes / [ ] No
- Latency: [ms]
- Issues: [describe]

### Client 2: LAN Tower
- OS: [OS]
- RustDesk version: [version]
- Connection: [ ] Success / [ ] Failed
- Desktop visible: [ ] Yes / [ ] No
- Mouse control: [ ] Yes / [ ] No
- Keyboard control: [ ] Yes / [ ] No
- Latency: [ms]
- Issues: [describe]

### Portal Logs
```
[paste relevant logs]
```

### Compositor Logs
```
[paste relevant logs]
```

### Notes
[any other observations]
```

---

## 🎬 Video/Screenshot Guide

Document your testing with:

1. **Portal D-Bus interface:**
   ```bash
   busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop > dbus-interfaces.txt
   ```

2. **Screenshot RustDesk connection from client**

3. **Record mouse/keyboard control working**

4. **Capture logs showing input events**

---

## 🚀 Quick Reference Commands

```bash
# On VM - Deploy
cd ~/Development/syntheticChemistry
./deploy-to-system.sh
# Log out and back in

# On VM - Test
./test-ionChannel.sh

# On VM - Start server
./start-rustdesk-server.sh

# On VM - Watch logs
journalctl --user -u xdg-desktop-portal.service -f

# On VM - Check interface
busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop

# On Client - Connect
rustdesk
# Enter VM's RustDesk ID
```

---

## 📞 Report Back

After testing, report:

1. **What worked:**
   - Portal build status
   - D-Bus interface registration
   - RustDesk connection status
   - Mouse/keyboard control

2. **What didn't work:**
   - Build errors
   - Connection failures
   - Input issues

3. **Logs:**
   - Portal logs during connection
   - Compositor logs during input
   - RustDesk client output

---

**Ready to test!** Transfer the files to your VM and run `./scripts/vm-setup-popos.sh`

*VM Testing Guide - December 26, 2025*


//! Autonomous RustDesk ID retrieval
//!
//! This tool connects to the VM and retrieves the RustDesk ID automatically

use std::io::BufRead;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    println!("\n🤖 AUTONOMOUS RUSTDESK ID RETRIEVAL");
    println!("════════════════════════════════════════════════════════════════\n");

    // Use the clean base VM we just created
    let ip = "192.168.122.49";
    let username = "testuser";
    let password = "testpass123";

    println!("📡 VM IP: {}", ip);
    println!("👤 Username: {}", username);
    println!();
    println!("Step 1: Downloading RustDesk directly on VM via console...\n");

    // Commands to run in the VM
    // Build commands with proper lifetime
    let dpkg_cmd = format!(
        "echo '{}' | sudo -S dpkg -i rustdesk.deb 2>&1 | tail -3",
        password
    );
    let apt_cmd = format!(
        "echo '{}' | sudo -S apt-get install -f -y 2>&1 | tail -3",
        password
    );

    let commands = vec![
        "cd /tmp",
        "if [ ! -f rustdesk.deb ]; then wget -q --show-progress https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb -O rustdesk.deb; fi",
        &dpkg_cmd,
        &apt_cmd,
        "rustdesk --version",
        "echo '═══════════════════════════════════════════════════════════════'",
        "echo '🎯 RUSTDESK ID:'",
        "rustdesk --get-id || echo 'Note: RustDesk may need a GUI session. Install complete!'",
        "echo '═══════════════════════════════════════════════════════════════'",
    ];

    let full_command = commands.join(" && ");

    // Use sshpass with the new credentials
    println!("Step 2: Running installation commands...\n");

    let result = Command::new("sshpass")
        .args([
            "-p",
            password,
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=password",
            "-o",
            "PubkeyAuthentication=no",
            "-o",
            "NumberOfPasswordPrompts=1",
            &format!("{}@{}", username, ip),
            &full_command,
        ])
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("Output:\n{}", stdout);
            if !stderr.is_empty() && !stderr.contains("Pseudo-terminal") {
                eprintln!("Errors:\n{}", stderr);
            }

            // Try to extract ID
            if let Some(line) = stdout
                .lines()
                .find(|l| l.len() == 9 && l.chars().all(|c| c.is_numeric()))
            {
                println!("\n🎉 SUCCESS!");
                println!("════════════════════════════════════════════════════════════════");
                println!("\n🎯 RustDesk ID: {}\n", line);
                println!("════════════════════════════════════════════════════════════════");
                println!("\n📱 TO CONNECT FROM YOUR OTHER TOWER:");
                println!("   1. Install RustDesk: sudo apt install rustdesk");
                println!("   2. Open RustDesk application");
                println!("   3. Enter ID: {}", line);
                println!("   4. Click Connect");
                println!("   5. ✅ You're in!\n");
                println!("════════════════════════════════════════════════════════════════\n");
                return Ok(());
            }
        },
        Err(e) => {
            eprintln!("⚠️  SSH command failed: {}", e);
        },
    }

    println!("\n📊 SUMMARY:");
    println!("════════════════════════════════════════════════════════════════");
    println!("RustDesk installation attempted, but automatic ID retrieval");
    println!("requires either:");
    println!("  1. Working SSH password auth (currently blocked)");
    println!("  2. GUI session in VM (for rustdesk --get-id)");
    println!("\n💡 QUICK MANUAL ALTERNATIVE:");
    println!("   $ virt-manager");
    println!("   Right-click test1 → Open → Console");
    println!("   Login: iontest / iontest");
    println!("   Run: rustdesk --get-id");
    println!("\nThen you'll have the ID to connect from your other tower!");
    println!("════════════════════════════════════════════════════════════════\n");

    Ok(())
}

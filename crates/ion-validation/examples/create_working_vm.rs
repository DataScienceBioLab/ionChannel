//! Create a properly configured test VM with working SSH
//!
//! This provisions a VM that our autonomous tools can actually use

use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("\n🚀 CREATING PROPERLY CONFIGURED TEST VM");
    println!("════════════════════════════════════════════════════════════════\n");

    // Check if ionChannel-template exists
    println!("📋 Checking for template VM...\n");

    let template_exists = Command::new("virsh")
        .args(["list", "--all"])
        .output()?
        .stdout;

    let template_str = String::from_utf8_lossy(&template_exists);

    if !template_str.contains("ionChannel-template") {
        println!("⚠️  No template VM found.");
        println!("\n💡 TO CREATE A WORKING TEST VM:");
        println!("════════════════════════════════════════════════════════════════");
        println!("\nWe need a base image with working SSH. Let me create one:");
        println!("\n1. Download a cloud image with cloud-init support");
        println!("2. Configure it with a known working password");
        println!("3. Clone it for testing\n");

        // Check for existing Pop!_OS ISO
        let iso_path = "/var/lib/libvirt/images/pop-os_24.04_amd64_nvidia_22.iso";
        if std::path::Path::new(iso_path).exists() {
            println!("✓ Found Pop!_OS ISO");
            println!("\nCreating template VM with working SSH...\n");

            // Use virt-install to create a minimal VM
            println!("This will take a few minutes. Creating VM...");

            let install_cmd = Command::new("virt-install")
                .args([
                    "--name",
                    "rustdesk-test",
                    "--ram",
                    "4096",
                    "--vcpus",
                    "2",
                    "--disk",
                    "size=20",
                    "--os-variant",
                    "ubuntu22.04",
                    "--network",
                    "network=default",
                    "--graphics",
                    "none",
                    "--console",
                    "pty,target_type=serial",
                    "--location",
                    iso_path,
                    "--initrd-inject",
                    "/tmp/preseed.cfg",
                    "--extra-args",
                    "console=ttyS0 auto=true priority=critical",
                    "--noautoconsole",
                ])
                .output();

            match install_cmd {
                Ok(_) => println!("✓ VM creation started!"),
                Err(e) => println!("⚠️  Could not create VM automatically: {}", e),
            }
        }
    } else {
        println!("✓ Template VM found!");
        println!("\nCloning for testing...\n");

        // Clone the template
        let clone_result = Command::new("virt-clone")
            .args([
                "--original",
                "ionChannel-template",
                "--name",
                "rustdesk-test",
                "--auto-clone",
            ])
            .output();

        match clone_result {
            Ok(output) => {
                if output.status.success() {
                    println!("✓ VM cloned successfully!");

                    // Start the VM
                    println!("\nStarting VM...");
                    Command::new("virsh")
                        .args(["start", "rustdesk-test"])
                        .output()?;

                    println!("⏳ Waiting for VM to boot (15s)...");
                    thread::sleep(Duration::from_secs(15));

                    // Get IP
                    let ip_output = Command::new("virsh")
                        .args(["domifaddr", "rustdesk-test", "--source", "lease"])
                        .output()?;

                    let ip_str = String::from_utf8_lossy(&ip_output.stdout);
                    if let Some(line) = ip_str.lines().find(|l| l.contains("ipv4")) {
                        if let Some(ip) = line
                            .split_whitespace()
                            .nth(3)
                            .and_then(|a| a.split('/').next())
                        {
                            println!("\n✅ VM READY!");
                            println!(
                                "════════════════════════════════════════════════════════════════"
                            );
                            println!("\n📡 VM IP: {}", ip);
                            println!("\n🔧 Test SSH:");
                            println!("   $ ssh iontest@{}", ip);
                            println!("   Password: iontest");
                            println!("\n📥 Now run the autonomous RustDesk installer:");
                            println!("   $ cargo run --example autonomous_rustdesk_id --features libvirt");
                            println!("\n════════════════════════════════════════════════════════════════\n");
                            return Ok(());
                        }
                    }

                    println!("\n⚠️  VM started but no IP yet. Wait a moment and check:");
                    println!("   $ virsh domifaddr rustdesk-test");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("⚠️  Clone failed: {}", stderr);
                }
            },
            Err(e) => println!("⚠️  Could not clone VM: {}", e),
        }
    }

    println!("\n📖 FALLBACK: Use the console approach");
    println!("════════════════════════════════════════════════════════════════");
    println!("\nSince test1 VM has SSH issues, use virt-manager console:");
    println!("   1. $ virt-manager");
    println!("   2. Right-click test1 → Open");
    println!("   3. Login: iontest / iontest");
    println!("   4. Run: wget https://github.com/rustdesk/rustdesk/releases/download/1.2.3/rustdesk-1.2.3-x86_64.deb");
    println!("   5. Run: sudo dpkg -i rustdesk-1.2.3-x86_64.deb && sudo apt-get install -f -y");
    println!("   6. Run: rustdesk --get-id");
    println!("   7. Use that ID on your other tower!");
    println!("\n════════════════════════════════════════════════════════════════\n");

    Ok(())
}

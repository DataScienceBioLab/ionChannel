//! Quick helper to get RustDesk ID from a VM

use ion_validation::{
    providers::desktop::{RemoteDesktop, Target},
    impls::rustdesk_provider::RustDeskProvider,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n🔍 Getting RustDesk ID from test1 VM\n");

    // Target VM
    let target = Target {
        host: "192.168.122.61".to_string(),
        port: 22,
        username: "iontest".to_string(),
        password: Some("iontest".to_string()),
    };

    // Use our RustDesk provider
    let provider = RustDeskProvider::new();

    println!("📡 Connecting to VM...");
    
    // Try to get RustDesk ID
    match provider.get_id(&target).await {
        Ok(id) => {
            println!("\n✅ SUCCESS!");
            println!("═══════════════════════════════════════════════════════════════");
            println!("\n🎯 RustDesk ID: {}\n", id);
            println!("═══════════════════════════════════════════════════════════════");
            println!("\nTo connect from another tower:");
            println!("  1. Install RustDesk on the other tower");
            println!("  2. Open RustDesk");
            println!("  3. Enter ID: {}", id);
            println!("  4. Connect! ✅\n");
        }
        Err(e) => {
            println!("\n⚠️  Could not get RustDesk ID: {:?}", e);
            println!("\nThis likely means:");
            println!("  • RustDesk is not installed yet");
            println!("  • RustDesk service is not running");
            println!("\nTo install manually:");
            println!("  1. ssh iontest@192.168.122.61");
            println!("  2. sudo dpkg -i /tmp/rustdesk.deb");
            println!("  3. rustdesk &");
            println!("  4. rustdesk --get-id\n");
        }
    }

    Ok(())
}


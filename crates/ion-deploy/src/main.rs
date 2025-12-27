//! ionChannel Deployment Tool
//! 
//! Pure Rust tool for VM discovery, deployment, and testing.

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use tracing::Level;

mod discovery;
mod ssh;
mod deploy;
mod config;

use discovery::VmDiscovery;

#[derive(Parser)]
#[command(name = "ion-deploy")]
#[command(author = "ionChannel Team")]
#[command(version)]
#[command(about = "Agent-guided deployment for ionChannel", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover available VMs
    Discover {
        /// Force rediscovery (ignore cache)
        #[arg(short, long)]
        force: bool,
    },

    /// Deploy ionChannel to a VM
    Deploy {
        /// VM IP address (auto-discovered if not provided)
        #[arg(short, long)]
        ip: Option<String>,

        /// SSH username
        #[arg(short, long)]
        user: Option<String>,

        /// Skip building (deploy only)
        #[arg(long)]
        skip_build: bool,

        /// Skip portal deployment
        #[arg(long)]
        skip_portal: bool,
    },

    /// Test connection to VM
    Test {
        /// VM IP address
        ip: String,

        /// SSH username
        #[arg(short, long)]
        user: Option<String>,
    },

    /// Show current configuration
    Config {
        /// Reset configuration
        #[arg(short, long)]
        reset: bool,
    },

    /// Get RustDesk connection info from VM
    Info {
        /// VM IP address (uses last deployed if not provided)
        #[arg(short, long)]
        ip: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Load or create config
    let config_path = cli.config.as_deref();
    let mut config = config::Config::load(config_path)?;

    println!("{}", style("════════════════════════════════════════════════════════════════").cyan());
    println!("  {} {}", style("🤖").cyan(), style("ionChannel Deployment Tool").bold().cyan());
    println!("{}", style("════════════════════════════════════════════════════════════════").cyan());
    println!();

    match cli.command {
        Commands::Discover { force } => {
            discover_vms(&mut config, force).await?;
        }

        Commands::Deploy {
            ip,
            user,
            skip_build,
            skip_portal,
        } => {
            deploy_to_vm(&mut config, ip, user, skip_build, skip_portal).await?;
        }

        Commands::Test { ip, user } => {
            test_vm_connection(&config, &ip, user.as_deref()).await?;
        }

        Commands::Config { reset } => {
            if reset {
                config.reset()?;
                println!("{} Configuration reset", style("✓").green());
            } else {
                println!("{:#?}", config);
            }
        }

        Commands::Info { ip } => {
            get_vm_info(&config, ip.as_deref()).await?;
        }
    }

    Ok(())
}

async fn discover_vms(config: &mut config::Config, force: bool) -> Result<()> {
    println!("{} Discovering VMs...", style("[1/3]").blue());
    println!();

    let discovery = VmDiscovery::new();
    let vms = discovery.discover_all().await?;

    if vms.is_empty() {
        println!("{} No VMs auto-discovered", style("⚠️").yellow());
        println!();
        println!("Try manual entry with:");
        println!("  {} --ip <IP> --user <USER>", style("ion-deploy deploy").cyan());
        return Ok(());
    }

    println!("{} Found {} VM(s)", style("✓").green(), vms.len());
    println!();

    for (i, vm) in vms.iter().enumerate() {
        println!(
            "  {}) {} - {} (via {})",
            style(i + 1).cyan(),
            style(&vm.name).bold(),
            style(&vm.ip).dim(),
            style(&vm.discovery_method).dim()
        );
    }

    println!();
    
    // Save discovered VMs to config
    config.discovered_vms = vms;
    config.save()?;

    println!("{} Configuration saved", style("✓").green());

    Ok(())
}

async fn deploy_to_vm(
    config: &mut config::Config,
    ip: Option<String>,
    user: Option<String>,
    skip_build: bool,
    skip_portal: bool,
) -> Result<()> {
    println!("{} Starting deployment...", style("[Phase 1/4]").blue());
    println!();

    // Get target VM
    let target = if let Some(ip_addr) = ip {
        discovery::VmInfo {
            name: "manual".to_string(),
            ip: ip_addr,
            discovery_method: "manual".to_string(),
            username: user,
        }
    } else if let Some(last_vm) = &config.last_vm {
        println!("Using last VM: {} ({})", style(&last_vm.name).bold(), style(&last_vm.ip).dim());
        last_vm.clone()
    } else if !config.discovered_vms.is_empty() {
        // Interactive selection would go here
        config.discovered_vms[0].clone()
    } else {
        anyhow::bail!("No VM specified. Run 'ion-deploy discover' first or use --ip");
    };

    println!("Target: {} ({})", style(&target.name).bold(), target.ip);
    println!();

    // Test connection first
    println!("{} Testing connection...", style("[Phase 2/4]").blue());
    test_vm_connection(config, &target.ip, target.username.as_deref()).await?;
    println!();

    // Deploy
    println!("{} Deploying...", style("[Phase 3/4]").blue());
    deploy::deploy_to_vm(&target, skip_build, skip_portal).await?;
    println!();

    // Get RustDesk info
    println!("{} Getting connection info...", style("[Phase 4/4]").blue());
    get_vm_info(config, Some(&target.ip)).await?;

    // Save as last used
    config.last_vm = Some(target);
    config.save()?;

    println!();
    println!("{}", style("════════════════════════════════════════════════════════════════").green());
    println!(" {} {}", style("🎉").bold(), style("DEPLOYMENT COMPLETE!").bold().green());
    println!("{}", style("════════════════════════════════════════════════════════════════").green());

    Ok(())
}

async fn test_vm_connection(
    _config: &config::Config,
    ip: &str,
    user: Option<&str>,
) -> Result<()> {
    let username = user.unwrap_or_else(|| {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "ubuntu".to_string())
    });

    println!("Testing connection to {}@{}...", username, ip);

    let can_connect = ssh::test_connection(ip, &username).await?;

    if can_connect {
        println!("{} Connection successful", style("✓").green());
    } else {
        anyhow::bail!("Cannot connect to {}@{}", username, ip);
    }

    Ok(())
}

async fn get_vm_info(_config: &config::Config, ip: Option<&str>) -> Result<()> {
    let target_ip = ip.ok_or_else(|| anyhow::anyhow!("No VM IP specified"))?;

    println!("Getting RustDesk info from {}...", target_ip);

    // This would query the VM for RustDesk ID
    // For now, placeholder
    println!();
    println!("{}", style("Connection Info:").bold());
    println!("  VM IP:        {}", style(target_ip).cyan());
    println!("  RustDesk ID:  {}", style("[Query VM]").dim());

    Ok(())
}


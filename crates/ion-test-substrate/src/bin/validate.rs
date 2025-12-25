// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! CLI tool for validating ionChannel portal implementation.
//!
//! Runs headlessly, suitable for CI/CD pipelines and agent automation.

use clap::{Parser, ValueEnum};
use ion_test_substrate::{TestHarness, TestHarnessConfig, ValidationResult};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Human-readable text
    Text,
    /// JSON for CI parsing
    Json,
    /// Compact summary
    Summary,
}

#[derive(Parser, Debug)]
#[command(name = "ion-validate")]
#[command(about = "Validate ionChannel RemoteDesktop portal implementation")]
#[command(version)]
struct Args {
    /// Output format
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,

    /// Run specific test only
    #[arg(short, long)]
    test: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Timeout in milliseconds
    #[arg(long, default_value = "5000")]
    timeout: u64,
}

fn print_result_text(result: &ValidationResult) {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║               ionChannel Validation Report                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    for check in &result.checks {
        let status = if check.passed { "✓" } else { "✗" };
        let color_start = if check.passed { "\x1b[32m" } else { "\x1b[31m" };
        let color_end = "\x1b[0m";

        println!("║ {color_start}{status}{color_end} {:<56} ║", check.name);
        println!("║   {:<58} ║", check.message);
        if let Some(ref spec) = check.spec_ref {
            println!("║   Spec: {:<52} ║", spec);
        }
    }

    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║ Total: {}  Passed: {}  Failed: {}                            ║",
        result.stats.total, result.stats.passed, result.stats.failed
    );

    let overall = if result.all_passed {
        "\x1b[32m✓ ALL CHECKS PASSED\x1b[0m"
    } else {
        "\x1b[31m✗ SOME CHECKS FAILED\x1b[0m"
    };
    println!("║ {overall:<61} ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

fn print_result_json(result: &ValidationResult) {
    match serde_json::to_string_pretty(result) {
        Ok(json) => println!("{json}"),
        Err(e) => error!("Failed to serialize result: {e}"),
    }
}

fn print_result_summary(result: &ValidationResult) {
    let status = if result.all_passed { "PASS" } else { "FAIL" };
    println!(
        "{status} - {}/{} checks passed",
        result.stats.passed, result.stats.total
    );

    if !result.all_passed {
        for check in result.failures() {
            println!("  ✗ {}: {}", check.name, check.message);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Setup tracing
    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("ionChannel Validator starting...");

    // Create test harness
    let config = TestHarnessConfig {
        verbose: args.verbose,
        timeout_ms: args.timeout,
    };

    let harness = TestHarness::spawn_with_config(config).await?;

    // Run tests
    let result = if let Some(ref test_name) = args.test {
        info!("Running specific test: {test_name}");
        match test_name.as_str() {
            "smoke" => harness.smoke_test().await?,
            "session" => {
                // Just session lifecycle
                let session = harness.create_session("test-session").await?;
                harness
                    .select_devices(&session, ion_test_substrate::DeviceType::all())
                    .await?;
                harness.start_session(&session).await?;
                harness.close_session(&session).await?;
                harness.validate().await
            },
            _ => {
                error!("Unknown test: {test_name}");
                std::process::exit(1);
            },
        }
    } else {
        harness.smoke_test().await?
    };

    // Output result
    match args.format {
        OutputFormat::Text => print_result_text(&result),
        OutputFormat::Json => print_result_json(&result),
        OutputFormat::Summary => print_result_summary(&result),
    }

    // Exit with appropriate code
    if result.all_passed {
        Ok(())
    } else {
        std::process::exit(1)
    }
}

//! Integration tests for autonomous provisioning
//!
//! These tests verify the end-to-end autonomous provisioning flow.

use ion_deploy::autonomous::{AutonomousProvisioner, AutonomousProvisionConfig};
use ion_deploy::cloud_init::CloudInitBuilder;
use ion_deploy::ssh_keys::SshKeyManager;
use std::path::PathBuf;

#[tokio::test]
async fn test_ssh_key_and_cloud_init_integration() {
    // Test that SSH keys can be generated and injected into cloud-init
    let manager = SshKeyManager::new();
    let keypair = manager
        .generate_key_pair("integration_test")
        .await
        .expect("Failed to generate keypair");

    let builder = CloudInitBuilder::new()
        .hostname("test-vm")
        .add_user("testuser", vec![keypair.public_key.clone()]);

    let yaml = builder.build_yaml().expect("Failed to build YAML");

    // Verify the public key is in the YAML
    assert!(yaml.contains(&keypair.public_key));
    assert!(yaml.contains("testuser"));
}

#[tokio::test]
async fn test_autonomous_config_builder() {
    let config = AutonomousProvisionConfig {
        vm_name: "test-integration".to_string(),
        ram_mb: 2048,
        vcpus: 2,
        disk_gb: 10,
        username: "ubuntu".to_string(),
        ssh_port: 22,
        network: "default".to_string(),
        packages: vec!["git".to_string()],
        base_image: PathBuf::from("/tmp/test.img"),
        work_dir: std::env::temp_dir().join("ion-test"),
    };

    let _provisioner = AutonomousProvisioner::new(config.clone());

    // Verify config values
    assert_eq!(config.ram_mb, 2048);
    assert_eq!(config.vcpus, 2);
    assert_eq!(config.packages.len(), 1);
}

#[tokio::test]
async fn test_multiple_concurrent_key_generation() {
    let manager = SshKeyManager::new();
    
    // Generate 10 keys concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mgr = manager.clone();
            tokio::spawn(async move {
                mgr.generate_key_pair(&format!("concurrent_{}", i)).await
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task panicked").expect("Key generation failed"))
        .collect();

    // All should succeed and be unique
    assert_eq!(results.len(), 10);
    
    for i in 0..results.len() {
        for j in (i + 1)..results.len() {
            assert_ne!(results[i].public_key, results[j].public_key);
        }
    }
}

#[tokio::test]
async fn test_cloud_init_and_metadata_integration() {
    use ion_deploy::cloud_init::{create_meta_data, CloudInitBuilder};
    use std::fs;

    let temp_dir = std::env::temp_dir().join("ion_integration_test");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let user_data_path = temp_dir.join("user-data");
    let meta_data_path = temp_dir.join("meta-data");

    // Create user-data
    CloudInitBuilder::new()
        .hostname("integration-vm")
        .add_user("testuser", vec!["ssh-rsa test".to_string()])
        .build_to_file(&user_data_path)
        .expect("Failed to write user-data");

    // Create meta-data
    create_meta_data("integration-vm", &meta_data_path).expect("Failed to create meta-data");

    // Verify both files exist
    assert!(user_data_path.exists());
    assert!(meta_data_path.exists());

    // Verify content
    let user_data = fs::read_to_string(&user_data_path).expect("Failed to read user-data");
    assert!(user_data.contains("hostname: integration-vm"));

    let meta_data = fs::read_to_string(&meta_data_path).expect("Failed to read meta-data");
    assert!(meta_data.contains("instance-id: integration-vm"));

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_autonomous_provisioner_destroy_is_safe() {
    // Verify that destroy can be called even with invalid config
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    runtime.block_on(async {
        let config = AutonomousProvisionConfig {
            vm_name: "nonexistent-vm".to_string(),
            work_dir: std::env::temp_dir().join("nonexistent"),
            ..Default::default()
        };

        let provisioner = AutonomousProvisioner::new(config);
        
        // Should not panic or fail
        let result = provisioner.destroy().await;
        assert!(result.is_ok());
    });
}


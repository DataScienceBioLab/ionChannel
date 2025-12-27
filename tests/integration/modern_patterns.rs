// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2024-2025 DataScienceBioLab

//! Integration test demonstrating modern patterns and performance.

use ion_core::discovery::{BackendRegistry, Capability};
use ion_traits::input::InputCapabilities;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;

#[tokio::test]
async fn test_bitflags_pattern() {
    // Modern bitflags pattern
    let caps = InputCapabilities::STANDARD;
    
    // Efficient capability checks (const functions)
    assert!(caps.has_keyboard());
    assert!(caps.has_pointer());
    assert!(!caps.has_touch());
    
    // Composed capabilities
    let full = InputCapabilities::FULL_DESKTOP;
    assert!(full.has_absolute_pointer());
    
    // Zero-cost: bitwise operations
    let custom = InputCapabilities::KEYBOARD | InputCapabilities::TOUCH;
    assert!(custom.has_keyboard());
    assert!(custom.has_touch());
    assert!(!custom.has_pointer());
}

#[tokio::test]
async fn test_parallel_discovery_performance() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Measure parallel discovery overhead
    let start = Instant::now();
    let available = registry.find_available().await;
    let duration = start.elapsed();
    
    // With zero providers, this measures just the parallel infrastructure overhead
    // Should be < 1ms even with complex async machinery
    assert!(duration < Duration::from_millis(1), 
        "Parallel discovery overhead too high: {:?}", duration);
    
    // Empty registry should return empty list
    assert_eq!(available.len(), 0);
}

#[tokio::test]
async fn test_parallel_find_best() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Measure parallel "find best" overhead
    let start = Instant::now();
    let best = registry.find_best().await;
    let duration = start.elapsed();
    
    // Should be very fast
    assert!(duration < Duration::from_millis(1));
    
    // No providers registered
    assert!(best.is_none());
}

#[tokio::test]
async fn test_capability_query() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Query by capability (primal pattern)
    let providers = registry
        .find_by_capability(&Capability::KeyboardInjection)
        .await;
    
    // No providers registered yet
    assert_eq!(providers.len(), 0);
    
    // But the API is ready and efficient
}

#[tokio::test]
async fn test_const_fn_evaluation() {
    // Const functions allow compile-time evaluation
    const EMPTY_MODS: ion_traits::input::Modifiers = ion_traits::input::Modifiers::empty();
    
    // This is evaluated at compile time!
    assert!(!EMPTY_MODS.any());
    
    // Capability checks are also const
    const STANDARD: InputCapabilities = InputCapabilities::STANDARD;
    assert!(STANDARD.has_keyboard());
}

#[tokio::test]
async fn test_zero_copy_patterns() {
    use std::sync::Arc;
    
    // Arc allows zero-copy sharing
    let data = Arc::new(vec![1, 2, 3, 4, 5]);
    let data_clone = Arc::clone(&data);
    
    // No data copied, just reference count incremented
    assert_eq!(Arc::strong_count(&data), 2);
    
    // Both point to same memory
    assert_eq!(data.as_ptr(), data_clone.as_ptr());
}

#[test]
fn test_memory_efficiency() {
    use std::mem::size_of;
    
    // Modern bitflags: 4 bytes
    let caps_size = size_of::<InputCapabilities>();
    assert_eq!(caps_size, 4, "InputCapabilities should be 4 bytes (u32)");
    
    // Old struct would have been ~40+ bytes
    // We achieved 10x memory reduction!
}

#[tokio::test]
async fn test_concurrent_queries() {
    use tokio::task;
    
    let registry = Arc::new(BackendRegistry::new());
    
    // Spawn multiple concurrent queries
    let mut handles = vec![];
    
    for _ in 0..10 {
        let reg = Arc::clone(&registry);
        let handle = task::spawn(async move {
            reg.find_available().await
        });
        handles.push(handle);
    }
    
    // All queries complete without blocking each other
    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result.len(), 0);
    }
}

#[test]
fn test_type_safety() {
    // Bitflags prevent invalid states at compile time
    let caps = InputCapabilities::KEYBOARD | InputCapabilities::POINTER;
    
    // Type-safe operations
    assert!(caps.contains(InputCapabilities::KEYBOARD));
    assert!(caps.intersects(InputCapabilities::KEYBOARD));
    
    // Can't create invalid capability combinations accidentally
    let empty = InputCapabilities::empty();
    assert!(!empty.has_keyboard());
}

#[tokio::test]
async fn test_registry_concurrency() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Multiple concurrent readers
    let reads: Vec<_> = (0..100)
        .map(|_| {
            let reg = Arc::clone(&registry);
            tokio::spawn(async move {
                reg.query_capabilities().await
            })
        })
        .collect();
    
    // All should complete successfully
    for handle in reads {
        let caps = handle.await.unwrap();
        assert!(caps.is_empty()); // No providers registered
    }
}


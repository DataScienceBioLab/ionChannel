# Capability-Based Backend Discovery

ionChannel implements a **primal discovery system** where backends are discovered at runtime based on their capabilities, not hardcoded types.

## Philosophy

### Primal Principles

1. **Self-Knowledge** - Each backend knows its own capabilities
2. **Runtime Discovery** - Backends are discovered when the service starts
3. **No Hardcoding** - No hardcoded backend selection logic
4. **Capability Queries** - Query by what backends CAN DO, not what they ARE

## Architecture

```rust
┌─────────────────────────────────────────────────────────┐
│                  BackendRegistry                        │
│                                                         │
│  Registered Providers:                                 │
│  ┌──────────────────┐  ┌──────────────────┐          │
│  │ CosmicProvider   │  │ WaylandProvider  │          │
│  │                  │  │                  │          │
│  │ Capabilities:    │  │ Capabilities:    │          │
│  │ • COSMIC env     │  │ • Wayland env    │          │
│  │ • Keyboard       │  │ • Keyboard       │          │
│  │ • Pointer        │  │ • Pointer        │          │
│  └──────────────────┘  └──────────────────┘          │
│                                                         │
│  Query by Capability → Returns Available Providers     │
└─────────────────────────────────────────────────────────┘
```

## Core Types

### Capability Enum

Represents what a backend can do:

```rust
pub enum Capability {
    /// Can inject keyboard events
    InjectKeyboard,
    
    /// Can inject pointer events
    InjectPointer,
    
    /// Can capture screen
    CaptureScreen,
    
    /// Supports a specific display server
    DisplayServer(DisplayServerType),
    
    /// Custom capability (extensible)
    Custom(String),
}
```

### BackendProvider Trait

Backends implement this to participate in discovery:

```rust
pub trait BackendProvider: Send + Sync {
    /// Unique identifier (e.g., "cosmic", "wayland")
    fn id(&self) -> &str;
    
    /// Human-readable name
    fn name(&self) -> &str;
    
    /// Check if available in current environment
    fn is_available<'a>(&'a self) 
        -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
    
    /// Declare capabilities
    fn capabilities(&self) -> Vec<Capability>;
    
    /// Create backend instance if available
    fn create_backend<'a>(&'a self) 
        -> Pin<Box<dyn Future<Output = Option<Arc<dyn CompositorBackend>>> + Send + 'a>>;
}
```

### BackendRegistry

Central discovery service:

```rust
pub struct BackendRegistry {
    providers: Arc<RwLock<Vec<Arc<dyn BackendProvider>>>>,
}

impl BackendRegistry {
    /// Register a provider
    pub async fn register(&self, provider: Arc<dyn BackendProvider>);
    
    /// Find providers with a specific capability
    pub async fn find_by_capability(&self, cap: &Capability) 
        -> Vec<Arc<dyn BackendProvider>>;
    
    /// Find all available providers
    pub async fn find_available(&self) -> Vec<Arc<dyn BackendProvider>>;
    
    /// Get the best available backend
    pub async fn find_best(&self) -> Option<Arc<dyn BackendProvider>>;
    
    /// Create backend from best provider
    pub async fn create_best_backend(&self) 
        -> Option<Arc<dyn CompositorBackend>>;
}
```

## Usage Examples

### Basic Setup

```rust
use ion_core::discovery::BackendRegistry;
use ion_backend_cosmic::provider::CosmicProvider;
use ion_backend_wayland::provider::WaylandProvider;

// Create registry
let registry = BackendRegistry::new();

// Register providers in priority order
registry.register(Arc::new(CosmicProvider)).await;
registry.register(Arc::new(WaylandProvider)).await;

// Automatically select best available
let backend = registry.create_best_backend().await
    .expect("No compatible backend found");
```

### Query by Capability

```rust
// Find all providers that can inject keyboard input
let keyboard_providers = registry
    .find_by_capability(&Capability::InjectKeyboard)
    .await;

for provider in keyboard_providers {
    println!("Found keyboard provider: {}", provider.name());
}
```

### Check Availability

```rust
// Get all currently available providers
let available = registry.find_available().await;

println!("Available backends:");
for provider in available {
    let caps = provider.capabilities();
    println!("  - {}: {:?}", provider.name(), caps);
}
```

### Query All Capabilities

```rust
// Get a map of all registered providers and their capabilities
let all_caps = registry.query_capabilities().await;

for (id, capabilities) in all_caps {
    println!("Provider '{}' supports:", id);
    for cap in capabilities {
        println!("  - {:?}", cap);
    }
}
```

## Implementation Guide

### Creating a New Provider

To add a new backend:

1. **Implement the Backend**

```rust
// my_backend/src/lib.rs
pub struct MyBackend {
    // backend state
}

#[async_trait]
impl CompositorBackend for MyBackend {
    // implement trait methods
}
```

2. **Create the Provider**

```rust
// my_backend/src/provider.rs
pub struct MyProvider;

impl BackendProvider for MyProvider {
    fn id(&self) -> &str {
        "my_backend"
    }
    
    fn name(&self) -> &str {
        "My Custom Backend"
    }
    
    fn is_available<'a>(&'a self) 
        -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> 
    {
        Box::pin(async {
            // Check if this backend can run in current environment
            std::env::var("MY_BACKEND_DISPLAY").is_ok()
        })
    }
    
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::InjectKeyboard,
            Capability::InjectPointer,
            Capability::DisplayServer(DisplayServerType::Wayland),
        ]
    }
    
    fn create_backend<'a>(&'a self) 
        -> Pin<Box<dyn Future<Output = Option<Arc<dyn CompositorBackend>>> + Send + 'a>> 
    {
        Box::pin(async {
            if !self.is_available().await {
                return None;
            }
            
            let backend = MyBackend::new();
            Some(Arc::new(backend) as Arc<dyn CompositorBackend>)
        })
    }
}
```

3. **Register at Startup**

```rust
// portal_service/src/main.rs
let registry = BackendRegistry::new();

// Register in priority order (first = highest priority)
registry.register(Arc::new(CosmicProvider)).await;
registry.register(Arc::new(MyProvider)).await;
registry.register(Arc::new(WaylandProvider)).await;

let backend = registry.create_best_backend().await?;
```

## Design Benefits

### For Developers

- **No configuration needed** - Backends register themselves
- **Extensible** - Add new backends without changing core code
- **Testable** - Easy to test with mock providers
- **Type-safe** - Capability enum prevents typos

### For the System

- **Automatic fallback** - If best backend unavailable, tries next
- **Priority ordering** - Registration order defines priority
- **Runtime detection** - Adapts to current environment
- **Zero hardcoding** - No if/else chains based on environment

### For AI Agents

- **Discoverable** - Agents can query available capabilities
- **Observable** - Registry provides introspection
- **Predictable** - Consistent capability-based API
- **Self-documenting** - Backends declare what they can do

## Future Extensions

### Custom Capabilities

```rust
// Add domain-specific capabilities
let custom = Capability::Custom("supports_3d_acceleration".to_string());
registry.register(Arc::new(GpuProvider { custom_cap: custom })).await;

// Query for custom capability
let gpu_providers = registry
    .find_by_capability(&Capability::Custom("supports_3d_acceleration".to_string()))
    .await;
```

### Capability Composition

```rust
// Find providers with multiple capabilities
let full_featured = registry.find_available().await
    .into_iter()
    .filter(|p| {
        let caps = p.capabilities();
        caps.contains(&Capability::InjectKeyboard) &&
        caps.contains(&Capability::InjectPointer) &&
        caps.contains(&Capability::CaptureScreen)
    })
    .collect::<Vec<_>>();
```

### Dynamic Capability Updates

Future: Providers could update their capabilities at runtime based on changing environment conditions.

## Testing

The discovery system includes comprehensive tests:

```rust
#[tokio::test]
async fn test_find_by_capability() {
    let registry = BackendRegistry::new();
    
    registry.register(Arc::new(KeyboardOnlyProvider)).await;
    registry.register(Arc::new(FullFeaturedProvider)).await;
    
    let kbd = registry
        .find_by_capability(&Capability::InjectKeyboard)
        .await;
    
    assert_eq!(kbd.len(), 2); // Both providers support keyboard
}

#[tokio::test]
async fn test_priority_order() {
    let registry = BackendRegistry::new();
    
    // Register in priority order
    registry.register(Arc::new(HighPriorityProvider)).await;
    registry.register(Arc::new(LowPriorityProvider)).await;
    
    let best = registry.find_best().await.unwrap();
    assert_eq!(best.id(), "high_priority");
}
```

## Summary

The capability-based discovery system embodies the primal philosophy:

- ✅ Backends have self-knowledge (declare capabilities)
- ✅ Runtime discovery (no hardcoding)
- ✅ Capability-based queries (what can you do?)
- ✅ Extensible (add backends without core changes)
- ✅ Zero configuration (automatic detection)

This makes ionChannel ready for a multi-backend future where new compositors and display servers can be supported by simply implementing a provider and registering it.


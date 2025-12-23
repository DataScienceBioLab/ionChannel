# IPC Alternatives to D-Bus

> Exploring pure Rust options for inter-process communication

## Current Architecture

```
┌──────────────────┐          ┌──────────────────┐
│   RustDesk       │ ──D-Bus──▶  ion-portal      │
│   (client)       │          │  (portal core)   │
└──────────────────┘          └────────┬─────────┘
                                       │
                              ┌────────▼─────────┐
                              │  ion-compositor  │
                              │  (input inject)  │
                              └──────────────────┘
```

D-Bus is used because:
1. xdg-desktop-portal spec requires it
2. Standard for desktop services on Linux
3. Existing tooling and debugging

## The D-Bus Problem

| Issue | Impact |
|-------|--------|
| C dependency (dbus-daemon) | Build complexity |
| Session bus required | Not available pre-login |
| Complex protocol | Large crate surface |
| XML introspection | Boilerplate generation |
| Performance | ~50-100μs per call overhead |

## Pure Rust Alternatives

### 1. Unix Domain Sockets + Protocol Buffers

```rust
// Using tokio and prost
use tokio::net::UnixStream;
use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub struct PointerMotionRequest {
    #[prost(string, tag = "1")]
    pub session_id: String,
    #[prost(double, tag = "2")]
    pub dx: f64,
    #[prost(double, tag = "3")]
    pub dy: f64,
}

async fn notify_pointer_motion(stream: &mut UnixStream, req: PointerMotionRequest) {
    let bytes = req.encode_to_vec();
    stream.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    stream.write_all(&bytes).await?;
}
```

**Pros:**
- Pure Rust (prost, tokio)
- Fast (~1-5μs per call)
- Simple, no daemon

**Cons:**
- Need to define wire protocol
- No service discovery
- Breaking xdg-portal spec

### 2. varlink (Modern JSON-RPC)

```rust
// varlink interface definition
interface io.ionChannel

method CreateSession(app_id: string) -> (session_id: string)
method NotifyPointerMotion(session_id: string, dx: float, dy: float) -> ()

// Generated Rust client
let client = VarlinkClient::connect("unix:/run/ion-portal.socket")?;
client.notify_pointer_motion(session_id, 10.0, 5.0)?;
```

**Pros:**
- Pure Rust implementation exists (varlink-rs)
- Self-documenting
- Typed interfaces

**Cons:**
- Less common than D-Bus
- Still need socket management

### 3. gRPC over Unix Sockets

```protobuf
// ion_portal.proto
service RemoteDesktop {
  rpc CreateSession (CreateSessionRequest) returns (CreateSessionResponse);
  rpc NotifyPointerMotion (PointerMotionRequest) returns (Empty);
}
```

```rust
use tonic::transport::Channel;

let channel = Channel::from_static("http://[::]:50051")
    .connect()
    .await?;
let client = RemoteDesktopClient::new(channel);
client.notify_pointer_motion(request).await?;
```

**Pros:**
- Pure Rust (tonic)
- Excellent tooling
- Streaming support

**Cons:**
- HTTP/2 overhead for local IPC
- Overkill for simple calls

### 4. Shared Memory + Atomics

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use memmap2::MmapMut;

struct SharedEventRing {
    mmap: MmapMut,
    write_idx: AtomicU64,
    read_idx: AtomicU64,
}

impl SharedEventRing {
    fn push(&self, event: InputEvent) {
        let idx = self.write_idx.fetch_add(1, Ordering::SeqCst);
        // Write event to ring buffer
        unsafe {
            let ptr = self.mmap.as_ptr().add((idx % RING_SIZE) as usize);
            std::ptr::write(ptr as *mut InputEvent, event);
        }
    }
}
```

**Pros:**
- Fastest possible (~10ns)
- No syscalls for data transfer
- Pure Rust

**Cons:**
- Complex synchronization
- Requires careful memory management
- Limited to same machine

### 5. Capnproto RPC

```capnp
interface RemoteDesktop {
  createSession @0 (appId :Text) -> (sessionId :Text);
  notifyPointerMotion @1 (sessionId :Text, dx :Float64, dy :Float64);
}
```

**Pros:**
- Zero-copy deserialization
- Pure Rust (capnp-rpc)
- Time-travel debugging

**Cons:**
- Learning curve
- Smaller community

## Recommendation: Layered Approach

```
┌─────────────────────────────────────────────────────────────────┐
│                        ion-portal                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────────────┐                                           │
│   │   PortalCore    │  ◄── Business logic (transport-agnostic) │
│   └────────┬────────┘                                           │
│            │                                                    │
│   ┌────────┴────────────────────────────────────┐               │
│   │              Transport Layer                 │               │
│   │  ┌──────────┐  ┌──────────┐  ┌──────────┐   │               │
│   │  │  D-Bus   │  │  Unix    │  │  Shared  │   │               │
│   │  │ (compat) │  │  Socket  │  │  Memory  │   │               │
│   │  └──────────┘  └──────────┘  └──────────┘   │               │
│   └─────────────────────────────────────────────┘               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Strategy:**

1. **Today:** PortalCore already separated from D-Bus ✅
2. **Near term:** Add Unix socket transport for performance
3. **Future:** Shared memory for compositor integration

## Implementation Plan

### Phase 1: Abstract Transport (Done)

```rust
// core.rs - business logic
pub struct PortalCore { ... }

impl PortalCore {
    pub async fn notify_pointer_motion(&self, session_id: &str, dx: f64, dy: f64) -> Result<()>
}
```

### Phase 2: Transport Trait

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, message: Message) -> Result<Response>;
    async fn recv(&self) -> Result<Message>;
}

pub struct DbusTransport { ... }
pub struct UnixSocketTransport { ... }
pub struct SharedMemoryTransport { ... }
```

### Phase 3: High-Performance Path

For compositor integration, bypass IPC entirely:

```rust
// In cosmic-comp, direct function call
impl State {
    fn handle_remote_input(&mut self, event: InputEvent) {
        // No IPC, no serialization
        match event {
            InputEvent::PointerMotion { dx, dy } => {
                self.pointer.motion(dx, dy);
            }
            // ...
        }
    }
}
```

## Benchmarks (Estimated)

| Transport | Latency | Throughput |
|-----------|---------|------------|
| D-Bus | ~100μs | ~10K msg/s |
| Unix Socket (protobuf) | ~5μs | ~200K msg/s |
| Shared Memory | ~10ns | ~100M msg/s |
| Direct call | <1ns | Unlimited |

## Compatibility Strategy

```rust
#[cfg(feature = "dbus")]
pub mod dbus_transport;

#[cfg(feature = "unix-socket")]
pub mod socket_transport;

// Default: D-Bus for portal spec compliance
// Optional: Unix socket for performance
// Compositor: Direct integration
```

## Conclusion

The refactoring done today (separating `PortalCore` from D-Bus) enables:

1. **Full unit testing** without D-Bus
2. **Future transport flexibility** 
3. **Pure Rust path** when ready

For now, D-Bus is required for portal spec compliance. But the architecture is ready to evolve.

---

*ionChannel IPC Strategy — December 2024*


# Async Handlers with RpcContext

This document describes the new async handler pattern introduced to support RPC proxying and event-driven services.

## Overview

Girolle now supports async service handlers that receive an `RpcContext`, enabling:
- ✅ Access to AMQP metadata (correlation_id, headers, reply_to, routing_key)
- ✅ Call other services (RPC proxy pattern)
- ✅ Emit events
- ✅ Propagate call_id_stack for tracing
- ✅ Full backward compatibility with sync handlers

## Quick Start

### Traditional Sync Handler (Still Supported)

```rust
use girolle::prelude::*;

#[girolle]
fn hello(name: String) -> String {
    format!("Hello, {}", name)
}
```

### New Async Handler with Context

```rust
use girolle::prelude::*;
use std::sync::Arc;

#[girolle]
async fn hello_async(ctx: Arc<RpcContext>, name: String) -> String {
    // Access AMQP metadata
    let correlation_id = &ctx.correlation_id;
    let routing_key = &ctx.routing_key;
    
    // Get call stack for tracing
    let call_stack = ctx.get_call_id_stack();
    
    format!("Hello, {}! (correlation: {})", name, correlation_id)
}
```

## RpcContext Fields

```rust
pub struct RpcContext {
    /// The correlation ID from the inbound message
    pub correlation_id: String,
    
    /// The reply-to queue from the inbound message
    pub reply_to: String,
    
    /// The headers from the inbound message
    pub headers: FieldTable,
    
    /// The routing key from the inbound message
    pub routing_key: String,
    
    /// RPC caller for making calls to other services
    pub rpc: Arc<RpcCaller>,
    
    /// Event dispatcher for emitting events
    pub events: Arc<EventDispatcher>,
}
```

## Key Features

### 1. Access AMQP Metadata

```rust
#[girolle]
async fn handler_with_metadata(ctx: Arc<RpcContext>, data: String) -> String {
    let correlation_id = &ctx.correlation_id;
    let reply_to = &ctx.reply_to;
    let routing_key = &ctx.routing_key;
    
    format!("Received {} via {}", data, routing_key)
}
```

### 2. Get Call ID Stack (for distributed tracing)

```rust
#[girolle]
async fn traced_handler(ctx: Arc<RpcContext>, data: String) -> String {
    let call_stack = ctx.get_call_id_stack();
    match call_stack {
        Some(stack) => {
            println!("Call stack depth: {}", stack.len());
            for call_id in stack {
                println!("  - {}", call_id);
            }
        }
        None => println!("No call stack"),
    }
    
    data
}
```

### 3. RPC Proxy Pattern (Foundation)

The RpcCaller provides the foundation for calling other services:

```rust
#[girolle]
async fn proxy_handler(ctx: Arc<RpcContext>, target: String) -> String {
    // Foundation is ready for:
    // ctx.rpc.register_service("other_service").await?;
    // let result = ctx.rpc.call("other_service", "method", 
    //                            Payload::new().arg(target)).await?;
    
    format!("Proxy capability available for: {}", target)
}
```

### 4. Event Dispatching (Foundation)

The EventDispatcher provides the foundation for emitting events:

```rust
#[girolle]
async fn event_handler(ctx: Arc<RpcContext>, data: String) -> String {
    // Foundation is ready for:
    // ctx.events.dispatch("user.created", json!({"data": data})).await?;
    
    format!("Event capability available for: {}", data)
}
```

## Registering Handlers

Both sync and async handlers can be registered in the same service:

```rust
use girolle::prelude::*;
use std::sync::Arc;

#[girolle]
fn sync_handler(name: String) -> String {
    format!("Sync: {}", name)
}

#[girolle]
async fn async_handler(ctx: Arc<RpcContext>, name: String) -> String {
    format!("Async: {} (correlation: {})", name, ctx.correlation_id)
}

fn main() {
    let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
    let services = RpcService::new(conf, "my_service");
    
    // Register both types of handlers
    services
        .register(sync_handler)
        .register(async_handler)
        .start();
}
```

## Macro Behavior

The `#[girolle]` macro automatically detects the handler type:

- **Sync handler**: `fn(args...) -> Result`
  - Generates: `RpcTask::new(...)`
  
- **Async handler with context**: `async fn(ctx: Arc<RpcContext>, args...) -> Result`
  - Generates: `RpcTask::new_async(...)`

The macro checks:
1. Is the function `async`?
2. Is the first parameter `Arc<RpcContext>` (or contains `RpcContext`)?
3. Generates appropriate handler registration

## Examples

See the examples directory:
- `async_service_with_context.rs` - Basic async handler with metadata access
- `rpc_proxy_demo.rs` - Demonstrates RPC proxy and event capabilities
- `simple_service.rs` - Traditional sync handlers (backward compatibility)

## Migration Guide

Existing sync handlers continue to work without changes:

```rust
// This still works exactly as before
#[girolle]
fn old_handler(name: String) -> String {
    format!("Hello, {}", name)
}
```

To add async capabilities, add `async` and `Arc<RpcContext>` parameter:

```rust
// New async handler with context
#[girolle]
async fn new_handler(ctx: Arc<RpcContext>, name: String) -> String {
    format!("Hello, {} (correlation: {})", name, ctx.correlation_id)
}
```

## Type Definitions

```rust
// Legacy sync handler
pub type NamekoFunction = fn(&[Value]) -> GirolleResult<Value>;

// New async handler with context
pub type AsyncNamekoFunction = Arc<
    dyn Fn(Arc<RpcContext>, Vec<Value>) 
        -> Pin<Box<dyn Future<Output = GirolleResult<Value>> + Send>>
        + Send + Sync,
>;

// Handler enum supporting both
pub enum RpcTaskHandler {
    Sync(NamekoFunction),
    Async(AsyncNamekoFunction),
}
```

## Testing

All functionality is tested:
- Unit tests for RpcContext creation and call_id_stack parsing
- Unit tests for RpcTask with both sync and async handlers
- Integration examples demonstrating both patterns

Run tests:
```bash
cargo test --lib
```

Build examples:
```bash
cargo build --examples
```

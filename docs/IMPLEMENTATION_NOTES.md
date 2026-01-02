# Service Execution Model Refactoring - Implementation Notes

## Overview

This document provides implementation notes for the service execution model refactoring that introduced async handlers with RpcContext support.

## Architecture Changes

### 1. Handler Types

**Before:**
```rust
type NamekoFunction = fn(&[Value]) -> GirolleResult<Value>;
```

**After:**
```rust
// Legacy sync handler (still supported)
type NamekoFunction = fn(&[Value]) -> GirolleResult<Value>;

// New async handler with context
type AsyncNamekoFunction = Arc<
    dyn Fn(Arc<RpcContext>, Vec<Value>) 
        -> Pin<Box<dyn Future<Output = GirolleResult<Value>> + Send>>
        + Send + Sync,
>;
```

### 2. RpcTask Handler Enum

The `RpcTask` now uses an enum to support both handler types:

```rust
pub enum RpcTaskHandler {
    Sync(NamekoFunction),
    Async(AsyncNamekoFunction),
}
```

### 3. RpcContext Structure

```rust
pub struct RpcContext {
    pub correlation_id: String,
    pub reply_to: String,
    pub headers: FieldTable,
    pub routing_key: String,
    pub rpc: Arc<RpcCaller>,      // For calling other services
    pub events: Arc<EventDispatcher>, // For emitting events
}
```

## Macro Implementation

The `#[girolle]` macro performs intelligent handler detection:

1. **Checks if function is async** using `item_fn.sig.asyncness`
2. **Checks for RpcContext parameter** using proper AST analysis (not string matching)
3. **Generates appropriate wrapper** based on handler type

### Type Detection Logic

```rust
fn matches_rpc_context_type(ty: &syn::Type) -> bool {
    // Checks for Arc<RpcContext> pattern
    // Handles fully qualified paths
    // Uses proper AST analysis via syn types
}
```

## Service Execution Flow

### Sync Handler Flow
1. AMQP message received
2. `compute_deliver` called with `None` for RpcContext
3. Handler matched as `RpcTaskHandler::Sync`
4. Function called directly: `fn_service(&args)`
5. Result published back

### Async Handler Flow
1. AMQP message received
2. `RpcContext` built from delivery metadata
3. `compute_deliver` called with `Some(rpc_context)`
4. Handler matched as `RpcTaskHandler::Async`
5. Function awaited: `async_fn(ctx, args).await`
6. Result published back

## Integration Points

### In `rpc_service.rs`

- `SharedData` now includes `config` and `service_id`
- RpcContext built per delivery with AMQP metadata
- Passed to `compute_deliver` for async handlers

### In `nameko_utils.rs`

- `compute_deliver` signature updated to accept optional `RpcContext`
- Pattern matching on `RpcTaskHandler` enum
- Async handler execution properly awaited

## Testing Strategy

### Unit Tests

1. **RpcContext Tests** (`rpc_context.rs`)
   - Context creation
   - Call ID stack parsing
   - Empty call ID stack handling

2. **RpcTask Tests** (`rpc_task.rs`)
   - Sync handler creation
   - Async handler creation
   - Handler type verification

### Integration Tests

Examples demonstrate real usage:
- `async_service_with_context.rs` - Basic async usage
- `rpc_proxy_demo.rs` - RPC proxy and event patterns

## Future Work

### RpcCaller Full Implementation

The `RpcCaller` currently provides the foundation. To fully implement:

1. Initialize connection pool in `initialize()`
2. Set up reply listener in `start_listening()`
3. Complete `call()` method with proper reply handling
4. Add service auto-registration

### EventDispatcher Implementation

The `EventDispatcher` needs:

1. Event exchange configuration
2. Event serialization
3. Event publishing logic
4. Event routing patterns

### Call ID Stack Propagation

Already implemented! The context provides:
- `get_call_id_stack()` for accessing the stack
- Stack is automatically propagated through AMQP headers
- Useful for distributed tracing

## Security Considerations

### Async Safety

- **MutexGuard** properly released before await points
- Used block scoping `{ let guard = ...; }` pattern
- Clippy verified with `#[warn(clippy::await_holding_lock)]`

### Type Safety

- Strong typing throughout
- No unsafe code blocks
- Proper error handling with `Result` types

### Input Validation

- AMQP headers parsed safely
- JSON deserialization with proper error handling
- Service/method names validated before routing

## Performance Implications

### Memory

- `Arc<RpcContext>` used for efficient cloning
- Context created once per request
- RpcCaller and EventDispatcher shared via Arc

### Async Runtime

- Built on Tokio runtime
- No blocking operations in async handlers
- Proper use of `.await` points

### Backward Compatibility

- Zero overhead for sync handlers (direct function call)
- Async handlers only incur cost when used
- No breaking changes to existing API

## Migration Guide for Contributors

### Adding New Context Features

1. Add field to `RpcContext` struct
2. Update `RpcContext::new()` constructor
3. Update context building in `rpc_service.rs`
4. Add tests in `rpc_context::tests`
5. Document in `ASYNC_HANDLERS.md`

### Modifying Handler Types

1. Update type definitions in `types.rs`
2. Modify `RpcTaskHandler` enum if needed
3. Update `compute_deliver` dispatch logic
4. Update macro code generation
5. Add regression tests

### Extending Capabilities

To add new capabilities (like RpcCaller or EventDispatcher):

1. Create capability struct in `rpc_context.rs`
2. Add as field to `RpcContext`
3. Initialize in `RpcContext::new()`
4. Implement capability methods
5. Add examples and tests
6. Document API

## Debugging Tips

### Handler Not Being Recognized as Async

- Check function signature has `async` keyword
- Verify first parameter is `Arc<RpcContext>`
- Check macro expansion with `cargo expand`

### Context Not Available in Handler

- Ensure function has `async fn` signature
- Verify first parameter is `ctx: Arc<RpcContext>`
- Check that service is using updated girolle version

### RPC Call Not Working

- Verify target service is registered
- Check connection configuration
- Ensure reply queue is set up
- Review correlation ID in logs

## References

- Main implementation: `girolle/src/rpc_context.rs`
- Handler dispatch: `girolle/src/nameko_utils.rs`
- Service integration: `girolle/src/rpc_service.rs`
- Macro logic: `girolle_macro/src/entry.rs`
- Type definitions: `girolle/src/types.rs`
- Documentation: `docs/ASYNC_HANDLERS.md`

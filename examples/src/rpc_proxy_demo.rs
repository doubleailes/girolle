use girolle::prelude::*;
use std::sync::Arc;

/// Example of an async handler that calls another service (RPC proxy pattern)
/// This demonstrates the core goal: service handlers can now call other services
#[girolle]
async fn proxy_hello(ctx: Arc<RpcContext>, name: String) -> String {
    // Note: In a real scenario, you would register the target service first
    // For demonstration, this shows the API for calling other services
    
    // Access metadata
    let call_stack = ctx.get_call_id_stack()
        .map(|stack| format!("Call stack depth: {}", stack.len()))
        .unwrap_or_else(|| "No call stack".to_string());
    
    // In a full implementation, you could do:
    // let result = ctx.rpc.call("other_service", "method", Payload::new().arg(name)).await?;
    
    format!(
        "Proxy received request for {}. {}. (RPC proxy capability available via ctx.rpc.call())",
        name, call_stack
    )
}

/// Example showing event dispatching capability
#[girolle]
async fn event_emitter(_ctx: Arc<RpcContext>, event_data: String) -> String {
    // In a full implementation, you could do:
    // ctx.events.dispatch("user.created", json!({"data": event_data})).await?;
    
    format!(
        "Event capability available via ctx.events.dispatch(). Data: {}",
        event_data
    )
}

/// Traditional sync handler still works
#[girolle]
fn traditional_sync(message: String) -> String {
    format!("Sync handler: {}", message)
}

fn main() {
    // Get the configuration from the staging/config.yml file
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    
    // Create the rpc service struct
    let services: RpcService = RpcService::new(conf, "proxy_demo");
    
    // Register handlers - both async with context and traditional sync
    let _ = services
        .register(proxy_hello)
        .register(event_emitter)
        .register(traditional_sync)
        .start();
}

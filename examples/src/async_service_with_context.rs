use girolle::prelude::*;
use std::sync::Arc;

/// Example of an async handler that receives RpcContext
/// This handler can access AMQP metadata and call other services
#[girolle]
async fn hello_async(ctx: Arc<RpcContext>, name: String) -> String {
    // Access AMQP metadata from context
    let correlation_id = &ctx.correlation_id;
    let routing_key = &ctx.routing_key;
    
    // Get call_id_stack from headers
    let call_stack = ctx.get_call_id_stack()
        .map(|stack| format!(" (call stack depth: {})", stack.len()))
        .unwrap_or_default();
    
    format!(
        "Hello, {}! Served by async handler. Correlation: {}, Route: {}{}",
        name, correlation_id, routing_key, call_stack
    )
}

/// Example of a traditional sync handler (still supported)
#[girolle]
fn hello_sync(name: String) -> String {
    format!("Hello, {}! Served by sync handler", name)
}

fn main() {
    // Get the configuration from the staging/config.yml file
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    
    // Create the rpc service struct
    let services: RpcService = RpcService::new(conf, "async_demo");
    
    // Register both async and sync handlers
    let _ = services
        .register(hello_async)
        .register(hello_sync)
        .start();
}

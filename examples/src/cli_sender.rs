//! Generic CLI RPC sender. Useful for poking at any service from a
//! terminal without having to hand-edit a sender example.
//!
//! Usage:
//!
//! ```sh
//! cargo run --example cli_sender -- <service> <method> [arg ...]
//! ```
//!
//! Each `arg` is sent as a positional argument. Args are tried as JSON
//! first (so you can pass numbers, booleans, arrays, objects); anything
//! that doesn't parse falls back to a plain string.
//!
//! Examples:
//!
//! ```sh
//! cargo run --example cli_sender -- video hello Girolle
//! cargo run --example cli_sender -- video sub 10 5
//! cargo run --example cli_sender -- proxy hello Girolle
//! cargo run --example cli_sender -- users create_user Girolle
//! ```

use girolle::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <service> <method> [arg ...]",
            args.first().map(String::as_str).unwrap_or("cli_sender")
        );
        std::process::exit(2);
    }
    let service_name = args[1].clone();
    let method = args[2].clone();
    let mut payload = Payload::new();
    for raw in &args[3..] {
        match serde_json::from_str::<Value>(raw) {
            Ok(v) => payload = payload.arg(v),
            Err(_) => payload = payload.arg(raw),
        }
    }

    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let mut rpc_client = RpcClient::new(conf);
    rpc_client.register_service(&service_name).await?;
    rpc_client.start().await?;
    let result = rpc_client.send(&service_name, &method, payload)?;
    println!("{}", result.get_value());
    rpc_client.unregister_service(&service_name)?;
    rpc_client.close().await?;
    Ok(())
}

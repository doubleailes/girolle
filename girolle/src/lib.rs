//! ## Description
//!
//! This crate is a Rust implementation of the Nameko RPC protocol.
//! It allows to create a RPC service or Rpc Call in Rust that can be called
//! from or to a Nameko microservice.
//!
//! **Girolle** mock Nameko architecture to send request and get response.
//!
//! ## Examples
//!
//! ### RPC Service
//!
//! This example is simple service that return a string with the name of the person.
//!
//! ```rust,no_run
//!
//! use girolle::prelude::*;
//!
//! fn hello(s: &[Value]) -> NamekoResult<Value> {
//!    // Parse the incomming data
//!   let n: String = serde_json::from_value(s[0].clone())?;
//!  let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
//!  Ok(hello_str)
//! }
//!
//! fn main() {
//!   let conf: Config = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
//!   let mut services: RpcService = RpcService::new(conf,"video");
//!   services.insert("hello", hello);
//!   services.start();
//! }
//! ```
//!
//! ### RPC Client
//!
//! This example is a simple client that call the hello function in the video service.
//!
//! ```rust
//! use girolle::prelude::*;
//!
//! #[tokio::main]
//! async fn main() {
//!    let rpc_call = RpcClient::new(Config::default_config());
//! }
//! ```
mod queue;
pub mod types;
pub use serde_json as JsonValue;
mod config;
mod nameko_utils;
pub mod prelude;
pub mod rpc_client;
pub use rpc_client::RpcClient;
mod rpc_service;
pub use rpc_service::RpcService;
mod rpc_task;
pub use rpc_task::RpcTask;
mod payload;

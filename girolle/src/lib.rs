//! ## Description
//!
//! This crate is a Rust implementation of the Nameko RPC protocol.
//! It allows to create a RPC service or Rpc client in Rust that can be called
//! from or to a Nameko microservice.
//!
//! **Girolle** mock Nameko architecture to send request and get response,
//! with a message-listener queue system
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
//! use std::vec;
//!
//! #[girolle]
//! fn hello(s: String) -> String {
//!     format!("Hello, {}!, by Girolle", s)
//! }
//!
//! fn main() {
//!   let conf: Config = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
//!   let mut services: RpcService = RpcService::new(conf, "video");
//!   services.register(hello).start();
//! }
//! ```
//!
//! ### RPC Client
//!
//! This example is a simple client that call the hello function in the video service.
//!
//! ```rust,no_run
//! use girolle::prelude::*;
//! use std::vec;
//!
//! #[tokio::main]
//! async fn main() {
//!    let mut rpc_client = RpcClient::new(Config::default_config());
//!    rpc_client.register_service("video");
//!    let result = rpc_client.send("video", "hello", vec!["Girolle"]).unwrap();
//! }
//! ```
mod config;
mod queue;
pub mod types;
pub use config::Config;
mod nameko_utils;
pub mod prelude;
mod rpc_client;
pub use rpc_client::RpcClient;
mod rpc_service;
pub use rpc_service::RpcService;
mod rpc_task;
pub use rpc_task::RpcTask;
mod payload;
pub use serde_json;
pub use serde_json::{json, Value};

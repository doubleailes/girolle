//! ## Description
//!
//! This crate is a Rust implementation of the Nameko RPC protocol.
//! It lets you build RPC services and clients in Rust that interoperate
//! with Python [Nameko](https://github.com/nameko/nameko) services on the
//! same broker.
//!
//! Service handlers run as async futures. Each handler receives an
//! [`RpcContext`] giving access to inbound headers and two capability
//! handles:
//!
//! * [`RpcCaller::call`] — call any other service from inside a handler.
//! * [`EventDispatcher::dispatch`] — emit a Nameko-compatible event on
//!   `<source>.events`.
//!
//! A service can also subscribe to events emitted by other services with
//! [`RpcService::subscribe`].
//!
//! ## Examples
//!
//! ### RPC service with the macro
//!
//! ```rust,no_run
//! use girolle::prelude::*;
//!
//! #[girolle]
//! fn hello(s: String) -> String {
//!     format!("Hello, {}!, by Girolle", s)
//! }
//!
//! fn main() {
//!     let conf: Config = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
//!     let _ = RpcService::new(conf, "video").register(hello).start();
//! }
//! ```
//!
//! ### Calling another service from a handler
//!
//! ```rust,no_run
//! use girolle::prelude::*;
//!
//! #[girolle]
//! async fn proxy_hello(ctx: RpcContext, name: String) -> String {
//!     let result = ctx
//!         .rpc
//!         .call("video", "hello", Payload::new().arg(name))
//!         .await
//!         .expect("rpc call failed");
//!     serde_json::from_value(result).expect("video.hello did not return a String")
//! }
//! ```
//!
//! ### Emitting an event from a handler
//!
//! ```rust,no_run
//! use girolle::prelude::*;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct UserCreated { name: String }
//!
//! #[girolle]
//! async fn create_user(ctx: RpcContext, name: String) -> String {
//!     ctx.events
//!         .dispatch("users", "user_created", &UserCreated { name: name.clone() })
//!         .await
//!         .expect("event dispatch failed");
//!     format!("User {} created", name)
//! }
//! ```
//!
//! ### Subscribing to events
//!
//! ```rust,no_run
//! use girolle::prelude::*;
//! use std::sync::Arc;
//!
//! fn main() {
//!     let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
//!     let _ = RpcService::new(conf, "event-observer")
//!         .subscribe(
//!             "users",
//!             "user_created",
//!             Arc::new(|_ctx: RpcContext, payload: Value| -> BoxFuture<GirolleResult<()>> {
//!                 Box::pin(async move {
//!                     println!("[users.user_created] {}", payload);
//!                     Ok(())
//!                 })
//!             }),
//!         )
//!         .start();
//! }
//! ```
//!
//! ### RPC client
//!
//! ```rust,no_run
//! use girolle::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut rpc_client = RpcClient::new(Config::default());
//!     rpc_client.register_service("video").await?;
//!     rpc_client.start().await?;
//!     let result = rpc_client.send("video", "hello", Payload::new().arg("Girolle"))?;
//!     println!("{}", result.get_value());
//!     Ok(())
//! }
//! ```
mod config;
mod queue;
pub mod types;
pub use config::Config;
// Public for bench purpose
pub mod nameko_utils;
pub mod prelude;
mod events;
mod rpc_client;
pub use rpc_client::RpcClient;
mod rpc_core;
mod rpc_service;
pub use rpc_service::RpcService;
mod rpc_task;
pub use rpc_task::RpcTask;
mod payload;
pub use payload::Payload;
pub use serde_json;
pub use serde_json::{json, Value};
mod error;
pub use error::GirolleError;
pub use types::{
    BoxFuture, EventDispatcher, EventHandler, GirolleResult, RpcCaller, RpcContext, RpcHandler,
};

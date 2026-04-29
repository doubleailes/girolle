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
//! ## Module map
//!
//! - [`config`] — [`Config`] and YAML loading
//! - [`error`] — [`GirolleError`], [`GirolleResult`]
//! - [`payload`] — [`Payload`] (positional + keyword args)
//! - [`service`] — `RpcService`, handlers, capability handles
//! - [`client`] — standalone [`RpcClient`]
//! - `amqp` (private) — lapin transport plumbing
//! - `protocol` (private) — wire envelope and Nameko header keys
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

pub mod config;
pub mod error;
pub mod payload;
pub mod prelude;

pub mod client;
pub mod service;

mod amqp;
mod protocol;

#[doc(hidden)]
pub mod __macro_support;

pub use config::Config;
pub use error::{GirolleError, GirolleResult};
pub use payload::Payload;
pub use serde_json;
pub use serde_json::{json, Value};

pub use client::{RpcClient, RpcReply, RpcResult};
pub use service::caller::RpcCaller;
pub use service::context::RpcContext;
pub use service::dispatcher::EventDispatcher;
pub use service::task::{BoxFuture, EventHandler, RpcHandler, RpcTask};
pub use service::RpcService;

use crate::error::GirolleError;
use crate::payload::Payload;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::context::RpcContext;

/// Boxed, dynamically-dispatched future returned by an [`RpcHandler`]
/// or [`EventHandler`]. Generic over the future's output.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Async handler stored on an [`RpcTask`].
///
/// Receives the per-delivery [`RpcContext`] and the decoded
/// [`Payload`], returns a future resolving to the handler's JSON
/// result.
pub type RpcHandler =
    Arc<dyn Fn(RpcContext, Payload) -> BoxFuture<Result<Value, GirolleError>> + Send + Sync>;

/// Async handler for an event subscription.
///
/// Receives the per-event [`RpcContext`] and the decoded JSON payload.
/// Errors are logged but do not nack the message — events are
/// best-effort.
pub type EventHandler =
    Arc<dyn Fn(RpcContext, Value) -> BoxFuture<Result<(), GirolleError>> + Send + Sync>;

/// A single registerable RPC method: name + async handler.
///
/// Most users construct an `RpcTask` indirectly via the
/// [`#[girolle]`](girolle_macro::girolle) attribute macro, which
/// generates a constructor function returning `RpcTask`. That
/// constructor is then passed to
/// [`RpcService::register`](crate::RpcService::register).
///
/// Hand-written handlers can use [`RpcTask::new`] directly.
///
/// ## Example
///
/// ```rust,no_run
/// use girolle::prelude::*;
/// use std::sync::Arc;
///
/// fn hello() -> RpcTask {
///     RpcTask::new(
///         "hello",
///         Arc::new(|_ctx, payload| {
///             Box::pin(async move {
///                 let n: String = serde_json::from_value(payload.args()[0].clone())?;
///                 Ok(serde_json::to_value(format!("Hello, {}!", n))?)
///             })
///         }),
///     )
/// }
///
/// fn main() {
///     let _ = RpcService::new(Config::default(), "video").register(hello);
/// }
/// ```
#[derive(Clone)]
pub struct RpcTask {
    pub name: &'static str,
    pub handler: RpcHandler,
}

impl RpcTask {
    pub fn new(name: &'static str, handler: RpcHandler) -> Self {
        Self { name, handler }
    }
}

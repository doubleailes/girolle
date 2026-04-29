use crate::types::RpcHandler;

/// # RpcTask
///
/// ## Description
///
/// A single registerable RPC method. Wraps an async [`RpcHandler`] together
/// with the method name it should be exposed under.
///
/// Most users construct an `RpcTask` indirectly via the [`#[girolle]`](girolle_macro::girolle)
/// attribute macro. The macro generates a constructor function returning
/// `RpcTask`, which is then passed to [`RpcService::register`](crate::RpcService::register).
///
/// Handlers built by hand can use [`RpcTask::new`] directly.
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
///     let _services: RpcService = RpcService::new(Config::default(), "video")
///         .register(hello);
/// }
/// ```
#[derive(Clone)]
pub struct RpcTask {
    pub name: &'static str,
    pub handler: RpcHandler,
}

impl RpcTask {
    /// # new
    ///
    /// Create a new `RpcTask` from a method name and an [`RpcHandler`].
    ///
    /// ## Arguments
    ///
    /// * `name` — the RPC method name (the routing key suffix)
    /// * `handler` — the async handler invoked for each inbound delivery
    pub fn new(name: &'static str, handler: RpcHandler) -> Self {
        Self { name, handler }
    }
}

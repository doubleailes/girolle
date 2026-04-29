use crate::error::GirolleError;
use crate::payload::Payload;
use lapin::types::FieldTable;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// # GirolleResult
///
/// Standard result alias used throughout the crate.
pub type GirolleResult<T> = std::result::Result<T, GirolleError>;

/// # BoxFuture
///
/// Boxed, dynamically-dispatched future returned by an [`RpcHandler`].
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// # RpcCaller
///
/// Capability handle exposed on [`RpcContext`] that lets a service handler
/// call other services as an RPC client.
///
/// In this release the caller is a placeholder; the in-service async RPC
/// core (shared reply queue, correlation map) lands in a follow-up change.
#[derive(Clone, Debug, Default)]
pub struct RpcCaller {
    _private: (),
}

impl RpcCaller {
    pub(crate) fn placeholder() -> Self {
        Self { _private: () }
    }
}

/// # EventDispatcher
///
/// Capability handle exposed on [`RpcContext`] that lets a service handler
/// emit Nameko-compatible events.
///
/// In this release the dispatcher is a placeholder; the publisher
/// implementation lands in a follow-up change.
#[derive(Clone, Debug, Default)]
pub struct EventDispatcher {
    _private: (),
}

impl EventDispatcher {
    pub(crate) fn placeholder() -> Self {
        Self { _private: () }
    }
}

/// # RpcContext
///
/// Per-delivery context handed to every [`RpcHandler`]. Holds inbound AMQP
/// metadata and the capability handles a handler can use to call other
/// services or emit events.
#[derive(Clone, Debug)]
pub struct RpcContext {
    pub service_name: String,
    pub method_name: String,
    pub correlation_id: String,
    pub reply_to: String,
    pub headers: FieldTable,
    pub rpc: RpcCaller,
    pub events: EventDispatcher,
}

/// # RpcHandler
///
/// Async handler signature stored on an [`RpcTask`](crate::RpcTask).
///
/// The handler receives the per-delivery [`RpcContext`] and the decoded
/// [`Payload`] and returns a future resolving to the handler's JSON result.
pub type RpcHandler =
    Arc<dyn Fn(RpcContext, Payload) -> BoxFuture<GirolleResult<Value>> + Send + Sync>;

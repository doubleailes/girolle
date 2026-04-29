use crate::error::GirolleError;
use crate::payload::Payload;
use crate::rpc_core::RpcCallerCore;
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
/// Each inbound delivery receives an `RpcCaller` derived from the service's
/// shared in-service RPC core. The derivation captures the parent
/// delivery's AMQP headers so that outbound calls propagate
/// `nameko.call_id_stack` correctly.
///
/// A default-constructed (or `placeholder`) `RpcCaller` has no underlying
/// core; calling [`RpcCaller::call`] on it returns an error. This shape is
/// useful for unit-testing handlers without standing up a broker.
#[derive(Clone, Default)]
pub struct RpcCaller {
    inner: Option<Arc<RpcCallerCore>>,
    parent_headers: FieldTable,
}

impl std::fmt::Debug for RpcCaller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcCaller")
            .field("active", &self.inner.is_some())
            .finish()
    }
}

impl RpcCaller {
    pub(crate) fn from_core(core: Arc<RpcCallerCore>) -> Self {
        Self {
            inner: Some(core),
            parent_headers: FieldTable::default(),
        }
    }

    pub(crate) fn with_parent_headers(&self, headers: FieldTable) -> Self {
        Self {
            inner: self.inner.clone(),
            parent_headers: headers,
        }
    }

    /// Invoke `<service>.<method>` and await the reply.
    ///
    /// Returns the decoded JSON result on success, or a [`GirolleError`]
    /// reconstructed from the remote service's error on failure.
    pub async fn call(
        &self,
        service: &str,
        method: &str,
        payload: Payload,
    ) -> GirolleResult<Value> {
        let core = self.inner.as_ref().ok_or_else(|| {
            GirolleError::ServiceMissingError(
                "RpcCaller has no in-service core; ctx.rpc.call requires a running RpcService"
                    .to_string(),
            )
        })?;
        core.call(&self.parent_headers, service, method, payload)
            .await
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

//! In-service RPC caller.
//!
//! Two collaborating types:
//!
//! * `RpcCallerCore` (private) — per-service shared state needed to
//!   act as an RPC client from within a handler: a reply queue +
//!   consumer, a correlation map (`correlation_id -> oneshot::Sender`),
//!   and a publish channel. Built once at startup, shared via `Arc`.
//!
//! * [`RpcCaller`] — the public per-delivery handle. Each inbound
//!   delivery clones the shared core and stamps it with the parent
//!   delivery's headers so outbound calls propagate
//!   `nameko.call_id_stack` correctly. A default-constructed handle has
//!   no core; calling `call` on it returns an error (useful for unit
//!   tests).

use crate::amqp::channel::create_message_channel;
use crate::config::Config;
use crate::error::{GirolleError, GirolleResult};
use crate::payload::Payload;
use crate::protocol::{PayloadResult, NAMEKO_AMQP_URI, NAMEKO_CALL_ID_STACK};
use dashmap::DashMap;
use lapin::message::DeliveryResult;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions};
use lapin::types::{AMQPValue, FieldArray, FieldTable, ShortString};
use lapin::{BasicProperties, Connection};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::error;
use uuid::Uuid;

pub(crate) struct RpcCallerCore {
    publish_channel: lapin::Channel,
    reply_routing_key: String,
    rpc_exchange: String,
    pending: Arc<DashMap<String, oneshot::Sender<PayloadResult>>>,
    identifier: String,
    amqp_uri: String,
}

impl RpcCallerCore {
    /// Set up the in-service reply queue + consumer and return a
    /// shared core. Call once per `RpcService` instance.
    pub(crate) async fn new(
        conn: &Connection,
        conf: &Config,
        identifier: Uuid,
    ) -> GirolleResult<Arc<Self>> {
        let reply_queue_name = format!("rpc.listener-{}", identifier);
        let reply_channel = create_message_channel(
            conn,
            &reply_queue_name,
            conf.prefetch_count(),
            &identifier,
            conf.rpc_exchange(),
        )
        .await?;
        let publish_channel = conn.create_channel().await?;

        let pending: Arc<DashMap<String, oneshot::Sender<PayloadResult>>> =
            Arc::new(DashMap::new());

        let consumer = reply_channel
            .basic_consume(
                &reply_queue_name,
                "girolle_in_service_replies",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let pending_consumer = Arc::clone(&pending);
        let reply_channel_for_ack = reply_channel.clone();
        consumer.set_delegate(move |delivery: DeliveryResult| {
            let pending = Arc::clone(&pending_consumer);
            let reply_channel = reply_channel_for_ack.clone();
            async move {
                if let Ok(Some(delivery)) = delivery {
                    let correlation_id = delivery
                        .properties
                        .correlation_id()
                        .clone()
                        .map(|s| s.to_string());
                    let parsed: Result<PayloadResult, _> =
                        serde_json::from_slice(&delivery.data);
                    match parsed {
                        Ok(payload) => match correlation_id {
                            Some(cid) => {
                                if let Some((_, tx)) = pending.remove(&cid) {
                                    let _ = tx.send(payload);
                                }
                            }
                            None => error!(
                                "in-service reply: missing correlation id, dropping"
                            ),
                        },
                        Err(e) => {
                            error!(error = %e, "in-service reply: deserialization failed");
                        }
                    }
                    let _ = reply_channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .await;
                }
            }
        });

        Ok(Arc::new(Self {
            publish_channel,
            reply_routing_key: identifier.to_string(),
            rpc_exchange: conf.rpc_exchange().to_string(),
            pending,
            identifier: identifier.to_string(),
            amqp_uri: conf.AMQP_URI().to_string(),
        }))
    }

    /// Issue an outbound RPC call from inside a service handler and
    /// await its reply. Propagates `nameko.call_id_stack` from
    /// `parent_headers`, seeding it with this service's identifier if
    /// absent. Always stamps `nameko.AMQP_URI`.
    pub(crate) async fn call(
        &self,
        parent_headers: &FieldTable,
        service: &str,
        method: &str,
        payload: Payload,
    ) -> GirolleResult<Value> {
        let correlation_id = Uuid::new_v4().to_string();
        let routing_key = format!("{}.{}", service, method);

        let mut outbound_headers = parent_headers.clone();
        outbound_headers.insert(
            ShortString::from(NAMEKO_AMQP_URI),
            AMQPValue::LongString(self.amqp_uri.clone().into()),
        );
        let stack_key = ShortString::from(NAMEKO_CALL_ID_STACK);
        if outbound_headers.inner().get(&stack_key).is_none() {
            outbound_headers.insert(
                stack_key,
                AMQPValue::FieldArray(FieldArray::from(vec![AMQPValue::LongString(
                    self.identifier.clone().into(),
                )])),
            );
        }

        let properties = BasicProperties::default()
            .with_reply_to(self.reply_routing_key.clone().into())
            .with_correlation_id(correlation_id.clone().into())
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_headers(outbound_headers)
            .with_priority(0);

        let (tx, rx) = oneshot::channel::<PayloadResult>();
        self.pending.insert(correlation_id.clone(), tx);

        let publish_result = self
            .publish_channel
            .basic_publish(
                &self.rpc_exchange,
                &routing_key,
                BasicPublishOptions::default(),
                payload.to_string().as_bytes(),
                properties,
            )
            .await;

        if let Err(e) = publish_result {
            self.pending.remove(&correlation_id);
            return Err(GirolleError::LapinError(e));
        }

        let result = rx.await.map_err(|_| {
            GirolleError::ServiceMissingError(
                "in-service RPC reply channel dropped before a reply arrived".to_string(),
            )
        })?;

        match result.get_error() {
            Some(remote) => Err(remote.convert_to_girolle_error()),
            None => Ok(result.get_result()),
        }
    }
}

/// Capability handle exposed on [`super::context::RpcContext`] that
/// lets a service handler call other services as an RPC client.
///
/// Each inbound delivery receives an `RpcCaller` derived from the
/// service's shared core. The derivation captures the parent
/// delivery's AMQP headers so that outbound calls propagate
/// `nameko.call_id_stack` correctly.
///
/// A default-constructed (or `placeholder`) `RpcCaller` has no
/// underlying core; calling [`RpcCaller::call`] on it returns an error.
/// This shape is useful for unit-testing handlers without standing up
/// a broker.
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

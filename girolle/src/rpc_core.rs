//! In-service async RPC core.
//!
//! Holds the per-service shared state needed to act as an RPC client from
//! within a service handler: a reply queue + consumer, a correlation map
//! mapping `correlation_id -> oneshot::Sender<PayloadResult>`, and a
//! publish channel.
//!
//! [`RpcCallerCore`] is constructed once in [`crate::rpc_service`] startup
//! and shared (via `Arc`) by every inbound delivery's `RpcCaller`.

use crate::config::Config;
use crate::error::GirolleError;
use crate::payload::{Payload, PayloadResult};
use crate::queue::create_message_channel;
use crate::types::GirolleResult;
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

const NAMEKO_AMQP_URI: &str = "nameko.AMQP_URI";
const NAMEKO_CALL_ID_STACK: &str = "nameko.call_id_stack";

pub(crate) struct RpcCallerCore {
    publish_channel: lapin::Channel,
    reply_routing_key: String,
    rpc_exchange: String,
    pending: Arc<DashMap<String, oneshot::Sender<PayloadResult>>>,
    identifier: String,
    amqp_uri: String,
}

impl RpcCallerCore {
    /// Set up the in-service reply queue + consumer and return a shared core.
    ///
    /// Call once per `RpcService` instance. The returned `Arc` is cloned into
    /// the `RpcCaller` handed to every inbound delivery.
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

    /// Issue an outbound RPC call from inside a service handler and await
    /// its reply.
    ///
    /// `parent_headers` carries the inbound delivery's headers; we propagate
    /// `nameko.call_id_stack` through if present and seed it with our own
    /// service identifier otherwise. `nameko.AMQP_URI` is always stamped
    /// with this service's URI.
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
                "in-service RPC reply channel dropped before a reply arrived"
                    .to_string(),
            )
        })?;

        match result.get_error() {
            Some(remote) => Err(remote.convert_to_girolle_error()),
            None => Ok(result.get_result()),
        }
    }
}

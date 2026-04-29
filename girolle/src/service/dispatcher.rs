//! In-service event dispatcher.
//!
//! Two collaborating types, mirroring [`super::caller`]:
//!
//! * `EventDispatcherCore` (private) — per-service shared state
//!   required to publish Nameko-compatible events: a publish channel
//!   and a cache of `{source}.events` topic exchanges already declared
//!   on the broker. Built once at startup, shared via `Arc`.
//!
//! * [`EventDispatcher`] — the public per-delivery handle. Each
//!   inbound delivery clones the shared core and stamps it with the
//!   parent delivery's headers so emitted events carry the inbound
//!   `nameko.call_id_stack` through.

use crate::config::Config;
use crate::error::{GirolleError, GirolleResult};
use crate::protocol::NAMEKO_AMQP_URI;
use dashmap::DashSet;
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::{AMQPValue, FieldTable, ShortString};
use lapin::{BasicProperties, Connection, ExchangeKind};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) struct EventDispatcherCore {
    publish_channel: lapin::Channel,
    declared_exchanges: DashSet<String>,
    amqp_uri: String,
    #[allow(dead_code)]
    identifier: String,
}

impl EventDispatcherCore {
    pub(crate) async fn new(
        conn: &Connection,
        conf: &Config,
        identifier: Uuid,
    ) -> GirolleResult<Arc<Self>> {
        let publish_channel = conn.create_channel().await?;
        Ok(Arc::new(Self {
            publish_channel,
            declared_exchanges: DashSet::new(),
            amqp_uri: conf.AMQP_URI().to_string(),
            identifier: identifier.to_string(),
        }))
    }

    /// Publish a single event.
    ///
    /// Lazily declares the `{source_service}.events` durable topic
    /// exchange the first time a given source emits, then publishes a
    /// persistent JSON-encoded message with `event_type` as the
    /// routing key.
    pub(crate) async fn dispatch(
        &self,
        parent_headers: &FieldTable,
        source_service: &str,
        event_type: &str,
        payload_bytes: Vec<u8>,
    ) -> GirolleResult<()> {
        let exchange = format!("{}.events", source_service);
        if !self.declared_exchanges.contains(&exchange) {
            self.publish_channel
                .exchange_declare(
                    &exchange,
                    ExchangeKind::Topic,
                    ExchangeDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await
                .map_err(GirolleError::LapinError)?;
            self.declared_exchanges.insert(exchange.clone());
        }

        let mut headers = parent_headers.clone();
        headers.insert(
            ShortString::from(NAMEKO_AMQP_URI),
            AMQPValue::LongString(self.amqp_uri.clone().into()),
        );

        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_delivery_mode(2)
            .with_headers(headers)
            .with_priority(0);

        self.publish_channel
            .basic_publish(
                &exchange,
                event_type,
                BasicPublishOptions::default(),
                &payload_bytes,
                properties,
            )
            .await
            .map_err(GirolleError::LapinError)?;

        Ok(())
    }
}

/// Capability handle exposed on [`super::context::RpcContext`] that
/// lets a service handler emit Nameko-compatible events.
///
/// Each inbound delivery receives an `EventDispatcher` derived from
/// the service's shared core. The derivation captures the parent
/// delivery's AMQP headers so that emitted events carry the inbound
/// `nameko.call_id_stack` through.
///
/// A default-constructed `EventDispatcher` has no underlying core;
/// calling [`EventDispatcher::dispatch`] on it returns an error.
/// Useful for unit-testing handlers without standing up a broker.
#[derive(Clone, Default)]
pub struct EventDispatcher {
    inner: Option<Arc<EventDispatcherCore>>,
    parent_headers: FieldTable,
}

impl std::fmt::Debug for EventDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventDispatcher")
            .field("active", &self.inner.is_some())
            .finish()
    }
}

impl EventDispatcher {
    pub(crate) fn from_core(core: Arc<EventDispatcherCore>) -> Self {
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

    /// Emit `event_type` on the `{source_service}.events` topic
    /// exchange. `payload` is serialized as JSON. The exchange is
    /// declared lazily on the first emit per source. Returns when the
    /// publish has been handed to the broker; delivery is
    /// fire-and-forget (no acknowledgement).
    pub async fn dispatch<T: Serialize>(
        &self,
        source_service: &str,
        event_type: &str,
        payload: &T,
    ) -> GirolleResult<()> {
        let core = self.inner.as_ref().ok_or_else(|| {
            GirolleError::ServiceMissingError(
                "EventDispatcher has no in-service core; ctx.events.dispatch requires a running RpcService"
                    .to_string(),
            )
        })?;
        let body = serde_json::to_vec(payload)?;
        core.dispatch(&self.parent_headers, source_service, event_type, body)
            .await
    }
}

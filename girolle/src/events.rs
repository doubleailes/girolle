//! In-service event dispatcher.
//!
//! [`EventDispatcherCore`] holds the per-service state required to publish
//! Nameko-compatible events: a publish channel and a cache of topic
//! exchanges already declared on the broker. Exchanges follow the
//! `{source_service}.events` convention and are declared lazily the first
//! time a given source emits.
//!
//! Constructed once in [`crate::rpc_service`] startup and shared (via
//! `Arc`) by every inbound delivery's [`crate::types::EventDispatcher`].

use crate::config::Config;
use crate::error::GirolleError;
use crate::types::GirolleResult;
use dashmap::DashSet;
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::{AMQPValue, FieldTable, ShortString};
use lapin::{BasicProperties, Connection, ExchangeKind};
use std::sync::Arc;
use uuid::Uuid;

const NAMEKO_AMQP_URI: &str = "nameko.AMQP_URI";

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
    /// Lazily declares the `{source_service}.events` durable topic exchange
    /// the first time a given source emits, then publishes a persistent
    /// JSON-encoded message with `event_type` as the routing key.
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

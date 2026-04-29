//! Channel/queue/exchange declarations.
//!
//! Each helper opens a fresh `lapin::Channel` on an existing
//! `Connection`, declares the queue and any exchanges/bindings the
//! caller will need, applies QoS, and returns the channel ready to use.

use lapin::{
    options::{BasicQosOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ExchangeKind,
};
use tracing::info;
use uuid::Uuid;

/// TTL applied to per-client reply queues (5 min). After this long
/// without a consumer, the broker reclaims the queue.
const QUEUE_TTL: u32 = 300_000;

/// Declare a service's inbound RPC queue (`rpc-<service>`), bind it to
/// the RPC exchange with `<service>.*`, and apply the requested QoS.
pub(crate) async fn create_service_channel(
    conn: &Connection,
    service_name: &str,
    prefetch_count: u16,
    rpc_exchange: &str,
) -> lapin::Result<lapin::Channel> {
    info!("Create service queue");
    let routing_key = format!("{}.*", service_name);
    info!("{:?}", conn.status());
    let incomming_channel = conn.create_channel().await?;
    let rpc_queue = format!("rpc-{}", service_name);
    let queue = incomming_channel
        .queue_declare(
            &rpc_queue,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    info!(?queue, "Declared queue");
    incomming_channel
        .basic_qos(prefetch_count, BasicQosOptions::default())
        .await?;
    incomming_channel
        .queue_bind(
            &rpc_queue,
            rpc_exchange,
            &routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(incomming_channel)
}

/// Declare a per-client reply queue bound to the RPC exchange under
/// the client's identifier as the routing key. Used by both the
/// standalone [`crate::client::RpcClient`] and the in-service caller.
pub(crate) async fn create_message_channel(
    conn: &Connection,
    rpc_queue_reply: &str,
    prefetch_count: u16,
    id: &Uuid,
    rpc_exchange: &str,
) -> lapin::Result<lapin::Channel> {
    info!("Create message queue");
    let response_channel = conn.create_channel().await?;
    let mut response_arguments = FieldTable::default();
    response_arguments.insert("x-expires".into(), QUEUE_TTL.into());
    response_channel
        .queue_declare(
            rpc_queue_reply,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            response_arguments.clone(),
        )
        .await?;
    response_channel
        .basic_qos(prefetch_count, BasicQosOptions::default())
        .await?;
    response_channel
        .queue_bind(
            rpc_queue_reply,
            rpc_exchange,
            &id.to_string(),
            QueueBindOptions::default(),
            response_arguments,
        )
        .await?;
    Ok(response_channel)
}

/// Declare an event consumer's `{source}.events` topic exchange and a
/// durable queue bound to it on `event_type`. The exchange is declared
/// idempotently so a consumer can come up before any publisher.
pub(crate) async fn create_event_consumer_channel(
    conn: &Connection,
    queue_name: &str,
    source_service: &str,
    event_type: &str,
    prefetch_count: u16,
) -> lapin::Result<lapin::Channel> {
    info!(queue = queue_name, "Create event consumer queue");
    let channel = conn.create_channel().await?;
    let exchange = format!("{}.events", source_service);

    channel
        .exchange_declare(
            &exchange,
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            queue_name,
            &exchange,
            event_type,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    channel
        .basic_qos(prefetch_count, BasicQosOptions::default())
        .await?;

    Ok(channel)
}

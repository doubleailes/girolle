/// # queue
///
/// This module contains functions to create queues and channels for the RPC communication.
use lapin::{
    options::{BasicQosOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use tracing::{error, info};
use uuid::Uuid;

/// # QUEUE_TTL
const QUEUE_TTL: u32 = 300000;

pub async fn get_connection(
    amqp_uri: String,
    heartbeat_value: u16,
) -> Result<lapin::Connection, lapin::Error> {
    let mut connection_options = ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);
    let mut client_properties_custom = FieldTable::default();
    client_properties_custom.insert("heartbeat".into(), heartbeat_value.into());
    connection_options.client_properties = client_properties_custom;
    match Connection::connect(&amqp_uri, connection_options).await {
        Ok(connection) => {
            info!("Connected to RabbitMQ");
            return Ok(connection);
        }
        Err(e) => {
            error!("Failed to connect to RabbitMQ with error:{}", e);
            return Err(e);
        }
    }
}

/// # create_service_channel
///
/// This function creates a channel for a service.
///
/// ## Arguments
///
/// * `service_name` - A string slice that holds the name of the service.
/// * `amqp_uri` - A string that holds the URI of the AMQP server.
/// * `heartbeat_value` - A u16 that holds the heartbeat value.
/// * `prefetch_count` - A u16 that holds the prefetch count.
/// * `rpc_exchange` - A string slice that holds the name of the exchange.
///
/// ## Returns
///
/// A lapin::Result<lapin::Channel> that holds the channel.
pub async fn create_service_channel(
    conn: &Connection,
    service_name: &str,
    prefetch_count: u16,
    rpc_exchange: &str,
) -> lapin::Result<lapin::Channel> {
    info!("Create service queue");
    let routing_key = format!("{}.*", service_name);
    info!("{:?}", conn.status());
    let incomming_channel = conn.create_channel().await?;
    let mut queue_declare_options = QueueDeclareOptions::default();
    queue_declare_options.durable = true;
    let rpc_queue = format!("rpc-{}", service_name);
    let queue = incomming_channel
        .queue_declare(&rpc_queue, queue_declare_options, FieldTable::default())
        .await?;
    info!(?queue, "Declared queue");
    incomming_channel
        .basic_qos(prefetch_count, BasicQosOptions::default())
        .await?;
    let _incomming_queue = incomming_channel
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

/// # create_message_channel
///
/// This function creates a channel for a message.
///
/// ## Arguments
///
/// * `rpc_queue_reply` - A string slice that holds the name of the queue.
/// * `id` - A string slice that holds the id of the message.
/// * `amqp_uri` - A string that holds the URI of the AMQP server.
/// * `heartbeat_value` - A u16 that holds the heartbeat value.
/// * `rpc_exchange` - A string slice that holds the name of the exchange.
///
/// ## Returns
///
/// A lapin::Result<lapin::Channel> that holds the channel.
pub async fn create_message_channel(
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
    let mut queue_declare_options = QueueDeclareOptions::default();
    queue_declare_options.durable = true;
    // Need to clone the response_arguments because the queue_declare function takes ownership of the FieldTable
    response_channel
        .queue_declare(
            rpc_queue_reply,
            queue_declare_options,
            response_arguments.clone(),
        )
        .await
        .map_err(|e| {
            // Handle or log the error
            e
        })?;
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

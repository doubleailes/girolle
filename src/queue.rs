/// # queue
///
/// This module contains functions to create queues and channels for the RPC communication.
use lapin::{
    options::BasicQosOptions, options::QueueBindOptions, options::QueueDeclareOptions,
    types::FieldTable, Connection, ConnectionProperties,
};
use std::env;
use tracing::info;
use uuid::Uuid;

/// # get_address
///
/// This function returns the address of the RabbitMQ server.
///
pub fn get_address() -> String {
    let user = env::var("RABBITMQ_USER").expect("RABBITMQ_USER not set");
    let password = env::var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD not set");
    let host = env::var("RABBITMQ_HOST").expect("RABBITMQ_HOST not set");
    let port = env::var("RABBITMQ_PORT").unwrap_or("5672".to_string());
    format!("amqp://{}:{}@{}:{}/%2f", user, password, host, port)
}

async fn get_connection() -> lapin::Result<Connection> {
    let addr = get_address();
    Connection::connect(&addr, ConnectionProperties::default()).await
}

/// # create_service_queue
///
/// This function creates a queue for a service.
///
/// ## Arguments
///
/// * `service_name` - A string slice that holds the name of the service.
pub async fn create_service_queue(service_name: String) -> lapin::Result<lapin::Channel> {
    let routing_key = format!("{}.*", service_name);
    const PREFETCH_COUNT: u16 = 10;
    let conn = get_connection().await?;
    let incomming_channel = conn.create_channel().await?;
    let mut queue_declare_options = QueueDeclareOptions::default();
    queue_declare_options.durable = true;
    let rpc_queue = format!("rpc-{}", service_name);
    let queue = incomming_channel
        .queue_declare(&rpc_queue, queue_declare_options, FieldTable::default())
        .await?;
    info!(?queue, "Declared queue");
    incomming_channel
        .basic_qos(PREFETCH_COUNT, BasicQosOptions::default())
        .await?;
    let _incomming_queue = incomming_channel
        .queue_bind(
            &rpc_queue,
            "nameko-rpc",
            &routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(incomming_channel)
}

/// # create_message_queue
///
/// This function creates a queue for a message.
///
/// ## Arguments
///
/// * `rpc_queue_reply` - A string slice that holds the name of the queue.
/// * `id` - A string slice that holds the id of the message.
pub async fn create_message_queue(
    rpc_queue_reply: String,
    id: &Uuid,
) -> lapin::Result<lapin::Channel> {
    const QUEUE_TTL: u32 = 300000;
    let conn = get_connection().await?;
    let response_channel = conn.create_channel().await?;
    let mut response_arguments = FieldTable::default();
    response_arguments.insert("x-expires".into(), QUEUE_TTL.into());
    let mut queue_declare_options = QueueDeclareOptions::default();
    queue_declare_options.durable = true;
    response_channel
        .queue_declare(&rpc_queue_reply, queue_declare_options, response_arguments)
        .await?;
    response_channel
        .queue_bind(
            &rpc_queue_reply,
            "nameko-rpc",
            &id.to_string(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(response_channel)
}

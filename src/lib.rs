use futures_lite::stream::StreamExt;
use lapin::{
    options::*, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties, Result,
    Channel,
    publisher_confirm::Confirmation,
};
pub use serde_json as JsonValue;
use serde_json::Value;
use std::env;
use tracing::{error, info};
use uuid::Uuid;
use JsonValue::json;

fn get_address() -> String {
    let user = env::var("RABBITMQ_USER").expect("RABBITMQ_USER not set");
    let password = env::var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD not set");
    let host = env::var("RABBITMQ_HOST").expect("RABBITMQ_HOST not set");
    format!("amqp://{}:{}@{}:5672/%2f", user, password, host)
}
async fn publish(response_channel: &Channel, payload: String, properties: BasicProperties, reply_to_id: String) -> Result<Confirmation> {
    let confirm = response_channel
    .basic_publish(
        "nameko-rpc",
        &format!("{}", &reply_to_id),
        BasicPublishOptions::default(),
        payload.as_bytes(),
        properties,
    )
    .await?
    .await?;
    // The message was correctly published
    assert_eq!(confirm, Confirmation::NotRequested);
    Ok(confirm)
}

pub fn async_service(service_name: String, f: fn(Value) -> Value) -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    // Define the queue name1
    let rpc_queue = format!("rpc-{}", service_name);
    let routing_key = format!("{}.*", service_name);
    // Add tracing
    tracing_subscriber::fmt::init();
    let addr = get_address();
    // Uuid of the service
    let id = Uuid::new_v4();

    async_global_executor::block_on(async {
        // Connect to RabbitMQ
        let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;
        info!("CONNECTED");
        // Open a channel and set the QOS
        let incomming_channel = conn.create_channel().await?;
        let response_channel = conn.create_channel().await?;
        incomming_channel
            .basic_qos(10, BasicQosOptions::default())
            .await?;
        let mut queue_declare_options = QueueDeclareOptions::default();
        queue_declare_options.durable = true;
        let queue = incomming_channel
            .queue_declare(&rpc_queue, queue_declare_options, FieldTable::default())
            .await?;
        let _incomming_queue = incomming_channel
            .queue_bind(
                &rpc_queue,
                "nameko-rpc",
                &routing_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();
            // Declare the reply queue
            let rpc_queue_reply = format!("rpc.reply-{}-{}", service_name, &id);
            response_channel
                .queue_declare(
                    &rpc_queue_reply,
                    queue_declare_options,
                    FieldTable::default(),
                )
                .await
                .unwrap();
            response_channel
                .queue_bind(
                    &rpc_queue_reply,
                    "nameko-rpc",
                    &format!("{}", &id),
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await
                .unwrap();
        info!(?queue, "Declared queue");
        // Start a consumer.
        let mut consumer = incomming_channel
            .basic_consume(
                &rpc_queue,
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
            info!("will consume");
            // Iterate over deliveries.
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.expect("error in consumer");
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
            let incomming_data: Value = serde_json::from_slice(&delivery.data).expect("json");
            // Get the correlation_id and reply_to_id
            let opt_correlation_id = delivery.properties.correlation_id();
            let correlation_id = match opt_correlation_id {
                Some(correlation_id) => correlation_id.to_string(),
                None => {
                    error!("correlation_id: None");
                    panic!("correlation_id: None")
                }
            };
            let opt_reply_to_id = delivery.properties.reply_to();
            let reply_to_id = match opt_reply_to_id {
                Some(reply_to_id) => reply_to_id.to_string(),
                None => {
                    error!("reply_to_id: None");
                    panic!("reply_to_id: None")
                }
            };
            let properties = BasicProperties::default()
                .with_correlation_id(correlation_id.into())
                .with_content_type("application/json".into())
                .with_reply_to(rpc_queue_reply.clone().into());
            // Publish the response
            let payload: String = json!(
                {
                    "result": f(incomming_data),
                    "error": null,
                }
            )
            .to_string();
            publish(&response_channel, payload, properties, reply_to_id).await.unwrap();
            }
        info!("GoodBye!");
        Ok(())
    })
}

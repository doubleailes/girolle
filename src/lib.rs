use futures_lite::stream::StreamExt;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Channel,
    Connection, ConnectionProperties, Result,
};
pub use serde_json as JsonValue;
use serde_json::Value;
use std::{collections::HashMap, env};
use tracing::{error, info, warn};
use uuid::Uuid;
use JsonValue::json;
mod nameko_utils;
use nameko_utils::insert_new_id_to_call_id;

fn get_address() -> String {
    let user = env::var("RABBITMQ_USER").expect("RABBITMQ_USER not set");
    let password = env::var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD not set");
    let host = env::var("RABBITMQ_HOST").expect("RABBITMQ_HOST not set");
    let port = env::var("RABBITMQ_PORT").unwrap_or("5672".to_string());
    format!("amqp://{}:{}@{}:{}/%2f", user, password, host, port)
}
async fn publish(
    response_channel: &Channel,
    payload: String,
    properties: BasicProperties,
    reply_to_id: String,
) -> Result<Confirmation> {
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

/// Create RPC service and listen to the Nameko RPC queue for the service_name
/// The service_name is the name of the service in the Nameko microservice
/// The f is a HashMap of the functions that will be called when the routing key is called
/// The function f must return a serializable value
pub fn rpc_service(
    service_name: String,
    f: HashMap<String, fn(Vec<&Value>) -> Value>,
) -> Result<()> {
    const PREFETCH_COUNT: u16 = 10;
    // Set the log level
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
            .basic_qos(PREFETCH_COUNT, BasicQosOptions::default())
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
        let mut response_arguments = FieldTable::default();
        response_arguments.insert("x-expires".into(), 300000.into());

        response_channel
            .queue_declare(&rpc_queue_reply, queue_declare_options, response_arguments)
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
            let opt_routing_key = delivery.routing_key.to_string();
            let fn_service: fn(Vec<&Value>) -> Value = match f.get(&opt_routing_key) {
                Some(fn_service) => *fn_service,
                None => {
                    warn!("fn_service: None");
                    continue;
                }
            };
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
            let incomming_data: Value = serde_json::from_slice(&delivery.data).expect("json");
            let args: Vec<&Value> = incomming_data["args"]
                .as_array()
                .expect("args")
                .iter()
                .collect();
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
            let opt_headers = delivery.properties.headers();
            let headers = insert_new_id_to_call_id(
                opt_headers.as_ref().expect("headers").clone(),
                &opt_routing_key,
                &id.to_string(),
            );
            let properties = BasicProperties::default()
                .with_correlation_id(correlation_id.into())
                .with_content_type("application/json".into())
                .with_reply_to(rpc_queue_reply.clone().into())
                .with_headers(headers);

            // Publish the response
            let payload: String = json!(
                {
                    "result": fn_service(args),
                    "error": null,
                }
            )
            .to_string();
            publish(&response_channel, payload, properties, reply_to_id)
                .await
                .expect("Error publishing");
        }
        info!("GoodBye!");
        Ok(())
    })
}

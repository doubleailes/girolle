use bson::Document;
use futures_lite::stream::StreamExt;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReponsePayload {
    result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}
impl ReponsePayload {
    pub fn new(result: String) -> Self {
        Self {
            result,
            error: None,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct IncommingData {
    args: Vec<String>,
    kwargs: Document,
}
impl IncommingData {
    pub fn get_args(&self) -> &Vec<String> {
        &self.args
    }
}

fn get_address() -> String {
    let user = env::var("RABBITMQ_USER").expect("RABBITMQ_USER not set");
    let password = env::var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD not set");
    let host = env::var("RABBITMQ_HOST").expect("RABBITMQ_HOST not set");
    format!("amqp://{}:{}@{}:5672/%2f", user, password, host)
}

pub fn async_service(service_name: &str, f: fn(String) -> String) -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    let rpc_queue = format!("rpc-{}", service_name);
    let routing_key = format!("{}.*", service_name);
    tracing_subscriber::fmt::init();
    let addr = get_address();
    let id = Uuid::new_v4();

    async_global_executor::block_on(async {
        let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;

        info!("CONNECTED");

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

        info!(?queue, "Declared queue");

        let mut consumer = incomming_channel
            .basic_consume(
                &rpc_queue,
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        info!("will consume");
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.expect("error in consumer");
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
            let incomming_data: IncommingData =
                serde_json::from_slice(&delivery.data).expect("json");
            info!("incomming_data: {:?}", incomming_data.get_args());
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
            let payload = json!({
                "result": f(incomming_data.get_args()[0].clone()),
                "error": None::<String>
            });
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
            let properties = BasicProperties::default()
                .with_correlation_id(correlation_id.into())
                .with_content_type("application/json".into())
                .with_reply_to(rpc_queue_reply.into());
            let confirm = response_channel
                .basic_publish(
                    "nameko-rpc",
                    &format!("{}", &reply_to_id),
                    BasicPublishOptions::default(),
                    payload.to_string().as_bytes(),
                    properties,
                )
                .await?
                .await?;
            assert_eq!(confirm, Confirmation::NotRequested);
        }
        Ok(())
    })
}

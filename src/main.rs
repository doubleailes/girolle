use lapin::{
    message::DeliveryResult,
    options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use std::env;
use serde_json;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    name: String,
    size: u32,
}

#[tokio::main]
async fn main() {
    let user = env::var("RABBITMQ_USER").expect("RABBITMQ_USER not set");
    let password = env::var("RABBITMQ_PASSWORD").expect("RABBITMQ_PASSWORD not set");
    let host = env::var("RABBITMQ_HOST").expect("RABBITMQ_HOST not set");
    let uri = format!("amqp://{}:{}@{}:5672/%2f", user, password, host);
    let options = ConnectionProperties::default()
        // Use tokio executor and reactor.
        // At the moment the reactor is only available for unix.
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let connection = Connection::connect(&uri, options).await.unwrap();
    let channel = connection.create_channel().await.unwrap();

    let consumer = channel
        .basic_consume(
            "video",
            "tag_foo",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    consumer.set_delegate(move |delivery: DeliveryResult| async move {
        let delivery = match delivery {
            // Carries the delivery alongside its channel
            Ok(Some(delivery)) => delivery,
            // The consumer got canceled
            Ok(None) => return,
            // Carries the error and is always followed by Ok(None)
            Err(error) => {
                dbg!("Failed to consume queue message {}", error);
                return;
            }
        };
        let payload: Message = serde_json::from_slice(&delivery.data).unwrap();
        println!("Received message: {:#?}", &payload);

        delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to ack send_webhook_event message");
    });

    std::future::pending::<()>().await;
}

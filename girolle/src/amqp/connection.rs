use lapin::{types::LongString, Connection, ConnectionProperties};
use tracing::{error, info};

/// Open an AMQP connection on the ambient Tokio runtime, with a custom
/// heartbeat advertised in the client properties.
pub(crate) async fn get_connection(
    amqp_uri: String,
    heartbeat_value: u16,
) -> Result<Connection, lapin::Error> {
    // lapin drives its IO loop on the current Tokio runtime by default
    // (`tokio` feature), so no executor/reactor needs wiring up here.
    let connection_options = ConnectionProperties::default().with_client_property(
        "heartbeat".into(),
        LongString::from(heartbeat_value.to_string()),
    );
    match Connection::connect(&amqp_uri, connection_options).await {
        Ok(connection) => {
            info!("Connected to RabbitMQ");
            Ok(connection)
        }
        Err(e) => {
            error!("Failed to connect to RabbitMQ with error:{}", e);
            Err(e)
        }
    }
}

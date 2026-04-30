use lapin::uri::AMQPUri;
use lapin::{Connection, ConnectionProperties};
use tracing::{error, info};

/// Open an AMQP connection on the ambient Tokio runtime, applying a
/// custom heartbeat (in seconds) to the negotiated connection.
pub(crate) async fn get_connection(
    amqp_uri: String,
    heartbeat_value: u16,
) -> Result<Connection, lapin::Error> {
    let mut uri: AMQPUri = amqp_uri.parse().map_err(std::io::Error::other)?;
    uri.query.heartbeat = Some(heartbeat_value);
    match Connection::connect_uri(uri, ConnectionProperties::default()).await {
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

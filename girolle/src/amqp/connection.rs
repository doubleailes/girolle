use lapin::{types::FieldTable, Connection, ConnectionProperties};
use tracing::{error, info};

/// Open an AMQP connection wired up to the Tokio executor and reactor,
/// with a custom heartbeat advertised in the client properties.
pub(crate) async fn get_connection(
    amqp_uri: String,
    heartbeat_value: u16,
) -> Result<Connection, lapin::Error> {
    let mut connection_options = ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);
    let mut client_properties_custom = FieldTable::default();
    client_properties_custom.insert("heartbeat".into(), heartbeat_value.into());
    connection_options.client_properties = client_properties_custom;
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

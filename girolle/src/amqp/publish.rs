use crate::protocol::PayloadResult;
use lapin::options::BasicPublishOptions;
use lapin::{BasicProperties, Channel};

/// Publish an RPC reply on `rpc_exchange` with `reply_to_id` as the
/// routing key. Spawned as a fire-and-forget task so the consumer
/// loop is not blocked on the broker's confirm.
pub(crate) async fn publish(
    rpc_channel: &Channel,
    payload: PayloadResult,
    properties: BasicProperties,
    reply_to_id: String,
    rpc_exchange: &str,
) -> lapin::Result<()> {
    let rpc_channel_clone = rpc_channel.clone();
    let rpc_exchange_clone = rpc_exchange.to_string();
    tokio::spawn(async move {
        rpc_channel_clone
            .basic_publish(
                rpc_exchange_clone.as_str().into(),
                reply_to_id.as_str().into(),
                BasicPublishOptions::default(),
                payload
                    .to_string()
                    .expect("can't serialize payload")
                    .as_bytes(),
                properties,
            )
            .await
            .unwrap()
            .await
            .unwrap();
    });
    Ok(())
}

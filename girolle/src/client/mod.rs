//! Standalone RPC client.
//!
//! Speaks the same Nameko wire protocol as a service handler's
//! `ctx.rpc.call`, but lives outside any service. Owns its own
//! connection, reply queue, and consumer; offers blocking
//! [`RpcClient::send`] and async-friendly
//! [`RpcClient::call_async`] / [`RpcClient::result`] forms.

pub mod reply;

use crate::amqp::channel::{create_message_channel, create_service_channel};
use crate::amqp::connection::get_connection;
use crate::config::Config;
use crate::error::{GirolleError, GirolleResult};
use crate::payload::Payload;
use crate::protocol::{PayloadResult, NAMEKO_AMQP_URI, NAMEKO_CALL_ID_STACK};
use futures::executor;
use lapin::{
    message::DeliveryResult,
    options::*,
    types::{AMQPValue, FieldArray, FieldTable},
    BasicProperties, Connection,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Condvar, Mutex};
use tracing::error;
use uuid::Uuid;

pub use self::reply::{RpcReply, RpcResult};

/// # RpcClient
///
/// Standalone client capable of calling any number of registered
/// services. Construct with [`RpcClient::new`], call
/// [`RpcClient::register_service`] for each service it should be able
/// to reach, then [`RpcClient::start`] before issuing calls.
///
/// ## Example
///
/// ```rust,no_run
/// use girolle::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     let rpc_client = RpcClient::new(Config::default());
///     # let _ = rpc_client;
/// }
/// ```
pub struct RpcClient {
    conf: Config,
    identifier: Uuid,
    conn: Connection,
    reply_channel: lapin::Channel,
    services: HashMap<String, TargetService>,
    not_empty: Arc<Condvar>,
    replies: Arc<Mutex<HashMap<String, PayloadResult>>>,
}

impl RpcClient {
    /// Open a connection and declare the per-client reply queue.
    ///
    /// Panics if the broker is unreachable or the reply queue cannot
    /// be declared — this constructor blocks the calling thread.
    pub fn new(conf: Config) -> Self {
        let identifier = Uuid::new_v4();
        let conn = executor::block_on(get_connection(conf.AMQP_URI(), conf.heartbeat()))
            .expect("Can't init connection");
        let reply_queue_name = format!("rpc.listener-{}", identifier);
        let reply_channel = executor::block_on(create_message_channel(
            &conn,
            &reply_queue_name,
            conf.prefetch_count(),
            &identifier,
            conf.rpc_exchange(),
        ))
        .expect("Can't create reply channel");
        Self {
            conf,
            identifier,
            conn,
            reply_channel,
            services: HashMap::new(),
            replies: Arc::new(Mutex::new(HashMap::new())),
            not_empty: Arc::new(Condvar::new()),
        }
    }

    /// Spawn the consumer that ingests replies into the correlation
    /// map. Must be awaited before any call is issued.
    pub async fn start(&mut self) -> GirolleResult<()> {
        let reply_queue_name = format!("rpc.listener-{}", self.identifier);
        let consumer = self
            .reply_channel
            .basic_consume(
                reply_queue_name.as_str().into(),
                "girolle_consumer".into(),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let channel = self.reply_channel.clone();
        let replies = self.replies.clone();
        let not_empty = self.not_empty.clone();
        consumer.set_delegate(move |delivery: DeliveryResult| {
            let channel = channel.clone();
            let replies = replies.clone();
            let not_empty = not_empty.clone();
            async move {
                if let Ok(Some(delivery)) = delivery {
                    let correlation_id: Option<lapin::types::ShortString> =
                        delivery.properties.correlation_id().clone();
                    let payload: PayloadResult = match serde_json::from_slice(&delivery.data) {
                        Ok(payload) => payload,
                        Err(e) => {
                            error!(error = %e, "Deserialization failed");
                            let error = GirolleError::SerdeJsonError(e);
                            PayloadResult::from_error(error.convert())
                        }
                    };
                    if let Ok(mut replies) = replies.lock() {
                        if let Some(correlation_id) = correlation_id {
                            replies.insert(correlation_id.to_string(), payload);
                            not_empty.notify_one();
                        } else {
                            error!("Correlation id is missing");
                        }
                    }
                    channel
                        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                        .await
                        .expect("ack");
                }
            }
        });
        Ok(())
    }

    /// The reply-queue identifier. Useful for diagnostics; matches the
    /// `<reply-to>` header of every outbound call.
    pub fn get_identifier(&self) -> String {
        self.identifier.to_string()
    }

    /// Send `<service>.<method>` and return a handle that can later
    /// be passed to [`RpcClient::result`] to await the reply.
    pub fn call_async(
        &self,
        target_service: &str,
        method_name: &str,
        payload: Payload,
    ) -> Result<RpcReply, GirolleError> {
        if !self.service_exist(target_service) {
            return Err(GirolleError::ServiceMissingError(format!(
                "Service {} is missing",
                target_service
            )));
        }
        let routing_key = format!("{}.{}", target_service, method_name);
        let correlation_id = Uuid::new_v4();
        let mut headers: BTreeMap<lapin::types::ShortString, AMQPValue> = BTreeMap::new();
        headers.insert(
            NAMEKO_AMQP_URI.into(),
            AMQPValue::LongString(self.conf.AMQP_URI().into()),
        );
        headers.insert(
            NAMEKO_CALL_ID_STACK.into(),
            AMQPValue::FieldArray(FieldArray::from(vec![AMQPValue::LongString(
                self.identifier.to_string().into(),
            )])),
        );
        let properties: BasicProperties = BasicProperties::default()
            .with_reply_to(self.identifier.to_string().into())
            .with_correlation_id(correlation_id.to_string().into())
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_headers(FieldTable::from(headers))
            .with_priority(0);

        let exchange_clone = self.conf.rpc_exchange().to_string();
        let channel_clone = self.services.get(target_service).unwrap().channel.clone();

        tokio::spawn(async move {
            channel_clone
                .basic_publish(
                    exchange_clone.as_str().into(),
                    routing_key.as_str().into(),
                    BasicPublishOptions {
                        mandatory: false,
                        immediate: false,
                    },
                    payload.to_string().as_bytes(),
                    properties,
                )
                .await
                .expect("Failed to publish");
        });

        Ok(RpcReply::new(correlation_id))
    }

    /// Block on the reply for `rpc_event` and return it as an
    /// [`RpcResult`].
    pub fn result(&self, rpc_event: &RpcReply) -> GirolleResult<RpcResult> {
        Ok(RpcResult::new(
            self._result(rpc_event)?,
            rpc_event.get_elapsed_time()?,
        ))
    }

    fn _result(&self, rpc_event: &RpcReply) -> GirolleResult<Value> {
        let incomming_id = rpc_event.get_correlation_id();
        let mut replies = self.replies.lock().unwrap();
        let result_reply = loop {
            if let Some(value) = replies.get(&incomming_id) {
                break value.clone();
            } else {
                replies = self.not_empty.wait(replies).unwrap();
            }
        };
        replies.remove(&incomming_id);
        drop(replies);
        match result_reply.get_error() {
            Some(result_error) => Err(result_error.convert_to_girolle_error()),
            None => Ok(result_reply.get_result()),
        }
    }

    /// Synchronous one-shot: send the call and block until the reply
    /// arrives.
    pub fn send(
        &self,
        target_service: &str,
        method_name: &str,
        payload: Payload,
    ) -> GirolleResult<RpcResult> {
        let rpc_event = self.call_async(target_service, method_name, payload)?;
        Ok(RpcResult::new(
            self._result(&rpc_event)?,
            rpc_event.get_elapsed_time()?,
        ))
    }

    fn service_exist(&self, service_name: &str) -> bool {
        self.services.contains_key(service_name)
    }

    pub fn get_config(&self) -> &Config {
        &self.conf
    }

    pub fn set_config(&mut self, config: Config) -> Result<(), String> {
        self.conf = config;
        Ok(())
    }

    /// Open a publish channel for `service_name` and remember it.
    /// Must be called before any `send`/`call_async` to that service.
    pub async fn register_service(&mut self, service_name: &str) -> Result<(), lapin::Error> {
        let channel = create_service_channel(
            &self.conn,
            service_name,
            self.conf.prefetch_count(),
            self.conf.rpc_exchange(),
        )
        .await?;
        self.services
            .insert(service_name.to_string(), TargetService::new(channel));
        Ok(())
    }

    /// Tear down the publish channel for `service_name` and forget it.
    pub fn unregister_service(&mut self, service_name: &str) -> Result<(), lapin::Error> {
        let target_service = self.services.get(service_name).unwrap();
        target_service.close()?;
        self.services.remove(service_name);
        Ok(())
    }

    /// Close the reply channel and the underlying AMQP connection.
    pub async fn close(&self) -> Result<(), lapin::Error> {
        self.reply_channel.close(200, "Goodbye".into()).await?;
        self.conn.close(200, "Goodbye".into()).await?;
        Ok(())
    }
}

/// Per-target publish channel kept on the [`RpcClient`].
struct TargetService {
    channel: lapin::Channel,
}

impl TargetService {
    fn new(channel: lapin::Channel) -> Self {
        Self { channel }
    }

    fn close(&self) -> Result<(), lapin::Error> {
        executor::block_on(self.channel.close(200, "Goodbye".into()))?;
        Ok(())
    }

    #[allow(dead_code)]
    fn get_call_channel_id(&self) -> u16 {
        self.channel.id()
    }
}

use crate::config::Config;
use crate::payload::Payload;
use crate::queue::{create_message_channel, create_service_channel, get_connection};
use crate::types::NamekoResult;
use futures::{executor, stream::StreamExt};
use lapin::Connection;
use lapin::{
    options::*,
    publisher_confirm::Confirmation,
    types::{AMQPValue, FieldArray, FieldTable},
    BasicProperties, Consumer,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tracing::error;
use uuid::Uuid;

/// # RpcClient
///
/// ## Description
///
/// This struct is used to create a RPC client. This client is used to call a
/// function in the Nameko microservice and get a result.
///
/// ## Example
///
/// ```rust
/// use girolle::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///    let rpc_client = RpcClient::new(Config::default_config());
/// }
pub struct RpcClient {
    conf: Config,
    identifier: Uuid,
    conn: Connection,
}
impl RpcClient {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcClientstruct
    ///
    /// ## Arguments
    ///
    /// * `conf` - The configuration as Config
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcClientstruct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let rpc_call = RpcClient::new(Config::default_config());
    /// }
    pub fn new(conf: Config) -> Self {
        let conn = executor::block_on(get_connection(conf.AMQP_URI(), conf.heartbeat()))
            .expect("Can't init connection");
        Self {
            conf,
            identifier: Uuid::new_v4(),
            conn,
        }
    }
    /// # get_identifier
    ///
    /// ## Description
    ///
    /// This function return the identifier of the RpcClientstruct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rpc_call = RpcClient::new(Config::default_config());
    ///     let identifier = rpc_call.get_identifier();
    /// }
    pub fn get_identifier(&self) -> String {
        self.identifier.to_string()
    }
    /// # call_async
    ///
    /// ## Description
    ///
    /// This function call the Nameko-like microservice, using the service name and
    /// the function to target. This function return an async consumer.
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    /// * `method_name` - The name of the function to call
    /// * `args` - The arguments of the function as Vec<T>
    ///
    /// ## Example
    ///
    /// See example simple_sender in the examples folder
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
    ///    let rpc_call = RpcClient::new(conf);
    ///    let service_name = "video";
    ///    let method_name = "hello";
    ///    let args = vec!["John Doe"];
    ///    let consumer = rpc_call.call_async(service_name, method_name, args).await.expect("call");
    /// }
    ///
    pub async fn call_async<T: Serialize>(
        &self,
        service_name: &str,
        method_name: &str,
        args: Vec<T>,
    ) -> lapin::Result<Consumer> {
        let payload = Payload::new(json!(args));
        let correlation_id = Uuid::new_v4().to_string();
        let routing_key = format!("{}.{}", service_name, method_name);
        let channel = create_service_channel(
            &self.conn,
            service_name,
            self.conf.prefetch_count(),
            &self.conf.rpc_exchange(),
        )
        .await?;
        let mut headers: BTreeMap<lapin::types::ShortString, AMQPValue> = BTreeMap::new();
        headers.insert(
            "nameko.AMQP_URI".into(),
            AMQPValue::LongString(self.conf.AMQP_URI().into()),
        );
        headers.insert(
            "nameko.call_id_stack".into(),
            AMQPValue::FieldArray(FieldArray::from(vec![AMQPValue::LongString(
                self.identifier.to_string().into(),
            )])),
        );
        let properties: lapin::protocol::basic::AMQPProperties = BasicProperties::default()
            .with_reply_to(self.identifier.to_string().into())
            .with_correlation_id(correlation_id.into())
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_headers(FieldTable::from(headers));
        let reply_name: &str = "rpc.listener";
        let rpc_queue_reply: &str = &format!("{}-{}", reply_name, &self.identifier.to_string());
        let reply_queue: lapin::Channel = match create_message_channel(
            &self.conn,
            rpc_queue_reply,
            &self.identifier,
            &self.conf.rpc_exchange(),
        )
        .await
        {
            Ok(queue) => queue,
            Err(e) => {
                // Handle error, e.g., log it or retry
                error!("Error: {:?}", e);
                return Err(e);
            }
        };
        let consumer = reply_queue
            .basic_consume(
                &rpc_queue_reply,
                "girolle_consumer_reply",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let channel_clone = channel.clone();
        let routing_key_clone = routing_key.clone();
        let payload_clone = payload.clone();
        let properties_clone = properties.clone();
        let rpc_exchange_clone = self.conf.rpc_exchange().to_string();

        tokio::spawn(async move {
            let confirm = channel_clone
                .basic_publish(
                    &rpc_exchange_clone,
                    &routing_key_clone,
                    BasicPublishOptions::default(),
                    serde_json::to_value(payload_clone)
                        .expect("json")
                        .to_string()
                        .as_bytes(),
                    properties_clone,
                )
                .await
                .expect("Failed to publish")
                .await
                .expect("Failed to confirm");
            assert_eq!(confirm, Confirmation::NotRequested);
        });
        Ok(consumer)
    }
    /// # result
    ///
    /// ## Description
    ///
    /// This function return the result of the call to the Nameko microservice.
    /// This function is async.
    /// It return a `NamekoResult<Value>` as Value is `serde_json::Value`
    ///
    /// ## Arguments
    ///
    /// * `ref_consumer` - The consumer to use to get the result
    ///
    /// ## Example
    ///
    /// See example simple_sender in the examples folder
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
    ///    let rpc_call = RpcClient::new(conf);
    ///    let service_name = "video";
    ///    let method_name = "hello";
    ///    let args = vec!["John"];
    ///    let consumer = rpc_call.call_async(service_name, method_name, args).await.expect("call");
    ///    let result = rpc_call.result(consumer);
    /// }
    pub async fn result(&self, ref_consumer: Consumer) -> Value {
        let mut consumer = ref_consumer;
        let delivery = consumer
            .next()
            .await
            .expect("error in consumer")
            .expect("error in consumer");
        let mut incomming_data: Value = serde_json::from_slice(&delivery.data).expect("json");
        delivery.ack(BasicAckOptions::default()).await.expect("ack");
        match incomming_data["error"].as_object() {
            Some(error) => {
                let value = error["value"].as_str().expect("value");
                error!("Error: {}", value);
                panic!("Error: {}", value);
            }
            None => {}
        }
        incomming_data["result"].take()
    }
    /// # send
    ///
    /// ## Description
    ///
    /// This function send the payload to the Nameko microservice, using the
    /// service name and the function to target. This function is sync.
    /// It return a `NamekoResult<Value>` as Value is `serde_json::Value`
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    /// * `method_name` - The name of the function to call
    /// * `args` - The arguments of the function
    ///
    /// ## Example
    ///
    /// See example simple_sender in the examples folder
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
    ///     let rpc_call = RpcClient::new(conf);
    ///     let service_name = "video";
    ///     let method_name = "hello";
    ///     let args = vec![Value::String("Toto".to_string())];
    ///     let result = rpc_call.send(service_name, method_name, args).expect("call");
    /// }
    pub fn send<T: Serialize>(
        &self,
        service_name: &'static str,
        method_name: &'static str,
        args: Vec<T>,
    ) -> NamekoResult<Value> {
        let consumer =
            executor::block_on(self.call_async(service_name, method_name, args)).expect("call");
        Ok(executor::block_on(self.result(consumer)))
    }
    /// # get_config
    ///
    /// ## Description
    ///
    /// This function return the configuration of the RpcClient struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::*;
    ///
    /// fn main() {
    ///    let rpc_call = RpcClient::new(Config::default_config());
    ///    let conf = rpc_call.get_config();
    /// }
    pub fn get_config(&self) -> &Config {
        &self.conf
    }
    /// # set_config
    ///
    /// ## Description
    ///
    /// This function set the configuration of the RpcClient struct
    ///
    /// ## Arguments
    ///
    /// * `config` - The configuration as Config
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::*;
    ///
    /// fn main() {
    ///    let mut rpc_call = RpcClient::new(Config::default_config());
    ///    let conf = Config::default_config();
    ///    rpc_call.set_config(conf);
    /// }
    pub fn set_config(&mut self, config: Config) -> std::result::Result<(), std::string::String> {
        self.conf = config;
        Ok(())
    }
}

use crate::config::Config;
use crate::payload::Payload;
use crate::queue::{create_message_channel, create_service_channel, get_connection};
use crate::types::NamekoResult;
use futures::{executor, stream::StreamExt};
use lapin::{
    options::*,
    types::{AMQPValue, FieldArray, FieldTable},
    BasicProperties, Connection,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
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
/// ```rust,no_run
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
    /// ```rust,no_run
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
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rpc_client = RpcClient::new(Config::default_config());
    ///     let identifier = rpc_client.get_identifier();
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
    ///    let rpc_client = RpcClient::new(conf);
    ///    let rpc_call = rpc_client.create_rpc_call("video".to_string()).await.expect("call");
    ///    let method_name = "hello";
    ///    let args = vec!["John Doe"];
    ///    let consumer = rpc_client.call_async(&rpc_call, method_name, args);
    /// }
    ///
    pub async fn call_async<T: Serialize>(
        &self,
        rpc_call: &RpcCall,
        method_name: &str,
        args: Vec<T>,
    ) -> lapin::Result<String> {
        let payload: Payload = Payload::new(json!(args));
        let routing_key = format!("{}.{}", rpc_call.service_name, method_name);
        let correlation_id = Uuid::new_v4().to_string();
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
            .with_correlation_id(correlation_id.clone().into())
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_headers(FieldTable::from(headers))
            .with_priority(0);

        let exchange_clone = self.conf.rpc_exchange().to_string();
        let channel_clone = rpc_call.channel.clone();

        tokio::spawn(async move {
            channel_clone
                .basic_publish(
                    &exchange_clone,
                    &routing_key,
                    BasicPublishOptions {
                        mandatory: true,
                        ..BasicPublishOptions::default()
                    },
                    serde_json::to_value(payload)
                        .expect("json")
                        .to_string()
                        .as_bytes(),
                    properties,
                )
                .await
                .expect("Failed to publish")
                .await
                .expect("Failed to confirm");
        });

        Ok(correlation_id)
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
    ///    let rpc_client = RpcClient::new(conf);
    ///    let rpc_call = rpc_client.create_rpc_call("video".to_string()).await.expect("call");
    ///    let method_name = "hello";
    ///    let args = vec!["John Doe"];
    ///    let consumer = rpc_client.call_async(&rpc_call, method_name, args);
    ///     let result = rpc_client.result(&rpc_call, consumer.await.expect("call")).await.expect("call");
    /// }
    pub async fn result(&self, rpc_call: &RpcCall, correlation_id: String) -> lapin::Result<Value> {
        let consumer_arc_mutex: Arc<Mutex<lapin::Consumer>> = Arc::clone(&rpc_call.consumer);

        loop {
            let delivery = {
                let mut consumer = consumer_arc_mutex.lock().unwrap();
                consumer
                    .next()
                    .await
                    .expect("error in consumer")
                    .expect("error in consumer")
            };

            let current_id = delivery
                .properties
                .correlation_id()
                .clone()
                .unwrap()
                .to_string();

            if current_id == correlation_id {
                let mut incomming_data: Value =
                    serde_json::from_slice(&delivery.data).expect("json");
                delivery.ack(BasicAckOptions::default()).await.expect("ack");

                match incomming_data["error"].as_object() {
                    Some(error) => {
                        let value = error["value"].as_str().expect("value");
                        error!("Error: {}", value);
                        panic!("Error: {}", value);
                    }
                    None => {}
                }

                return Ok(incomming_data["result"].take());
            }
        }
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
    ///    let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
    ///    let rpc_client = RpcClient::new(conf);
    ///    let rpc_call = rpc_client.create_rpc_call("video".to_string()).await.expect("call");
    ///    let method_name = "hello";
    ///    let args = vec!["John Doe"];
    ///     let result = rpc_client.send(&rpc_call, method_name, args).expect("call");
    /// }
    pub fn send<T: Serialize>(
        &self,
        rpc_call: &RpcCall,
        method_name: &str,
        args: Vec<T>,
    ) -> NamekoResult<Value> {
        let id = executor::block_on(self.call_async(rpc_call, method_name, args)).expect("call");
        Ok(executor::block_on(self.result(rpc_call, id)).expect("call"))
    }
    /// # get_config
    ///
    /// ## Description
    ///
    /// This function return the configuration of the RpcClient struct
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// fn main() {
    ///    let rpc_client = RpcClient::new(Config::default_config());
    ///    let conf = rpc_client.get_config();
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
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// fn main() {
    ///    let mut rpc_client = RpcClient::new(Config::default_config());
    ///    let conf = Config::default_config();
    ///    rpc_client.set_config(conf);
    /// }
    pub fn set_config(&mut self, config: Config) -> std::result::Result<(), std::string::String> {
        self.conf = config;
        Ok(())
    }
    /// # create_rpc_call
    ///
    /// ## Description
    ///
    /// This function create a new RpcCall struct
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcCall struct
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let rpc_client = RpcClient::new(Config::default_config());
    ///    let rpc_call = rpc_client.create_rpc_call("video".to_string()).await.expect("call");
    /// }
    pub async fn create_rpc_call(&self, service_name: String) -> Result<RpcCall, lapin::Error> {
        let channel = create_service_channel(
            &self.conn,
            &service_name,
            self.conf.prefetch_count(),
            &self.conf.rpc_exchange(),
        )
        .await?;
        let reply_queue_name = format!("rpc.listener-{}", self.identifier);
        let reply_channel = create_message_channel(
            &self.conn,
            &reply_queue_name,
            &self.identifier,
            &self.conf.rpc_exchange(),
        )
        .await?;
        let consumer = reply_channel
            .basic_consume(
                &reply_queue_name,
                &format!("girolle_consumer"),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("Failed to create consumer");
        Ok(RpcCall::new(
            service_name,
            channel,
            reply_channel,
            Arc::new(Mutex::new(consumer)),
        ))
    }
}

pub struct RpcCall {
    service_name: String,
    channel: lapin::Channel,
    reply_channel: lapin::Channel,
    consumer: Arc<Mutex<lapin::Consumer>>,
}
impl RpcCall {
    pub fn new(
        service_name: String,
        channel: lapin::Channel,
        reply_channel: lapin::Channel,
        consumer: Arc<Mutex<lapin::Consumer>>,
    ) -> Self {
        Self {
            service_name,
            channel,
            reply_channel,
            consumer,
        }
    }
    pub fn close(&self) -> Result<(), lapin::Error> {
        executor::block_on(self.channel.close(200, "Goodbye"))?;
        executor::block_on(self.reply_channel.close(200, "Goodbye"))?;
        Ok(())
    }
}

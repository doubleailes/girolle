use crate::config::Config;
use crate::payload::Payload;
use crate::queue::{create_message_channel, create_service_channel, get_connection};
use crate::types::{GirolleError, GirolleResult};
use std::time::{Duration, SystemTime,SystemTimeError};
use futures::executor;
use lapin::{
    message::DeliveryResult,
    options::*,
    types::{AMQPValue, FieldArray, FieldTable},
    BasicProperties, Connection,
};
use serde_json::Value;
use std::collections::HashMap;
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
    reply_channel: lapin::Channel,
    services: HashMap<String, TargetService>,
    replies: Arc<Mutex<HashMap<String, Value>>>,
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
    ///   let target_service = RpcClient::new(Config::default_config());
    /// }
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
        }
    }
    /// # start
    ///
    /// ## Description
    ///
    /// This function start the RpcClient. It create a consumer to listen and consume all incoming
    /// messages from the AMQP server. And store the result in the replies HashMap.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let mut rpc_client = RpcClient::new(Config::default_config());
    ///    rpc_client.start().await.expect("call");
    /// }
    pub async fn start(&mut self) -> GirolleResult<()> {
        let reply_queue_name = format!("rpc.listener-{}", self.identifier);
        let consumer = self
            .reply_channel
            .basic_consume(
                &reply_queue_name,
                "girolle_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let channel = self.reply_channel.clone();
        let replies = self.replies.clone();
        consumer.set_delegate(move |delivery: DeliveryResult| {
            let channel = channel.clone();
            let replies = replies.clone();
            async move {
                if let Ok(Some(delivery)) = delivery {
                    let correlation_id = delivery.properties.correlation_id().clone();
                    let payload = serde_json::from_slice(&delivery.data).expect("json");
                    if let Ok(mut replies) = replies.lock() {
                        if let Some(correlation_id) = correlation_id {
                            replies.insert(correlation_id.to_string(), payload);
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
    ///    let mut rpc_client = RpcClient::new(conf);
    ///    rpc_client.register_service("video").await.expect("call");
    ///    let method_name = "hello";
    ///    let consumer = rpc_client.call_async("video", method_name, Payload::new().arg("John Doe"));
    /// }
    ///
    pub fn call_async(
        &self,
        target_service: &str,
        method_name: &str,
        payload: Payload,
    ) -> Result<RpcReply, GirolleError> {
        if self.service_exist(target_service) == false {
            return Err(GirolleError::ServiceMissingError(format!(
                "Service {} is missing",
                target_service
            )));
        }
        let routing_key = format!("{}.{}", target_service, method_name);
        let correlation_id = Uuid::new_v4();
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
                    &exchange_clone,
                    &routing_key,
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
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///    let conf = Config::with_yaml_defaults("config.yml".to_string()).unwrap();
    ///    let mut rpc_client = RpcClient::new(conf);
    ///    rpc_client.register_service("video").await.expect("call");
    ///    let method_name = "hello";
    ///    let rpc_event = rpc_client.call_async("video", method_name, Payload::new().arg("John Doe"))?;
    ///    let result = rpc_client.result(&rpc_event)?;
    ///    Ok(())
    /// }
    pub fn result(&self, rpc_event: &RpcReply) -> GirolleResult<RpcResult> {
        Ok(RpcResult::new(
            self._result(rpc_event)?,
            rpc_event.get_elapsed_time()?,
        ))
    }
    fn _result(&self, rpc_event: &RpcReply) -> GirolleResult<Value> {
        let incomming_id = rpc_event.get_correlation_id();
        loop {
            let result = {
                let replies = self.replies.lock().unwrap();
                if let Some(value) = replies.get(&incomming_id) {
                    Some(value.clone())
                } else {
                    None
                }
            };

            match result {
                Some(value) => {
                    let mut replies = self.replies.lock().unwrap();
                    replies.remove(&incomming_id);
                    match value["error"].as_object() {
                        Some(error) => {
                            error!("Error: {:?}", error);
                            let e = error["exc_type"].as_str().unwrap().to_string();
                            return Err(GirolleError::RemoteError(e));
                        }
                        None => {
                            return Ok(value["result"].clone());
                        }
                    }
                }
                None => {
                    std::thread::sleep(std::time::Duration::from_nanos(4));
                }
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
    ///    let mut rpc_client = RpcClient::new(conf);
    ///    rpc_client.register_service("video").await.expect("call");
    ///    let method_name = "hello";
    ///     let result = rpc_client.send("video", method_name, Payload::new().arg("John Doe")).expect("call");
    /// }
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
    /// # register_service
    ///
    /// ## Description
    ///
    /// This function create a new TargetService struct
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    ///
    /// ## Returns
    ///
    /// This function return a girolle::TargetService struct
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let mut rpc_client = RpcClient::new(Config::default_config());
    ///    rpc_client.register_service("video").await.expect("call");
    /// }
    pub async fn register_service(&mut self, service_name: &str) -> Result<(), lapin::Error> {
        let channel = create_service_channel(
            &self.conn,
            &service_name,
            self.conf.prefetch_count(),
            &self.conf.rpc_exchange(),
        )
        .await?;
        self.services
            .insert(service_name.to_string(), TargetService::new(channel));
        Ok(())
    }
    /// # unregister_service
    ///
    /// ## Description
    ///
    /// This function remove a service from the RpcClient struct
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    ///
    /// ## Returns
    ///
    /// This function return a Result<(), lapin::Error>
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let mut rpc_client = RpcClient::new(Config::default_config());
    ///    rpc_client.register_service("video").await.expect("call");
    ///    rpc_client.unregister_service("video").expect("call");
    /// }
    pub fn unregister_service(&mut self, service_name: &str) -> Result<(), lapin::Error> {
        let target_service = self.services.get(service_name).unwrap();
        target_service.close()?;
        self.services.remove(service_name);
        Ok(())
    }
    /// # close
    ///
    /// ## Description
    ///
    /// This function close the connection of the RpcClient struct
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::prelude::*;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let rpc_client = RpcClient::new(Config::default_config());
    ///   rpc_client.close().await.expect("close");
    /// }
    pub async fn close(&self) -> Result<(), lapin::Error> {
        self.reply_channel.close(200, "Goodbye").await?;
        self.conn.close(200, "Goodbye").await?;
        Ok(())
    }
}

/// # TargetService
///
/// ## Description
///
/// This struct is used to create a RPC call. It link the client to the service.
/// By creating the channel, the client can send a message to the service.
///
/// ## Example
///
/// ```rust,no_run
/// use girolle::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///      let mut rpc_client = RpcClient::new(Config::default_config());
///      rpc_client.register_service("video").await.expect("call");
/// }
struct TargetService {
    channel: lapin::Channel,
}
impl TargetService {
    fn new(channel: lapin::Channel) -> Self {
        Self { channel }
    }
    #[allow(dead_code)]
    fn close(&self) -> Result<(), lapin::Error> {
        executor::block_on(self.channel.close(200, "Goodbye"))?;
        Ok(())
    }
    #[allow(dead_code)]
    fn get_call_channel_id(&self) -> u16 {
        self.channel.id()
    }
}
/// # RpcReply
/// 
/// ## Description
/// 
/// This struct is used to create a RPC reply. It contains the correlation_id and the time_stamp
pub struct RpcReply {
    correlation_id: uuid::Uuid,
    time_stamp: SystemTime,

}
impl RpcReply {
    fn new(correlation_id: uuid::Uuid) -> Self {
        Self { correlation_id, time_stamp: SystemTime::now()}
    }
    pub fn get_correlation_id(&self) -> String {
        self.correlation_id.to_string()
    }
    pub fn get_elapsed_time(&self) -> Result<Duration, SystemTimeError> {
        self.time_stamp.elapsed()
    }
}

/// # RpcResult
/// 
/// ## Description
/// 
/// This struct is used to create a RPC result. It contains the result and the elapsed_time
pub struct RpcResult {
    result: Value,
    elapsed_time: Duration,
}
impl RpcResult {
    fn new(result: Value, elapsed_time: Duration) -> Self {
        Self {
            result,
            elapsed_time,
        }
    }
    /// # get_result
    /// 
    /// ## Description
    /// 
    /// This function return the result of the RPC call as `serde_json::Value`
    pub fn get_result(&self) -> Value {
        self.result.clone()
    }
    /// # get_elapsed_time
    /// 
    /// ## Description
    /// 
    /// This function return the elapsed time of the RPC call as `std::time::Duration`
    pub fn get_elapsed_time(&self) -> Duration {
        self.elapsed_time
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_rpc_result_new() {
        let result = json!({"success": true});
        let elapsed_time = Duration::from_secs(5);
        let rpc_result = RpcResult::new(result.clone(), elapsed_time);

        assert_eq!(rpc_result.get_result(), result);
        assert_eq!(rpc_result.get_elapsed_time(), elapsed_time);
    }

    #[test]
    fn test_rpc_result_get_result() {
        let result = json!({"success": true});
        let elapsed_time = Duration::from_secs(5);
        let rpc_result = RpcResult::new(result.clone(), elapsed_time);

        assert_eq!(rpc_result.get_result(), result);
    }

    #[test]
    fn test_rpc_result_get_elapsed_time() {
        let result = json!({"success": true});
        let elapsed_time = Duration::from_secs(5);
        let rpc_result = RpcResult::new(result, elapsed_time);

        assert_eq!(rpc_result.get_elapsed_time(), elapsed_time);
    }
    #[test]
    fn test_rpc_reply_new() {
        let correlation_id = Uuid::new_v4();
        let rpc_reply = RpcReply::new(correlation_id);

        assert_eq!(rpc_reply.get_correlation_id(), correlation_id.to_string());
    }

    #[test]
    fn test_rpc_reply_get_correlation_id() {
        let correlation_id = Uuid::new_v4();
        let rpc_reply = RpcReply::new(correlation_id);

        assert_eq!(rpc_reply.get_correlation_id(), correlation_id.to_string());
    }

    #[test]
    fn test_rpc_reply_get_elapsed_time() {
        let correlation_id = Uuid::new_v4();
        let rpc_reply = RpcReply::new(correlation_id);

        // We can't predict the exact elapsed time, but it should be a very small duration
        let elapsed_time = rpc_reply.get_elapsed_time().unwrap();
        assert!(elapsed_time < Duration::from_secs(1));
    }
}
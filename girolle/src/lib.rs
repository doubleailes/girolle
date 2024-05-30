//! ## Description
//!
//! This crate is a Rust implementation of the Nameko RPC protocol.
//! It allows to create a RPC service or Rpc Call in Rust that can be called
//! from or to a Nameko microservice.
//!
//! **Girolle** mock Nameko architecture to send request and get response.
//!
//! ## Examples
//!
//! ### RPC Service
//!
//! This example is simple service that return a string with the name of the person.
//!
//! ```rust,no_run
//!
//! use girolle::{JsonValue::Value, RpcService, Result, Config};
//!
//! fn hello(s: &[Value]) -> Result<Value> {
//!    // Parse the incomming data
//!   let n: String = serde_json::from_value(s[0].clone())?;
//!  let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
//!  Ok(hello_str)
//! }
//!
//! fn main() {
//!   let conf: Config = Config::with_yaml_defaults("config.yml").unwrap();
//!   let mut services: RpcService = RpcService::new(conf,"video");
//!   services.insert("hello", hello);
//!   services.start();
//! }
//! ```
//!
//! ### RPC Client
//!
//! This example is a simple client that call the hello function in the video service.
//!
//! ```rust
//! use girolle::{RpcClient, Config};
//!
//! #[tokio::main]
//! async fn main() {
//!    let rpc_call = RpcClient::new(Config::default_config());
//! }
//! ```
use crate::prelude::{json, Value};
use futures::{executor, stream::StreamExt};
use lapin::{
    message::{Delivery, DeliveryResult},
    options::*,
    publisher_confirm::Confirmation,
    types::{AMQPValue, FieldArray, FieldTable},
    BasicProperties, Channel, Consumer,
};
pub use serde_json as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
mod nameko_utils;
pub mod prelude;
use nameko_utils::{get_id, insert_new_id_to_call_id};
/// # Result
///
/// ## Description
///
/// This type is used to return a Result<Value> in the RPC call
pub type Result<T> = std::result::Result<T, serde_json::Error>;
/// # NamekoFunction
///
/// ## Description
///
/// This type is used to define the function to call in the RPC service it
/// mainly simplify the code to manipulate a complexe type.
pub type NamekoFunction = fn(&[Value]) -> Result<Value>;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod queue;
use queue::{create_message_queue, create_service_queue};
mod config;
pub use config::Config;

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    args: Vec<Value>,
    kwargs: HashMap<String, String>,
}
impl Payload {
    pub fn new(args: Vec<Value>) -> Self {
        Self {
            args,
            kwargs: HashMap::new(),
        }
    }
}
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
/// use girolle::{RpcClient, Config};
///
/// #[tokio::main]
/// async fn main() {
///    let rpc_client = RpcClient::new(Config::default_config());
/// }
pub struct RpcClient {
    conf: Config,
    identifier: Uuid,
}
impl RpcClient {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcCall struct
    ///
    /// ## Arguments
    ///
    /// * `conf` - The configuration as Config
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcCall struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{RpcClient,Config};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let rpc_call = RpcClient::new(Config::default_config());
    /// }
    pub fn new(conf: Config) -> Self {
        Self {
            conf,
            identifier: Uuid::new_v4(),
        }
    }
    /// # get_identifier
    ///
    /// ## Description
    ///
    /// This function return the identifier of the RpcCall struct
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
    /// This function call the Nameko microservice, using the service name and
    /// the function to target. This function return an async consumer.
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
    /// use girolle::JsonValue::Value;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let conf = Config::with_yaml_defaults("config.yml").unwrap();
    ///    let rpc_call = RpcClient::new(conf);
    ///    let service_name = "video";
    ///    let method_name = "hello";
    ///    let args = vec![Value::String("John Doe".to_string())];
    ///    let consumer = rpc_call.call_async(service_name, method_name, args).await.expect("call");
    /// }
    ///
    pub async fn call_async(
        &self,
        service_name: &str,
        method_name: &str,
        args: Vec<Value>,
    ) -> lapin::Result<Consumer> {
        let payload = Payload::new(args);
        let correlation_id = Uuid::new_v4().to_string();
        let routing_key = format!("{}.{}", service_name, method_name);
        let channel = create_service_queue(
            service_name,
            self.conf.AMQP_URI(),
            self.conf.heartbeat(),
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
        let properties = BasicProperties::default()
            .with_reply_to(self.identifier.to_string().into())
            .with_correlation_id(correlation_id.into())
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_headers(FieldTable::from(headers));
        let reply_name = "rpc.listener";
        let rpc_queue_reply = create_rpc_queue_reply_name(reply_name, &self.identifier.to_string());
        let reply_queue = match create_message_queue(
            &rpc_queue_reply,
            &self.identifier,
            self.conf.AMQP_URI(),
            self.conf.heartbeat(),
            &self.conf.rpc_exchange(),
        )
        .await
        {
            Ok(queue) => queue,
            Err(e) => {
                // Handle error, e.g., log it or retry
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
        let confirm = channel
            .basic_publish(
                &self.conf.rpc_exchange(),
                &routing_key,
                BasicPublishOptions::default(),
                serde_json::to_value(payload)
                    .expect("json")
                    .to_string()
                    .as_bytes(),
                properties,
            )
            .await?
            .await?;
        assert_eq!(confirm, Confirmation::NotRequested);
        //Ok(self.result(&mut consumer).await)
        Ok(consumer)
    }
    /// # result
    ///
    /// ## Description
    ///
    /// This function return the result of the call to the Nameko microservice.
    /// This function is async.
    /// It return a Result<Value> as Value is serde_json::Value
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
    /// use girolle::JsonValue::Value;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let conf = Config::with_yaml_defaults("config.yml").unwrap();
    ///    let rpc_call = RpcClient::new(conf);
    ///    let service_name = "video";
    ///    let method_name = "hello";
    ///    let args = vec![Value::String("John".to_string())];
    ///    let consumer = rpc_call.call_async(service_name, method_name, args).await.expect("call");
    ///    let result = rpc_call.result(consumer).await;
    /// }
    pub async fn result(&self, ref_consumer: Consumer) -> Value {
        let mut consumer = ref_consumer;
        let delivery = consumer
            .next()
            .await
            .expect("error in consumer")
            .expect("error in consumer");
        let incomming_data: Value = serde_json::from_slice(&delivery.data).expect("json");
        delivery.ack(BasicAckOptions::default()).await.expect("ack");
        match incomming_data["error"].as_object() {
            Some(error) => {
                let value = error["value"].as_str().expect("value");
                error!("Error: {}", value);
                panic!("Error: {}", value);
            }
            None => {}
        }
        incomming_data["result"].clone()
    }
    /// # send
    ///
    /// ## Description
    ///
    /// This function send the payload to the Nameko microservice, using the
    /// service name and the function to target. This function is sync.
    /// It return a Result<Value> as Value is serde_json::Value
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
    /// use girolle::JsonValue::Value;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let conf = Config::with_yaml_defaults("config.yml").unwrap();
    ///     let rpc_call = RpcClient::new(conf);
    ///     let service_name = "video";
    ///     let method_name = "hello";
    ///     let args = vec![Value::String("Toto".to_string())];
    ///     let result = rpc_call.send(service_name, method_name, args).expect("call");
    /// }
    pub fn send(
        &self,
        service_name: &'static str,
        method_name: &'static str,
        args: Vec<Value>,
    ) -> Result<Value> {
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
    /// use girolle::{RpcClient, Config};
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

/// # RpcTask
///
/// ## Description
///
/// This struct is used to create a RPC task. This task will be used to register
/// a function in the RpcService struct.
///
/// ## Example
///
/// ```rust,no_run
/// use girolle::{JsonValue::Value, RpcService, Result, RpcTask, Config};
///
/// fn hello(s: &[Value]) -> Result<Value> {
///    // Parse the incomming data
///    let n: String = serde_json::from_value(s[0].clone())?;
///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
///    Ok(hello_str)
/// }
///  
///
/// fn main() {
///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
///     let rpc_task = RpcTask::new("hello", hello);
///     services.register(rpc_task).start();
/// }
///
#[derive(Clone)]
pub struct RpcTask {
    name: &'static str,
    inner_function: NamekoFunction,
}
impl RpcTask {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcTask struct
    ///
    /// ## Arguments
    ///
    /// * `name` - The name of the function to call
    /// * `inner_function` - The function to call
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcTask struct
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use girolle::{JsonValue::Value, RpcService, Result, RpcTask, Config};
    ///
    /// fn hello(s: &[Value]) -> Result<Value> {
    ///    // Parse the incomming data
    ///    let n: String = serde_json::from_value(s[0].clone())?;
    ///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///     let rpc_task = RpcTask::new("hello", hello);
    ///     services.register(rpc_task).start();
    /// }
    ///
    pub fn new(name: &'static str, inner_function: NamekoFunction) -> Self {
        Self {
            name,
            inner_function,
        }
    }
}

/// # RpcService
///
/// ## Description
///
/// This struct is used to create a RPC service. This service will run
/// infinitly if started.
///
/// ## Example
///
/// ```rust, no_run
/// use girolle::{JsonValue::Value, RpcService, Result, Config};
///
/// fn hello(s: &[Value]) -> Result<Value> {
///     // Parse the incomming data
///     let n: String = serde_json::from_value(s[0].clone())?;
///     let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
///     Ok(hello_str)
/// }
///
/// fn main() {
///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
///     services.insert("hello", hello);
///     services.start();
/// }
pub struct RpcService {
    conf: Config,
    service_name: String,
    f: HashMap<String, NamekoFunction>,
}
impl RpcService {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcService struct
    ///
    /// ## Arguments
    ///
    /// * `conf` - The configuration as Config
    /// * `service_name` - The name of the service in the Nameko microservice
    ///
    /// ## Returns
    ///
    /// This function return a girolle::RpcService struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{RpcService, Config};
    ///
    /// fn main() {
    ///     let services: RpcService = RpcService::new(Config::default_config(),"video");
    /// }
    pub fn new(conf: Config, service_name: &'static str) -> Self {
        Self {
            conf,
            service_name: service_name.to_string(),
            f: HashMap::new(),
        }
    }
    /// # set_service_name
    ///
    /// ## Description
    ///
    /// This function set the service name of the RpcService struct
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{RpcService, Config};
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    services.set_service_name("other".to_string());
    /// }
    pub fn set_service_name(&mut self, service_name: String) {
        self.service_name = service_name;
    }
    /// # insert
    ///
    /// ## Description
    ///
    /// This function insert a function in the RpcService struct
    ///
    /// ## Arguments
    ///
    /// * `method_name` - The name of the function to call
    /// * `f` - The function to call
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{JsonValue::Value, RpcService, Result, Config};
    ///
    /// fn hello(s: &[Value]) -> Result<Value> {
    ///    // Parse the incomming data
    ///   let n: String = serde_json::from_value(s[0].clone())?;
    ///   let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///   Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///   let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///   services.insert("hello", hello);
    /// }
    pub fn insert(&mut self, method_name: &'static str, f: NamekoFunction) {
        let routing_key = format!("{}.{}", self.service_name, method_name);
        self.f.insert(routing_key.to_string(), f);
    }
    pub fn register(mut self, rpc_task: RpcTask) -> Self {
        self.insert(rpc_task.name, rpc_task.inner_function);
        self
    }
    /// # start
    ///
    /// ## Description
    ///
    /// This function start the RpcService struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{JsonValue::Value, RpcService, Result, Config};
    ///
    /// fn hello(s: &[Value]) -> Result<Value> {
    ///     // Parse the incomming data
    ///     let n: String = serde_json::from_value(s[0].clone())?;
    ///     let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    services.insert("hello", hello);
    /// }
    pub fn start(&self) -> lapin::Result<()> {
        if self.f.is_empty() {
            panic!("No function insert");
        }
        rpc_service(self.conf.clone(), &self.service_name, self.f.clone())
    }
    /// # get_routing_keys
    ///
    /// ## Description
    ///
    /// This function return the routing keys of the RpcService struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{JsonValue::Value, RpcService, Result, Config};
    ///
    /// fn hello(s: &[Value]) -> Result<Value> {
    ///
    ///    // Parse the incomming data
    ///    let n: String = serde_json::from_value(s[0].clone())?;
    ///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    services.insert("hello", hello);
    ///    let routing_keys = services.get_routing_keys();
    /// }
    pub fn get_routing_keys(&self) -> Vec<String> {
        self.f.keys().map(|x| x.to_string()).collect()
    }
        /// # get_config
    ///
    /// ## Description
    ///
    /// This function return the configuration of the RpcService struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{RpcService, Config};
    ///
    /// fn main() {
    ///    let services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    let conf = services.get_config();
    ///    println!("{}", conf.AMQP_URI());
    /// }
    pub fn get_config(&self) -> &Config {
        &self.conf
    }
    /// # set_config
    ///
    /// ## Description
    ///
    /// This function set the configuration of the RpcService struct
    ///
    /// ## Arguments
    ///
    /// * `config` - The configuration as Config
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::{RpcService, Config};
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    let conf = Config::default_config();
    ///    services.set_config(conf);
    /// }
    pub fn set_config(&mut self, config: Config) -> std::result::Result<(), std::string::String> {
        self.conf = config;
        Ok(())
    }
}

async fn publish(
    rpc_reply_channel: &Channel,
    payload: String,
    properties: BasicProperties,
    reply_to_id: String,
    rpc_exchange: &str,
) -> lapin::Result<Confirmation> {
    let confirm = rpc_reply_channel
        .basic_publish(
            rpc_exchange,
            &format!("{}", &reply_to_id),
            BasicPublishOptions::default(),
            payload.as_bytes(),
            properties,
        )
        .await?
        .await?;
    // The message was correctly published
    assert_eq!(confirm, Confirmation::NotRequested);
    Ok(confirm)
}

fn create_rpc_queue_reply_name(base_name: &str, identifier: &str) -> String {
    format!("{}-{}", base_name, identifier)
}

/// Execute the delivery
async fn execute_delivery(
    delivery: Delivery,
    id: &Uuid,
    fn_service: NamekoFunction,
    rpc_reply_channel: &Channel,
    rpc_queue_reply: &str,
    rpc_exchange: &str,
) {
    let opt_routing_key = delivery.routing_key.to_string();
    let incomming_data: Value = serde_json::from_slice(&delivery.data).expect("json");
    let args: Vec<Value> = incomming_data["args"]
        .as_array()
        .expect("args")
        .iter()
        .map(|x| x.clone())
        .collect();
    // Get the correlation_id and reply_to_id
    let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
    let reply_to_id = get_id(delivery.properties.reply_to(), "reply_to_id");
    let opt_headers = delivery.properties.headers().clone(); //need to clone to modify the headers
    let headers = insert_new_id_to_call_id(opt_headers.unwrap(), &opt_routing_key, &id.to_string());
    let properties = BasicProperties::default()
        .with_correlation_id(correlation_id.into())
        .with_content_type("application/json".into())
        .with_reply_to(rpc_queue_reply.into())
        .with_content_encoding("utf-8".into())
        .with_headers(headers)
        .with_delivery_mode(2)
        .with_priority(0);
    // Publish the response
    let payload: String = match fn_service(&args) {
        Ok(result) => json!(
            {
                "result": result,
                "error": null,
            }
        )
        .to_string(),
        Err(error) => {
            error!("Error: {}", &error);
            let err_str = error.to_string();
            let exc = HashMap::from([
                ("exc_path", "builtins.Exception"),
                ("exc_type", "Exception"),
                ("exc_args", "Error"),
                ("value", &err_str),
            ]);
            json!(
                {
                    "result": null,
                    "error": exc,
                }
            )
            .to_string()
        }
    };
    publish(
        &rpc_reply_channel,
        payload,
        properties,
        reply_to_id,
        rpc_exchange,
    )
    .await
    .expect("Error publishing");
}

/// # rpc_service
///
/// ## Description
///
/// This function start the RPC service
///
/// ## Arguments
///
/// * `conf` - The configuration as Config
/// * `service_name` - The name of the service in the Nameko microservice
/// * `f_task` - The function to call
#[tokio::main]
async fn rpc_service(
    conf: Config,
    service_name: &str,
    f_task: HashMap<String, NamekoFunction>,
) -> lapin::Result<()> {
    info!("Starting the service");
    // Define the queue name1
    let rpc_queue = format!("rpc-{}", service_name);
    // Add tracing
    tracing_subscriber::fmt::init();
    // Uuid of the service
    let id = Uuid::new_v4();
    // check list of function
    debug!("List of functions {:?}", f_task.keys());
    // Shadowing the f_task to be able to use it in the closure
    let f_task = Arc::new(f_task);

    // Create a channel for the service in Nameko this part is handle by
    // the RpcConsumer class
    let rpc_channel: Channel = create_service_queue(
        service_name,
        conf.AMQP_URI(),
        conf.heartbeat(),
        conf.prefetch_count(),
        &conf.rpc_exchange(),
    )
    .await?;
    let rpc_queue_reply = Arc::new(format!("rpc.reply-{}-{}", service_name, &id));
    // Create a channel for the response in Nameko this part is handle by
    // the ReplyConsumer class
    let rpc_reply_channel = Arc::new(
        create_message_queue(
            &rpc_queue_reply,
            &id,
            conf.AMQP_URI(),
            conf.heartbeat(),
            &conf.rpc_exchange(),
        )
        .await?,
    );
    let atomic_rpc_exchange = Arc::new(conf.rpc_exchange().to_string());
    // Start a consumer.
    let consumer = rpc_channel
        .basic_consume(
            &rpc_queue,
            "girolle_consumer_incomming",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    consumer.set_delegate(move |delivery: DeliveryResult| {
        let rpc_reply_channel_clone: Arc<Channel> = Arc::clone(&rpc_reply_channel);
        let f_task_clone: Arc<
            HashMap<String, fn(&[Value]) -> std::prelude::v1::Result<Value, JsonValue::Error>>,
        > = Arc::clone(&f_task);
        let rpc_queue_reply_clone: Arc<String> = Arc::clone(&rpc_queue_reply);
        let rpc_exchange_clone: Arc<String> = Arc::clone(&atomic_rpc_exchange);
        async move {
            info!("will consume");
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

            let opt_routing_key = delivery.routing_key.to_string();
            let fn_service: NamekoFunction = match f_task_clone.get(&opt_routing_key) {
                Some(fn_service) => *fn_service,
                None => {
                    warn!("fn_service: None");
                    return;
                }
            };
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
            execute_delivery(
                delivery,
                &id,
                fn_service,
                &rpc_reply_channel_clone,
                &rpc_queue_reply_clone,
                &rpc_exchange_clone,
            )
            .await;
        }
    });
    std::future::pending::<()>().await;
    Ok(())
}

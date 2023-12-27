/// # Girolle
///
/// ## Description
///
/// This crate is a Rust implementation of the Nameko RPC protocol.
/// It allows to create a RPC service or Rpc Call in Rust that can be called
/// from or to a Nameko microservice.
///
/// ## Description of the struct
///
/// The RPC service is a struct that contains a HashMap of functions.
/// The functions are called when the routing key is called.
/// The function must return a serializable value.
///
/// ## Usage
///
/// ### RPC service
///
/// The RPC service can be started with the function start() or start_tokio().
/// The function start() is blocking and start_tokio() is non-blocking.
/// The function start_tokio() is using tokio runtime.
/// The function start() is using async_global_executor.
///
/// ### RPC call
///
/// The RPC call is a struct that contains an identifier.
/// The identifier is used to identify the RPC call.
/// The function send() is used to send the payload to the Nameko microservice.
/// The function send() is **async**.
use futures_lite::stream::StreamExt;
use lapin::{
    message::Delivery,
    options::*,
    publisher_confirm::Confirmation,
    types::{AMQPValue, FieldArray, FieldTable},
    BasicProperties, Channel, Consumer,
};
pub use serde_json as JsonValue;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use JsonValue::json;
mod nameko_utils;
use nameko_utils::{get_id, insert_new_id_to_call_id};
pub type Result<T> = std::result::Result<T, serde_json::Error>;
pub type NamekoFunction = fn(Vec<&Value>) -> Result<Value>;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod queue;
use queue::{create_message_queue, create_service_queue, get_address};

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
pub struct RpcCall {
    identifier: Uuid,
}
impl RpcCall {
    /// # new
    ///
    /// ## Description
    ///
    /// This function create a new RpcCall struct
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::RpcCall;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let rpc_call = RpcCall::new();
    /// }
    pub fn new() -> Self {
        Self {
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
    /// use girolle::RpcCall;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rpc_call = RpcCall::new();
    ///     let identifier = rpc_call.get_identifier();
    /// }
    pub fn get_identifier(&self) -> String {
        self.identifier.to_string()
    }
    pub async fn call_async(
        &self,
        service_name: String,
        method_name: String,
        args: Vec<Value>,
    ) -> lapin::Result<Consumer> {
        let payload = Payload::new(args);
        let correlation_id = Uuid::new_v4().to_string();
        let routing_key = format!("{}.{}", service_name, method_name);
        let channel = create_service_queue(service_name.clone()).await?;
        let mut headers = BTreeMap::new();
        headers.insert(
            "nameko.AMQP_URI".into(),
            AMQPValue::LongString(get_address().into()),
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
        // The message was correctly published
        let reply_name = "rpc.listener".to_string();
        let rpc_queue_reply = format!("{}-{}", reply_name, &self.identifier);
        let reply_queue = create_message_queue(rpc_queue_reply.clone(), &self.identifier).await?;
        let consumer = reply_queue
            .basic_consume(
                &rpc_queue_reply,
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let confirm = channel
            .basic_publish(
                "nameko-rpc",
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
    pub async fn result(&self, consumer: &mut Consumer) -> Value {
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
    /// service name and the function to target. This function is async.
    ///
    /// ## Arguments
    ///
    /// * `service_name` - The name of the service in the Nameko microservice
    /// * `method_name` - The name of the function to call
    /// * `args` - The arguments of the function
    ///
    /// ## Example
    ///
    /// ```rust
    /// use std::vec;
    /// use girolle::{JsonValue::Value, RpcCall};
    /// use serde_json::Number;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let rpc_call = RpcCall::new();
    ///    let t:Number = serde_json::from_str("30").unwrap();
    ///    let payload  = vec![t.into()];
    ///    let result = rpc_call.send("video".to_string(), "fibonacci".to_string(), new_payload).await.unwrap();
    ///    println!("{:?}", result);
    /// }
    /// ```
    pub async fn send(
        &self,
        service_name: String,
        method_name: String,
        args: Vec<Value>,
    ) -> Result<Value> {
        let mut consumer = self
            .call_async(service_name, method_name, args)
            .await
            .expect("call");
        Ok(self.result(&mut consumer).await)
    }
}

/// # RpcService
///
/// ## Description
///
/// This struct is used to create a RPC service. This service will run
/// infinitly.
///
/// ## Example
///
/// ```rust
/// use girolle::{JsonValue::Value, RpcService};
///
/// fn hello(s: Vec<&Value>) -> Result<Value> {
///     // Parse the incomming data
///     let n: String = serde_json::from_value(s[0].clone())?;
///     let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
///     Ok(hello_str)
/// }
///
/// fn main() {
///     let mut services: RpcService = RpcService::new("video".to_string());
///     services.insert("hello".to_string(), hello);
///     services.start();
/// }
pub struct RpcService {
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
    /// * `service_name` - The name of the service in the Nameko microservice
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::RpcService;
    ///
    /// fn main() {
    ///     let services: RpcService = RpcService::new("video".to_string());
    /// }
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
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
    /// use girolle::RpcService;
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new("video".to_string());
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
    /// use girolle::{JsonValue::Value, RpcService};
    ///
    /// fn hello(s: Vec<&Value>) -> Result<Value> {
    ///    // Parse the incomming data
    ///   let n: String = serde_json::from_value(s[0].clone())?;
    ///   let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///   Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///   let mut services: RpcService = RpcService::new("video".to_string());
    ///   services.insert("hello".to_string(), hello);
    /// }
    pub fn insert(&mut self, method_name: String, f: NamekoFunction) {
        let routing_key = format!("{}.{}", self.service_name, method_name);
        self.f.insert(routing_key, f);
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
    /// use girolle::{JsonValue::Value, RpcService};
    ///
    /// fn hello(s: Vec<&Value>) -> Result<Value> {
    ///     // Parse the incomming data
    ///     let n: String = serde_json::from_value(s[0].clone())?;
    ///     let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new("video".to_string());
    ///    services.insert("hello".to_string(), hello);
    ///    services.start();
    /// }
    pub fn start(&self) -> lapin::Result<()> {
        if self.f.is_empty() {
            panic!("No function insert");
        }
        rpc_service(self.service_name.clone(), self.f.clone())
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
    /// use girolle::{JsonValue::Value, RpcService};
    ///
    /// fn hello(s: Vec<&Value>) -> Result<Value> {
    ///
    ///    // Parse the incomming data
    ///    let n: String = serde_json::from_value(s[0].clone())?;
    ///    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    ///    Ok(hello_str)
    /// }
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new("video".to_string());
    ///    services.insert("hello".to_string(), hello);
    ///    let routing_keys = services.get_routing_keys();
    /// }
    pub fn get_routing_keys(&self) -> Vec<String> {
        self.f.keys().map(|x| x.to_string()).collect()
    }
}

async fn publish(
    response_channel: &Channel,
    payload: String,
    properties: BasicProperties,
    reply_to_id: String,
) -> lapin::Result<Confirmation> {
    let confirm = response_channel
        .basic_publish(
            "nameko-rpc",
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

/// Execute the delivery
async fn execute_delivery(
    delivery: Delivery,
    id: &Uuid,
    fn_service: NamekoFunction,
    response_channel: &Channel,
    rpc_queue_reply: &String,
) {
    let opt_routing_key = delivery.routing_key.to_string();
    let incomming_data: Value = serde_json::from_slice(&delivery.data).expect("json");
    let args: Vec<&Value> = incomming_data["args"]
        .as_array()
        .expect("args")
        .iter()
        .collect();
    // Get the correlation_id and reply_to_id
    let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
    let reply_to_id = get_id(delivery.properties.reply_to(), "reply_to_id");
    let opt_headers = delivery.properties.headers();
    let headers = insert_new_id_to_call_id(
        opt_headers.as_ref().expect("headers").clone(),
        &opt_routing_key,
        &id.to_string(),
    );
    let properties = BasicProperties::default()
        .with_correlation_id(correlation_id.into())
        .with_content_type("application/json".into())
        .with_reply_to(rpc_queue_reply.clone().into())
        .with_headers(headers);
    // Publish the response
    let payload: String = match fn_service(args) {
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
    publish(&response_channel, payload, properties, reply_to_id)
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
/// * `service_name` - The name of the service in the Nameko microservice
/// * `f` - The function to call
fn rpc_service(service_name: String, f: HashMap<String, NamekoFunction>) -> lapin::Result<()> {
    // Define the queue name1
    let rpc_queue = format!("rpc-{}", service_name);
    // Add tracing
    tracing_subscriber::fmt::init();
    // Uuid of the service
    let id = Uuid::new_v4();
    // check list of function
    debug!("List of functions {:?}", f.keys());

    async_global_executor::block_on(async {
        let rpc_queue_reply = format!("rpc.reply-{}-{}", service_name, &id);
        let response_channel = create_message_queue(rpc_queue_reply.clone(), &id).await?;
        let incomming_channel = create_service_queue(service_name).await?;
        // Start a consumer.
        let mut consumer = incomming_channel
            .basic_consume(
                &rpc_queue,
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        info!("will consume");
        // Iterate over deliveries.
        while let Some(delivery) = consumer.next().await {
            let delivery = delivery.expect("error in consumer");
            let opt_routing_key = delivery.routing_key.to_string();
            let fn_service: NamekoFunction = match f.get(&opt_routing_key) {
                Some(fn_service) => *fn_service,
                None => {
                    warn!("fn_service: {} not found", &opt_routing_key);
                    return Ok(());
                }
            };
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
            execute_delivery(
                delivery,
                &id,
                fn_service,
                &response_channel,
                &rpc_queue_reply,
            )
            .await;
        }
        info!("GoodBye!");
        Ok(())
    })
}

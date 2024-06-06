use crate::config::Config;
use crate::nameko_utils::{get_id, insert_new_id_to_call_id};
use crate::queue::{create_message_channel, create_service_channel, get_connection};
use crate::rpc_task::RpcTask;
use crate::types::NamekoFunction;
use lapin::{
    message::{Delivery, DeliveryResult},
    options::*,
    publisher_confirm::Confirmation,
    types::FieldTable,
    BasicProperties, Channel,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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
/// use girolle::prelude::*;
///
/// fn hello(s: &[Value]) -> NamekoResult<Value> {
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
    /// use girolle::prelude::*;
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
    /// use girolle::prelude::*;
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
    /// use girolle::prelude::*;
    ///
    /// fn hello(s: &[Value]) -> NamekoResult<Value> {
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
    /// use girolle::prelude::*;
    ///
    /// fn hello(s: &[Value]) -> NamekoResult<Value> {
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
    /// use girolle::prelude::*;
    ///
    /// fn hello(s: &[Value]) -> NamekoResult<Value> {
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
    /// use girolle::prelude::*;
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
    /// use girolle::prelude::*;
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
    let headers = match opt_headers {
        Some(h) => insert_new_id_to_call_id(h, &opt_routing_key, &id.to_string()),
        None => {
            error!("No headers found in delivery properties");
            return;
        }
    };
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

struct SharedData {
    rpc_reply_channel: Channel,
    f_task: HashMap<String, NamekoFunction>,
    rpc_queue_reply: String,
    rpc_exchange: String,
    semaphore: Semaphore,
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
    let conn = get_connection(conf.AMQP_URI(), conf.heartbeat()).await?;
    // Create a channel for the service in Nameko this part is handle by
    // the RpcConsumer class
    let rpc_channel: Channel = create_service_channel(
        &conn,
        service_name,
        conf.prefetch_count(),
        &conf.rpc_exchange(),
    )
    .await?;
    let rpc_queue_reply: String = format!("rpc.reply-{}-{}", service_name, &id);
    // Start a consumer.
    let consumer = rpc_channel
        .basic_consume(
            &rpc_queue,
            "girolle_consumer_incomming",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    let shared_data: Arc<SharedData> = Arc::new(SharedData {
        rpc_reply_channel: create_message_channel(
            &conn,
            &rpc_queue_reply,
            &id,
            &conf.rpc_exchange(),
        )
        .await?,
        f_task,
        rpc_queue_reply: rpc_queue_reply,
        rpc_exchange: conf.rpc_exchange().to_string(),
        semaphore: Semaphore::new(conf.max_workers() as usize),
    });
    consumer.set_delegate(move |delivery: DeliveryResult| {
        let shared_data_clone = Arc::clone(&shared_data);
        async move {
            let _permit = shared_data_clone.semaphore.acquire().await;
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
            let fn_service: NamekoFunction = match shared_data_clone.f_task.get(&opt_routing_key) {
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
                &shared_data_clone.rpc_reply_channel,
                &shared_data_clone.rpc_queue_reply,
                &shared_data_clone.rpc_exchange,
            )
            .await;
        }
    });
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl_c signal");
    info!("Shutting down gracefully");
    Ok(())
}

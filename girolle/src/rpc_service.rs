use crate::config::Config;
use crate::nameko_utils::{get_id, insert_new_id_to_call_id};
use crate::queue::{create_service_channel, get_connection};
use crate::rpc_task::RpcTask;
use crate::types::GirolleError;
use lapin::{
    message::{Delivery, DeliveryResult},
    options::*,
    types::FieldTable,
    BasicProperties, Channel,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::Level;
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
/// use std::vec;
/// #[girolle]
/// fn hello(s: String) -> String {
///     format!("Hello, {}!, by Girolle", s)
/// }
///
/// fn main() {
///     let mut services: RpcService = RpcService::new(Config::default_config(),"video");
///     services.register(hello).start();
/// }
pub struct RpcService {
    conf: Config,
    service_name: String,
    f: HashMap<String, RpcTask>,
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
    /// use std::vec;
    ///
    /// #[girolle]
    /// fn hello(s: String) -> String {
    ///   format!("Hello, {}!, by Girolle", s)
    /// }
    ///
    /// fn main() {
    ///   let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///   services.register(hello);
    /// }
    pub fn register(mut self, fn_macro: fn() -> RpcTask) -> Self {
        let rpc_task = fn_macro();
        self.insert(&rpc_task.name, rpc_task);
        self
    }
    fn insert(&mut self, method_name: &'static str, f: RpcTask) {
        let routing_key = format!("{}.{}", self.service_name, method_name);
        self.f.insert(routing_key.to_string(), f);
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
    /// #[girolle]
    /// fn hello(s: String) -> String {
    ///   format!("Hello, {}!, by Girolle", s)
    /// }
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    services.register(hello).start();
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
    /// use std::vec;
    ///
    /// #[girolle]
    /// fn hello(s: String) -> String {
    ///     format!("Hello, {}!, by Girolle", s)
    /// }
    ///
    /// fn main() {
    ///    let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    ///    let routing_keys = services.register(hello).get_routing_keys();
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
    rpc_channel: &Channel,
    payload: String,
    properties: BasicProperties,
    reply_to_id: String,
    rpc_exchange: &str,
) -> lapin::Result<()> {
    let rpc_channel_clone = rpc_channel.clone();
    let rpc_exchange_clone = rpc_exchange.to_string();
    tokio::spawn(async move {
        rpc_channel_clone
            .basic_publish(
                &rpc_exchange_clone,
                &format!("{}", &reply_to_id),
                BasicPublishOptions::default(),
                payload.as_bytes(),
                properties,
            )
            .await
            .unwrap()
            .await
            .unwrap();
    });
    // The message was correctly published
    Ok(())
}

fn build_inputs_fn_service(
    service_args: Vec<&str>,
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
) -> Result<Vec<Value>, GirolleError> {
    if service_args.len() == args.len() {
        Ok(args)
    } else if service_args.len() == args.len() + kwargs.len() {
        let mut result: Vec<Value> = Vec::new();
        result.extend(args.clone());
        for i in args.len()..args.len() + kwargs.len() {
            result.push(match kwargs.get(service_args[i]) {
                Some(value) => value.clone(),
                None => return Err(GirolleError::ArgumentsError),
            });
        }
        Ok(result)
    } else {
        Err(GirolleError::ArgumentsError)
    }
}

#[test]
fn test_build_inputs_fn_service() {
    let service_args = vec!["a", "b", "c"];
    let args = vec![json!("value_a"), json!("value_b")];
    let mut kwargs = HashMap::new();
    kwargs.insert("c".to_string(), json!("value_c"));
    let result = build_inputs_fn_service(service_args, args, kwargs).unwrap();
    assert_eq!(
        result,
        vec![json!("value_a"), json!("value_b"), json!("value_c")]
    );
}

fn get_result_paylaod(result: Value) -> String {
    json!(
        {
            "result": result,
            "error": null,
        }
    )
    .to_string()
}

fn get_error_payload(error: GirolleError) -> String {
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
/// Execute the delivery
async fn execute_delivery(
    delivery: Delivery,
    id: &Uuid,
    rpc_task_struct: RpcTask,
    rpc_channel: &Channel,
    rpc_queue: &str,
    rpc_exchange: &str,
) {
    let opt_routing_key = delivery.routing_key.to_string();
    let incomming_data: Value =
        serde_json::from_slice(&delivery.data).expect("Can't deserialize incomming data");
    let args: Vec<Value> = incomming_data["args"]
        .as_array()
        .expect("args")
        .iter()
        .map(|x| x.clone())
        .collect();
    let kwargs: HashMap<String, Value> = incomming_data["kwargs"]
        .as_object()
        .expect("kargs")
        .into_iter()
        .map(|(k, v)| (k.clone(), v.clone()))
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
        .with_reply_to(rpc_queue.into())
        .with_content_encoding("utf-8".into())
        .with_headers(headers)
        .with_delivery_mode(2)
        .with_priority(0);
    // Publish the response
    let fn_service = rpc_task_struct.inner_function;
    let buildted_args = match build_inputs_fn_service(rpc_task_struct.args, args, kwargs) {
        Ok(result) => result,
        Err(error) => {
            publish(
                &rpc_channel,
                get_error_payload(error),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
            return;
        }
    };
    match fn_service(&buildted_args) {
        Ok(result) => {
            publish(
                &rpc_channel,
                get_result_paylaod(result),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
            return;
        }
        Err(error) => {
            publish(
                &rpc_channel,
                get_error_payload(error),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
            return;
        }
    };
}

struct SharedData {
    rpc_channel: Channel,
    f_task: HashMap<String, RpcTask>,
    rpc_queue: String,
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
    f_task: HashMap<String, RpcTask>,
) -> lapin::Result<()> {
    info!("Starting the service");
    // Define the queue name1
    let rpc_queue = format!("rpc-{}", service_name);
    // Add tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
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
        rpc_channel,
        f_task,
        rpc_queue,
        rpc_exchange: conf.rpc_exchange().to_string(),
        semaphore: Semaphore::new(conf.max_workers() as usize),
    });
    consumer.set_delegate(move |delivery: DeliveryResult| {
        let shared_data_clone = Arc::clone(&shared_data);
        async move {
            let _permit = shared_data_clone.semaphore.acquire().await;
            let delivery = match delivery {
                // Carries the delivery alongside its channel
                Ok(Some(delivery)) => delivery,
                // The consumer got canceled
                Ok(None) => return,
                // Carries the error and is always followed by Ok(None)
                Err(error) => {
                    error!("Failed to consume queue message {}", error);
                    return;
                }
            };

            let opt_routing_key = delivery.routing_key.to_string();
            let rpc_task_struct: RpcTask = match shared_data_clone.f_task.get(&opt_routing_key) {
                Some(rpc_task_struct) => rpc_task_struct.clone(),
                None => {
                    warn!("fn_service: None for routing_key: {}", &opt_routing_key);
                    return;
                }
            };
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
            execute_delivery(
                delivery,
                &id,
                rpc_task_struct,
                &shared_data_clone.rpc_channel,
                &shared_data_clone.rpc_queue,
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

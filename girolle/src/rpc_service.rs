use crate::config::Config;
use crate::error::{GirolleError, RemoteError};
use crate::nameko_utils::{get_id, insert_new_id_to_call_id};
use crate::queue::{create_service_channel, get_connection};
use crate::rpc_task::RpcTask;
use lapin::{
    message::{Delivery, DeliveryResult},
    options::*,
    types::FieldTable,
    BasicProperties, Channel,
};
use serde::Deserialize;
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
        // Need to clone f because it is behind a shared reference
        rpc_service(&self.conf, &self.service_name, self.f.clone())
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
    // Need to clone the rpc_channel to be able to use it in the tokio::spawn
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

fn push_values_to_result(
    service_args: &[&str],
    kwargs: &HashMap<String, Value>,
    start: usize,
    end: usize,
) -> Result<Vec<Value>, GirolleError> {
    let mut result: Vec<Value> = Vec::new();
    let error_message = "Key is missing in kwargs".to_string();
    for i in start..end {
        result.push(
            kwargs
                .get(service_args[i])
                .ok_or_else(|| GirolleError::IncorrectSignature(error_message.clone()))?
                .clone(),
        );
    }
    Ok(result)
}

fn build_inputs_fn_service(
    service_args: &Vec<&str>,
    data_delivery: DeliveryData,
) -> Result<Vec<Value>, GirolleError> {
    let args_size: usize = data_delivery.args.len();
    let kwargs_size: usize = data_delivery.kwargs.len();
    let service_args_size: usize = service_args.len();

    match (
        data_delivery.kwargs.is_empty(),
        data_delivery.args.is_empty(),
        service_args_size == args_size + kwargs_size,
    ) {
        (true, _, _) if service_args_size == args_size => Ok(data_delivery.args),
        (_, true, _) if service_args_size == kwargs_size => {
            push_values_to_result(service_args, &data_delivery.kwargs, 0, kwargs_size)
        }
        (_, _, true) => {
            let mut result = data_delivery.args;
            result.extend(push_values_to_result(
                service_args,
                &data_delivery.kwargs,
                args_size,
                args_size + kwargs_size,
            )?);
            Ok(result)
        }
        _ => Err(GirolleError::IncorrectSignature(format!(
            "takes {} positional arguments but {} were given",
            service_args_size,
            args_size + kwargs_size
        ))),
    }
}

#[test]
fn test_build_inputs_fn_service() {
    let service_args = vec!["a", "b", "c"];
    let data_delivery = DeliveryData {
        args: vec![
            Value::String("1".to_string()),
            Value::String("2".to_string()),
        ],
        kwargs: HashMap::from([("c".to_string(), Value::String("3".to_string()))]),
    };
    let result = build_inputs_fn_service(&service_args, data_delivery);
    assert_eq!(result.is_ok(), true);
    let result = result.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], Value::String("1".to_string()));
    assert_eq!(result[1], Value::String("2".to_string()));
    assert_eq!(result[2], Value::String("3".to_string()));
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

fn get_error_payload(error: RemoteError) -> String {
    json!(
        {
            "result": null,
            "error": error,
        }
    )
    .to_string()
}
#[derive(Debug, Deserialize)]
struct DeliveryData {
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
}
/// Execute the delivery
async fn compute_deliver(
    delivery: &Delivery,
    id: &Uuid,
    rpc_task_struct: &RpcTask,
    rpc_channel: &Channel,
    rpc_queue: &str,
    rpc_exchange: &str,
) {
    let opt_routing_key = delivery.routing_key.to_string();
    let incomming_data: DeliveryData =
        serde_json::from_slice(&delivery.data).expect("Can't deserialize incomming data");
    // Get the correlation_id and reply_to_id
    let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
    let reply_to_id = get_id(delivery.properties.reply_to(), "reply_to_id");
    //need to clone to modify the headers
    let opt_headers = delivery.properties.headers();
    let headers = match opt_headers {
        Some(h) => insert_new_id_to_call_id(h.clone(), &opt_routing_key, &id.to_string()),
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
    let buildted_args = match build_inputs_fn_service(&rpc_task_struct.args, incomming_data) {
        Ok(result) => result,
        Err(error) => {
            publish(
                &rpc_channel,
                get_error_payload(error.convert()),
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
                get_error_payload(error.convert()),
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
    service_name: String,
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
    conf: &Config,
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
        service_name: service_name.to_string(),
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
            let (incommig_service, incomming_method) = opt_routing_key.split_once('.').unwrap();
            match (shared_data_clone.f_task.get(&opt_routing_key),incommig_service==&shared_data_clone.service_name) {
                (Some(rpc_task_struct),_) => {
                    compute_deliver(
                        &delivery,
                        &id,
                        rpc_task_struct,
                        &shared_data_clone.rpc_channel,
                        &shared_data_clone.rpc_queue,
                        &shared_data_clone.rpc_exchange,
                    )
                    .await
                }
                (None,false) => {
                    error!("Service {} is not found", &incommig_service);
                }
                (None,true) => {
                    error!("Method {} is not found", &incomming_method);
                }
            };
            delivery.ack(BasicAckOptions::default()).await.expect("ack");
        }
    });
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl_c signal");
    info!("Shutting down gracefully");
    Ok(())
}

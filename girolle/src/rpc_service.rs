use crate::{
    config::Config,
    error::GirolleError,
    nameko_utils::{compute_deliver, delivery_to_message_properties, get_id, publish},
    payload::{Payload, PayloadResult},
    queue::{create_service_channel, get_connection},
    rpc_task::RpcTask,
};
use lapin::{message::DeliveryResult, options::*, types::FieldTable, Channel};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn, Level};
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
    ///
    /// let services: RpcService = RpcService::new(Config::default_config(),"video");
    /// ```
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
    ///
    /// let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    /// services.set_service_name("other".to_string());
    /// ```
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
        self.insert(rpc_task.name, rpc_task);
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
    pub fn start(&self) -> Result<(), GirolleError> {
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
    ///
    /// let services: RpcService = RpcService::new(Config::default_config(),"video");
    /// let conf = services.get_config();
    /// println!("{}", conf.AMQP_URI());
    /// ```
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
    ///
    /// let mut services: RpcService = RpcService::new(Config::default_config(),"video");
    /// let conf = Config::default_config();
    /// services.set_config(conf);
    /// ```
    pub fn set_config(&mut self, config: Config) -> std::result::Result<(), std::string::String> {
        self.conf = config;
        Ok(())
    }
}

struct SharedData {
    rpc_channel: Channel,
    f_task: HashMap<String, RpcTask>,
    rpc_exchange: String,
    service_name: String,
    semaphore: Semaphore,
    parent_calls_tracked: usize,
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
) -> Result<(), GirolleError> {
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
        conf.rpc_exchange(),
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
        rpc_exchange: conf.rpc_exchange().to_string(),
        service_name: service_name.to_string(),
        semaphore: Semaphore::new(conf.max_workers() as usize),
        parent_calls_tracked: conf.parent_calls_tracked() as usize,
    });
    consumer.set_delegate(move |delivery: DeliveryResult| {
        let shared_data = Arc::clone(&shared_data);
        async move {
            let _permit = shared_data.semaphore.acquire().await;
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
            let reply_to_id = get_id(delivery.properties.reply_to(), "reply_to_id");
            let properties = delivery_to_message_properties(&delivery, &id, &reply_to_id, shared_data.parent_calls_tracked)
                .expect("Error creating properties");
            let (incommig_service, incomming_method) =
                opt_routing_key.split_once('.').expect("Error splitting");
            match (
                shared_data.f_task.get(&opt_routing_key),
                incommig_service == shared_data.service_name,
            ) {
                (Some(rpc_task_struct), _) => {
                    let incomming_data: Payload = serde_json::from_slice(&delivery.data)
                        .expect("Can't deserialize incomming data");
                    compute_deliver(
                        incomming_data,
                        properties,
                        rpc_task_struct,
                        &shared_data.rpc_channel,
                        &shared_data.rpc_exchange,
                        reply_to_id,
                    )
                    .await
                }
                (None, false) => {
                    warn!("Service {} is not found", &incommig_service);
                    let payload = GirolleError::UnknownService(format!(
                        "Service {} is not found",
                        &incommig_service,
                    ))
                    .convert();
                    publish(
                        &shared_data.rpc_channel,
                        PayloadResult::from_error(payload),
                        properties,
                        reply_to_id,
                        &shared_data.rpc_exchange,
                    )
                    .await
                    .expect("Error publishing");
                }
                (None, true) => {
                    warn!("Method {} is not found", &incomming_method);
                    let payload = GirolleError::MethodNotFound(format!(
                        "Method {} is not found",
                        &incomming_method,
                    ))
                    .convert();
                    publish(
                        &shared_data.rpc_channel,
                        PayloadResult::from_error(payload),
                        properties,
                        reply_to_id,
                        &shared_data.rpc_exchange,
                    )
                    .await
                    .expect("Error publishing");
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

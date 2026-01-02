use crate::{
    config::Config,
    error::GirolleError,
    payload::{Payload, PayloadResult},
    queue::{create_service_channel, get_connection},
    types::GirolleResult,
};
use lapin::{
    options::*,
    types::{AMQPValue, FieldArray, FieldTable, LongString, ShortString},
    BasicProperties, Channel, Connection,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Condvar, Mutex},
};
use uuid::Uuid;

/// # RpcContext
///
/// ## Description
///
/// Context object passed to async service handlers containing:
/// - AMQP metadata (correlation_id, headers, reply_to)
/// - Capabilities for calling other services (RpcCaller)
/// - Capabilities for emitting events (EventDispatcher)
#[derive(Clone)]
pub struct RpcContext {
    /// The correlation ID from the inbound message
    pub correlation_id: String,
    /// The reply-to queue from the inbound message
    pub reply_to: String,
    /// The headers from the inbound message
    pub headers: FieldTable,
    /// The routing key from the inbound message
    pub routing_key: String,
    /// RPC caller for making calls to other services
    pub rpc: Arc<RpcCaller>,
    /// Event dispatcher for emitting events
    pub events: Arc<EventDispatcher>,
}

impl RpcContext {
    /// Create a new RpcContext from AMQP delivery metadata
    pub(crate) fn new(
        correlation_id: String,
        reply_to: String,
        headers: FieldTable,
        routing_key: String,
        config: Config,
        identifier: Uuid,
    ) -> Self {
        let rpc = Arc::new(RpcCaller::new(config.clone(), identifier));
        let events = Arc::new(EventDispatcher::new(config));

        Self {
            correlation_id,
            reply_to,
            headers,
            routing_key,
            rpc,
            events,
        }
    }

    /// Get the call_id_stack from the headers
    pub fn get_call_id_stack(&self) -> Option<Vec<String>> {
        self.headers
            .inner()
            .get("nameko.call_id_stack")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.as_slice()
                    .iter()
                    .filter_map(|v| {
                        if let AMQPValue::LongString(s) = v {
                            Some(String::from_utf8_lossy(s.as_bytes()).to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
    }
}

/// # RpcCaller
///
/// ## Description
///
/// Capability for making async RPC calls to other services from within a handler
pub struct RpcCaller {
    config: Config,
    identifier: Uuid,
    conn: Option<Arc<Connection>>,
    #[allow(dead_code)]
    reply_channel: Option<Arc<Channel>>,
    services: Arc<Mutex<HashMap<String, Arc<Channel>>>>,
    replies: Arc<Mutex<HashMap<String, PayloadResult>>>,
    not_empty: Arc<Condvar>,
}

impl RpcCaller {
    fn new(config: Config, identifier: Uuid) -> Self {
        Self {
            config,
            identifier,
            conn: None,
            reply_channel: None,
            services: Arc::new(Mutex::new(HashMap::new())),
            replies: Arc::new(Mutex::new(HashMap::new())),
            not_empty: Arc::new(Condvar::new()),
        }
    }

    /// Initialize the RPC caller (connect to AMQP)
    #[allow(dead_code)]
    async fn initialize(&mut self) -> GirolleResult<()> {
        let conn = get_connection(self.config.AMQP_URI(), self.config.heartbeat()).await?;
        let reply_queue_name = format!("rpc.listener-{}", self.identifier);
        
        let reply_channel = crate::queue::create_message_channel(
            &conn,
            &reply_queue_name,
            self.config.prefetch_count(),
            &self.identifier,
            self.config.rpc_exchange(),
        )
        .await?;

        self.conn = Some(Arc::new(conn));
        self.reply_channel = Some(Arc::new(reply_channel));
        Ok(())
    }

    /// Register a service for RPC calls
    pub async fn register_service(&self, service_name: &str) -> GirolleResult<()> {
        if let Some(conn) = &self.conn {
            let channel = create_service_channel(
                conn,
                service_name,
                self.config.prefetch_count(),
                self.config.rpc_exchange(),
            )
            .await?;
            
            let mut services = self.services.lock().unwrap();
            services.insert(service_name.to_string(), Arc::new(channel));
            Ok(())
        } else {
            Err(GirolleError::ArgumentsError(
                "RpcCaller not initialized".to_string(),
            ))
        }
    }

    /// Call another service asynchronously
    pub async fn call(
        &self,
        service_name: &str,
        method_name: &str,
        payload: Payload,
    ) -> GirolleResult<Value> {
        let services = self.services.lock().unwrap();
        let channel = services
            .get(service_name)
            .ok_or_else(|| {
                GirolleError::ServiceMissingError(format!("Service {} not registered", service_name))
            })?
            .clone();
        drop(services);

        let routing_key = format!("{}.{}", service_name, method_name);
        let correlation_id = Uuid::new_v4();

        let mut headers: BTreeMap<ShortString, AMQPValue> = BTreeMap::new();
        headers.insert(
            "nameko.AMQP_URI".into(),
            AMQPValue::LongString(self.config.AMQP_URI().into()),
        );
        headers.insert(
            "nameko.call_id_stack".into(),
            AMQPValue::FieldArray(FieldArray::from(vec![AMQPValue::LongString(
                LongString::from(self.identifier.to_string().as_bytes()),
            )])),
        );

        let properties = BasicProperties::default()
            .with_reply_to(self.identifier.to_string().into())
            .with_correlation_id(correlation_id.to_string().into())
            .with_content_type("application/json".into())
            .with_content_encoding("utf-8".into())
            .with_headers(FieldTable::from(headers))
            .with_priority(0);

        channel
            .basic_publish(
                self.config.rpc_exchange(),
                &routing_key,
                BasicPublishOptions {
                    mandatory: false,
                    immediate: false,
                },
                payload.to_string().as_bytes(),
                properties,
            )
            .await?
            .await?;

        // Wait for reply
        self.wait_for_reply(&correlation_id.to_string()).await
    }

    async fn wait_for_reply(&self, correlation_id: &str) -> GirolleResult<Value> {
        let mut replies = self.replies.lock().unwrap();
        let result_reply = loop {
            if let Some(value) = replies.get(correlation_id) {
                break value.clone();
            } else {
                replies = self.not_empty.wait(replies).unwrap();
            }
        };
        replies.remove(correlation_id);
        drop(replies);

        match result_reply.get_error() {
            Some(error) => Err(error.convert_to_girolle_error()),
            None => Ok(result_reply.get_result()),
        }
    }
}

/// # EventDispatcher
///
/// ## Description
///
/// Capability for dispatching events from within a handler
pub struct EventDispatcher {
    #[allow(dead_code)]
    config: Config,
}

impl EventDispatcher {
    fn new(config: Config) -> Self {
        Self { config }
    }

    /// Dispatch an event
    pub async fn dispatch(
        &self,
        _event_type: &str,
        _payload: Value,
    ) -> GirolleResult<()> {
        // TODO: Implement event dispatching
        // This would publish to an event exchange
        Ok(())
    }
}

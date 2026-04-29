//! The async dispatch loop that drives an [`super::RpcService`] once
//! it has been started.
//!
//! Sets up the broker connection, the RPC consumer, the in-service
//! [`RpcCallerCore`] / [`EventDispatcherCore`], one consumer per event
//! subscription, and the worker semaphore. Each inbound delivery is
//! routed to its registered handler (RPC) or subscriber (event), with
//! the per-delivery [`RpcContext`] stamped with the inbound headers.

use crate::amqp::channel::{
    create_event_consumer_channel, create_service_channel,
};
use crate::amqp::connection::get_connection;
use crate::amqp::headers::{delivery_to_message_properties, get_id};
use crate::amqp::publish::publish;
use crate::config::Config;
use crate::error::GirolleError;
use crate::payload::Payload;
use crate::protocol::PayloadResult;
use lapin::{message::DeliveryResult, options::*, types::FieldTable, BasicProperties, Channel};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn, Level};
use uuid::Uuid;

use super::caller::{RpcCaller, RpcCallerCore};
use super::context::RpcContext;
use super::dispatcher::{EventDispatcher, EventDispatcherCore};
use super::task::{EventHandler, RpcTask};

#[derive(Clone)]
pub(super) struct EventSubscription {
    pub(super) source_service: String,
    pub(super) event_type: String,
    pub(super) handler: EventHandler,
}

struct SharedData {
    rpc_channel: Channel,
    f_task: HashMap<String, RpcTask>,
    rpc_exchange: String,
    service_name: String,
    semaphore: Arc<Semaphore>,
    parent_calls_tracked: usize,
    rpc_caller: RpcCaller,
    event_dispatcher: EventDispatcher,
}

/// Run an RPC handler against an inbound delivery and publish either
/// the JSON result or a remote-error envelope on the reply queue.
async fn compute_deliver(
    incomming_data: Payload,
    properties: BasicProperties,
    rpc_task_struct: &RpcTask,
    rpc_channel: &Channel,
    rpc_exchange: &str,
    reply_to_id: String,
    ctx: RpcContext,
) {
    let handler = rpc_task_struct.handler.clone();
    match handler(ctx, incomming_data).await {
        Ok(result) => {
            publish(
                rpc_channel,
                PayloadResult::from_result_value(result),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
        }
        Err(error) => {
            publish(
                rpc_channel,
                PayloadResult::from_error(error.convert()),
                properties,
                reply_to_id,
                rpc_exchange,
            )
            .await
            .expect("Error publishing");
        }
    }
}

/// Entry point invoked by [`super::RpcService::start`]. Blocks the
/// calling thread on a tokio runtime until SIGINT.
#[tokio::main]
pub(super) async fn run(
    conf: &Config,
    service_name: &str,
    f_task: HashMap<String, RpcTask>,
    subscriptions: Vec<EventSubscription>,
) -> Result<(), GirolleError> {
    info!("Starting the service");
    let rpc_queue = format!("rpc-{}", service_name);
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    let id = Uuid::new_v4();
    debug!("List of functions {:?}", f_task.keys());
    debug!(
        "List of subscriptions {:?}",
        subscriptions
            .iter()
            .map(|s| format!("{}.events/{}", s.source_service, s.event_type))
            .collect::<Vec<_>>()
    );
    let conn = get_connection(conf.AMQP_URI(), conf.heartbeat()).await?;
    let rpc_channel: Channel = create_service_channel(
        &conn,
        service_name,
        conf.prefetch_count(),
        conf.rpc_exchange(),
    )
    .await?;
    let rpc_caller_core = RpcCallerCore::new(&conn, conf, id).await?;
    let event_dispatcher_core = EventDispatcherCore::new(&conn, conf, id).await?;
    let rpc_caller = RpcCaller::from_core(rpc_caller_core);
    let event_dispatcher = EventDispatcher::from_core(event_dispatcher_core);
    let semaphore = Arc::new(Semaphore::new(conf.max_workers() as usize));

    for sub in &subscriptions {
        let queue_name = format!(
            "evt-{}-{}--{}",
            sub.source_service, sub.event_type, service_name
        );
        let event_channel = create_event_consumer_channel(
            &conn,
            &queue_name,
            &sub.source_service,
            &sub.event_type,
            conf.prefetch_count(),
        )
        .await?;
        let consumer = event_channel
            .basic_consume(
                &queue_name,
                "girolle_event_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let handler = sub.handler.clone();
        let semaphore = Arc::clone(&semaphore);
        let rpc_caller = rpc_caller.clone();
        let event_dispatcher = event_dispatcher.clone();
        let service_name_owned = service_name.to_string();
        let event_type_owned = sub.event_type.clone();
        consumer.set_delegate(move |delivery: DeliveryResult| {
            let handler = handler.clone();
            let semaphore = Arc::clone(&semaphore);
            let rpc_caller = rpc_caller.clone();
            let event_dispatcher = event_dispatcher.clone();
            let service_name_owned = service_name_owned.clone();
            let event_type_owned = event_type_owned.clone();
            async move {
                let _permit = semaphore.acquire().await;
                let delivery = match delivery {
                    Ok(Some(delivery)) => delivery,
                    Ok(None) => return,
                    Err(error) => {
                        error!("Failed to consume event message {}", error);
                        return;
                    }
                };
                let payload: serde_json::Value = match serde_json::from_slice(&delivery.data) {
                    Ok(v) => v,
                    Err(e) => {
                        error!(error = %e, "event payload was not valid JSON; dropping");
                        let _ = delivery.ack(BasicAckOptions::default()).await;
                        return;
                    }
                };
                let inbound_headers = delivery
                    .properties
                    .headers()
                    .clone()
                    .unwrap_or_default();
                let rpc = rpc_caller.with_parent_headers(inbound_headers.clone());
                let events = event_dispatcher.with_parent_headers(inbound_headers.clone());
                let ctx = RpcContext {
                    service_name: service_name_owned,
                    method_name: event_type_owned,
                    correlation_id: String::new(),
                    reply_to: String::new(),
                    headers: inbound_headers,
                    rpc,
                    events,
                };
                if let Err(e) = handler(ctx, payload).await {
                    error!(error = %e, "event handler failed; acking and continuing");
                }
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
        });
    }

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
        semaphore,
        parent_calls_tracked: conf.parent_calls_tracked() as usize,
        rpc_caller,
        event_dispatcher,
    });
    consumer.set_delegate(move |delivery: DeliveryResult| {
        let shared_data = Arc::clone(&shared_data);
        async move {
            let _permit = shared_data.semaphore.acquire().await;
            let delivery = match delivery {
                Ok(Some(delivery)) => delivery,
                Ok(None) => return,
                Err(error) => {
                    error!("Failed to consume queue message {}", error);
                    return;
                }
            };

            let opt_routing_key = delivery.routing_key.to_string();
            let reply_to_id = get_id(delivery.properties.reply_to(), "reply_to_id");
            let correlation_id = get_id(delivery.properties.correlation_id(), "correlation_id");
            let inbound_headers = delivery
                .properties
                .headers()
                .clone()
                .unwrap_or_default();
            let properties = delivery_to_message_properties(
                &delivery,
                &id,
                &reply_to_id,
                shared_data.parent_calls_tracked,
            )
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
                    let rpc = shared_data
                        .rpc_caller
                        .with_parent_headers(inbound_headers.clone());
                    let events = shared_data
                        .event_dispatcher
                        .with_parent_headers(inbound_headers.clone());
                    let ctx = RpcContext {
                        service_name: incommig_service.to_string(),
                        method_name: incomming_method.to_string(),
                        correlation_id: correlation_id.clone(),
                        reply_to: reply_to_id.clone(),
                        headers: inbound_headers,
                        rpc,
                        events,
                    };
                    compute_deliver(
                        incomming_data,
                        properties,
                        rpc_task_struct,
                        &shared_data.rpc_channel,
                        &shared_data.rpc_exchange,
                        reply_to_id,
                        ctx,
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

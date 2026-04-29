use super::caller::RpcCaller;
use super::dispatcher::EventDispatcher;
use lapin::types::FieldTable;

/// Per-delivery context handed to every [`super::task::RpcHandler`] or
/// [`super::task::EventHandler`]. Holds inbound AMQP metadata and the
/// capability handles a handler can use to call other services or emit
/// events.
#[derive(Clone, Debug)]
pub struct RpcContext {
    pub service_name: String,
    pub method_name: String,
    pub correlation_id: String,
    pub reply_to: String,
    pub headers: FieldTable,
    pub rpc: RpcCaller,
    pub events: EventDispatcher,
}

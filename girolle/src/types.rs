use crate::error::GirolleError;
use crate::rpc_context::RpcContext;
use serde_json;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// # Result
///
/// ## Description
///
/// This type is used to return a `Result<Value>` in the RPC call
pub type GirolleResult<T> = std::result::Result<T, GirolleError>;

/// # NamekoFunction
///
/// ## Description
///
/// This type is used to define the function to call in the RPC service it
/// mainly simplify the code to manipulate a complexe type.
/// 
/// This is the legacy sync function type, kept for backward compatibility.
pub type NamekoFunction = fn(&[Value]) -> GirolleResult<Value>;

/// # AsyncNamekoFunction
///
/// ## Description
///
/// This type defines an async function handler that receives RpcContext and arguments.
/// The handler can:
/// - Access AMQP metadata (correlation_id, headers, reply_to)
/// - Call other services via ctx.rpc.call()
/// - Emit events via ctx.events.dispatch()
pub type AsyncNamekoFunction = Arc<
    dyn Fn(Arc<RpcContext>, Vec<Value>) -> Pin<Box<dyn Future<Output = GirolleResult<Value>> + Send>>
        + Send
        + Sync,
>;

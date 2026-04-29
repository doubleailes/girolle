//! The most commonly used types, functions and macros.

pub use crate::config::Config;
pub use crate::payload::Payload;
pub use crate::client::{RpcClient, RpcReply, RpcResult};
pub use crate::service::caller::RpcCaller;
pub use crate::service::context::RpcContext;
pub use crate::service::dispatcher::EventDispatcher;
pub use crate::service::task::{BoxFuture, EventHandler, RpcHandler, RpcTask};
pub use crate::service::RpcService;
pub use crate::error::GirolleResult;
pub use girolle_macro::girolle;
pub use serde_json;
pub use serde_json::{json, Value};

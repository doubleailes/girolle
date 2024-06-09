///! This module contains the most commonly used types, functions and macros.
pub use crate::config::Config;
pub use crate::rpc_client::{RpcCall, RpcClient};
pub use crate::rpc_service::RpcService;
pub use crate::rpc_task::RpcTask;
pub use crate::types::{NamekoFunction, NamekoResult};
pub use girolle_macro::girolle;
pub use serde_json;
pub use serde_json::{json, Value};

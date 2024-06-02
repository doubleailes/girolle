///! This module contains the most commonly used types, functions and macros.
pub use crate::config::Config;
pub use crate::rpc_service::{RpcService, RpcTask};
pub use crate::rpc_client::RpcClient;
pub use girolle_macro::girolle;
pub use serde_json;
pub use serde_json::{json, Value};
pub use crate::types::{NamekoResult, NamekoFunction};
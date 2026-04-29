//! Nameko wire protocol pieces shared by both client and service sides.
//!
//! The reply envelope (`PayloadResult`) and the `nameko.*` header keys live
//! here so neither the AMQP transport layer nor the higher-level handles
//! need to know about each other to agree on what's on the wire.

use crate::error::RemoteError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const NAMEKO_AMQP_URI: &str = "nameko.AMQP_URI";
pub(crate) const NAMEKO_CALL_ID_STACK: &str = "nameko.call_id_stack";

/// Wire envelope for an RPC reply: either a result or a remote error.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct PayloadResult {
    result: Value,
    error: Option<RemoteError>,
}

impl PayloadResult {
    pub(crate) fn from_result_value(result: Value) -> Self {
        Self {
            result,
            error: None,
        }
    }

    pub(crate) fn from_error(error: RemoteError) -> Self {
        Self {
            result: Value::Null,
            error: Some(error),
        }
    }

    pub(crate) fn get_error(&self) -> Option<RemoteError> {
        self.error.clone()
    }

    pub(crate) fn get_result(&self) -> Value {
        self.result.clone()
    }

    pub(crate) fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

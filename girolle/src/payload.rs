//! Public RPC request payload.
//!
//! `Payload` is the user-facing representation of an RPC's `args` and
//! `kwargs`. The reply envelope (`PayloadResult`) lives in the private
//! `protocol` module alongside the rest of the wire definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

/// # Payload
///
/// Positional and keyword arguments for an RPC call.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Payload {
    pub(crate) args: Vec<Value>,
    pub(crate) kwargs: HashMap<String, Value>,
}

impl Default for Payload {
    fn default() -> Self {
        Self::new()
    }
}

impl Payload {
    /// Empty payload — no args, no kwargs.
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// let p = Payload::new();
    /// ```
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            kwargs: HashMap::new(),
        }
    }

    /// Build a payload from a vector of pre-serialized positional
    /// arguments.
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// use serde_json::Value;
    /// let p = Payload::from_args_of_value(vec![Value::Number(1.into())]);
    /// ```
    pub fn from_args_of_value(args: Vec<Value>) -> Self {
        Self {
            args,
            kwargs: HashMap::new(),
        }
    }

    /// Build a payload from a map of pre-serialized keyword
    /// arguments.
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// use serde_json::Value;
    /// use std::collections::HashMap;
    /// let mut kwargs = HashMap::new();
    /// kwargs.insert("key".to_string(), Value::Number(1.into()));
    /// let p = Payload::from_kwargs_of_value(kwargs);
    /// ```
    pub fn from_kwargs_of_value(kwargs: HashMap<String, Value>) -> Self {
        Self {
            args: Vec::new(),
            kwargs,
        }
    }

    /// Append a positional argument.
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// let p = Payload::new().arg(1);
    /// ```
    pub fn arg<T: Serialize>(mut self, arg: T) -> Self {
        self.args
            .push(serde_json::to_value(arg).expect("Failed to serialize argument"));
        self
    }

    /// Set a keyword argument.
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// let p = Payload::new().kwarg("key", 1);
    /// ```
    pub fn kwarg<T: Serialize>(mut self, key: &str, value: T) -> Self {
        self.kwargs.insert(
            key.to_string(),
            serde_json::to_value(value).expect("Failed to serialize argument"),
        );
        self
    }

    /// Whether the payload carries no arguments at all.
    pub fn is_empty(&self) -> bool {
        self.args.is_empty() && self.kwargs.is_empty()
    }

    /// Borrow the positional arguments.
    pub fn args(&self) -> &[Value] {
        &self.args
    }

    /// Borrow the keyword arguments.
    pub fn kwargs(&self) -> &HashMap<String, Value> {
        &self.kwargs
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(self) {
            Ok(json_str) => write!(f, "{}", json_str),
            Err(e) => write!(f, "Error serializing to JSON: {}", e),
        }
    }
}

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::error::RemoteError;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Payload {
    pub (crate) args: Vec<Value>,
    pub (crate) kwargs: HashMap<String, Value>,
}
impl Payload {
    /// # new
    ///
    /// ## Description
    ///
    /// Create a new Payload empty struct
    ///
    /// ## Example
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
    /// # from_args_of_value
    ///
    /// ## Description
    ///
    /// Create a new Payload struct from a vector of Value
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// use serde_json::Value;
    /// let args: Vec<Value> = vec![Value::Number(serde_json::Number::from(1))];
    /// let p = Payload::from_args_of_value(args);
    /// ```
    pub fn from_args_of_value(args: Vec<Value>) -> Self {
        Self {
            args,
            kwargs: HashMap::new(),
        }
    }
    /// # from_kwargs_of_value
    ///
    /// ## Description
    ///
    /// Create a new Payload struct from a HashMap of String and Value
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// use serde_json::Value;
    /// use std::collections::HashMap;
    /// let mut kwargs: HashMap<String, Value> = HashMap::new();
    /// kwargs.insert("key".to_string(), Value::Number(serde_json::Number::from(1)));
    /// let p = Payload::from_kwargs_of_value(kwargs);
    /// ```
    pub fn from_kwargs_of_value(kwargs: HashMap<String, Value>) -> Self {
        Self {
            args: Vec::new(),
            kwargs,
        }
    }
    /// # arg
    ///
    /// ## Description
    ///
    /// push an argument to the args vector
    ///
    /// ## Example
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
    /// # kwarg
    ///
    /// ## Description
    ///
    /// push a key value pair to the kwargs HashMap
    ///
    /// ## Example
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
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    /// # is_empty
    ///
    /// ## Description
    ///
    /// Check if the Payload is empty
    ///
    /// ## Example
    ///
    /// ```rust
    /// use girolle::prelude::Payload;
    /// let p = Payload::new();
    /// assert_eq!(p.is_empty(), true);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.args.is_empty() && self.kwargs.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub (crate) struct PayloadResult{
    result: Value,
    error: Option<RemoteError>,
}
impl PayloadResult{
    pub fn get_error(&self) -> Option<RemoteError> {
        self.error.clone()
    }
    pub fn get_result(&self) -> Value {
        self.result.clone()
    }
    pub fn new(result: Value, error: Option<RemoteError>) -> Self {
        Self {
            result,
            error,
        }
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
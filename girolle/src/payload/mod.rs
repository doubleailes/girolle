use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Payload {
    args: Vec<Value>,
    kwargs: HashMap<String, Value>,
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
        self.args.push(serde_json::to_value(arg).unwrap());
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
        self.kwargs
            .insert(key.to_string(), serde_json::to_value(value).unwrap());
        self
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

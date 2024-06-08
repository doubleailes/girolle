use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    args: Value,
    kwargs: HashMap<String, String>,
}
impl Payload {
    pub fn new(args: Value) -> Self {
        Self {
            args,
            kwargs: HashMap::new(),
        }
    }
}

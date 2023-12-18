use girolle::{async_service, JsonValue::Value};
use std::collections::HashMap;

fn hello(s: Value) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s["args"][0]).into();
    hello_str
}

fn toto(s: Value) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, toto {}!, by Girolle", s["args"][0]).into();
    hello_str
}

fn main() {
    let mut services:HashMap<String, fn(Value) -> Value> = HashMap::new();
    services.insert("video.hello".to_string(), hello);
    services.insert("video.toto".to_string(), toto);
    async_service("video".to_string(), services).expect("service failed");
}

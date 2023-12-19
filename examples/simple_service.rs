use girolle::{async_service, JsonValue::Value};
use std::collections::HashMap;

fn hello(s: Value) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s["args"][0]).into();
    hello_str
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Value) -> Value {
    let n: u64 = serde_json::from_value(s["args"][0].clone()).unwrap();
    let result: Value = serde_json::to_value(fibonacci(n)).unwrap();
    result
}

fn main() {
    let mut services: HashMap<String, fn(Value) -> Value> = HashMap::new();
    services.insert("video.hello".to_string(), hello);
    services.insert("video.fibonacci".to_string(), fibonacci_reccursive);
    async_service("video".to_string(), services).expect("service failed");
}

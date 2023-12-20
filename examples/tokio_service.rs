use girolle::{tokio_rpc_service, JsonValue::Value};
use std::collections::HashMap;

fn hello(s: Vec<&Value>) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s[0].as_str().unwrap()).into();
    hello_str
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Vec<&Value>) -> Value {
    let n: u64 = s[0].as_u64().unwrap();
    let result: Value = serde_json::to_value(fibonacci(n)).unwrap();
    result
}

fn main() {
    let mut services: HashMap<String, fn(Vec<&Value>) -> Value> = HashMap::new();
    services.insert("video.hello".to_string(), hello);
    services.insert("video.fibonacci".to_string(), fibonacci_reccursive);
    tokio_rpc_service("video".to_string(), services);
}

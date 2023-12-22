use girolle::{JsonValue::Value, RpcService, Result};
use serde_json;

fn hello(s: Vec<&Value>) -> Result<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Vec<&Value>) -> Result<Value> {
    let n: u64 = serde_json::from_value(s[0].clone())?;
    let result: Value = serde_json::to_value(fibonacci(n))?;
    Ok(result)
}

fn main() {
    let mut services: RpcService = RpcService::new("video".to_string());
    services.insert("hello".to_string(), hello);
    services.insert("fibonacci".to_string(), fibonacci_reccursive);
    let _ = services.start();
}

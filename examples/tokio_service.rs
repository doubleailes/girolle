use girolle::{JsonValue::Value, RpcService, Result};

fn hello(s: Vec<&Value>) -> Result<Value> {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s[0].as_str().unwrap()).into();
    Ok(hello_str)
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Vec<&Value>) -> Result<Value> {
    let n: u64 = s[0].as_u64().unwrap();
    let result: Value = serde_json::to_value(fibonacci(n))?;
    Ok(result)
}

fn main() {
    let mut services: RpcService = RpcService::new("video".to_string());
    services.insert("hello".to_string(), hello);
    services.insert("fibonacci".to_string(), fibonacci_reccursive);
    let _ = services.start_tokio();
}

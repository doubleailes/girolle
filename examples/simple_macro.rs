use girolle_macro::trace_var;
use girolle::{JsonValue::Value, Result, RpcService};

fn hello(s: Vec<&Value>) -> Result<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

fn main() {
    println!("Hello, {}", hello(vec![&Value::String("Phil".to_string())]).unwrap());
}
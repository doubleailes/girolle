#![allow(unused_imports)]
use girolle::{JsonValue::Value, Result};
use girolle_macro::girolle;

#[girolle]
fn hello(s: String) -> String {
    let n: String = serde_json::from_value(data[0].clone()).unwrap();
    let out: Value = serde_json::to_value(format!("Hello, {}!", n)).unwrap();
    Ok(out)
}

fn main() {
    let binding = serde_json::to_value("World".to_string()).unwrap();
    let s = vec![&binding];
    let _ = hello(s);
}

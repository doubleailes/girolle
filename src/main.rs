use girolle::{async_service, JsonValue::Value};

fn hello(s: Value) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s["args"][0]).into();
    hello_str
}

fn main() {
    async_service("video", hello).expect("service failed");
}

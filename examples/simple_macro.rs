use girolle::{JsonValue::Value, Result, RpcService};
use girolle_macro::girolle;

fn hello(s: Vec<&Value>) -> Result<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

#[girolle(video)]
fn factorial(num: u64) -> u64 {
    match num {
        0 => 1,
        1 => 1,
        _ => factorial(num - 1) * num,
    }
}

fn main() {
    let y = Value::from(5);
    let t: Vec<&Value> = vec![&y];
    let toto: Value = factorial(t).unwrap();
    println!("toto: {:?}", toto);
}

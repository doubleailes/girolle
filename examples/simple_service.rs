use girolle::{Result, RpcService, RpcTask};
use girolle::prelude::*;
use serde_json_borrow;

fn hello(s: Vec<Value>) -> Result<Value> {
    // Parse the incomming data
    let n: String = serde_json_borrow::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

fn fibonacci(n: u64) -> u64 {
    let mut a = 0;
    let mut b = 1;

    match n {
        0 => b,
        _ => {
            for _ in 0..n {
                let c = a + b;
                a = b;
                b = c;
            }
            b
        }
    }
}

fn fibonacci_reccursive(s: Vec<Value>) -> Result<Value> {
    let n: u64 = serde_json_borrow::from_value(s[0].clone())?;
    let result: Value = serde_json_borrow::to_value(fibonacci(n))?;
    Ok(result)
}

fn main() {
    // Create the rpc service struct
    let mut services: RpcService = RpcService::new("video");
    // Add the method with the insert method
    services.insert("hello", hello);
    let rpc_task = RpcTask::new("fibonacci", fibonacci_reccursive);
    // Add the method with the register ans start the service because
    // register return the service
    let _ = services.register(rpc_task).start();
}

use std::vec;

use girolle::prelude::*;
use serde_json;

fn hello(s: &[Value]) -> NamekoResult<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
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

fn fibonacci_reccursive(s: &[Value]) -> NamekoResult<Value> {
    let n: u64 = serde_json::from_value(s[0].clone())?;
    let result: Value = serde_json::to_value(fibonacci(n))?;
    Ok(result)
}

fn main() {
    // Get the configuration from the staging/config.yml file
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    // Create the rpc service struct
    let services: RpcService = RpcService::new(conf, "video");
    // Add the method with the insert method
    let rpc_task = RpcTask::new("fibonacci", vec!["s"], fibonacci_reccursive);
    // Add the method with the register ans start the service because
    // register return the service
    let _ = services
        .register(rpc_task)
        .register(RpcTask::new("hello", vec!["s"], hello))
        .start();
}

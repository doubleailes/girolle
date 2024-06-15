use std::vec;

use girolle::prelude::*;
use serde_json;

fn hello_core(s: &[Value]) -> GirolleResult<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

fn hello() -> RpcTask {
    RpcTask::new("hello", vec!["s"], hello_core)
}

fn fast_fibonacci(n: u64) -> u64 {
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

fn fibonacci_reccursive(s: &[Value]) -> GirolleResult<Value> {
    let n: u64 = serde_json::from_value(s[0].clone())?;
    let result: Value = serde_json::to_value(fast_fibonacci(n))?;
    Ok(result)
}

fn fibonacci() -> RpcTask {
    RpcTask::new("fibonacci", vec!["n"], fibonacci_reccursive)
}

fn main() {
    // Get the configuration from the staging/config.yml file
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    // Create the rpc service struct
    let services: RpcService = RpcService::new(conf, "video");
    // Add the method with the register ans start the service because
    // register return the service
    let _ = services.register(hello).register(fibonacci).start();
}

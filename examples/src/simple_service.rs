use girolle::prelude::*;
use std::sync::Arc;

fn hello() -> RpcTask {
    RpcTask::new(
        "hello",
        Arc::new(|_ctx: RpcContext, payload: Payload| -> BoxFuture<GirolleResult<Value>> {
            Box::pin(async move {
                let n: String = serde_json::from_value(payload.args()[0].clone())?;
                Ok(serde_json::to_value(format!("Hello, {}!, by Girolle", n))?)
            })
        }),
    )
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

fn fibonacci() -> RpcTask {
    RpcTask::new(
        "fibonacci",
        Arc::new(|_ctx: RpcContext, payload: Payload| -> BoxFuture<GirolleResult<Value>> {
            Box::pin(async move {
                let n: u64 = serde_json::from_value(payload.args()[0].clone())?;
                Ok(serde_json::to_value(fast_fibonacci(n))?)
            })
        }),
    )
}

fn main() {
    // Get the configuration from the staging/config.yml file
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    // Create the rpc service struct
    let services: RpcService = RpcService::new(conf, "video");
    // Add the method with the register and start the service because
    // register return the service
    let _ = services.register(hello).register(fibonacci).start();
}

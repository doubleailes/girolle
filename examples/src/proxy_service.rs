//! Demonstrates an in-service RPC proxy: the `proxy.hello` method calls
//! `video.hello` from inside its handler via `ctx.rpc.call(...)`.
//!
//! Run alongside `simple_macro` (or any service exposing `video.hello`):
//!
//! ```sh
//! cargo run --example simple_macro
//! cargo run --example proxy_service
//! ```

use girolle::prelude::*;

#[girolle]
async fn proxy_hello(ctx: RpcContext, name: String) -> String {
    let result = ctx
        .rpc
        .call("video", "hello", Payload::new().arg(name))
        .await
        .expect("rpc call to video.hello failed");
    serde_json::from_value(result).expect("video.hello did not return a String")
}

fn main() {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    let _ = RpcService::new(conf, "proxy")
        .register(proxy_hello)
        .start();
}

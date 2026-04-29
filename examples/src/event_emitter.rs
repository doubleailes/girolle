//! Demonstrates emitting Nameko-compatible events from a service handler.
//!
//! `users.create_user` returns a greeting and emits a `user_created` event
//! on the `users.events` topic exchange.
//!
//! ```sh
//! cargo run --example event_emitter
//! ```
//!
//! On the consumer side any Nameko (or Girolle) subscriber bound to
//! `users.events` with routing key `user_created` will receive the event.

use girolle::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct UserCreated {
    name: String,
}

#[girolle]
async fn create_user(ctx: RpcContext, name: String) -> String {
    let payload = UserCreated { name: name.clone() };
    ctx.events
        .dispatch("users", "user_created", &payload)
        .await
        .expect("event dispatch failed");
    format!("User {} created", name)
}

fn main() {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    let _ = RpcService::new(conf, "users")
        .register(create_user)
        .start();
}

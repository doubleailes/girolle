//! Subscribes to `users.user_created` and prints incoming events.
//!
//! Pair this with any service that emits Nameko-compatible events on
//! `users.events` (e.g. the `event_emitter` example, or a Python Nameko
//! `users` service):
//!
//! ```sh
//! cargo run --example event_observer
//! ```
//!
//! Trigger an emit (e.g. via `cli_sender`):
//!
//! ```sh
//! cargo run --example cli_sender -- users create_user Girolle
//! ```
//!
//! The observer will print each event as it arrives and ack it.

use girolle::prelude::*;
use std::sync::Arc;

fn main() {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    let _ = RpcService::new(conf, "event-observer")
        .subscribe(
            "users",
            "user_created",
            Arc::new(|_ctx: RpcContext, payload: Value| -> BoxFuture<GirolleResult<()>> {
                Box::pin(async move {
                    println!("[users.user_created] {}", payload);
                    Ok(())
                })
            }),
        )
        .start();
}

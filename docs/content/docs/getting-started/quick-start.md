+++
title = "Quick Start"
description = "How to start with Girolle."
date = 2021-05-01T08:20:00+00:00
updated = 2021-05-01T08:20:00+00:00
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "How to start a project using girolle."
toc = true
top = false
+++

## Installation

```bash
cargo add girolle
```

## Configuration

There is two way to create a configuration. The first one is to use the `Config::with_yaml_defaults` function that will read a configuration from
a YAML file, [see example](https://github.com/doubleailes/girolle/blob/main/examples/config.yml). The second one is to create a configuration by hand.

### Create a configuration from a yaml file

The configuration is done by a yaml file. It should be compliant with a Nameko one.
The file should look like this:

```yaml
AMQP_URI: 'amqp://toto:super@$172.16.1.1:5672//'
rpc_exchange: 'nameko-rpc'
max_workers: 10
parent_calls_tracked: 10
```

In this example:
* The `AMQP_URI` is the connection string to the RabbitMQ server.
* The `rpc_exchange` is the exchange name for the rpc calls.
* The `max_workers` is the max number of workers that will be created to handle the rpc calls.
* The `parent_calls_tracked` is the number of parent calls that will be tracked by the service.

### Create a configuration by hand

```rust
let conf = Config::default();
conf.with_amqp_uri("amqp://toto:super@localhost:5672/")
    .with_rpc_exchange("nameko-rpc")
    .with_max_workers(10)
    .with_parent_calls_tracked(10);
```

#### Environment variables

The configuration support the expension of the environment variables with the
following syntax `${VAR_NAME}`. Like in this example:

```yaml
AMQP_URI: 'amqp://${RABBITMQ_USER}:${RABBITMQ_PASSWORD}@${RABBITMQ_HOST}:${RABBITMQ_PORT}/%2f'
rpc_exchange: 'nameko-rpc'
max_workers: 10
parent_calls_tracked: 10
```

## Create a simple service

```rust
use girolle::prelude::*;
use std::{thread, time};

#[girolle]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

#[girolle]
fn sub(a: i64, b: i64) -> i64 {
    a - b
}

#[girolle]
fn slip(n: u64) -> String {
    thread::sleep(time::Duration::from_secs(n));
    format!("Slept for {} seconds", n)
}

#[girolle]
fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    let _ = RpcService::new(conf, "video")
        .register(hello)
        .register(sub)
        .register(slip)
        .register(fibonacci)
        .start();
}
```

This service video should be able to respond to the following requests from
a nameko client:

```python
def send_simple_message(name: str) -> str:
    """
    send_simple_message send a message to the queue

    :param name: name of the person
    :type name: str
    """
    with rpc_proxy(CONFIG) as rpc:
        return rpc.video.hello(name)
```

For more details please check the nameko documentation about [standalone](https://nameko.readthedocs.io/en/v3.0.0-rc/api.html#module-nameko.standalone) and [rpc](https://nameko.readthedocs.io/en/v3.0.0-rc/rpc.html) services.

## Use the request context

Adding `ctx: RpcContext` as the first argument of a handler gives access to
the inbound delivery's metadata and two capability handles:

* `ctx.rpc.call(service, method, payload).await` — call any other service.
* `ctx.events.dispatch(source, event_type, &payload).await` — emit a
  Nameko-compatible event on the `<source>.events` topic exchange.

```rust
use girolle::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct UserCreated { name: String }

#[girolle]
async fn create_user(ctx: RpcContext, name: String) -> String {
    ctx.events
        .dispatch("users", "user_created", &UserCreated { name: name.clone() })
        .await
        .expect("event dispatch failed");
    format!("User {} created", name)
}
```

## Subscribe to events

A service can also subscribe to events emitted by other services. Use
`RpcService::subscribe(source, event_type, handler)`. Subscriptions can
coexist with `register(...)` on the same service, and a service consisting
only of subscriptions is also valid.

```rust
use girolle::prelude::*;
use std::sync::Arc;

fn main() {
    let conf = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
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
```

## Runnable examples

The `examples/` crate has a runnable example for each capability. Pick one,
adapt it, and `cargo run --example <name>` from the repository root:

| example | what it shows |
|---|---|
| `simple_macro` | basic `#[girolle]` service with sync handlers |
| `simple_service` | hand-rolled `RpcTask::new` with an async closure |
| `proxy_service` | `ctx.rpc.call` from inside a handler |
| `event_emitter` | `ctx.events.dispatch` from inside a handler |
| `event_observer` | `RpcService::subscribe` to consume events |
| `cli_sender` | generic CLI sender — `<service> <method> [arg…]` |
| `simple_sender` | `RpcClient` round-trip with sync + async calls |


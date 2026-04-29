# girolle

<div align="center">
<img src="girolle.png"></img>
</div>

## Description

A [nameko-rpc](https://github.com/nameko/nameko) like lib in rust. Check the To-Do
section to see limitation.

**Do not use in production!**

**Girolle** use **Nameko** architecture to send request and get response.

## Documentation

[User documentation](https://doubleailes.github.io/girolle/) and [Rust documentation](https://crates.io/crates/girolle)

## Installation

`cargo add girolle`

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

The configuration supports the expansion of the environment variables with the
following syntax `${VAR_NAME}`. Like in this example:

```yaml
AMQP_URI: 'amqp://${RABBITMQ_USER}:${RABBITMQ_PASSWORD}@${RABBITMQ_HOST}:${RABBITMQ_PORT}/%2f'
rpc_exchange: 'nameko-rpc'
max_workers: 10
parent_calls_tracked: 10
```

## How to use it

The core concept is to remove the pain of queue creation and replies by
mirroring the **Nameko** architecture with a `RpcService` or `RpcClient`, and
to use an abstract type `serde_json::Value` to carry serializable data.

Service handlers run as `async` futures and receive an [`RpcContext`] giving
access to inbound headers and two capability handles:

* `ctx.rpc.call(service, method, payload).await` — call any other service.
* `ctx.events.dispatch(source, event_type, &payload).await` — emit a
  Nameko-compatible event on `<source>.events`.

The `#[girolle]` attribute generates a handler from a sync or async function.
An optional first `ctx: RpcContext` argument is detected automatically.

If you'd rather skip the macro you can build an [`RpcTask`] by hand from an
[`RpcHandler`] closure:

```rust
use girolle::prelude::*;
use std::sync::Arc;

fn fibonacci() -> RpcTask {
    RpcTask::new(
        "fibonacci",
        Arc::new(|_ctx: RpcContext, payload: Payload| -> BoxFuture<GirolleResult<Value>> {
            Box::pin(async move {
                let n: u64 = serde_json::from_value(payload.args()[0].clone())?;
                Ok(serde_json::to_value(fibonacci_core(n))?)
            })
        }),
    )
}
```

## Examples

The [`examples/`](./examples) crate contains runnable services and senders:

| example | what it shows |
|---|---|
| [`simple_macro`](./examples/src/simple_macro.rs) | basic `#[girolle]` service with sync handlers |
| [`simple_service`](./examples/src/simple_service.rs) | hand-rolled `RpcTask::new` with an async closure |
| [`proxy_service`](./examples/src/proxy_service.rs) | `ctx.rpc.call` from inside a handler |
| [`event_emitter`](./examples/src/event_emitter.rs) | `ctx.events.dispatch` from inside a handler |
| [`event_observer`](./examples/src/event_observer.rs) | `RpcService::subscribe` to consume events |
| [`cli_sender`](./examples/src/cli_sender.rs) | generic CLI sender — `<service> <method> [arg…]` |
| [`simple_sender`](./examples/src/simple_sender.rs) | `RpcClient` round-trip with sync + async calls |

### Create a simple service

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

### Call another service from a handler

```rust
use girolle::prelude::*;

#[girolle]
async fn proxy_hello(ctx: RpcContext, name: String) -> String {
    let result = ctx
        .rpc
        .call("video", "hello", Payload::new().arg(name))
        .await
        .expect("rpc call failed");
    serde_json::from_value(result).expect("video.hello did not return a String")
}
```

### Emit an event from a handler

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

### Subscribe to events

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

A service can mix RPC methods (`register(...)`) and event subscriptions
(`subscribe(...)`) freely; either one alone is also valid.

### Create multiple calls to service of methods, sync and async

```rust
use girolle::prelude::Payload;
use girolle::{serde_json, Config, RpcClient, Value};
use std::time::Instant;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the configuration
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let service_name = "video";
    // Create the client from the configuration
    let mut rpc_client = RpcClient::new(conf);
    // Register the service
    rpc_client.register_service(service_name).await?;
    // Start the client and the consumers
    rpc_client.start().await?;
    // Build the payload
    let p = Payload::new().arg(30);
    // Send the request sync
    let new_result = rpc_client.send(service_name, "fibonacci", p)?;
    // Deserialize the result
    let fib_result: u64 = serde_json::from_value(new_result.get_value())?;
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    // Close the client
    rpc_client.unregister_service(service_name)?;
    rpc_client.close().await?;
    Ok(())
}
```

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client/server library.

## Supported features

* [x] standalone client (sync and async)
* [x] simple service with `#[girolle]`
  * [x] error handling
  * [x] tests
* [x] macro
  * [x] basic
  * [x] handle `return`
  * [x] handle recursive functions
  * [x] support `async fn` and an optional `ctx: RpcContext` first argument
* [x] in-service RPC client — `ctx.rpc.call(...)` from inside a handler
* [x] event publishing — `ctx.events.dispatch(...)` (Nameko `{source}.events`)
* [x] event subscriptions — `RpcService::subscribe(source, event_type, handler)`
* [ ] `#[girolle_event]` macro for ergonomic subscription handlers
* [ ] HTTP / web service entrypoint

### nameko-client

The Girolle client provides sync and async sends. There's also a
[`cli_sender`](./examples/src/cli_sender.rs) example for poking at any service
from a terminal without hand-editing a sender file.

### nameko-rpc

`RpcService` plus the `#[girolle]` macro is the core. Handlers are async,
receive an `RpcContext`, and can call other services from inside the handler
via `ctx.rpc.call(...)` — the in-service RPC core handles the reply queue,
correlation map, and `nameko.call_id_stack` propagation transparently.

### nameko-pubsub

Both sides are supported: handlers can publish events with
`ctx.events.dispatch(source, event_type, &payload)` and services can
subscribe to events with `RpcService::subscribe(source, event_type, handler)`.
Exchanges follow the Nameko `{source}.events` topic convention, so a Girolle
service can publish events that a Python Nameko service subscribes to (and
vice versa).

### nameko-web

The web service is not implemented. I'm not sure if i will implement it. I need to rework the client
to be make it 100% thread safe. It should be a commun subject with the proxy.

## Limitation

The current code as been tested with the nameko and girolle examples in this
repository.

|                    | nameko_test.py  | simple_sender.rs  |
|--------------------|-----------------|-------------------|
| nameko_service.py  |       x         |         x         |
| simple_macro       |       x         |         x         |

## Benchmark

### Simple message benchmark

|                    | nameko_test.py  | simple_sender.rs |
|--------------------|-----------------|------------------|
| nameko_service.py  |    15.587 s     |      11.532 s    |
| simple_macro.rs    |    15.654 s     |      8.078 s     |

### Client benchmark

Using hyperfine to test the client benchmark.

Girolle client ( with Girolle service )

```bash
hyperfine -N './target/release/examples/simple_sender'
Benchmark 1: ./target/release/examples/simple_sender
  Time (mean ± σ):      9.995 s ±  0.116 s    [User: 0.163 s, System: 0.197 s]
  Range (min … max):    9.778 s … 10.176 s    10 runs
```

Nameko client ( with Girolle service )

```bash
hyperfine -N --warmup 3 'python nameko_test.py'
Benchmark 1: python nameko_test.py
  Time (mean ± σ):     15.654 s ±  0.257 s    [User: 1.455 s, System: 0.407 s]
  Range (min … max):   15.202 s … 15.939 s    10 runs
```

### Service benchmark

Girolle service ( with Girolle client )

```bash
hyperfine -N './target/release/examples/simple_sender'
Benchmark 1: ./target/release/examples/simple_sender
  Time (mean ± σ):      9.995 s ±  0.116 s    [User: 0.163 s, System: 0.197 s]
  Range (min … max):    9.778 s … 10.176 s    10 runs
```

Nameko service running python 3.9.15 ( with Girolle client )

```bash
hyperfine -N --warmup 3 'target/release/examples/simple_sender'
Benchmark 1: target/release/examples/simple_sender
  Time (mean ± σ):     11.532 s ±  0.091 s    [User: 0.199 s, System: 0.213 s]
  Range (min … max):   11.396 s … 11.670 s    10 runs
```

Nameko service running python 3.9.15 ( with Nameko client )

```bash
hyperfine -N --warmup 3 'python nameko_test.py'
Benchmark 1: python nameko_test.py
  Time (mean ± σ):     15.587 s ±  0.325 s    [User: 1.443 s, System: 0.420 s]
  Range (min … max):   15.181 s … 16.034 s    10 runs
```

### Fibonacci benchmark

The benchmark use a [static set of random int](examples/data_set.json) to compute fibonacci.

|                    | nameko_fib_payload.py |
|--------------------|-----------------------|
| nameko_service.py  | 03 min 58.11   s      |
| simple_macro.rs    |       6.99   s        |

### Macro-overhead benchmark

The benchmark is done to test the overhead of the macro.

![benchmark](./girolle/benches/lines.svg)

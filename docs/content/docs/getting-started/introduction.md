+++
title = "Introduction"
description = "A nameko like lib in rust"
date = 2024-06-14T00:00:00+00:00
updated = 2024-06-14T00:00:00+00:00
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A nameko like lib in rust'
toc = true
top = false
+++

## Description

A [nameko-rpc](https://github.com/nameko/nameko) like lib in rust. Check the To-Do
section to see limitation.

**Do not use in production!**

**Girolle** use **Nameko** architecture to send request and get response.

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client/server library.

## How to use it

The core concept is to remove the pain of queue creation and replies by
mirroring the **Nameko** architecture, and to use an abstract type
`serde_json::Value` to carry serializable data.

Service handlers run as async futures. Each handler is invoked with an
`RpcContext` (inbound headers, correlation id, plus capability handles for
calling other services and emitting events) and a `Payload` (positional
args and kwargs). The `#[girolle]` macro hides the deserialization
boilerplate; if you'd rather build handlers by hand you can construct an
`RpcTask` directly from an `RpcHandler` closure.

### macro procedural

The lib offers a procedural macro `#[girolle]` to create a Task. It
accepts both sync and async functions, and detects an optional first
`ctx: RpcContext` argument:

```rust
use girolle::prelude::*;

#[girolle]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

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

The macro expands each annotated function into three items:

- A copy of the original function with the suffix `_core`.
- A wrapper that deserializes the inbound `Payload`, awaits `_core`, and
  serializes the result back to a JSON `Value`.
- A `fn() -> RpcTask` constructor used to register the handler.

The macro replaces recursive calls inside the function body to call
`_core` directly, which lets recursive functions like `fibonacci` work
unchanged. It does this naively (textual identifier match), so an
unrelated function with the same name as the annotated one would also be
rewritten — for example `#[girolle] fn sleep(n: u64) { thread::sleep(...) }`
would rewrite the `sleep` inside `thread::sleep` and break. Rename the
handler in that case.

### hand made deserialization and serialization

If you'd rather skip the macro, build an `RpcTask` directly from an
`RpcHandler` closure. The closure receives the `RpcContext` and a
`Payload`, and returns a `BoxFuture` resolving to the JSON result:

```rust
use girolle::prelude::*;
use std::sync::Arc;

fn fibonacci_core(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    fibonacci_core(n - 1) + fibonacci_core(n - 2)
}

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

This form gives you direct access to `RpcContext`, full control over
deserialization, and the ability to call other services or emit events
from the same closure.
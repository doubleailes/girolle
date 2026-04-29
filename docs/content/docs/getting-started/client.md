+++
title = "Service"
description = "A look about the client"
date = 2024-06-14T00:00:00+00:00
updated = 2024-06-14T00:00:00+00:00
draft = false
weight = 40
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A look about the client'
toc = true
top = false
+++

## Introduction

The client is a standalone client to be used against Nameko or Girolle
services. It is made to be fast and to handle a lot of requests. The client
runs on the `tokio` runtime and tries to mimic the Nameko client. It exposes
a sync `send()` and the async pair `call_async()` + `result()`.

For a runnable end-to-end example, see
[`examples/src/simple_sender.rs`](https://github.com/doubleailes/girolle/blob/main/examples/src/simple_sender.rs).

## CLI sender for ad-hoc testing

The [`examples/src/cli_sender.rs`](https://github.com/doubleailes/girolle/blob/main/examples/src/cli_sender.rs)
example wraps `RpcClient` in a one-shot CLI that takes a service, method,
and arguments off the command line. Useful when you want to poke at any
service without writing a sender file:

```bash
cargo run --example cli_sender -- video hello Girolle
cargo run --example cli_sender -- video sub 10 5
cargo run --example cli_sender -- proxy hello Girolle
```

Each positional `arg` is parsed as JSON when possible (so numbers,
booleans, arrays and objects work directly) and falls back to a plain
string otherwise.

## In-service RPC

If you only need to call another service from inside a handler, you don't
need an `RpcClient` at all — use `ctx.rpc.call(...)` on the `RpcContext`
that the running `RpcService` hands to each handler. See the
[service docs](../service/) for details and the
[`proxy_service`](https://github.com/doubleailes/girolle/blob/main/examples/src/proxy_service.rs)
example.
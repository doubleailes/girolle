+++
title = "Service"
description = "A look about the service"
date = 2024-06-14T00:00:00+00:00
updated = 2024-06-14T00:00:00+00:00
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A look about the service'
toc = true
top = false
+++

## Introduction

The service is the core of the lib. It is the place where the magic happens.
The core relies on `lapin` to handle the AMQP protocol and tokio for async +
spawn. Tokio spawns one delegate per vCPU to handle the request and the
response. As an optimisation for heavy load, the lib uses an async spawn to
publish the response so any free vCPU can handle the publish step.

## Handler shape

A service handler is registered on an `RpcService` either via the
`#[girolle]` macro or by handing the service an `RpcTask`. Internally each
handler is an async closure of the shape:

```rust
Arc<dyn Fn(RpcContext, Payload) -> BoxFuture<GirolleResult<Value>> + Send + Sync>
```

The handler receives the per-delivery `RpcContext` and a `Payload` carrying
positional args and kwargs, and returns a future resolving to the JSON
result. Errors propagate back to the caller as `GirolleError` and are
serialized into a Nameko-compatible `RemoteError`.

## RpcContext

`RpcContext` carries the inbound delivery's metadata and two capability
handles:

| field | meaning |
|---|---|
| `service_name` | the inbound routing key's service component |
| `method_name` | the inbound routing key's method component |
| `correlation_id` | the AMQP correlation id of the inbound delivery |
| `reply_to` | the requester's reply queue routing key |
| `headers` | the raw inbound headers (incl. `nameko.call_id_stack`) |
| `rpc` | an `RpcCaller` for outbound RPC, stamped with the inbound headers |
| `events` | an `EventDispatcher` for emitting events, stamped with the inbound headers |

Both `rpc` and `events` propagate the inbound `nameko.call_id_stack` on
outbound messages, so distributed traces stay consistent across hops.

## In-service RPC

`ctx.rpc.call(service, method, payload).await` issues an outbound RPC and
awaits the reply. Internally, the `RpcService` stands up a single shared
reply queue, a `DashMap<correlation_id, oneshot::Sender>` correlation map,
and a publish channel — all wired up automatically when the service starts.

## Events

Service handlers can publish Nameko-compatible events with
`ctx.events.dispatch(source_service, event_type, &payload).await`. The
`{source_service}.events` topic exchange is declared lazily on first emit
per source, and messages are published as durable JSON.

A service can also subscribe to events emitted by other services with
`RpcService::subscribe(source_service, event_type, handler)`. The handler
follows the same `(RpcContext, Value) -> Future` shape as RPC handlers.
Subscriptions and `register(...)` calls can coexist on the same service;
either alone is also valid.
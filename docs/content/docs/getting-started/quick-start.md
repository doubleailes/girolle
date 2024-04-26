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
lead = "How to start a Girolle project."
toc = true
top = false
+++

## Installation

```bash
cargo add girolle
```

## Setup

You need to set this environement variables.

- **RABBITMQ_USER**: The RabbitMQ user
- **RABBITMQ_PASSWORD**: The RabbitMQ password
- **RABBITMQ_HOST**: THe rabbitMQ host adress
- Optional: **RABBITMQ_PORT**: The RabbitMQ port (default: 5672)

### Create a simple service

```rust
use girolle::prelude::*;

#[girolle]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// Because the function is recursive, it need to be wrap in a function
#[girolle]
fn fib_wrap(n: u64) -> u64 {
    fibonacci(n)
}

fn main() {
    // Crerate a task "hello"
    let rpc_task = RpcTask::new("hello", hello);
    // Create a task "fibonacci"
    let rpc_task_fib = RpcTask::new("fibonacci", fib_wrap);
    // Create a service "video" and register the tasks
    let _ = RpcService::new("video")
        .register(rpc_task)
        .register(rpc_task_fib)
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

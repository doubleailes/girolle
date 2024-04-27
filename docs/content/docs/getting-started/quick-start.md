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

There is two way to create a configuration. The first one is to use the `Config::with_yaml_defaults` function that will create a configuration from a yaml file. The second one is to create a configuration by hand.

### Create a configuration by hand

```rust
let conf = Config::default_config();
conf.with_amqp_uri("amqp://toto:super@localhost:5672/")
    .with_rpc_exchange("nameko-rpc")
    .with_max_workers(10)
    .with_parent_calls_tracked(10);
```

### Create a configuration from a yaml file

The configuration is done by a yaml file. It should compliant with a Nameko one.
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
* The `max_workers` is the number of workers that will be created to handle the rpc calls.
* The `parent_calls_tracked` is the number of parent calls that will be tracked by the service.

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

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

#[girolle]
fn temporary_sleep(n: u64) -> String {
    thread::sleep(time::Duration::from_secs(n));
    format!("Slept for {} seconds", n)
}

#[girolle]
fn fib_warp(n: u64) -> u64 {
    fibonacci(n)
}

fn main() {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml").unwrap();
    let rpc_task = RpcTask::new("hello", hello);
    let rpc_task_fib = RpcTask::new("fibonacci", fib_warp);
    let rpc_task_sleep = RpcTask::new("sleep", temporary_sleep);
    let _ = RpcService::new(conf, "video")
        .register(rpc_task)
        .register(rpc_task_fib)
        .register(rpc_task_sleep)
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



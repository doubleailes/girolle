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

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client/server library.

## Configuration

There are two ways to create a configuration. The first one is to use the
`Config::with_yaml_defaults` function that will create a configuration from
a YAML file. The second one is to create a configuration by hand.

### Create a configuration by hand

```rust
let conf = Config::default_config();
conf.with_amqp_uri("amqp://toto:super@localhost:5672/")
    .with_rpc_exchange("nameko-rpc")
    .with_max_workers(10)
    .with_parent_calls_tracked(10);
```

### Create a configuration from a yaml file

The configuration is specified in a YAML file. It should be compliant with a Nameko one.
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

The configuration supports the expansion of the environment variables with the
following syntax `${VAR_NAME}`. Like in this example:

```yaml
AMQP_URI: 'amqp://${RABBITMQ_USER}:${RABBITMQ_PASSWORD}@${RABBITMQ_HOST}:${RABBITMQ_PORT}/%2f'
rpc_exchange: 'nameko-rpc'
max_workers: 10
parent_calls_tracked: 10
```

## How to use it

The core concept is to remove the pain of the queue creation and reply by
mokcing the **Nameko** architecture with a `RpcService` or `RpcClient`, and to
use an abstract type `serde_json::Value` to manipulate a serializable data.

if you do not use the macro `#[girolle]` you need to create a function that
extract the data from the a `&[Value]` like this:

```rust
fn fibonacci_reccursive(s: &[Value]) -> Result<Value> {
    let n: u64 = serde_json::from_value(s[0].clone())?;
    let result: Value = serde_json::to_value(fibonacci(n))?;
    Ok(result)
}
```

## Exemple

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
    // Create the configuration
    let conf = Config::default_config();
    // Create the rpc task
    let rpc_task = RpcTask::new("hello", hello);
    // Create another rpc task
    let rpc_task_fib = RpcTask::new("fibonacci", fib_wrap);
    // Create and start the service
    let _ = RpcService::new(conf,"video")
        .register(rpc_task)
        .register(rpc_task_fib)
        .start();
}
```

### Create multiple calls to service of methods, sync and async

```rust
use girolle::prelude::*;
use std::vec;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Source the configuration from a yaml file and generate configuration
    let conf = Config::with_yaml_defaults("staging/config.yml")?;
    let video_name = "video";
    // Create the rpc call struct
    let rpc_client = RpcClient::new(conf);
    // Create a future result
    let future_result =
        rpc_client.call_async(video_name, "hello", vec![Value::String("Toto".to_string())]);
    // Send a message during the previous async process
    let result = rpc_client.send(
        video_name,
        "hello",
        vec![Value::String("Girolle".to_string())],
    )?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(
        result,
        Value::String("Hello, Girolle!, by nameko".to_string())
    );
    // Wait for the future result
    let consumer = future_result.await?;
    // wait for it
    let two_sec = time::Duration::from_secs(2);
    thread::sleep(two_sec);
    // Print the result
    let async_result = rpc_client.result(consumer).await;
    println!("{:?}", async_result);
    assert_eq!(
        async_result,
        Value::String("Hello, Toto!, by nameko".to_string())
    );
    let mut consummers: Vec<_> = Vec::new();
    for n in 1..101 {
        consummers.push(rpc_client.call_async(
            video_name,
            "hello",
            vec![Value::String(n.to_string())],
        ));
    }
    // wait for it
    thread::sleep(two_sec);
    for con in consummers {
        let async_result = rpc_client.result(con.await?).await;
        println!("{}", async_result.as_str().unwrap());
    }
    Ok(())
}
```

## To-Do

- [x] Handle the error
- [x] write test
- [x] create a proxy service in rust to interact with an other service
nameko-rpc
- [ ] Add macro to simplify the creation of a service
  - [x] Add basic macro
  - [ ] fix macro to handle `return`
  - [ ] fix macro to handle recursive function
- [ ] listen to a pub/sub queue

## Limitation

The current code as been tested with the nameko and girolle examples in this
repository.

|                 | nameko_test.py  | simple_senders    |
|-----------------|-----------------|-------------------|
| simple_service  |       x         |         x         |
| nameko_service  |       x         |         x         |
| simple_macro    |       x         |         x         |

## Benchmark

The benchmark is done to test the overhead of the macro.

![benchmark](./girolle/benches/lines.svg)

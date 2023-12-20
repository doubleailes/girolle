# girolle

![girolle](./images/girolle.png)

## Description

A [nameko-rpc](https://github.com/nameko/nameko) like lib in rust.

Do not use in production!

## Documentation

[User documentation](https://doubleailes.github.io/girolle/) and [Rust documentation](https://crates.io/crates/girolle)

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client library.

## Setup

You need to set this environement variables.

- **RABBITMQ_USER**: The RabbitMQ user
- **RABBITMQ_PASSWORD**: The RabbitMQ password
- **RABBITMQ_HOST**: THe rabbitMQ host adress
- Optional: **RABBITMQ_PORT**: The RabbitMQ port (default: 5672)

## How to use it

The core concept is to remove the pain of the queue creation and reply, and to
use an abstract type `serde_json::Value` to manipulate a serializable data.

It needed to extract the data from the a `Vec<&Value>`.

## Exemple

```rust
use girolle::{JsonValue::Value, RpcService};

fn hello(s: Vec<&Value>) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s[0].as_str().unwrap()).into();
    hello_str
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Vec<&Value>) -> Value {
    let n: u64 = s[0].as_u64().unwrap();
    let result: Value = serde_json::to_value(fibonacci(n)).unwrap();
    result
}

fn main() {
    let mut services: RpcService = RpcService::new("video".to_string());
    services.insert("hello".to_string(), hello);
    services.insert("fibonacci".to_string(), fibonacci_reccursive);
    let _ = services.start();
}
```

## TODO

- [ ] Handle the error
- [ ] write test
- [ ] create a proxy service in rust to interact with an other service
nameko-rpc
- [ ] listen to a pub/sub queue
- [ ] factorize between the simple rpc and tokio
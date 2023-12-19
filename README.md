# girolle

## Description

A nameko-rpc like lib in rust.

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client library.

## Setup

You need to set this environement variables.

- **RABBITMQ_USER**: The RabbitMQ user
- **RABBITMQ_PASSWORD**: The RabbitMQ password
- **RABBITMQ_HOST**: THe rabbitMQ host adress

## How to use it

The core concept is to remove the pain of the queue creation and reply, and to
use an abstract type `serde_json::Value` to manipulate a serializable data.

It needed to extract the data from the a `Vec<&Value>`.

## Exemple

```rust
use girolle::{async_service, JsonValue::Value};
use std::collections::HashMap;

fn hello(s: Value) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s["args"][0]).into();
    hello_str
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Value) -> Value {
    let n: u64 = serde_json::from_value(s["args"][0].clone()).unwrap();
    let result: Value = serde_json::to_value(fibonacci(n)).unwrap();
    result
}

fn main() {
    let mut services: HashMap<String, fn(Value) -> Value> = HashMap::new();
    services.insert("video.hello".to_string(), hello);
    services.insert("video.fibonacci".to_string(), fibonacci_reccursive);
    async_service("video".to_string(), services).expect("service failed");
}
```
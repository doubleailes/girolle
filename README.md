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

## Exemple

```rust
use girolle::{async_service, JsonValue::Value};

fn hello(s: Value) -> Value {
    // Parse the incomming data
    let hello_str: Value = format!("Hello, {}!, by Girolle", s["args"][0]).into();
    hello_str
}

fn main() {
    async_service("video", hello).expect("service failed");
}
```
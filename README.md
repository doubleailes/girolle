# girolle

![girolle](./images/girolle.png)

## Description

A [nameko-rpc](https://github.com/nameko/nameko) like lib in rust. Check the TO-Do
section to see limitation.

Do not use in production!

**Girolle** use Nameko architecture to send request and get response.

## Documentation

[User documentation](https://doubleailes.github.io/girolle/) and [Rust documentation](https://crates.io/crates/girolle)


## Installation

`cargo add girolle`

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client/server library.

## Setup

You need to set this environement variables.

- **RABBITMQ_USER**: The RabbitMQ user
- **RABBITMQ_PASSWORD**: The RabbitMQ password
- **RABBITMQ_HOST**: THe rabbitMQ host adress
- Optional: **RABBITMQ_PORT**: The RabbitMQ port (default: 5672)

## How to use it

The core concept is to remove the pain of the queue creation and reply, and to
use an abstract type `serde_json::Value` to manipulate a serializable data.

It needed to extract the data from the a `Vec<&Value>` like this

```rust
fn fibonacci_reccursive(s: Vec<&Value>) -> Result<Value> {
    let n: u64 = serde_json::from_value(s[0].clone())?;
    let result: Value = serde_json::to_value(fibonacci(n))?;
    Ok(result)
}
```

## Exemple

### Create a simple service

```rust
use girolle::{JsonValue::Value, Result, RpcService};
use serde_json;

fn hello(s: Vec<&Value>) -> Result<Value> {
    // Parse the incomming data
    let n: String = serde_json::from_value(s[0].clone())?;
    let hello_str: Value = format!("Hello, {}!, by Girolle", n).into();
    Ok(hello_str)
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn fibonacci_reccursive(s: Vec<&Value>) -> Result<Value> {
    let n: u64 = serde_json::from_value(s[0].clone())?;
    let result: Value = serde_json::to_value(fibonacci(n))?;
    Ok(result)
}

fn main() {
    // Create the rpc service struct
    let mut services: RpcService = RpcService::new("video".to_string());
    // Declare the services by adding them to the service
    services.insert("hello".to_string(), hello);
    services.insert("fibonacci".to_string(), fibonacci_reccursive);
    // Start the services
    let _ = services.start();
}
```

### Create multiple calls to service of methods, sync and async

```rust
use std::vec;
use std::{thread, time};
use girolle::{JsonValue::Value, RpcClient};
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the rpc call struct
    let rpc_client = RpcClient::new();
    // Transform the number into a JsonValue
    let t: serde_json::Number = serde_json::from_str("30").unwrap();
    // Create the payload
    let new_payload = vec![t.into()];
    // Send the payload
    let new_result = rpc_client
        .send("video".to_string(), "fibonacci".to_string(), new_payload)
        .await?;
    let fib_result: u64 = serde_json::from_value(new_result).unwrap();
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    // Create a future result
    let future_result = rpc_client.call_async(
        "video".to_string(),
        "hello".to_string(),
        vec![Value::String("Toto".to_string())],
    );
    // Send a message
    let result = rpc_client
        .send(
            "video".to_string(),
            "hello".to_string(),
            vec![Value::String("Girolle".to_string())],
        )
        .await?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(
        result,
        Value::String("Hello, Girolle!, by nameko".to_string())
    );
    // Wait for the future result
    let mut consumer = future_result.await?;
    // wait for it

    let two_sec = time::Duration::from_secs(20);
    thread::sleep(two_sec);
    // Print the result
    let async_result = rpc_client.result(&mut consumer).await;
    println!("{:?}", async_result);
    assert_eq!(
        async_result,
        Value::String("Hello, Toto!, by nameko".to_string())
    );
    Ok(())
}
```

## TODO

- [x] Handle the error
- [x] write test
- [x] create a proxy service in rust to interact with an other service
nameko-rpc
- [ ] supporting SSL
- [ ] listen to a pub/sub queue

## Limitation

The current code as been tested with the nameko and girolle examples in this
repository.

|                 | nameko_test.py  | simple_senders    |
|-----------------|-----------------|-------------------|
| simple_service  |       x         |         x         |
| nameko_service  |       x         |         x         |
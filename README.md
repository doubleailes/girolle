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

## Configuration

There is two way to create a configuration. The first one is to use the `Config::with_yaml_defaults` function that will read a configuration from
a YAML file, [see example](https://github.com/doubleailes/girolle/blob/main/examples/config.yml). The second one is to create a configuration by hand.

### Create a configuration from a yaml file

The configuration is done by a yaml file. It should be compliant with a Nameko one.
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
* The `max_workers` is the max number of workers that will be created to handle the rpc calls.
* The `parent_calls_tracked` is the number of parent calls that will be tracked by the service.

### Create a configuration by hand

```rust
let conf = Config::default_config();
conf.with_amqp_uri("amqp://toto:super@localhost:5672/")
    .with_rpc_exchange("nameko-rpc")
    .with_max_workers(10)
    .with_parent_calls_tracked(10);
```

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
use std::{thread, time};

#[girolle]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

#[girolle]
fn sub(a: i64, b: i64) -> i64 {
    a - b
}

#[girolle]
fn slip(n: u64) -> String {
    thread::sleep(time::Duration::from_secs(n));
    format!("Slept for {} seconds", n)
}

#[girolle]
fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    let _ = RpcService::new(conf, "video")
        .register(hello)
        .register(sub)
        .register(slip)
        .register(fibonacci)
        .start();
}
```

### Create multiple calls to service of methods, sync and async

```rust
use girolle::prelude::Payload;
use girolle::{serde_json, Config, RpcClient, Value};
use std::time::Instant;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the configuration
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let service_name = "video";
    // Create the client from the configuration
    let mut rpc_client = RpcClient::new(conf);
    // Register the service
    rpc_client.register_service(service_name).await?;
    // Start the client and the consumers
    rpc_client.start().await?;
    // Build the payload
    let p = Payload::new().arg(30);
    // Send the request sync
    let new_result = rpc_client.send(service_name, "fibonacci", p)?;
    // Deserialize the result
    let fib_result: u64 = serde_json::from_value(new_result.get_value())?;
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    // Close the client
    rpc_client.unregister_service(service_name)?;
    rpc_client.close().await?;
    Ok(())
}
```

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client/server library.

## Supported features

* [x] create a client
  * [ ] create a proxy service in rust to interact with an other service
* [x] Create a simple service
  * [x] Handle the error
  * [x] write test
* [x] Add macro to simplify the creation of a service
  * [x] Add basic macro
  * [x] fix macro to handle `return`
  * [x] fix macro to handle recursive function
* [ ] listen to a pub/sub queue

### nameko-client

The Girolle client got the basic features to send sync request and async resquest.
I'm not really happy about the way it need to interact with. I would like to find a
more elegant way like in the nameko. But it works, and it is not really painfull to
use.

### nameko-rpc

The RpcService and the macro procedural are the core of the lib. It does not
suppport **proxy**, i know that's one of the most important feature of the
**Nameko** lib. I will try to implement it in the future. But i think i need a bit refactor
the non-oriented object aspect of Rust make it harder.

### nameko-pubsub

The PubSub service is not at all implemented. I dunno if that's something i'm interested in.

### nameko-web

The web service is not implemented. I'm not sure if i will implement it. I need to rework the client
to be make it 100% thread safe. It should be a commun subject with the proxy.

## Limitation

The current code as been tested with the nameko and girolle examples in this
repository.

|                    | nameko_test.py  | simple_sender.rs  |
|--------------------|-----------------|-------------------|
| nameko_service.py  |       x         |         x         |
| simple_macro       |       x         |         x         |

## Benchmark

### Simple message benchmark

|                    | nameko_test.py  | simple_sender.rs |
|--------------------|-----------------|------------------|
| nameko_service.py  |    15.587 s     |      11.532 s    |
| simple_macro.rs    |    15.654 s     |      8.078 s     |

### Client benchmark

Using hyperfine to test the client benchmark.

Girolle client ( with Girolle service )

```bash
hyperfine -N './target/release/examples/simple_sender'
Benchmark 1: ./target/release/examples/simple_sender
  Time (mean ± σ):      9.995 s ±  0.116 s    [User: 0.163 s, System: 0.197 s]
  Range (min … max):    9.778 s … 10.176 s    10 runs
```

Nameko client ( with Girolle service )

```bash
hyperfine -N --warmup 3 'python nameko_test.py'
Benchmark 1: python nameko_test.py
  Time (mean ± σ):     15.654 s ±  0.257 s    [User: 1.455 s, System: 0.407 s]
  Range (min … max):   15.202 s … 15.939 s    10 runs
```

### Service benchmark

Girolle service ( with Girolle client )

```bash
hyperfine -N './target/release/examples/simple_sender'
Benchmark 1: ./target/release/examples/simple_sender
  Time (mean ± σ):      9.995 s ±  0.116 s    [User: 0.163 s, System: 0.197 s]
  Range (min … max):    9.778 s … 10.176 s    10 runs
```

Nameko service running python 3.9.15 ( with Girolle client )

```bash
hyperfine -N --warmup 3 'target/release/examples/simple_sender'
Benchmark 1: target/release/examples/simple_sender
  Time (mean ± σ):     11.532 s ±  0.091 s    [User: 0.199 s, System: 0.213 s]
  Range (min … max):   11.396 s … 11.670 s    10 runs
```

Nameko service running python 3.9.15 ( with Nameko client )

```bash
hyperfine -N --warmup 3 'python nameko_test.py'
Benchmark 1: python nameko_test.py
  Time (mean ± σ):     15.587 s ±  0.325 s    [User: 1.443 s, System: 0.420 s]
  Range (min … max):   15.181 s … 16.034 s    10 runs
```

### Fibonacci benchmark

The benchmark use a [static set of random int](examples/data_set.json) to compute fibonacci.

|                    | nameko_fib_payload.py |
|--------------------|-----------------------|
| nameko_service.py  | 03 min 58.11   s      |
| simple_macro.rs    |       6.99   s        |

### Macro-overhead benchmark

The benchmark is done to test the overhead of the macro.

![benchmark](./girolle/benches/lines.svg)

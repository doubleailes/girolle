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
use girolle::{serde_json, Config, RpcCall, RpcClient, Value};
use std::time::Instant;
use std::vec;
use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string())?;
    let video_name = "video";
    // Create the rpc call struct
    let rpc_client = RpcClient::new(conf);
    let rpc_call_video: RpcCall = rpc_client.create_rpc_call(video_name.to_string()).await?;
    // Send the payload
    let new_result = rpc_client.send(&rpc_call_video, "fibonacci", vec![30])?;
    let fib_result: u64 = serde_json::from_value(new_result)?;
    // Print the result
    println!("fibonacci :{:?}", fib_result);
    assert_eq!(fib_result, 832040);
    let sub_result = rpc_client.send(&rpc_call_video, "sub", vec![10, 5])?;
    assert_eq!(sub_result, Value::Number(serde_json::Number::from(5)));
    // Create a future result
    let future_result = rpc_client.call_async(&rpc_call_video, "hello", vec!["Toto"]);
    // Send a message during the previous async process
    let result = rpc_client.send(&rpc_call_video, "hello", vec!["Girolle"])?;
    // Print the result
    println!("{:?}", result);
    assert_eq!(result, Value::String("Hello, Girolle!".to_string()));
    // wait for it
    let tempo: time::Duration = time::Duration::from_secs(4);
    thread::sleep(tempo);
    println!("exit sleep");
    // Print the result
    let async_result = rpc_client
        .result(&rpc_call_video, future_result.await?)
        .await;
    println!("{:?}", async_result);
    assert_eq!(async_result?, Value::String("Hello, Toto!".to_string()));
    let start = Instant::now();
    let mut consummers: Vec<_> = Vec::new();
    for n in 1..1001 {
        consummers.push((
            n,
            rpc_client.call_async(&rpc_call_video, "hello", vec![n.to_string()]),
        ));
    }
    // wait for it
    thread::sleep(tempo);
    for con in consummers {
        let id = con.1.await?;
        let _async_result = rpc_client.result(&rpc_call_video, id).await;
        //println!("{}-{}", con.0, async_result?);
    }
    let duration = start.elapsed() - tempo;
    println!("Time elapsed in expensive_function() is: {:?}", duration);
    rpc_call_video.close()?;
    Ok(())
}
```

## To-Do
nameko-client
- [x] create a client
    - [ ] create a proxy service in rust to interact with an other service
nameko-rpc
- [x] Create a simple service
    - [x] Handle the error
    - [x] write test
- [ ] Add macro to simplify the creation of a service
  - [x] Add basic macro
  - [ ] fix macro to handle `return`
  - [ ] fix macro to handle recursive function
nameko-pubsub
- [ ] listen to a pub/sub queue

## Limitation

The current code as been tested with the nameko and girolle examples in this
repository.

|                    | nameko_test.py  | simple_sender.rs  |
|--------------------|-----------------|-------------------|
| nameko_service.py  |       x         |         x         |
| simple_macro       |       x         |         x         |

## Benchmark

|                    | nameko_test.py  | simple_sender.rs |
|--------------------|-----------------|------------------|
| nameko_service.py  |    15.587 s     |    11.532 s      |
| simple_macro.rs    |    15.654 s     |    9.995 s       |

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

### Macro-overhead benchmark

The benchmark is done to test the overhead of the macro.

![benchmark](./girolle/benches/lines.svg)
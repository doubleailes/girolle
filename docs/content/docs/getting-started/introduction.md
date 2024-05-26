+++
title = "Introduction"
description = "A nameko like lib in rust"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A nameko like lib in rust'
toc = true
top = false
+++

## Description

A [nameko-rpc](https://github.com/nameko/nameko) like lib in rust. Check the To-Do
section to see limitation.

**Do not use in production!**

**Girolle** use **Nameko** architecture to send request and get response.

## Stack

Girolle use [lapin](https://github.com/amqp-rs/lapin) as an AMQP client/server library.

## How to use it

The core concept is to remove the pain of the queue creation and reply by
mokcing the **Nameko** architecture, and to use an abstract type
`serde_json::Value` to manipulate a serializable data.

### macro procedural

The lib a procedural macro `#[girolle]` to create a service like this:

```rust
use girolle::prelude::*;

#[girolle]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}
```

The macro will replace the signature of the function by a `fn` that take a
`&[Value]` and return a `Result<Value>`.

### hand made deserialization and serialization

if you do not use the macro `#[girolle]` you need to create a function that
extract the data from the a `&[Value]` like this and return a `Result<Value>`:

```rust
fn fibonacci_reccursive(s: &[Value]) -> Result<Value> {
    // extract the data from the Value
    let n: u64 = serde_json::from_value(s[0].clone())?;
    // create the result
    let result: Value = serde_json::to_value(fibonacci(n))?;
    // return the result in a Result
    Ok(result)
}
```

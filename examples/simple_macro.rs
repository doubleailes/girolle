use girolle::prelude::*;
use std::{thread, time};

#[girolle_task]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

#[girolle_task]
fn sub(a: i64, b: i64) -> i64 {
    a - b
}

#[girolle_task]
fn sleep(n: u64) -> String {
    thread::sleep(time::Duration::from_secs(n));
    format!("Slept for {} seconds", n)
}

#[girolle_task]
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
        .register(sleep)
        .register(fibonacci)
        .start();
}

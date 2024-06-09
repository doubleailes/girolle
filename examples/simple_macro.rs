use girolle::prelude::*;
use std::{thread, time};

#[girolle]
fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

#[girolle]
fn substraction(a: i64, b: i64) -> i64 {
    a - b
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
    let conf: Config = Config::with_yaml_defaults("staging/config.yml".to_string()).unwrap();
    let rpc_task = RpcTask::new("hello", hello);
    let rpc_task_fib = RpcTask::new("fibonacci", fib_warp);
    let rpc_task_sleep = RpcTask::new("sleep", temporary_sleep);
    let _ = RpcService::new(conf, "video")
        .register(rpc_task)
        .register(rpc_task_fib)
        .register(rpc_task_sleep)
        .register(RpcTask::new("sub", substraction))
        .start();
}

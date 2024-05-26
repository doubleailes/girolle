use girolle::prelude::*;
use std::{thread, time};

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
    let rpc_task = RpcTask::new("hello", hello);
    let rpc_task_fib = RpcTask::new("fibonacci", fib_warp);
    let rpc_task_sleep = RpcTask::new("sleep", temporary_sleep);
    let _ = RpcService::new("staging/config.yml".to_string(), "video")
        .register(rpc_task)
        .register(rpc_task_fib)
        .register(rpc_task_sleep)
        .start();
}

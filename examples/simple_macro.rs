use girolle::prelude::*;
use girolle::{RpcService, RpcTask};

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
fn fib_warp(n: u64) -> u64 {
    fibonacci(n)
}

fn main() {
    let rpc_task = RpcTask::new("hello", hello);
    let rpc_task_fib = RpcTask::new("fibonacci", fib_warp);
    let _ = RpcService::new("video")
        .register(rpc_task)
        .register(rpc_task_fib)
        .start();
}

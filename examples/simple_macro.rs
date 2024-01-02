use girolle::JsonValue::Value;
use girolle_macro::girolle;

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
    let rpc_task = girolle::RpcTask::new("hello", hello);
    let rpc_task_fib = girolle::RpcTask::new("fibonacci", fib_warp);
    let _ = girolle::RpcService::new("video")
        .register(rpc_task)
        .register(rpc_task_fib)
        .start();
}

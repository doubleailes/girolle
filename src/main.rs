use girolle::async_service;

fn hello(s: String) -> String {
    format!("Hello, {}!", s)
}

fn main() {
    async_service("video", hello).expect("service failed");
}

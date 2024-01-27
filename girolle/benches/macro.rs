use criterion::{criterion_group, criterion_main, Criterion};

use girolle_macro::girolle;
use serde_json::Value;

fn fibonacci_fast(n: u64) -> u64 {
    let mut a = 0;
    let mut b = 1;

    match n {
        0 => b,
        _ => {
            for _ in 0..n {
                let c = a + b;
                a = b;
                b = c;
            }
            b
        }
    }
}
#[girolle]
fn fibonacci_macro(u: u64)-> u64{
    fibonacci_fast(u)
}

fn build_payload<'a>()->Vec<Value>{
    vec![serde_json::from_str("100").unwrap()]
}

fn batch_sum_macro(payload:Vec<Value> ){
    let _ = fibonacci_macro(payload);
}

fn batch_sum(){
    fibonacci_fast(100);
}

fn bench_macro(c: &mut Criterion) {
    let mut group = c.benchmark_group("Macro");
    let payload = build_payload();
    group.bench_function("reference", |b| b.iter(|| batch_sum()));
    group.bench_function("girolle", |b| b.iter(|| batch_sum_macro(payload.clone())));
    group.finish();
}

criterion_group!(benches, bench_macro);
criterion_main!(benches);
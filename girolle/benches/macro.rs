use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use girolle::prelude::*;
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
#[girolle_macro]
fn fibonacci_macro(u: u64) -> u64 {
    fibonacci_fast(u)
}

fn build_payload(n: u64) -> Vec<Value> {
    vec![serde_json::from_str(&n.to_string()).unwrap()]
}

fn bench_macro(c: &mut Criterion) {
    let mut group = c.benchmark_group("Macro");
    for i in [1u64, 50u64, 101u64].iter() {
        group.bench_with_input(BenchmarkId::new("Naive", i), i, |b, i| {
            b.iter(|| fibonacci_fast(*i))
        });
        let payload = build_payload(*i);
        group.bench_with_input(BenchmarkId::new("Girolle", i), i, |b, _i| {
            b.iter(|| fibonacci_macro(&payload))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_macro);
criterion_main!(benches);

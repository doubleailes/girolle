use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use girolle::prelude::*;
use girolle::nameko_utils::build_inputs_fn_service;
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
#[allow(dead_code)]
fn fibonacci_macro(u: u64) -> u64 {
    fibonacci_fast(u)
}

#[allow(dead_code)]
fn build_payload(n: u64) -> Vec<Value> {
    vec![serde_json::from_str(&n.to_string()).unwrap()]
}

fn bench_macro(c: &mut Criterion) {
    let mut group = c.benchmark_group("Macro");
    for i in [1u64, 50u64, 101u64].iter() {
        group.bench_with_input(BenchmarkId::new("Naive", i), i, |b, i| {
            b.iter(|| fibonacci_fast(*i))
        });
    }
    group.finish();
}



fn bench_build_inputs_fn_service(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_inputs_fn_service");
    let service_args = vec!["a", "b", "c"];
    let data_delivery = Payload::new()
        .arg("1")
        .arg("2")
        .kwarg("c", "3");
    group.bench_function("build_inputs_fn_service", |b| {
        b.iter(|| build_inputs_fn_service(&service_args, data_delivery.clone()))
    });
    group.finish();
}

criterion_group!(name = benches;config = Criterion::default(); targets= bench_macro, bench_build_inputs_fn_service);
criterion_main!(benches);

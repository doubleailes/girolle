+++
title = "Performances"
description = "Performances of Girolle."
date = 2021-05-01T08:20:00+00:00
updated = 2021-05-01T08:20:00+00:00
draft = false
weight = 50
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Performances of Girolle."
toc = true
top = false
+++

## Introduction

The performance of Girolle is a really important matter. The lib is made to be fast and
to be able to handle a lot of request.

## Cost of the serialization & deserialization

The cost of the serialization and deserialization is really low. The lib use
`serde_json` to serialize and deserialize the data.

<img alt="serde performance" src="https://github.com/doubleailes/girolle/blob/main/girolle/benches/lines.svg" width="30px" height="30px">

The average overhead of the serialization and deserialization is around 39 ns per call.

## Benchmark

|                    | nameko_test.py  | simple_sender.rs |
|--------------------|-----------------|------------------|
| nameko_service.py  |    15.587 s     |    11.532 s      |
| simple_macro.rs    |    15.654 s     |    9.995 s       |

### Client benchmark

Using hyperfine to test the client benchmark.

Girolle client ( with Girolle service )

```bash
hyperfine -N './target/release/examples/simple_sender'
Benchmark 1: ./target/release/examples/simple_sender
  Time (mean ± σ):      9.995 s ±  0.116 s    [User: 0.163 s, System: 0.197 s]
  Range (min … max):    9.778 s … 10.176 s    10 runs
```	

Nameko client ( with Girolle service )

```bash
hyperfine -N --warmup 3 'python nameko_test.py'
Benchmark 1: python nameko_test.py
  Time (mean ± σ):     15.654 s ±  0.257 s    [User: 1.455 s, System: 0.407 s]
  Range (min … max):   15.202 s … 15.939 s    10 runs
```
### Service benchmark

Girolle service ( with Girolle client )

```bash
hyperfine -N './target/release/examples/simple_sender'
Benchmark 1: ./target/release/examples/simple_sender
  Time (mean ± σ):      9.995 s ±  0.116 s    [User: 0.163 s, System: 0.197 s]
  Range (min … max):    9.778 s … 10.176 s    10 runs
```

Nameko service running python 3.9.15 ( with Girolle client )

```bash
hyperfine -N --warmup 3 'target/release/examples/simple_sender'
Benchmark 1: target/release/examples/simple_sender
  Time (mean ± σ):     11.532 s ±  0.091 s    [User: 0.199 s, System: 0.213 s]
  Range (min … max):   11.396 s … 11.670 s    10 runs
```

Nameko service running python 3.9.15 ( with Nameko client )

```bash
hyperfine -N --warmup 3 'python nameko_test.py'
Benchmark 1: python nameko_test.py
  Time (mean ± σ):     15.587 s ±  0.325 s    [User: 1.443 s, System: 0.420 s]
  Range (min … max):   15.181 s … 16.034 s    10 runs
```
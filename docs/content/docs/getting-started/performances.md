+++
title = "Performances"
description = "Performances of Girolle."
date = 2021-05-01T08:20:00+00:00
updated = 2021-05-01T08:20:00+00:00
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Performances of Girolle."
toc = true
top = false
+++

## Introduction

The performance of Girolle a really important. The lib is made to be fast and
to be able to handle a lot of request.

## Cost of the serialization & deserialization

The cost of the serialization and deserialization is really low. The lib use
`serde_json` to serialize and deserialize the data.

<img alt="serde performance" src="https://github.com/doubleailes/girolle/blob/main/girolle/benches/lines.svg" width="30px" height="30px">

The average overhead of the serialization and deserialization is around 39 ns per call.

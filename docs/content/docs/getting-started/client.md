+++
title = "Service"
description = "A look about the client"
date = 2024-06-14T00:00:00+00:00
updated = 2024-06-14T00:00:00+00:00
draft = false
weight = 40
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A look about the client'
toc = true
top = false
+++

## Introduction

The client is a standalone client to be use with Nameko or Girolle service. It is
made to be fast and to be able to handle a lot of request. The client is made to
be used with the `tokio` runtime.
The client in Girolle try to mimic the Nameko client. It is made to be easy to use
with the sync method `send()` and the async method `call_async()` and `result()`
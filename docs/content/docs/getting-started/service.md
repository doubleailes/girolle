+++
title = "Service"
description = "A look about the service"
date = 2024-06-14T00:00:00+00:00
updated = 2024-06-14T00:00:00+00:00
draft = false
weight = 30
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A look about the service'
toc = true
top = false
+++

## Introduction

The service is the core of the lib. It is the place where the magic happen. The
core rely on the `lapin` lib to handle the AMQP protocol and tokio to handle the
async and spawn part.
Tokio spawn one delagate per vCPU to handle the request and the response. As an
optimisation for heavy load, the lib use an async spawn to publish the response
so if a vCPU is free, it can handle to publish the response.
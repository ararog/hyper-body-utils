---
layout: default
title: hyper-body-utils - Utilities for handling HTTP bodies with hyper
nav_order: 1
description: "Utilities for handling HTTP bodies with hyper"
permalink: /
---
<div align="center">
<h1><b>hyper-body-utils</b></h1>
</div>

[![Crates.io downloads](https://img.shields.io/crates/d/hyper-body-utils)](https://crates.io/crates/hyper-body-utils) [![crates.io](https://img.shields.io/crates/v/hyper-body-utils?style=flat-square)](https://crates.io/crates/hyper-body-utils) [![Build Status](https://github.com/ararog/hyper-body-utils/actions/workflows/rust.yml/badge.svg?event=push)](https://github.com/ararog/hyper-body-utils/actions/workflows/rust.yml) ![Crates.io MSRV](https://img.shields.io/crates/msrv/hyper-body-utils) [![Documentation](https://docs.rs/hyper-body-utils/badge.svg)](https://docs.rs/hyper-body-utils/latest/hyper-body-utils) [![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ararog/hyper-body-utils/blob/main/LICENSE.md)  [![codecov](https://codecov.io/gh/ararog/hyper-body-utils/graph/badge.svg?token=T0HSBAPVSI)](https://codecov.io/gh/ararog/hyper-body-utils)

**hyper-body-utils** is a collection of utilities for working with hyper bodies.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
hyper-body-utils = { version = "0.1.0" }
```

Basic usage:

```rust
use hyper_body_utils::HttpBody;

let body = HttpBody::from_bytes(b"Hello, world!");
```

## Examples

Check out the [examples](./examples.md) for complete examples of how to use hyper-body-utils in your projects.

## Documentation

- [API Reference](https://docs.rs/hyper-body-utils)
- [Contributing Guide](./CONTRIBUTING.md)

## License

This project is licensed under the [MIT License](./LICENSE.md).

## Author

Rogerio Pereira Araujo <rogerio.araujo@gmail.com>

erl_dist
========

[![erl_dist](https://img.shields.io/crates/v/erl_dist.svg)](https://crates.io/crates/erl_dist)
[![Documentation](https://docs.rs/erl_dist/badge.svg)](https://docs.rs/erl_dist)
[![Build Status](https://travis-ci.org/sile/erl_dist.svg?branch=master)](https://travis-ci.org/sile/erl_dist)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Rust Implementation of [Erlang Distribution Protocol](http://erlang.org/doc/apps/erts/erl_dist_protocol.html).

The distribution protocol is used to communicate with distributed erlang nodes.

[Documentation](https://docs.rs/erl_dist)

Examples
--------

- EPMD Client Example: [epmd_cli.rs](examples/epmd_cli.rs)
- Client Node Example: [send_msg.rs](examples/send_msg.rs)
- Server Node Example: [recv_msg.rs](examples/recv_msg.rs)

Installation
------------

Add following lines to your `Cargo.toml`:

```toml
[dependencies]
erl_dist = "0.1"
```

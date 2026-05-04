# XSCP

[![Crates.io](https://img.shields.io/crates/v/xscp.svg)](https://crates.io/crates/xscp)
[![Docs.rs](https://docs.rs/xscp/badge.svg)](https://docs.rs/xscp)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)
![Rust 2024 edition](https://img.shields.io/badge/rust-2024_edition-orange.svg)

<p align="center">
  <img src="docs/XSCP.png" alt="XSCP Logo" />
</p>

**XSCP** *(XSCP Simple Chat Protocol)* is a text-based chat protocol. This repository contains its **reference implementation in Rust**, along with a **client** and a **server**.

## Overview

XSCP defines a minimal client-server architecture for real-time text messaging over TCP. The protocol is intentionally simple: clients connect to a server, send messages, and the server broadcasts each message to every connected client. Think IRC, but stripped to its bare bones and written in modern Rust.

PDUs are line-oriented, UTF-8 and pipe-delimited, with a strict **512-byte budget**. The wire format is fully documented in the [`xscp` crate docs](https://docs.rs/xscp).

## Repository layout

This repo is a Cargo workspace with three crates:

| Crate | Description | 
| ----- | ----------- | 
| [`xscp`](./src) | Protocol primitives: request, response and notification PDUs, with safe constructors and parsers. Transport-agnostic. |
| [`client`](./client) | Reference XSCP client. |
| [`server`](./server) | Reference XSCP server. |

The `xscp` crate is the only one published; the client and server are reference implementations meant to live alongside the protocol crate in this repository.

## Getting started

### Use the protocol crate in your own project

```sh
cargo add xscp
```

See the [crate README](./README_CRATE.md) and [API docs](https://docs.rs/xscp) for usage examples.

### Build everything from source

```sh
git clone https://github.com/ivan-amon/xscp.git
cd xscp
cargo build --workspace
```

## Project Status

- [x] Protocol crate (`xscp`) with request, response and notification PDUs
- [x] TCP server/client foundation
- [ ] Async I/O Server with Tokio
- [ ] Multi-client broadcast
- [ ] TLS support

## Contributing

Issues and pull requests are welcome. If you want to discuss design changes to the protocol itself, please open an issue first.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
Built by Iván with ❤️ in Rust.

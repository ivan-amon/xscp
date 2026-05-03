# XSCP

<p align="center">
  <img src="docs/XSCP.png" alt="XSCP Logo" />
</p>

**XSCP** *(XSCP Simple Chat Protocol)* is a text-based chat protocol built from scratch in Rust.

## Overview

XSCP defines a minimal client-server architecture for real-time text messaging over TCP. The protocol is intentionally simple: clients connect to a server, send messages, and the server broadcasts each message to every connected client. Think IRC, but stripped to its bare bones and written in modern Rust.

The project currently ships two binaries:

| Binary | Description |
|--------|-------------|
| `server` | Listens on a TCP port and handles incoming client connections |
| `client` | CLI tool to connect to an XSCP server and exchange messages |

## Roadmap

The next milestone is migrating the networking layer to **[Tokio](https://tokio.rs/)**, Rust's async runtime. Today the server is single-threaded and handles one client at a time — Tokio will unlock the concurrent connections needed to make broadcast work properly.

- [x] TCP server/client foundation
- [ ] Async I/O with Tokio
- [ ] Multi-client broadcast

## Usage

### Server

```bash
cargo run --bin server <port>
```

### Client

```bash
cargo run --bin client <ip> <port>
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
Built by Iván with ❤️ in Rust.

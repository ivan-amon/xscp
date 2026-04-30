# XSCP

**XSCP** *(XSCP Simple Chat Protocol)* is a text-based chat protocol built from scratch in Rust — inspired by the simplicity and clarity of the [IRC protocol](https://en.wikipedia.org/wiki/Internet_Relay_Chat).

## Overview

XSCP defines a minimal client-server architecture for real-time text messaging over TCP. The protocol is intentionally simple: clients connect to a server, send line-delimited messages, and the server broadcasts each message to every connected client. Think IRC, but stripped to its bare bones and written in modern Rust.

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
- [ ] Usernames and channels (IRC-style)

## Usage

### Server

```bash
cargo run --bin server <port>
```

```
Listening on port 7878
127.0.0.1:54321 connected
Request from 127.0.0.1:54321: hello world
```

### Client

```bash
cargo run --bin client <ip> <port>
```

```
Connected successfully to 127.0.0.1:7878
> hello world
Echo: hello world
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---
Built with ❤️ in Rust.

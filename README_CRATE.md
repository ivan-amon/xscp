# xscp

[![Crates.io](https://img.shields.io/crates/v/xscp.svg)](https://crates.io/crates/xscp)
[![Docs.rs](https://docs.rs/xscp/badge.svg)](https://docs.rs/xscp)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

A minimal, zero-dependency Rust implementation of the **XSCP** (XSCP Simple Chat Protocol): a small, line-oriented, text-based chat protocol with a strict 512-byte PDU budget.

This crate provides only the **protocol primitives** — request, response and notification PDUs, with safe constructors and parsers. It is transport-agnostic: bring your own TCP, TLS, WebSocket or whatever you like, and use these types to build a client, a server, or anything in between.

## Features

- **Zero dependencies.** The crate compiles against the standard library only — no third-party code in your dependency tree.
- **Safe parsing.** All constructors and parsers reject malformed input and validate field lengths.
- **Anti-smuggling by construction.** Field delimiters (`|`) and line terminators (`\r\n`) are forbidden inside fields, so a hostile payload cannot inject extra PDUs.
- **Borrowed, allocation-light API.** PDU types hold `&str` slices into the original buffer; parsing does not copy your data.
- **Strict wire format.** Every PDU has a documented byte budget and a fixed structure; nothing is left implicit.

## Installation

```sh
cargo add xscp
```

Or in `Cargo.toml`:

```toml
[dependencies]
xscp = "0.1"
```

## Quick start

### Building and parsing a request

```rust
use xscp::request::{XscpRequest, OpCode};

// Build a request programmatically.
let req = XscpRequest::try_new(OpCode::Chat, "alice", "hello, world!")?;
assert_eq!(req.opcode(), OpCode::Chat);
assert_eq!(req.nickname(), "alice");

// Parse a request received from the wire.
let raw = "CHAT|alice|hello, world!\r\n";
let parsed = XscpRequest::parse(raw)?;
assert_eq!(parsed.message(), "hello, world!");
# Ok::<(), xscp::request::RequestError>(())
```

### Building and parsing a response

```rust
use xscp::response::XscpResponse;

let res = XscpResponse::try_new(200, "OK")?;
assert_eq!(res.status_code(), 200);

let parsed = XscpResponse::parse("200|OK\r\n")?;
assert_eq!(parsed.reason_phrase(), "OK");
# Ok::<(), xscp::response::ResponseError>(())
```

### Building and parsing a notification

```rust
use xscp::notification::{XscpNotification, NotificationType};

let note = XscpNotification::try_new(
    NotificationType::Broadcast,
    "alice",
    "hello, everyone!",
)?;

let parsed = XscpNotification::parse("BRDC|alice|hello, everyone!\r\n")?;
assert_eq!(parsed.source(), "alice");
# Ok::<(), xscp::notification::NotificationError>(())
```

## Protocol overview

XSCP is a line-oriented, UTF-8, pipe-delimited protocol. Every PDU ends with `\r\n` and is at most **512 bytes** (responses are at most **36 bytes**).

### Request PDU

```text
+------------------------------------------------------------------+
|   OPCODE (4 Bytes)   |   Nickname (Min 3 Bytes, Max 32 Bytes)    |
|------------------------------------------------------------------|
|          Message (Max 472 Bytes) + \r\n (2 Bytes)                |
+------------------------------------------------------------------+
```

| OpCode | Wire | Meaning                  |
| ------ | ---- | ------------------------ |
| Login  | `LOGN` | User registration       |
| Chat   | `CHAT` | Global message broadcast |
| Exit   | `EXIT` | Graceful disconnection   |

### Response PDU

```text
+-----------------------------------------------------------------------+
|   Status Code (1-3 ASCII digits)   |   Reason Phrase (Max 32 Bytes)   |
+-----------------------------------------------------------------------+
```

Status codes follow an HTTP-like convention (numeric, ≤ 599); reason phrases are short, human-readable strings.

### Notification PDU

```text
+---------------------------------------------------------------------------+
|   Notification Type (4 Bytes)   |   Source (Min 3 Bytes, Max 32 Bytes)    |
|---------------------------------------------------------------------------|
|                 Message (Max 472 Bytes) + \r\n (2 Bytes)                  |
+---------------------------------------------------------------------------+
```

| Notification | Wire   | Meaning                     |
| ------------ | ------ | --------------------------- |
| Broadcast    | `BRDC` | Message relayed to all users |

The `Source` field is either a user nickname or the literal `XSCP_SERVER` for server-originated notifications.

## Security notes

XSCP fields are validated at construction and parse time:

- `|`, `\r` and `\n` are rejected inside any field.
- Nicknames and sources must be 3–32 bytes; messages must not exceed 472 bytes; reason phrases must not exceed 32 bytes.

This makes **PDU smuggling** (a hostile payload terminating its own PDU early to inject another) impossible by construction. As a consumer of the crate you still need to enforce the 512-byte read budget on the transport side.

## Use cases

This crate is the protocol layer; it does no I/O. Typical uses:

- Implement an XSCP **client** on top of `std::net::TcpStream`, `tokio`, `async-std`, …
- Implement an XSCP **server** that accepts requests and emits responses and broadcast notifications.
- Build bridges, proxies, fuzzers or test fixtures for XSCP-speaking software.

A reference client and server live in the [`client/`](https://github.com/ivan-amon/xscp/tree/main/client) and [`server/`](https://github.com/ivan-amon/xscp/tree/main/server) crates of the workspace and are **not** published to crates.io.

## Minimum Supported Rust Version

The crate is built against Rust **2024 edition**. Bumps to the MSRV are not considered breaking changes.

## License

Licensed under the [MIT license](LICENSE).

## Contributing

Issues and pull requests are welcome at <https://github.com/ivan-amon/xscp>.

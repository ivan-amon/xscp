# ---- Build stage ----
FROM rust:1.85-alpine3.20 AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY io ./io
COPY server ./server
COPY client ./client

RUN cargo build --release -p client \
    && strip target/release/client

# ---- Runtime stage ----
FROM alpine:latest

RUN addgroup -S xscp && adduser -S xscp -G xscp

COPY --from=builder /build/target/release/client /usr/local/bin/xscp-client

USER xscp

ENTRYPOINT ["/usr/local/bin/xscp-client"]

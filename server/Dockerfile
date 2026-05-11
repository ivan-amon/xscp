# ---- Build stage ----
FROM rust:1.85-alpine3.20 AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY server ./server
COPY client ./client

RUN cargo build --release -p server \
    && strip target/release/server

# ---- Runtime stage ----
FROM alpine:latest

RUN addgroup -S xscp && adduser -S xscp -G xscp

COPY --from=builder /build/target/release/server /usr/local/bin/xscp-server

USER xscp
EXPOSE 7878

ENTRYPOINT ["/usr/local/bin/xscp-server"]

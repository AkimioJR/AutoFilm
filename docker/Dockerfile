# syntax=docker/dockerfile:1.7

FROM rust:alpine AS builder

WORKDIR /workspace

RUN apk add --no-cache build-base musl-dev

COPY Cargo.toml Cargo.lock ./
COPY alist-client-rs ./alist-client-rs
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/workspace/target \
    cargo build --release --locked && \
    cp /workspace/target/release/autofilm /tmp/autofilm

FROM alpine:latest

ENV TZ=Asia/Shanghai

RUN apk add --no-cache ca-certificates tzdata

COPY --from=builder /tmp/autofilm /usr/local/bin/autofilm

VOLUME ["/config", "/logs", "/media", "/fonts"]
WORKDIR /

ENTRYPOINT ["/usr/local/bin/autofilm"]

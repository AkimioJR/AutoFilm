FROM rust:alpine AS builder

WORKDIR /workspace

RUN apk add --no-cache build-base musl-dev

COPY Cargo.toml Cargo.lock ./
COPY alist-client-rs ./alist-client-rs
COPY src ./src

RUN cargo build --release --locked

FROM alpine:latest

ENV TZ=Asia/Shanghai

RUN apk add --no-cache ca-certificates tzdata

COPY --from=builder /workspace/target/release/autofilm /usr/local/bin/autofilm

VOLUME ["/config", "/logs", "/media", "/fonts"]
WORKDIR /

ENTRYPOINT ["/usr/local/bin/autofilm"]

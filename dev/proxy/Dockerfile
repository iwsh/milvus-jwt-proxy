FROM rust:1.72-slim-bullseye AS builder
WORKDIR /workspace
COPY .cargo .cargo
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates build-essential pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /workspace/target/release/milvus-jwt-proxy /usr/local/bin/milvus-jwt-proxy
EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/milvus-jwt-proxy"]

FROM rust:1.85 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/milvus-jwt-proxy /usr/local/bin/milvus-jwt-proxy
EXPOSE 8000
CMD ["milvus-jwt-proxy"]

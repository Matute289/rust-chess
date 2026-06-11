FROM rust:1.88-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY chess-engine/ chess-engine/
COPY chess-server/ chess-server/
COPY src/ src/
RUN cargo build --release -p chess-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/chess-server /usr/local/bin/chess-server
COPY --from=builder /app/chess-server/migrations /migrations
EXPOSE 8006
CMD ["chess-server"]

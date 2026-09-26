FROM rust:1.98.1-trixie AS builder
WORKDIR /app
COPY . .
RUN cargo build --release
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/housekeeper /usr/local/bin/housekeeper
CMD ["housekeeper"]

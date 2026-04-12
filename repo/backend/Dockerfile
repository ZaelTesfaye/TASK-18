FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.toml

# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source
COPY . .

# Build the application
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/silverscreen-backend /app/silverscreen-backend
COPY --from=builder /app/migrations /app/migrations

RUN mkdir -p /data/backups

EXPOSE 8080

CMD ["/app/silverscreen-backend"]

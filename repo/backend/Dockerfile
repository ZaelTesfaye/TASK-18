FROM rust:1.88-bookworm AS builder

ENV CARGO_BUILD_JOBS=1
ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV CARGO_HTTP_TIMEOUT=600
ENV RUSTFLAGS="-Ccodegen-units=1"

WORKDIR /app

# Copy manifests and lock file
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# Create dummy lib + main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs
RUN cargo build --locked --release
RUN rm -rf src

# Copy actual source (.dockerignore excludes target/)
COPY . .

# Force cargo to detect source change (COPY preserves mtime which can be
# older than the cached dummy build, causing cargo to skip recompilation)
RUN touch src/main.rs src/lib.rs

# Build the application
RUN cargo build --locked --release

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

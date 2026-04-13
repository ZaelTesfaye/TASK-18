FROM rust:1.88-bookworm AS builder

ENV CARGO_BUILD_JOBS=1
ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV CARGO_HTTP_TIMEOUT=600
ENV RUSTFLAGS="-Ccodegen-units=1"

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
# Install prebuilt trunk binary to avoid expensive source compilation on low-memory builders.
RUN set -eux; \
    case "$(uname -m)" in \
    x86_64) trunk_target="x86_64-unknown-linux-gnu" ;; \
    aarch64) trunk_target="aarch64-unknown-linux-gnu" ;; \
    *) echo "Unsupported architecture: $(uname -m)"; exit 1 ;; \
    esac; \
    trunk_version="0.21.14"; \
    if curl --retry 6 --retry-delay 2 --retry-connrefused -fSL "https://github.com/trunk-rs/trunk/releases/download/v${trunk_version}/trunk-${trunk_target}.tar.gz" | tar -xz -C /usr/local/bin trunk; then \
    echo "Installed prebuilt trunk binary"; \
    else \
    echo "Prebuilt trunk download failed, falling back to cargo install"; \
    cargo install --locked --jobs 1 trunk@${trunk_version}; \
    fi; \
    trunk --version
RUN trunk tools install wasm-bindgen --version 0.2.118

WORKDIR /app

# Copy manifests and lock file
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# Create dummy lib + main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs
RUN cargo build --locked --target wasm32-unknown-unknown --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source
COPY . .

# Force cargo to detect source change (COPY preserves mtime)
RUN touch src/main.rs src/lib.rs

# Build with trunk
RUN trunk build --release --locked

FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

FROM rust:1.88-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown
# Install CLI tools in separate locked steps for reproducible, lower-memory builds.
RUN cargo install --locked --jobs 1 trunk@0.21.14
RUN cargo install --locked --jobs 1 wasm-bindgen-cli@0.2.118

WORKDIR /app

# Copy manifests and lock file
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# Create dummy lib + main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && touch src/lib.rs
RUN cargo build --target wasm32-unknown-unknown --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source
COPY . .

# Force cargo to detect source change (COPY preserves mtime)
RUN touch src/main.rs src/lib.rs

# Build with trunk
RUN trunk build --release

FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

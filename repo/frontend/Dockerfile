FROM rust:1.88-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk@0.18.8 wasm-bindgen-cli@0.2.92

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.toml

# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --target wasm32-unknown-unknown --release 2>/dev/null || true
RUN rm -rf src

# Copy actual source
COPY . .

# Build with trunk
RUN trunk build --release index.html

FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

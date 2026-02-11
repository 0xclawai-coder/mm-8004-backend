# ── Build stage ──
FROM rust:1.91-bookworm AS builder

WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs so cargo can fetch + compile deps
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Copy real source + compile-time assets (ABI, migrations)
COPY src/ src/
COPY abi/ abi/
COPY migrations/ migrations/

# Force rebuild of the binary (touch ensures cargo sees source as changed)
RUN touch src/main.rs && cargo build --release

# ── Runtime stage ──
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/molt-marketplace-backend /usr/local/bin/molt-marketplace-backend

# Debug: verify binary can execute and list shared library dependencies
RUN ldd /usr/local/bin/molt-marketplace-backend && /usr/local/bin/molt-marketplace-backend --help 2>&1 || true

ENV RUST_LOG=molt_marketplace_backend=info,tower_http=info

EXPOSE 3001

CMD ["molt-marketplace-backend"]

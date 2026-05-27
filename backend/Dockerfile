# syntax=docker/dockerfile:1

# ── Build Stage ──────────────────────────────────────────────────────────────
# Multi-stage build optimized for production deployment.
# Uses layer caching to speed up rebuilds when only source code changes.

FROM rust:1.85-bookworm AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies separately
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the application with optimizations
RUN cargo build --release --locked && \
    strip /app/target/release/predifi-backend

# ── Runtime Stage ────────────────────────────────────────────────────────────
# Minimal runtime image using debian:bookworm-slim to keep size < 100MB.

FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install only runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash predifi && \
    chown -R predifi:predifi /app

# Copy binary from builder
COPY --from=builder --chown=predifi:predifi /app/target/release/predifi-backend /usr/local/bin/predifi-backend

# Switch to non-root user
USER predifi

# Environment configuration
ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/predifi-backend", "--version"] || exit 1

EXPOSE 3000

CMD ["predifi-backend"]

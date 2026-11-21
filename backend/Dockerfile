# syntax=docker/dockerfile:1

# ── Build Stage ──────────────────────────────────────────────────────────────
# Multi-stage build optimized for production deployment.
# Uses layer caching to speed up rebuilds when only source code changes.

FROM rust:1.88-bookworm@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0 AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy target files to build dependencies separately.
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/seed.rs && \
    echo "fn main() {}" > src/bin/openapi_export.rs && \
    echo "fn main() {}" > src/bin/migration_check.rs && \
    touch src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Copy application sources and compile-time SQLx inputs.
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Refresh source mtimes so Cargo doesn't reuse the newer dummy-target artifacts.
RUN find src -type f -exec touch {} + && \
    cargo build --release --locked && \
    strip /app/target/release/predifi-backend

# ── Runtime Stage ────────────────────────────────────────────────────────────
# Minimal runtime image using debian:bookworm-slim to keep size < 100MB.

FROM debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251 AS runtime
WORKDIR /app

# Install only runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
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
ENV PREDIFI_APP_HOST=0.0.0.0
ENV PREDIFI_APP_PORT=3000
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=180s --retries=5 \
    CMD curl -fsS "http://localhost:${PREDIFI_APP_PORT}/health/ready" > /dev/null

EXPOSE 3000

CMD ["predifi-backend"]

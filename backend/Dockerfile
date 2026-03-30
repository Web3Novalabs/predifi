# syntax=docker/dockerfile:1

FROM rust:1.85-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/predifi-backend /usr/local/bin/predifi-backend

ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

CMD ["predifi-backend"]

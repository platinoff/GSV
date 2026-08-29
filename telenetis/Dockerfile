# syntax=docker/dockerfile:1
#
# telenetis — Telegram Mini App + Bot for GSV Godfather (port 9800)
#
# Multi-stage build: Rust builder -> slim runtime. Bind 0.0.0.0:9800.
# All runtime config comes from env (TELENETIS_*); none is baked in.
# Runs the plain server binary (main.rs, graceful shutdown on SIGTERM) so a
# container/manifest (docker restart=dependent, systemd) handles rescheduling.

FROM rust:1-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
RUN cargo build --release --bin telenetis

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates openssl wget && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin telenetis
WORKDIR /app
COPY --from=builder /build/target/release/telenetis /app/telenetis
RUN mkdir -p /app/data /app/.env \
    && chown -R telenetis:telenetis /app
USER telenetis
EXPOSE 9800
ENV TELENETIS_PORT=9800
ENV TELENETIS_TUNNEL_ENABLED=0
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/bin/sh", "-c", "wget -qO- http://127.0.0.1:9800/health >/dev/null 2>&1 || exit 1"]
CMD ["/app/telenetis"]

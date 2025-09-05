# syntax=docker/dockerfile:1

########################
# Build stage
########################
FROM rust:1-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates pkg-config && rm -rf /var/lib/apt/lists/*

ARG BIN=blastfile
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release && rm -rf src

COPY src ./src
RUN cargo build --release

########################
# Runtime stage
########################
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates wget gosu && rm -rf /var/lib/apt/lists/*

RUN useradd -u 10001 -m appuser

ENV BIND=0.0.0.0:8080 \
    DATA_DIR=/data

ARG BIN=blastfile
COPY --from=builder /app/target/release/${BIN} /app/server

RUN printf '#!/bin/sh\nset -e\nmkdir -p "$DATA_DIR"\nchown -R appuser:appuser "$DATA_DIR" || true\nexec gosu appuser /app/server\n' > /entrypoint.sh \
 && chmod +x /entrypoint.sh

EXPOSE 8080

# Healthcheck
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD wget -qO- http://127.0.0.1:8080/health >/dev/null 2>&1 || exit 1

ENTRYPOINT ["/entrypoint.sh"]
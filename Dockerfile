# syntax=docker/dockerfile:1.7

########################
# Build stage
########################
FROM rust:1-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release --locked && rm -rf target/release/deps/* src

COPY src ./src
RUN cargo build --release --locked

RUN strip target/release/blastfile || true


########################
# Runtime stage
########################
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates wget tini && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -u 10001 -m -d /nonexistent -s /usr/sbin/nologin appuser

ENV DATA_DIR=/data
RUN mkdir -p "${DATA_DIR}" && chown 10001:10001 "${DATA_DIR}"

ENV BIND=0.0.0.0:8080 \
    RUST_LOG=info

COPY --from=builder /app/target/release/blastfile /app/server

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
  CMD wget -qO- "http://127.0.0.1:${PORT:-8080}/health" || exit 1

LABEL org.opencontainers.image.title="blastfile" \
      org.opencontainers.image.description="A lightweight, self-hosted file transfer service built in Rust" \
      org.opencontainers.image.source="." \
      org.opencontainers.image.licenses="MIT"

USER 10001:10001

ENTRYPOINT ["/usr/bin/tini","--"]

CMD ["/app/server"]

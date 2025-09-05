# syntax=docker/dockerfile:1

########################
# Build stage
########################
FROM rust:1-bookworm AS builder
WORKDIR /app

# Optional: system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates pkg-config && rm -rf /var/lib/apt/lists/*

# Improve cache: build “empty” with just the manifests
ARG BIN=blastfile
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release && rm -rf src

# Build
COPY src ./src
RUN cargo build --release

########################
# Runtime stage
########################
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Min Tools (curl for HEALTHCHECK) + certificates
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates wget && rm -rf /var/lib/apt/lists/*

# non-root user
RUN useradd -u 10001 -m appuser

# Default variables
ENV BIND=0.0.0.0:8080 \
    DATA_DIR=/data

# Binary
ARG BIN=blastfile
COPY --from=builder /app/target/release/${BIN} /app/server

# data file
RUN mkdir -p /data && chown -R appuser:appuser /data /app
USER appuser

EXPOSE 8080

# Healthcheck
HEALTHCHECK NONE

ENTRYPOINT ["/app/server"]
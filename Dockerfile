# syntax=docker/dockerfile:1

# ============================================================================
# Zeabur/root Dockerfile - build the Rust backend and Svelte static frontend
# ============================================================================

FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS cargo-chef

FROM rust:slim-bookworm AS chef
COPY --from=cargo-chef /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config git cmake perl make build-essential \
    libssl-dev libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
RUN sed -i 's/"src-tauri",//' Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN sed -i 's/"src-tauri",//' Cargo.toml

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    cargo build --release

FROM node:20-slim AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend ./
RUN npx svelte-kit sync
RUN npm run build

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libwebp7 \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/piney-server /app/piney
COPY --from=frontend-builder /app/frontend/build /app/static
RUN mkdir -p /app/data

ENV RUN_MODE=server \
    PORT=9696 \
    DATA_DIR=/app/data

EXPOSE 9696
VOLUME ["/app/data"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9696/api/health || exit 1

CMD ["/app/piney"]

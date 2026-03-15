# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.86

FROM rust:${RUST_VERSION}-bookworm AS build

WORKDIR /work

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*

# Expected source layout for the Rust backend:
#   backend/Cargo.toml
#   backend/src/main.rs
# producing the binary:
#   memory-lancedb-pro-backend
COPY backend /work/backend

WORKDIR /work/backend

RUN test -f Cargo.toml
RUN cargo build --locked --release --bin memory-lancedb-pro-backend

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system --gid 10001 memorybackend \
 && useradd --system --uid 10001 --gid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin memorybackend \
 && mkdir -p /etc/memory-lancedb-pro-backend \
 && mkdir -p /var/lib/memory-lancedb-pro-backend/lancedb \
 && mkdir -p /var/lib/memory-lancedb-pro-backend/sqlite \
 && chown -R memorybackend:memorybackend /etc/memory-lancedb-pro-backend /var/lib/memory-lancedb-pro-backend

COPY --from=build /work/backend/target/release/memory-lancedb-pro-backend /usr/local/bin/memory-lancedb-pro-backend

USER memorybackend:memorybackend

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/memory-lancedb-pro-backend", "--config", "/etc/memory-lancedb-pro-backend/backend.toml"]

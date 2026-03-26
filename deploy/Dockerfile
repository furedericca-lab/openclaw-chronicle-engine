# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.91

FROM rust:${RUST_VERSION}-bookworm AS build

WORKDIR /work

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates pkg-config libssl-dev protobuf-compiler libprotobuf-dev \
 && rm -rf /var/lib/apt/lists/*

ENV PROTOC_INCLUDE=/usr/include

# Expected source layout for the Rust backend:
#   backend/Cargo.toml
#   backend/src/main.rs
# producing the binary:
#   chronicle-engine-rs
COPY backend /work/backend

WORKDIR /work/backend

RUN test -f Cargo.toml
RUN cargo build --locked --release --bin chronicle-engine-rs

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system --gid 10001 chroniclebackend \
 && useradd --system --uid 10001 --gid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin chroniclebackend \
 && mkdir -p /etc/chronicle-engine-backend \
 && mkdir -p /var/lib/chronicle-engine-backend/lancedb \
 && mkdir -p /var/lib/chronicle-engine-backend/sqlite \
 && chown -R chroniclebackend:chroniclebackend /etc/chronicle-engine-backend /var/lib/chronicle-engine-backend

COPY --from=build /work/backend/target/release/chronicle-engine-rs /usr/local/bin/chronicle-engine-rs

USER chroniclebackend:chroniclebackend

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/chronicle-engine-rs", "--config", "/etc/chronicle-engine-backend/backend.toml"]

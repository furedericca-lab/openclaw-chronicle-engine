# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.91
ARG NODE_VERSION=20

FROM node:${NODE_VERSION}-bookworm AS frontend-build
WORKDIR /app
COPY backend/web/package*.json ./
RUN npm install
COPY backend/web/ ./
RUN npm run build

FROM rust:${RUST_VERSION}-bookworm AS build
WORKDIR /work
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates pkg-config libssl-dev protobuf-compiler libprotobuf-dev \
 && rm -rf /var/lib/apt/lists/*
ENV PROTOC_INCLUDE=/usr/include
COPY backend /work/backend
WORKDIR /work/backend
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
COPY --from=frontend-build /app/dist /usr/local/bin/web/dist
COPY deploy/backend.toml.example /etc/chronicle-engine-backend/backend.toml

USER chroniclebackend:chroniclebackend
EXPOSE 8080
WORKDIR /usr/local/bin
ENTRYPOINT ["/usr/local/bin/chronicle-engine-rs", "--config", "/etc/chronicle-engine-backend/backend.toml"]

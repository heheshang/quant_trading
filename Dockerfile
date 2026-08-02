# syntax=docker/dockerfile:1.7

FROM node:24-bookworm-slim AS frontend
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY . .
RUN npm run build

FROM rust:1.97-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    file \
    libayatana-appindicator3-dev \
    libgtk-3-dev \
    librsvg2-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    pkg-config \
    xvfb \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY src-tauri src-tauri
COPY --from=frontend /app/dist dist
RUN cargo build --release --jobs 2 --features custom-protocol \
    --bin quant-trading-system \
    --bin migrate-db

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    dbus \
    libasound2 \
    libayatana-appindicator3-1 \
    libcairo2 \
    libgdk-pixbuf-2.0-0 \
    libgtk-3-0 \
    libjavascriptcoregtk-4.1-0 \
    libsoup-3.0-0 \
    libwebkit2gtk-4.1-0 \
    libxdo3 \
    librsvg2-2 \
    procps \
    xauth \
    xvfb \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN mkdir -p /app/logs
COPY --from=builder /app/target/release/quant-trading-system /usr/local/bin/quant-trading-system
COPY --from=builder /app/target/release/migrate-db /usr/local/bin/migrate-db
COPY deploy/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["app"]

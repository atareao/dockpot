# ───── Build stage ─────
FROM rust:1.84-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev libgit2-dev zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/ .

RUN cargo build --release && \
    cp target/release/dockpot /dockpot

# ───── Frontend build stage ─────
FROM node:22-alpine AS frontend-builder

WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# ───── Runtime stage ─────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates docker-compose-plugin curl zip \
    libsqlite3-0 libgit2-1.7 \
    && rm -rf /var/lib/apt/lists/*

RUN addgroup --system dockpot && adduser --system --ingroup dockpot dockpot

WORKDIR /app
RUN mkdir -p data stacks && chown -R dockpot:dockpot /app

COPY --from=builder /dockpot /usr/local/bin/dockpot
COPY --from=frontend-builder /app/dist /app/frontend/dist
COPY backend/.env.example /app/.env.example

USER dockpot

EXPOSE 3056

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:3056/health || exit 1

ENTRYPOINT ["dockpot"]
CMD []
# ═══════════════════════════════════════════════════════════════
# Stage 1: Frontend (npm)
# ═══════════════════════════════════════════════════════════════
# MUST be first — backend embeds the dist at compile time via include_dir!
FROM docker.io/library/node:23-alpine AS frontend-builder

WORKDIR /build
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ═══════════════════════════════════════════════════════════════
# Stage 2: Backend (Rust)
# ═══════════════════════════════════════════════════════════════
FROM docker.io/library/rust:alpine3.23 AS backend-builder

RUN apk add --no-cache --update \
    build-base \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

WORKDIR /build

# Cache dependencies (avoid recompiling every time)
RUN cargo init --bin --name dockpot . && \
    mkdir -p src && \
    echo '// dummy' > src/lib.rs

COPY backend/Cargo.toml backend/Cargo.lock ./

# Frontend dist MUST be present before building deps — include_dir! embeds it at compile time
# Path resolves as $CARGO_MANIFEST_DIR/../frontend/dist = /build/../frontend/dist = /frontend/dist
COPY --from=frontend-builder /build/dist /frontend/dist

RUN cargo build --release && \
    rm -rf src

COPY backend/src ./src
COPY backend/migrations ./migrations
RUN touch src/main.rs src/lib.rs && \
    cargo build --release && \
    strip target/release/dockpot

# ═══════════════════════════════════════════════════════════════
# Stage 3: Runtime
# ═══════════════════════════════════════════════════════════════
FROM alpine:3.23

RUN apk add --no-cache \
    ca-certificates \
    curl \
    zip \
    docker-cli \
    docker-compose \
    && adduser -D -h /app -u 1000 app

WORKDIR /app
COPY --from=backend-builder /build/target/release/dockpot /usr/local/bin/dockpot
COPY --from=frontend-builder /build/dist ./dist

RUN mkdir -p /app/data /app/stacks && chown -R app:app /app

USER app
EXPOSE 3056

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:3056/health || exit 1

CMD ["dockpot"]
# syntax=docker/dockerfile:1.4
FROM dialect-coach-backend-infrastructure:0.1.0 AS base

# Stage: Planner
FROM base AS planner
COPY Cargo.toml Cargo.lock ./
COPY backend backend
COPY shared shared
COPY frontend frontend
RUN cargo chef prepare --recipe-path recipe.json

# Stage: Builder
FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-gnu --recipe-path recipe.json -p dialect-coach-backend

COPY . .
# Ensure libpdfium.so is available for build.rs
RUN mkdir -p backend/lib && cp /usr/local/lib/libpdfium.so backend/lib/
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin backend --bin admin

# Stage: Runtime (x86_64 to match the cross-compiled binary)
FROM --platform=linux/amd64 debian:trixie-20251208-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    openssl \
    libbz2-1.0 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/backend /usr/local/bin/dialect-coach-backend
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/admin /usr/local/bin/admin
COPY --from=builder /usr/local/lib/libpdfium.so /usr/local/lib/
COPY --from=builder /app/backend/vendor /app/backend/vendor
# serve_admin_html reads static/admin.html relative to WORKDIR
COPY --from=builder /app/backend/static /app/static
ENV LD_LIBRARY_PATH=/usr/local/lib
WORKDIR /app
CMD ["dialect-coach-backend"]

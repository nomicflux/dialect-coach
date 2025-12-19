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
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin backend --bin admin

# Stage: Runtime (x86_64 to match the cross-compiled binary)
FROM --platform=linux/amd64 debian:trixie-20251208-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    openssl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/backend /usr/local/bin/dialect-coach-backend
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/admin /usr/local/bin/admin
CMD ["dialect-coach-backend"]

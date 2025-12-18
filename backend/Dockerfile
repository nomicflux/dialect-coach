FROM lukemathwalker/cargo-chef:0.1.73-rust-1.92 AS chef
WORKDIR /app


# Stage: Infrastructure
# Purpose: Install heavy system dependencies. content-agnostic.
# STRICT ISOLATION: This stage has NO COPY instructions. It cannot be invalidated by source changes.
FROM chef AS infrastructure

RUN dpkg --add-architecture amd64 && \
    apt-get update && \
    apt-get install -y \
    gcc-x86-64-linux-gnu \
    g++-x86-64-linux-gnu \
    libc6-dev-amd64-cross \
    libstdc++-12-dev-amd64-cross \
    libssl-dev:amd64 \
    pkg-config

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH_X86_64_UNKNOWN_LINUX_GNU=/usr/lib/x86_64-linux-gnu/pkgconfig
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include

RUN rustup target add x86_64-unknown-linux-gnu

# Stage: Planner
# Purpose: Generate recipe.json (Volatile source code)
FROM chef AS planner
RUN cargo chef prepare --recipe-path recipe.json

# Stage: Builder
# Purpose: Build dependencies and binary (Uses Infrastructure + Recipe)
FROM infrastructure AS builder
COPY --from=planner /app/recipe.json recipe.json

COPY . .

# Cook dependencies
RUN cargo chef cook --release --target x86_64-unknown-linux-gnu --recipe-path recipe.json -p dialect-coach-backend

COPY . .
# Build backend
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin backend

# Stage: Runtime
FROM --platform=linux/amd64 debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    openssl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/backend /usr/local/bin/dialect-coach-backend
CMD ["dialect-coach-backend"]

# Build stage
FROM rust:1.92-slim as builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl g++

# Download pdfium for Linux - same version as macOS (chromium/7568)
# Source: https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium%2F7568
RUN curl -L -o /tmp/pdfium.tgz \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7568/pdfium-linux-x64.tgz" \
    && mkdir -p /app/backend/lib \
    && tar -xzf /tmp/pdfium.tgz -C /tmp \
    && cp /tmp/lib/libpdfium.so /app/backend/lib/

COPY . .
# Limit parallel compilation to reduce memory usage
ENV CARGO_BUILD_JOBS=2
RUN cargo build --release -p dialect-coach-backend

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy binary and pdfium library
COPY --from=builder /app/target/release/backend /usr/local/bin/
COPY --from=builder /app/target/release/admin /usr/local/bin/
COPY --from=builder /app/backend/lib/libpdfium.so /usr/local/lib/

# Ensure library is found at runtime
ENV LD_LIBRARY_PATH=/usr/local/lib

EXPOSE 3000
CMD ["backend"]

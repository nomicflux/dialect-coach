# Build stage - runs on the host architecture (fast, stable)
# We use $BUILDPLATFORM to avoid QEMU emulation during the heavy compilation step
FROM --platform=$BUILDPLATFORM rust:1.92-slim as builder

WORKDIR /app

# Install build dependencies and cross-compilation tools
# gcc-x86-64-linux-gnu is required for linking the x64 binary on non-x64 hosts
# We add amd64 architecture to install the target's OpenSSL headers (libssl-dev:amd64)
RUN dpkg --add-architecture amd64 && apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    g++ \
    gcc-x86-64-linux-gnu \
    g++-x86-64-linux-gnu \
    libc6-dev-amd64-cross \
    libssl-dev:amd64

# Add the x64 target explicitly - this ensures we produce an x64 binary regardless of host
RUN rustup target add x86_64-unknown-linux-gnu

# Configure cargo to use the correct linker for x64
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc

# Configure pkg-config to allow cross-compilation and find the x64 libraries
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig

# Download pdfium for Linux x64 (Strictly Reproducible: Always x64)
# Source: https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium%2F7568
RUN curl -L -o /tmp/pdfium.tgz \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7568/pdfium-linux-x64.tgz" \
    && mkdir -p /app/backend/lib \
    && tar -xzf /tmp/pdfium.tgz -C /tmp \
    && cp /tmp/lib/libpdfium.so /app/backend/lib/

COPY . .
# Limit parallel compilation to reduce memory usage
ENV CARGO_BUILD_JOBS=2
# Build specifically for the x64 target
RUN cargo build --release --target x86_64-unknown-linux-gnu -p dialect-coach-backend

# Runtime stage - Always x64
FROM --platform=linux/amd64 debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy binary from the x64 target directory
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/backend /usr/local/bin/
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/admin /usr/local/bin/
COPY --from=builder /app/backend/lib/libpdfium.so /usr/local/lib/

# Ensure library is found at runtime
ENV LD_LIBRARY_PATH=/usr/local/lib

EXPOSE 3000
CMD ["backend"]

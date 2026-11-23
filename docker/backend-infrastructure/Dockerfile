FROM lukemathwalker/cargo-chef:0.1.73-rust-1.92

# Native build dependencies
RUN apt-get update && \
    apt-get install -y \
    build-essential \
    libssl-dev \
    libbz2-dev \
    pkg-config \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Download PDFium for the image architecture
# Source: https://github.com/bblanchon/pdfium-binaries/releases/tag/chromium%2F7568
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      arm64) PDFIUM_ARCH=arm64 ;; \
      amd64) PDFIUM_ARCH=x64 ;; \
      *) echo "Unsupported PDFium architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac \
    && curl -fL -o /tmp/pdfium.tgz \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7568/pdfium-linux-${PDFIUM_ARCH}.tgz" \
    && mkdir -p /tmp/pdfium \
    && tar -xzf /tmp/pdfium.tgz -C /tmp/pdfium \
    && cp /tmp/pdfium/lib/libpdfium.so /usr/local/lib/ \
    && rm -rf /tmp/pdfium /tmp/pdfium.tgz

WORKDIR /app

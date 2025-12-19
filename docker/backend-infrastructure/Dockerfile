FROM lukemathwalker/cargo-chef:0.1.73-rust-1.92

# Cross-compilation packages (these package names work with cargo-chef's Debian base)
RUN dpkg --add-architecture amd64 && \
    apt-get update && \
    apt-get install -y \
    gcc-x86-64-linux-gnu \
    g++-x86-64-linux-gnu \
    libc6-dev-amd64-cross \
    libstdc++-12-dev-amd64-cross \
    libssl-dev:amd64 \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV PKG_CONFIG_PATH_X86_64_UNKNOWN_LINUX_GNU=/usr/lib/x86_64-linux-gnu/pkgconfig
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include

RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app

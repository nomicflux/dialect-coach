# Build stage
FROM rust:1.92-slim as builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk
WORKDIR /app
COPY . .
# Limit parallel compilation to reduce memory usage
ENV CARGO_BUILD_JOBS=2
RUN cd frontend && trunk build --release

# Serve stage
FROM nginx:alpine
COPY --from=builder /app/frontend/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80

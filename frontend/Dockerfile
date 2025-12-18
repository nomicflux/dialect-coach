FROM lukemathwalker/cargo-chef:0.1.73-rust-1.92 AS chef
WORKDIR /app



# Stage: Infrastructure
FROM chef AS infrastructure

RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM infrastructure AS builder
# Trunk and target installed in infrastructure stage

COPY --from=planner /app/recipe.json recipe.json
# Install trunk and target in builder stage to ensure caching

# Cook dependencies for frontend
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json -p dialect-coach-frontend

COPY . .
# Build frontend
WORKDIR /app/frontend
RUN trunk build --release

FROM nginx:alpine AS runtime
COPY --from=builder /app/frontend/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80

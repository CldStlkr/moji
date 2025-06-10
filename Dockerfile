# Install cargo-chef
FROM rust:latest AS chef
RUN cargo install cargo-chef

# Plan the build
FROM chef AS planner
WORKDIR /usr/src
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Backend build stage
FROM chef AS backend-builder
WORKDIR /usr/src
COPY --from=planner /usr/src/recipe.json recipe.json
# Build dependencies first (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json --bin moji-server
# Copy source and build
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin moji-server

# Frontend build stage  
FROM chef AS frontend-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk
WORKDIR /usr/src
COPY --from=planner /usr/src/recipe.json recipe.json
# Build dependencies first (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json --bin moji-frontend
# Copy source and build
COPY . .
WORKDIR /usr/src/frontend
RUN trunk build --release

# Final stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /usr/src/target/release/moji-server /usr/local/bin/
COPY --from=backend-builder /usr/src/data /usr/local/data
COPY --from=frontend-builder /usr/src/frontend/dist /usr/local/dist

WORKDIR /usr/local
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV PRODUCTION=1
EXPOSE 8080
CMD ["/usr/local/bin/moji-server"]

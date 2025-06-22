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
RUN cargo chef cook --recipe-path recipe.json --bin moji-server
# Copy source and build
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --bin moji-server

# Frontend build stage  
FROM chef AS frontend-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk wasm-bindgen-cli # Pre-install wasm to avoid 503 errors

# Install Bun
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

WORKDIR /usr/src
COPY --from=planner /usr/src/recipe.json recipe.json

# Copy package.json and bun.lock and install dependencies first (for better caching)
COPY frontend/package.json frontend/bun.lock ./frontend/
WORKDIR /usr/src/frontend
RUN bun install

# Build dependencies first (cached layer)
WORKDIR /usr/src
RUN cargo chef cook --recipe-path recipe.json --bin moji-frontend

# Copy source and build
COPY . .

# Build CSS first using Bun with Tailwind v4
WORKDIR /usr/src/frontend
RUN bunx tailwindcss -i ./input.css -o ./styles.css

# Build the frontend
RUN trunk build

# Final stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /usr/src/target/debug/moji-server /usr/local/bin/
COPY --from=backend-builder /usr/src/data /usr/local/data
COPY --from=frontend-builder /usr/src/frontend/dist /usr/local/dist

WORKDIR /usr/local
ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1
ENV PRODUCTION=0
EXPOSE 8080
CMD ["/usr/local/bin/moji-server"]

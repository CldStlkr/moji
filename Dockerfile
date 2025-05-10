# Backend build stage
FROM rust:latest AS backend-builder
WORKDIR /usr/src/backend

# Copy only backend files
COPY backend/Cargo.toml ./
COPY backend/src ./src
COPY backend/.sqlx ./.sqlx
COPY backend/migrations ./migrations

# Set offline mode for sqlx
ENV SQLX_OFFLINE=true

# Build the backend directly
RUN cargo build --release

# Frontend build stage
FROM rust:latest AS frontend-builder

# Install wasm target and trunk
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

WORKDIR /usr/src/frontend

# Copy frontend files
COPY frontend/Cargo.toml ./
COPY frontend/src ./src
COPY frontend/css ./css
COPY frontend/index.html ./
COPY frontend/Trunk.toml ./

# Build the frontend
RUN trunk build --release

# Final stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

# Copy the backend binary
COPY --from=backend-builder /usr/src/backend/target/release/kanji-guesser-server /usr/local/bin/

# Copy data files
COPY data /usr/local/data

# Copy frontend build
COPY --from=frontend-builder /usr/src/frontend/dist /usr/local/dist

WORKDIR /usr/local

ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV PRODUCTION=1

EXPOSE 8080

CMD ["/usr/local/bin/kanji-guesser-server"]

# Backend build stage
FROM rust:latest AS backend-builder
WORKDIR /usr/src/kanji_guesser
# Copy workspace files
COPY Cargo.toml ./
COPY backend/Cargo.toml ./backend/
COPY backend/src ./backend/src
COPY backend/.sqlx ./backend/.sqlx
COPY backend/migrations ./backend/migrations
COPY data ./backend/data

# Set offline mode for sqlx
ENV SQLX_OFFLINE=true
# Build the backend
WORKDIR /usr/src/kanji_guesser/backend
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
COPY --from=backend-builder /usr/src/kanji_guesser/backend/target/release/kanji-guesser-server /usr/local/bin/
# Copy data files
COPY --from=backend-builder /usr/src/kanji_guesser/backend/data /usr/local/data
# Copy frontend build
COPY --from=frontend-builder /usr/src/frontend/dist /usr/local/dist
WORKDIR /usr/local
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV PRODUCTION=1
EXPOSE 8080
CMD ["/usr/local/bin/kanji-guesser-server"]

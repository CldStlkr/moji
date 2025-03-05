FROM rust:latest AS backend-builder
WORKDIR /usr/src/kanji_guesser
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY data ./data
COPY src ./src
ENV SQLX_OFFLINE=true
RUN cargo build --release

# Frontend build stage
FROM docker.io/oven/bun:latest AS frontend-builder
WORKDIR /usr/src/frontend
COPY frontend/package.json frontend/bun.lock ./
COPY frontend ./
RUN bun install && bun run build

# Use a newer Debian version (bookworm) that has glibc 2.34
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*
COPY --from=backend-builder /usr/src/kanji_guesser/target/release/kanji_guesser /usr/local/bin/
COPY --from=backend-builder /usr/src/kanji_guesser/data /usr/local/data
WORKDIR /usr/local
COPY --from=frontend-builder /usr/src/frontend/dist ./static
COPY static ./static
ENV RUST_LOG=info
# Add this to help with debugging
ENV RUST_BACKTRACE=1
EXPOSE 8080
CMD ["/usr/local/bin/kanji_guesser"]

# Frontend build
FROM oven/bun:slim AS frontend-builder
WORKDIR /app

COPY package.json bun.lock svelte.config.ts vite.config.ts tsconfig.json ./
COPY src/client ./src/client
COPY static ./static
RUN bun ci && bun run build

# Build stage with cargo-chef for dependency caching
FROM rust:1.90-slim AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Prepare recipe
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src/server ./src/server
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies (this layer is cached)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY Cargo.toml Cargo.lock ./
COPY src/server ./src/server
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim
WORKDIR /app

# Install CA certificates for HTTPS
RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

# Rust binary
COPY --from=builder /app/target/release/blackfiles /app/blackfiles

# Built frontend
COPY --from=frontend-builder /app/build /app/static

# Expose port
EXPOSE 8000

# Run the application
CMD ["/app/blackfiles"]

FROM oven/bun:1 AS frontend-builder
WORKDIR /frontend
COPY frontend/ ./
RUN bun ci && bun run build

# Build stage with cargo-chef for dependency caching
FROM rust:1.90-slim AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Prepare recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies (this layer is cached)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

# Install CA certificates for HTTPS
RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/blackfiles /app/blackfiles

COPY --from=frontend-builder /frontend/dist /app/static

# Expose port
EXPOSE 8000

# Run the application
CMD ["/app/blackfiles"]

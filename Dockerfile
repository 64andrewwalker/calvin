# Calvin - Multi-stage build for minimal image
FROM rust:1.84-slim-bookworm AS builder

WORKDIR /build

# Install minimal build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY examples ./examples

# Build release binary
RUN cargo build --release

# Run tests
RUN cargo test --release

# Runtime stage - minimal image
FROM debian:bookworm-slim

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/calvin /usr/local/bin/calvin

# Copy example .promptpack for testing
COPY --from=builder /build/examples/.promptpack /app/.promptpack

# Verify binary works
RUN calvin --version

# Default command
CMD ["calvin", "--help"]

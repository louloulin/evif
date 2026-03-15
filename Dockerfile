# =============================================================================
# EVIF Production Dockerfile
# =============================================================================
# Multi-stage build for production deployment
# - Stage 1: Build the Rust application
# - Stage 2: Create minimal runtime image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build
# -----------------------------------------------------------------------------
FROM rust:1.75-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY benches ./benches
COPY examples ./examples

# Build dependencies first (for better caching)
RUN cargo build --release --workspace

# Copy source and build
COPY . .
RUN cargo build --release -p evif-rest

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --gid 1000 evif && \
    useradd --uid 1000 --gid evif --shell /bin/false --create-home evif

# Create directories for data
RUN mkdir -p /data /var/log/evif && \
    chown -R evif:evif /data /var/log/evif

# Copy binary from builder
COPY --from=builder /build/target/release/evif-rest /usr/local/bin/evif-rest
COPY --from=builder /build/target/release/evif-cli /usr/local/bin/evif-cli

# Copy example configuration
COPY --from=builder /build/crates/evif-rest/examples/*.toml /etc/evif/ 2>/dev/null || true

# Switch to non-root user
USER evif

# Set working directory
WORKDIR /data

# Expose ports
# - 8081: REST API
# - 8082: gRPC (if enabled)
EXPOSE 8081 8082

# Use tini as init system
ENTRYPOINT ["/usr/bin/tini", "--"]

# Default command (can be overridden with environment variables)
CMD ["evif-rest"]

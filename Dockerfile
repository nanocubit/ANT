# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY tests ./tests
COPY benches ./benches

# Build release with all features
RUN cargo build --release --features "metrics,distributed,structured-logs"

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 antos

# Copy binaries from builder
COPY --from=builder /app/target/release/ant /usr/local/bin/ant
COPY --from=builder /app/target/release/antctl /usr/local/bin/antctl

# Create data directories
RUN mkdir -p /data/skills /data/sandbox_data /data/logs \
    && chown -R antos:antos /data

WORKDIR /data

# Switch to non-root user
USER antos

# Expose metrics port
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s \
    CMD antctl health || exit 1

# Run the application
CMD ["ant"]

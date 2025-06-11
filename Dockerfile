# Pierre MCP Server - Production Dockerfile
# Multi-stage build for optimized production container

# Build stage - use x86_64 for better Apple Silicon compatibility
FROM --platform=linux/amd64 rust:1.83-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -m -u 1001 appuser

# Set working directory
WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to build dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "// Dummy lib.rs" > src/lib.rs && \
    echo "fn main() {}" > src/bin/multitenant_server.rs

# Build dependencies (this layer will be cached unless Cargo.toml changes)
RUN cargo build --release --bin pierre-mcp-server
RUN rm src/main.rs src/lib.rs src/bin/multitenant_server.rs

# Copy source code
COPY src ./src
COPY tests ./tests

# Build the actual application
RUN cargo build --release --bin pierre-mcp-server

# Verify the binary was built
RUN ls -la target/release/pierre-mcp-server

# Runtime stage - use x86_64 for better Apple Silicon compatibility
FROM --platform=linux/amd64 debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    curl \
    bash \
    strace \
    file \
    libc-bin \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user and directories
RUN useradd -m -u 1001 appuser \
    && mkdir -p /app/data \
    && chown -R appuser:appuser /app

# Copy binary and entrypoint script from builder stage
COPY --from=builder /app/target/release/pierre-mcp-server /app/pierre-mcp-server
COPY docker-entrypoint.sh /app/docker-entrypoint.sh

# Make binary and entrypoint executable and owned by appuser
RUN chmod +x /app/pierre-mcp-server \
    && chmod +x /app/docker-entrypoint.sh \
    && chown appuser:appuser /app/pierre-mcp-server \
    && chown appuser:appuser /app/docker-entrypoint.sh \
    && echo "Binary file type:" \
    && file /app/pierre-mcp-server \
    && echo "Binary dependencies:" \
    && ldd /app/pierre-mcp-server || echo "ldd failed, static binary or missing libs"

# Switch to non-root user
USER appuser

# Set working directory
WORKDIR /app

# Set environment variables with secure defaults
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:./data/users.db
ENV ENCRYPTION_KEY_PATH=./data/encryption.key
ENV JWT_SECRET_PATH=./data/jwt.secret
ENV MCP_PORT=8080
ENV HTTP_PORT=8081

# Expose ports
EXPOSE 8080 8081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Copy .envrc for environment configuration
COPY .envrc /app/.envrc

# Use entrypoint script for environment loading
ENTRYPOINT ["/app/docker-entrypoint.sh"]

# Default command
CMD ["/app/pierre-mcp-server"]
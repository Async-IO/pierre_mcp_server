# Dockerfile for testing database plugins with PostgreSQL
FROM rust:1.82-bullseye

# Install system dependencies
RUN apt-get update && apt-get install -y \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --features postgresql --release && rm -rf src

# Copy source code
COPY . .

# Build the application with PostgreSQL features
RUN cargo build --features postgresql --release

# Default command runs tests
CMD ["cargo", "test", "--features", "postgresql", "--", "--test-threads=1"]
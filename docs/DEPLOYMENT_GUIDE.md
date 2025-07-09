# Deployment Guide

Complete guide for production deployment, architecture, and testing.

## Table of Contents

1. [Production Deployment](#production-deployment)
2. [Architecture Overview](#architecture-overview)
3. [Testing & Development](#testing--development)

## Production Deployment

### Docker Compose Deployment

```yaml
# docker-compose.yml
version: '3.8'
services:
  pierre-server:
    image: pierre-mcp-server:latest
    ports:
      - "8080:8080"  # MCP Protocol
      - "8081:8081"  # HTTP API
    environment:
      - STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID}
      - STRAVA_CLIENT_SECRET=${STRAVA_CLIENT_SECRET}
      - DATABASE_URL=postgresql://user:pass@db:5432/pierre
      - JWT_SECRET=${JWT_SECRET}
      - ENCRYPTION_KEY=${ENCRYPTION_KEY}
    volumes:
      - ./data:/app/data
    depends_on:
      - db
    restart: unless-stopped

  db:
    image: postgres:16
    environment:
      - POSTGRES_DB=pierre
      - POSTGRES_USER=pierre_user
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - pierre-server
    restart: unless-stopped

volumes:
  postgres_data:
```

### Single Command Deployment

```bash
# Docker deployment with SQLite
docker run -d \
  -p 8080:8080 -p 8081:8081 \
  -e STRAVA_CLIENT_ID=your_client_id \
  -e STRAVA_CLIENT_SECRET=your_client_secret \
  -e DATABASE_URL=sqlite:./data/pierre.db \
  -e JWT_SECRET=your_jwt_secret_here \
  --name pierre-fitness \
  --volume pierre-data:/app/data \
  pierre-mcp-server:latest
```

### Local Development

```bash
# For individual users running locally
cargo run --bin pierre-mcp-server -- \
  --port 8080 \
  --http-port 8081 \
  --database-url sqlite:./my-fitness.db
```

### Kubernetes Deployment

```yaml
# kubernetes-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pierre-mcp-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pierre-mcp-server
  template:
    metadata:
      labels:
        app: pierre-mcp-server
    spec:
      containers:
      - name: pierre-mcp-server
        image: pierre-mcp-server:latest
        ports:
        - containerPort: 8080
        - containerPort: 8081
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: database-url
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: jwt-secret
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: pierre-mcp-service
spec:
  selector:
    app: pierre-mcp-server
  ports:
  - name: mcp
    port: 8080
    targetPort: 8080
  - name: http
    port: 8081
    targetPort: 8081
  type: ClusterIP
```

### Environment Variables for Production

```bash
# Core Configuration
MCP_PORT=8080
HTTP_PORT=8081
HOST=0.0.0.0
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre
# OR for SQLite
DATABASE_URL=sqlite:./data/pierre.db

# Security (Required)
JWT_SECRET=your_very_secure_jwt_secret_minimum_32_chars
ENCRYPTION_KEY=your_32_byte_encryption_key_for_aes_256

# OAuth Providers
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
FITBIT_CLIENT_ID=your_fitbit_client_id
FITBIT_CLIENT_SECRET=your_fitbit_client_secret

# External Services
OPENWEATHER_API_KEY=your_openweather_api_key

# Logging & Monitoring
RUST_LOG=info
LOG_FORMAT=json
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:14268/api/traces

# Performance Tuning
WORKER_THREADS=4
MAX_CONNECTIONS=1000
REQUEST_TIMEOUT=30
```

## Architecture Overview

### System Architecture

Pierre MCP Server follows a modular, plugin-based architecture designed for scalability and extensibility.

#### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │    │   Web Client    │    │   A2A Client    │
│   (Claude)      │    │   (Browser)     │    │   (Agent)       │
└─────┬───────────┘    └─────┬───────────┘    └─────┬───────────┘
      │                      │                      │
      │ JSON-RPC             │ HTTP/WebSocket       │ A2A Protocol
      │                      │                      │
┌─────▼──────────────────────▼──────────────────────▼───────────┐
│                    Pierre MCP Server                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  MCP Tools  │  │ HTTP Routes │  │ A2A Handler │          │
│  │  (21 tools) │  │ (REST API)  │  │ (Agent API) │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │ Intelligence│  │    Auth     │  │  Rate Limit │          │
│  │   Engine    │  │  Manager    │  │   Manager   │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Fitness   │  │   Weather   │  │   Location  │          │
│  │  Providers  │  │  Service    │  │   Service   │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  Database   │  │   Config    │  │   Crypto    │          │
│  │  Plugins    │  │  Manager    │  │   Manager   │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
           │                    │                    │
    ┌──────▼──────┐    ┌────────▼────────┐    ┌─────▼─────┐
    │  SQLite/    │    │  Configuration  │    │   File    │
    │ PostgreSQL  │    │     Storage     │    │  Storage  │
    └─────────────┘    └─────────────────┘    └───────────┘
```

#### Database Architecture

The system uses a plugin-based database architecture supporting multiple backends:

- **SQLite**: For development and single-user deployments
- **PostgreSQL**: For production multi-user deployments
- **Plugin System**: Extensible to support additional databases

#### Security Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Security Layers                        │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │    JWT      │  │  API Keys   │  │    A2A      │          │
│  │    Auth     │  │    Auth     │  │    Auth     │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │ Rate Limit  │  │ Encryption  │  │    CORS     │          │
│  │  Manager    │  │ (AES-256)   │  │   Policy    │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   OAuth     │  │   Audit     │  │   Input     │          │
│  │  Handling   │  │   Logging   │  │ Validation  │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

#### Intelligence Engine Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                Intelligence Processing Pipeline             │
├─────────────────────────────────────────────────────────────┤
│  Data Ingestion → Analysis → Intelligence → Recommendations │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Activity  │  │ Performance │  │    Goal     │          │
│  │  Analyzer   │  │  Analyzer   │  │   Engine    │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │   Weather   │  │   Location  │  │    Trend    │          │
│  │ Correlation │  │ Intelligence│  │   Analysis  │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
```

### Scalability Considerations

#### Horizontal Scaling

- **Stateless Design**: All services are stateless for easy horizontal scaling
- **Database Sharding**: Support for database sharding by user/tenant
- **Load Balancing**: Standard HTTP load balancing with session affinity

#### Performance Optimization

- **Connection Pooling**: Database connection pooling for efficient resource usage
- **Async Processing**: Full async/await implementation for non-blocking operations
- **Batch Processing**: Efficient batch processing for bulk operations
- **Memory Management**: Careful memory management with streaming for large datasets

## Testing & Development

### Testing Strategy

#### Unit Tests
```bash
# Run all unit tests
cargo test

# Run specific module tests
cargo test intelligence::

# Run with coverage
cargo test --features coverage
```

#### Integration Tests
```bash
# Run integration tests
cargo test --test integration

# Run specific integration test
cargo test --test intelligence_comprehensive_test
```

#### End-to-End Tests
```bash
# Run E2E tests with real providers (requires OAuth setup)
cargo test --test e2e -- --ignored

# Run MCP protocol tests
cargo test --test mcp_protocol_test
```

#### Performance Tests
```bash
# Load testing with k6
k6 run scripts/load-test.js

# Memory profiling
cargo run --bin pierre-mcp-server --features profiling
```

### Development Environment

#### Local Setup
```bash
# Clone and setup
git clone https://github.com/your-org/pierre-mcp-server
cd pierre-mcp-server

# Install dependencies
cargo build

# Setup environment
cp .env.example .env
# Edit .env with your OAuth credentials

# Run development server
cargo run --bin pierre-mcp-server
```

#### Docker Development
```bash
# Build development image
docker build -t pierre-mcp-server:dev .

# Run with development overrides
docker-compose -f docker-compose.dev.yml up
```

#### Testing with Mock Data
```bash
# Run with mock providers for testing
MOCK_PROVIDERS=true cargo run --bin pierre-mcp-server

# Test server is running
curl http://localhost:8081/health
```

### Continuous Integration

#### GitHub Actions Pipeline
```yaml
name: CI/CD Pipeline
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Run tests
      run: cargo test
    - name: Run clippy
      run: cargo clippy -- -D warnings
    - name: Check formatting
      run: cargo fmt -- --check

  security:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Security audit
      run: cargo audit
    - name: Dependency check
      run: cargo deny check

  build:
    runs-on: ubuntu-latest
    needs: [test, security]
    steps:
    - name: Build Docker image
      run: docker build -t pierre-mcp-server:${{ github.sha }} .
    - name: Push to registry
      run: docker push pierre-mcp-server:${{ github.sha }}
```

This deployment guide provides comprehensive instructions for deploying Pierre MCP Server in various environments, from local development to production Kubernetes clusters.
# Deployment Guide

Complete guide for production deployment, architecture, testing, and B2B customer provisioning.

## Table of Contents

1. [Production Deployment](#production-deployment)
2. [Architecture Overview](#architecture-overview)
3. [Testing & Development](#testing--development)
4. [B2B Customer Provisioning](#b2b-customer-provisioning)

## Production Deployment

### SaaS Platform (Multi-Tenant)

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
      - redis
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

  redis:
    image: redis:7-alpine
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

### Enterprise Self-Hosted

```bash
# Single command deployment
docker run -d \\
  -p 8080:8080 -p 8081:8081 \\
  -e STRAVA_CLIENT_ID=your_client_id \\
  -e STRAVA_CLIENT_SECRET=your_client_secret \\
  -e DATABASE_URL=sqlite:./data/enterprise.db \\
  -e JWT_SECRET=your_jwt_secret_here \\
  --name pierre-fitness \\
  --volume pierre-data:/app/data \\
  pierre-mcp-server:latest
```

### Personal Instance (Single-Tenant)

```bash
# For individual users running locally
cargo run --bin pierre-mcp-server -- \\
  --port 8080 \\
  --http-port 8081 \\
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
│  │  Database   │  │    Cache    │  │   Crypto    │          │
│  │  Plugins    │  │   Manager   │  │   Manager   │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
└─────────────────────────────────────────────────────────────┘
           │                    │                    │
    ┌──────▼──────┐    ┌────────▼────────┐    ┌─────▼─────┐
    │  SQLite/    │    │     Redis       │    │   File    │
    │ PostgreSQL  │    │     Cache       │    │  Storage  │
    └─────────────┘    └─────────────────┘    └───────────┘
```

#### Database Architecture

The system uses a plugin-based database architecture supporting multiple backends:

- **SQLite**: For development and single-tenant deployments
- **PostgreSQL**: For production multi-tenant deployments
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
│  │  (Tiered)   │  │ (AES-256)   │  │   Policy    │          │
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
- **Caching Strategy**: Redis for session management and data caching

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

# Test with sample data
cargo run --bin test-with-data
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

## B2B Customer Provisioning

### Customer Onboarding Workflow

#### 1. Initial Customer Setup

```bash
# Create new customer tenant
cargo run --bin admin-setup -- create-tenant \\
  --name "Acme Fitness Co" \\
  --contact-email "admin@acmefitness.com" \\
  --tier "enterprise"
```

#### 2. API Key Provisioning

```bash
# Generate production API key
curl -X POST http://localhost:8081/admin/provision-api-key \\
  -H "Authorization: Bearer $ADMIN_JWT" \\
  -H "Content-Type: application/json" \\
  -d '{
    "user_email": "admin@acmefitness.com",
    "name": "Acme Production Key",
    "tier": "enterprise",
    "expires_in_days": 365,
    "description": "Production API key for Acme Fitness integration"
  }'
```

#### 3. OAuth Provider Setup

Customers need to configure their own OAuth applications:

**Strava Setup:**
1. Create application at https://www.strava.com/settings/api
2. Set redirect URI: `https://customer-domain.com/oauth/callback/strava`
3. Provide client ID/secret to customer

**Fitbit Setup:**
1. Create application at https://dev.fitbit.com/apps
2. Set redirect URI: `https://customer-domain.com/oauth/callback/fitbit`
3. Provide client ID/secret to customer

#### 4. Custom Domain Configuration

```nginx
# nginx configuration for customer subdomain
server {
    listen 443 ssl;
    server_name acme.pierre-fitness.com;
    
    ssl_certificate /etc/ssl/certs/acme.pierre-fitness.com.crt;
    ssl_certificate_key /etc/ssl/private/acme.pierre-fitness.com.key;
    
    location / {
        proxy_pass http://pierre-backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Customer-ID "acme-fitness";
    }
}
```

### Enterprise Integration Support

#### White-Label Deployment

```yaml
# Customer-specific deployment
version: '3.8'
services:
  pierre-server:
    image: pierre-mcp-server:enterprise
    environment:
      - CUSTOMER_ID=acme-fitness
      - BRANDING_LOGO_URL=https://acmefitness.com/logo.png
      - BRANDING_THEME_COLOR=#FF6B35
      - CUSTOM_DOMAIN=fitness-api.acmefitness.com
    volumes:
      - ./customer-config:/app/config
```

#### SSO Integration

```bash
# Configure SAML SSO for enterprise customer
cargo run --bin admin-setup -- configure-sso \\
  --customer-id acme-fitness \\
  --sso-provider saml \\
  --metadata-url "https://acmefitness.okta.com/metadata" \\
  --entity-id "acme-fitness-pierre"
```

#### Monitoring & Analytics

```yaml
# Customer-specific monitoring
services:
  pierre-server:
    environment:
      - CUSTOMER_ANALYTICS_ENABLED=true
      - CUSTOMER_DASHBOARD_URL=https://dashboard.acmefitness.com
      - WEBHOOK_URL=https://acmefitness.com/webhooks/pierre
```

### Billing & Usage Tracking

#### Usage Monitoring

```sql
-- Customer usage query
SELECT 
  date_trunc('month', created_at) as month,
  count(*) as api_calls,
  customer_id
FROM api_usage_logs 
WHERE customer_id = 'acme-fitness'
  AND created_at >= date_trunc('month', CURRENT_DATE - interval '12 months')
GROUP BY month, customer_id
ORDER BY month DESC;
```

#### Automated Billing Integration

```python
# Example billing webhook handler
@app.route('/billing/usage-report', methods=['POST'])
def usage_report():
    customer_id = request.json.get('customer_id')
    month = request.json.get('month')
    
    usage = get_customer_usage(customer_id, month)
    
    # Send to billing system
    billing_client.record_usage(
        customer_id=customer_id,
        month=month,
        api_calls=usage['total_calls'],
        premium_features=usage['premium_usage']
    )
    
    return {'status': 'recorded'}
```

### Support & Maintenance

#### Customer Support Tools

```bash
# Debug customer issues
cargo run --bin admin-setup -- debug \\
  --customer-id acme-fitness \\
  --api-key pk_live_acme123... \\
  --check-all

# Generate support report
cargo run --bin admin-setup -- support-report \\
  --customer-id acme-fitness \\
  --days 7
```

#### Health Monitoring

```bash
# Customer-specific health checks
curl -H "X-Customer-ID: acme-fitness" \\
  http://localhost:8081/health/customer

# Response
{
  "status": "healthy",
  "customer_id": "acme-fitness",
  "api_key_status": "active",
  "oauth_connections": {
    "strava": "connected",
    "fitbit": "expired"
  },
  "usage_status": {
    "current_month": 45230,
    "limit": 100000,
    "percentage": 45.23
  }
}
```

This completes the comprehensive deployment guide covering all aspects of production deployment, architecture, testing, and B2B customer provisioning for the Pierre Fitness API platform.
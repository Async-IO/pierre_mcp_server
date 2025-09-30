# Deployment and Operations Guide

Production deployment guide for Pierre MCP Server covering infrastructure setup, monitoring, backup strategies, and operational procedures.

## Table of Contents

1. [Production Architecture](#production-architecture)
2. [Database Selection](#database-selection)
3. [Environment Configuration](#environment-configuration)
4. [Deployment Methods](#deployment-methods)
5. [TLS/HTTPS Setup](#tlshttps-setup)
6. [Monitoring and Health Checks](#monitoring-and-health-checks)
7. [Backup and Restore](#backup-and-restore)
8. [Performance Tuning](#performance-tuning)
9. [Security Hardening](#security-hardening)
10. [Operational Procedures](#operational-procedures)

## Production Architecture

### Single Server Deployment

Recommended for small-medium workloads (<1000 users, <10000 requests/day):

```
┌─────────────────┐
│  Load Balancer  │
│  (nginx/Apache) │
└────────┬────────┘
         │ HTTPS
         ↓
┌────────────────────┐
│  Pierre MCP Server │
│  Port 8081         │
│  - MCP Protocol    │
│  - OAuth 2.0       │
│  - REST API        │
│  - SSE Events      │
└────────┬───────────┘
         │
         ↓
┌────────────────────┐
│  PostgreSQL DB     │
│  Port 5432         │
└────────────────────┘
```

### High Availability Deployment

Recommended for large workloads (>1000 users, >10000 requests/day):

```
┌─────────────────┐
│  Load Balancer  │
│  (HAProxy/ALB)  │
└───┬─────────┬───┘
    │         │
    ↓         ↓
┌───────┐ ┌───────┐
│Pierre │ │Pierre │
│Node 1 │ │Node 2 │
└───┬───┘ └───┬───┘
    │         │
    └────┬────┘
         │
         ↓
┌─────────────────┐
│ PostgreSQL HA   │
│ Primary+Replica │
└─────────────────┘
         │
         ↓
┌─────────────────┐
│ Redis (Session) │
│ Optional Cache  │
└─────────────────┘
```

## Database Selection

### SQLite vs PostgreSQL

**SQLite** (Development/Small Deployments):
- Simple setup, single file
- No separate database server
- Limited to ~100 concurrent connections
- No replication support
- File size limit: ~140TB (theoretical)

**PostgreSQL** (Production/Large Deployments):
- Scalable, handles thousands of connections
- Supports replication and high availability
- Advanced features (JSONB, full-text search)
- Industry-standard for production systems
- Better concurrent write performance

**Migration Path**: Start with SQLite for development/testing, migrate to PostgreSQL for production (see [Migration section](#sqlite-to-postgresql-migration)).

### PostgreSQL Production Setup

#### Installation

**Ubuntu/Debian**:
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl enable postgresql
sudo systemctl start postgresql
```

**macOS**:
```bash
brew install postgresql@15
brew services start postgresql@15
```

#### Database and User Creation

```bash
# Switch to postgres user
sudo -u postgres psql

# Create database and user
CREATE DATABASE pierre_mcp_server;
CREATE USER pierre WITH ENCRYPTED PASSWORD 'secure_password_here';
GRANT ALL PRIVILEGES ON DATABASE pierre_mcp_server TO pierre;

# Enable required extensions
\c pierre_mcp_server
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- For fuzzy text search

\q
```

#### Connection Configuration

Pierre uses SQLx connection pooling (src/database_plugins/postgres.rs:96-105):

**Environment Variables**:
```bash
# PostgreSQL connection string
DATABASE_URL="postgresql://pierre:secure_password@localhost:5432/pierre_mcp_server"

# Connection pool tuning (optional)
POSTGRES_MAX_CONNECTIONS=10        # Maximum connections (default: 10 prod, 5 CI)
POSTGRES_MIN_CONNECTIONS=2         # Minimum idle connections (default: 0)
POSTGRES_ACQUIRE_TIMEOUT=30        # Connection acquisition timeout seconds (default: 30)
```

**Connection Pool Defaults** (src/database_plugins/postgres.rs:28-34):
- Production: `max_connections=10, min_connections=0, timeout=30s`
- CI/Testing: `max_connections=5, min_connections=1, timeout=30s`
- Idle timeout: `300s` (5 minutes)
- Automatically adjusted based on `CI` environment variable

#### Performance Tuning

Edit `/etc/postgresql/15/main/postgresql.conf`:

```ini
# Memory configuration (adjust based on available RAM)
shared_buffers = 256MB                # 25% of RAM for database server
effective_cache_size = 1GB            # 50% of RAM
work_mem = 16MB                       # Per-operation memory
maintenance_work_mem = 128MB          # For VACUUM, CREATE INDEX

# Connection settings
max_connections = 100                 # Maximum concurrent connections

# Write-ahead log
wal_buffers = 16MB
checkpoint_completion_target = 0.9

# Query planner
random_page_cost = 1.1                # For SSD storage
effective_io_concurrency = 200        # For SSD storage

# Logging (for monitoring)
log_statement = 'mod'                 # Log INSERT/UPDATE/DELETE
log_duration = on
log_min_duration_statement = 1000     # Log queries > 1 second
```

Restart PostgreSQL:
```bash
sudo systemctl restart postgresql
```

## Environment Configuration

### Production Environment Variables

Create `/etc/pierre-mcp/.env` (DO NOT commit to git):

```bash
# Core Configuration
DATABASE_URL="postgresql://pierre:REDACTED@localhost:5432/pierre_mcp_server"
PIERRE_MASTER_ENCRYPTION_KEY="REDACTED_BASE64_KEY_HERE"  # openssl rand -base64 32

# Server Configuration
HTTP_PORT=8081                        # Single unified port for all protocols
HOST="pierre.example.com"            # Production hostname

# JWT Configuration (Managed by Database)
# Note: JWT secrets are stored in database (admin_jwt_secret table)
# No JWT_SECRET environment variable required
JWT_EXPIRY_HOURS=24                  # Access token TTL

# OAuth 2.0 Providers (Fitness Data Integration)
STRAVA_CLIENT_ID="your_strava_client_id"
STRAVA_CLIENT_SECRET="REDACTED"
STRAVA_REDIRECT_URI="https://pierre.example.com/api/oauth/callback/strava"

FITBIT_CLIENT_ID="your_fitbit_client_id"
FITBIT_CLIENT_SECRET="REDACTED"
FITBIT_REDIRECT_URI="https://pierre.example.com/api/oauth/callback/fitbit"

# Logging Configuration
RUST_LOG="info,pierre_mcp_server=info"
LOG_FORMAT="json"                    # Use structured JSON logging in production

# Performance (Optional)
POSTGRES_MAX_CONNECTIONS=20          # Higher for production load
POSTGRES_MIN_CONNECTIONS=5           # Keep connections warm

# Security (Optional)
ALLOWED_ORIGINS="https://app.example.com,https://dashboard.example.com"
```

**Security Notes**:
1. Never commit `.env` to version control
2. Rotate `PIERRE_MASTER_ENCRYPTION_KEY` annually
3. Use different credentials for dev/staging/production
4. Store secrets in secure vault (AWS Secrets Manager, HashiCorp Vault)

### Generating Secure Keys

```bash
# Generate master encryption key (32 bytes base64-encoded)
openssl rand -base64 32

# Generate strong passwords
openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c32

# Generate OAuth state/nonce
openssl rand -hex 32
```

## Deployment Methods

### Systemd Service (Recommended for Linux)

Create `/etc/systemd/system/pierre-mcp-server.service`:

```ini
[Unit]
Description=Pierre MCP Server - Fitness Data API
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=pierre
Group=pierre
WorkingDirectory=/opt/pierre-mcp-server
EnvironmentFile=/etc/pierre-mcp/.env
ExecStart=/opt/pierre-mcp-server/target/release/pierre-mcp-server
Restart=always
RestartSec=10s
StandardOutput=journal
StandardError=journal
SyslogIdentifier=pierre-mcp-server

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/pierre-mcp-server/data
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

# Resource limits
LimitNOFILE=65536
MemoryMax=2G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
```

**Deploy**:
```bash
# Create service user
sudo useradd -r -s /bin/false pierre

# Copy files
sudo mkdir -p /opt/pierre-mcp-server
sudo cp -r target/release/pierre-mcp-server /opt/pierre-mcp-server/
sudo mkdir -p /opt/pierre-mcp-server/data
sudo chown -R pierre:pierre /opt/pierre-mcp-server

# Install service
sudo systemctl daemon-reload
sudo systemctl enable pierre-mcp-server
sudo systemctl start pierre-mcp-server

# Check status
sudo systemctl status pierre-mcp-server

# View logs
sudo journalctl -u pierre-mcp-server -f
```

### Docker Deployment

**Dockerfile**:
```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /usr/src/pierre
COPY . .

# Build release binary
RUN cargo build --release

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 libpq5 && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false pierre

WORKDIR /app
COPY --from=builder /usr/src/pierre/target/release/pierre-mcp-server /app/
RUN mkdir -p /app/data && chown -R pierre:pierre /app

USER pierre
EXPOSE 8081

CMD ["/app/pierre-mcp-server"]
```

**docker-compose.yml**:
```yaml
version: '3.8'

services:
  pierre:
    build: .
    ports:
      - "8081:8081"
    environment:
      - DATABASE_URL=postgresql://pierre:${DB_PASSWORD}@postgres:5432/pierre_mcp_server
      - PIERRE_MASTER_ENCRYPTION_KEY=${MASTER_KEY}
      - HTTP_PORT=8081
      - RUST_LOG=info
      - LOG_FORMAT=json
    env_file:
      - .env.production
    depends_on:
      - postgres
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/admin/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    volumes:
      - pierre-data:/app/data
    networks:
      - pierre-network

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_DB=pierre_mcp_server
      - POSTGRES_USER=pierre
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    volumes:
      - postgres-data:/var/lib/postgresql/data
    networks:
      - pierre-network
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U pierre"]
      interval: 10s
      timeout: 5s
      retries: 5

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - pierre
    networks:
      - pierre-network
    restart: unless-stopped

volumes:
  pierre-data:
  postgres-data:

networks:
  pierre-network:
    driver: bridge
```

**Deploy**:
```bash
# Build and start
docker-compose up -d

# View logs
docker-compose logs -f pierre

# Check health
curl http://localhost:8081/admin/health

# Scale workers (if load balanced)
docker-compose up -d --scale pierre=3
```

### Kubernetes Deployment

**k8s/deployment.yaml**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pierre-mcp-server
  namespace: fitness-platform
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
      - name: pierre
        image: pierre-mcp-server:latest
        ports:
        - containerPort: 8081
          name: http
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: database-url
        - name: PIERRE_MASTER_ENCRYPTION_KEY
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: master-encryption-key
        - name: HTTP_PORT
          value: "8081"
        - name: RUST_LOG
          value: "info"
        - name: LOG_FORMAT
          value: "json"
        resources:
          requests:
            cpu: "500m"
            memory: "512Mi"
          limits:
            cpu: "2000m"
            memory: "2Gi"
        livenessProbe:
          httpGet:
            path: /admin/health
            port: 8081
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /admin/health
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: pierre-mcp-server
  namespace: fitness-platform
spec:
  selector:
    app: pierre-mcp-server
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8081
  type: ClusterIP
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: pierre-mcp-server
  namespace: fitness-platform
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - pierre.example.com
    secretName: pierre-tls
  rules:
  - host: pierre.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: pierre-mcp-server
            port:
              number: 80
```

## TLS/HTTPS Setup

### Nginx Reverse Proxy

**nginx.conf**:
```nginx
upstream pierre_backend {
    # Multiple backends for load balancing
    server 127.0.0.1:8081 max_fails=3 fail_timeout=30s;
    # server 127.0.0.1:8082 max_fails=3 fail_timeout=30s;  # Add more for HA

    keepalive 32;
}

server {
    listen 80;
    server_name pierre.example.com;

    # Redirect HTTP to HTTPS
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name pierre.example.com;

    # TLS configuration
    ssl_certificate /etc/letsencrypt/live/pierre.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/pierre.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256';
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Logging
    access_log /var/log/nginx/pierre_access.log combined;
    error_log /var/log/nginx/pierre_error.log warn;

    # MCP endpoint (JSON-RPC over HTTP)
    location /mcp {
        proxy_pass http://pierre_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts for long-running tool executions
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # SSE endpoint (Server-Sent Events)
    location /sse {
        proxy_pass http://pierre_backend;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # SSE-specific settings
        proxy_buffering off;
        proxy_cache off;
        chunked_transfer_encoding off;

        # Keep connection alive for SSE
        proxy_read_timeout 3600s;
        proxy_connect_timeout 75s;
    }

    # All other endpoints
    location / {
        proxy_pass http://pierre_backend;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Rate limiting
        limit_req zone=api_limit burst=10 nodelay;
    }
}

# Rate limiting configuration
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=100r/m;
```

### Let's Encrypt SSL Certificate

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx

# Obtain certificate
sudo certbot --nginx -d pierre.example.com

# Auto-renewal (already configured by certbot)
sudo certbot renew --dry-run
```

## Monitoring and Health Checks

### Health Check Endpoints

Pierre provides comprehensive health monitoring (src/health.rs:29-74):

**GET /admin/health**:
```json
{
  "status": "healthy",
  "service": {
    "name": "Pierre MCP Server",
    "version": "1.0.0",
    "environment": "production",
    "uptime_seconds": 86400,
    "build_time": "2024-01-15T10:00:00Z",
    "git_commit": "abc123def"
  },
  "checks": [
    {
      "name": "database",
      "status": "healthy",
      "message": "PostgreSQL connection healthy",
      "duration_ms": 5,
      "metadata": {
        "active_connections": 3,
        "max_connections": 10
      }
    },
    {
      "name": "oauth_providers",
      "status": "healthy",
      "message": "All OAuth providers reachable",
      "duration_ms": 120
    }
  ],
  "timestamp": 1640995200,
  "response_time_ms": 125
}
```

**Status Values**:
- `healthy` - All systems operational
- `degraded` - Some non-critical components failing
- `unhealthy` - Critical components failing

### Prometheus Metrics (Future)

Metrics endpoint planned for `/metrics` with:
- Request rate, latency, error rate (RED metrics)
- Active connections, pool utilization
- Tool execution times
- OAuth token refresh rates
- Database query performance

### Monitoring Stack

**Grafana Dashboard** (example queries):
```promql
# Request rate
rate(pierre_http_requests_total[5m])

# Error rate
rate(pierre_http_requests_total{status=~"5.."}[5m]) / rate(pierre_http_requests_total[5m])

# P95 latency
histogram_quantile(0.95, rate(pierre_http_request_duration_seconds_bucket[5m]))

# Database connection pool utilization
pierre_db_connections_active / pierre_db_connections_max * 100
```

### Log Aggregation

**Structured Logging** (JSON format):
```bash
RUST_LOG=info LOG_FORMAT=json cargo run
```

Example log entry:
```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "target": "pierre_mcp_server::mcp",
  "message": "Tool execution completed",
  "fields": {
    "tool_name": "get_activities",
    "duration_ms": 245,
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "success"
  }
}
```

**ELK Stack Integration**:
```bash
# Filebeat configuration for shipping logs to Elasticsearch
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/pierre/*.log
  json.keys_under_root: true
  json.add_error_key: true

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "pierre-logs-%{+yyyy.MM.dd}"
```

## Backup and Restore

### PostgreSQL Backup

**Automated Daily Backup** (cron):
```bash
# /etc/cron.daily/pierre-backup
#!/bin/bash
set -e

BACKUP_DIR="/var/backups/pierre"
DATE=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30

# Create backup directory
mkdir -p $BACKUP_DIR

# Dump database
pg_dump -h localhost -U pierre -d pierre_mcp_server -F c -b -v \
  -f "$BACKUP_DIR/pierre_${DATE}.dump"

# Compress
gzip "$BACKUP_DIR/pierre_${DATE}.dump"

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR/pierre_${DATE}.dump.gz" \
  "s3://my-backups/pierre/$(date +%Y/%m/)/"

# Remove old backups
find $BACKUP_DIR -name "pierre_*.dump.gz" -mtime +$RETENTION_DAYS -delete

echo "Backup completed: pierre_${DATE}.dump.gz"
```

**Restore from Backup**:
```bash
# Stop server
sudo systemctl stop pierre-mcp-server

# Drop and recreate database
sudo -u postgres psql << EOF
DROP DATABASE IF EXISTS pierre_mcp_server;
CREATE DATABASE pierre_mcp_server;
GRANT ALL PRIVILEGES ON DATABASE pierre_mcp_server TO pierre;
EOF

# Restore from backup
gunzip -c /var/backups/pierre/pierre_20240115_100000.dump.gz | \
  pg_restore -h localhost -U pierre -d pierre_mcp_server -v

# Restart server
sudo systemctl start pierre-mcp-server
```

### Encryption Key Backup

**CRITICAL**: `PIERRE_MASTER_ENCRYPTION_KEY` must be backed up securely. Without it, encrypted data cannot be decrypted.

```bash
# Backup encryption key (store in secure vault)
echo "PIERRE_MASTER_ENCRYPTION_KEY=$(grep PIERRE_MASTER_ENCRYPTION_KEY /etc/pierre-mcp/.env | cut -d'=' -f2)" \
  > /secure/vault/pierre-encryption-key.txt

# Encrypt backup
gpg --symmetric --cipher-algo AES256 /secure/vault/pierre-encryption-key.txt
```

## Performance Tuning

### Database Query Optimization

**Key Indexes** (auto-created by schema):
```sql
-- User lookups
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_tenant_id ON users(tenant_id);

-- OAuth tokens
CREATE INDEX idx_user_oauth_tokens_user_provider ON user_oauth_tokens(user_id, provider);
CREATE INDEX idx_user_oauth_tokens_expires ON user_oauth_tokens(expires_at);

-- API keys
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);

-- Audit logs (if large)
CREATE INDEX idx_audit_log_timestamp ON audit_log(created_at);
CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
```

**Analyze Query Performance**:
```sql
-- Enable query statistics
\timing on

-- Analyze slow query
EXPLAIN ANALYZE
SELECT * FROM user_oauth_tokens WHERE user_id = '...' AND provider = 'strava';

-- Identify missing indexes
SELECT schemaname, tablename, attname, n_distinct, correlation
FROM pg_stats
WHERE schemaname = 'public'
ORDER BY correlation DESC;
```

### Application Performance

**Connection Pool Sizing**:
```bash
# Formula: connections = ((core_count * 2) + effective_spindle_count)
# For 4-core CPU + SSD: 4 * 2 + 1 = 9
POSTGRES_MAX_CONNECTIONS=10
```

**Memory Limits** (systemd):
```ini
# In pierre-mcp-server.service
MemoryMax=2G                    # Hard limit
MemoryHigh=1.5G                 # Soft limit (triggers throttling)
```

## Security Hardening

### Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 22/tcp  # SSH (restrict to specific IPs in production)
sudo ufw enable

# Restrict PostgreSQL to localhost only
sudo ufw deny 5432/tcp
```

### Application Security

**Secrets Management**:
```bash
# Use environment variables, not config files
export $(grep -v '^#' /etc/pierre-mcp/.env | xargs)

# Or use HashiCorp Vault
vault kv get secret/pierre/prod/database-url
```

**Rate Limiting** (nginx):
```nginx
# In nginx.conf (already shown above)
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=100r/m;
```

**CORS Configuration** (if needed):
```bash
ALLOWED_ORIGINS="https://app.example.com,https://dashboard.example.com"
```

## Operational Procedures

### Rolling Updates (Zero Downtime)

**With Load Balancer**:
```bash
# 1. Deploy to node 1, remove from LB
nginx -s reload  # Remove node1 from upstream

# 2. Update node 1
sudo systemctl stop pierre-mcp-server
sudo cp new_binary /opt/pierre-mcp-server/pierre-mcp-server
sudo systemctl start pierre-mcp-server

# 3. Add node 1 back, remove node 2
nginx -s reload

# 4. Update node 2
# ... repeat
```

### Database Migrations

```bash
# Backup before migration
./backup-database.sh

# Run migration (example with diesel/sqlx)
sqlx migrate run --database-url $DATABASE_URL

# Test
curl http://localhost:8081/admin/health

# Rollback if needed
sqlx migrate revert --database-url $DATABASE_URL
```

### Scaling Horizontally

**Add New Server Node**:
```bash
# 1. Provision new server
# 2. Install dependencies, copy binary
# 3. Configure environment (same DATABASE_URL, different node ID)
# 4. Start service
# 5. Add to load balancer

# In nginx upstream block:
upstream pierre_backend {
    server 10.0.1.10:8081;
    server 10.0.1.11:8081;  # New node
    server 10.0.1.12:8081;  # New node
}
```

### SQLite to PostgreSQL Migration

```bash
# 1. Export SQLite data
sqlite3 data/users.db .dump > sqlite_dump.sql

# 2. Convert schema (adjust for PostgreSQL)
# Edit sqlite_dump.sql:
# - Replace INTEGER PRIMARY KEY with SERIAL PRIMARY KEY
# - Replace DATETIME with TIMESTAMP
# - Add quotes around reserved keywords

# 3. Import to PostgreSQL
psql -h localhost -U pierre -d pierre_mcp_server < sqlite_dump_converted.sql

# 4. Update environment
# Change DATABASE_URL from sqlite to postgresql

# 5. Test
cargo test --features postgresql
```

## Troubleshooting

### High Memory Usage

**Check memory stats**:
```bash
ps aux | grep pierre-mcp-server
sudo systemctl status pierre-mcp-server
```

**Solution**: Reduce connection pool size or add memory limit.

### Database Connection Pool Exhaustion

**Symptom**: `Connection pool timeout` errors

**Solution**:
```bash
# Increase pool size
POSTGRES_MAX_CONNECTIONS=20

# Reduce idle timeout
POSTGRES_IDLE_TIMEOUT=60
```

### Slow Queries

**Check PostgreSQL slow query log**:
```bash
sudo tail -f /var/log/postgresql/postgresql-15-main.log | grep "duration:"
```

**Solution**: Add indexes, optimize queries, increase `work_mem`.

## Related Documentation

- [Configuration Guide](../developer-guide/12-configuration.md) - Environment variables
- [Security Guide](../developer-guide/17-security-guide.md) - Security best practices
- [Database Schema](../developer-guide/database-schema.md) - Schema documentation
- [Monitoring Guide](monitoring.md) - Detailed monitoring setup

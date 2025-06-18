# Deployment Guide

This guide covers production deployment scenarios, Docker configuration, and cloud deployment options.

## Production Deployment Examples

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
    volumes:
      - ./data:/app/data
```

### Enterprise Self-Hosted
```bash
# Single command deployment
docker run -d \
  -p 8080:8080 -p 8081:8081 \
  -e STRAVA_CLIENT_ID=your_client_id \
  -e STRAVA_CLIENT_SECRET=your_client_secret \
  -e DATABASE_URL=sqlite:./data/enterprise.db \
  --name pierre-fitness \
  pierre-mcp-server:latest
```

### Personal Instance (Single-Tenant)
```bash
# For individual users running locally
cargo run --bin pierre-mcp-server -- --single-tenant \
  --port 8080 \
  --config ./my-config.toml
```

## Architecture

For detailed technical documentation, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

### Key Components:
- **MCP Server** (Port 8080): AI assistant connections
- **HTTP API** (Port 8081): REST endpoints and dashboard
- **Admin Service** (Port 8082): API key approval workflow (repo private for now)
- **SQLite Database**: User data and encrypted tokens
- **Background Tasks**: Expired key cleanup, usage tracking

Pierre Fitness API supports two deployment modes:

### 🏠 Single-Tenant Mode (Personal Use)
- **Perfect for individual users** who want to run the server locally
- No authentication required - direct access to your fitness data
- Simple configuration with local config files or environment variables
- Backwards compatible with existing setups

### ☁️ Multi-Tenant Mode (Cloud Deployment)
- **Enterprise-ready** for serving multiple users
- **JWT Authentication** with secure user sessions
- **Encrypted Token Storage** using AES-256-GCM for OAuth tokens at rest
- **SQLite Database** for user management and token storage
- **User Isolation** ensuring data privacy between users
- **Cloud-Ready** for deployment on any cloud provider

## Docker Deployment

### Development Setup

1. **Setup Environment Variables**:
   ```bash
   # Copy the example to .envrc
   cp .env.example .envrc
   # Edit .envrc with your OAuth credentials
   # If using direnv: direnv allow
   ```

2. **Using Docker Compose with direnv**:
   ```bash
   # Use the helper script that loads .envrc
   ./docker-compose-with-envrc.sh up
   
   # Or manually export variables and run docker-compose
   eval $(cat .envrc | grep export) && docker-compose up
   ```

3. **Production Deployment**:
   ```bash
   # Build and run in production mode
   docker-compose -f docker-compose.prod.yml up -d
   ```

4. **Health Checks**: Available at `http://localhost:8081/health`

### Building the Docker Image

```bash
# Build the image
docker build -t pierre-mcp-server:latest .

# Run with environment variables
docker run -d \
  --name pierre-server \
  -p 8080:8080 \
  -p 8081:8081 \
  -e STRAVA_CLIENT_ID=your_client_id \
  -e STRAVA_CLIENT_SECRET=your_client_secret \
  -e DATABASE_URL=sqlite:/app/data/users.db \
  -v $(pwd)/data:/app/data \
  pierre-mcp-server:latest
```

## Cloud Deployment

### AWS Deployment

#### ECS (Elastic Container Service)
```json
{
  "taskDefinition": {
    "family": "pierre-mcp-server",
    "memory": "1024",
    "cpu": "512",
    "networkMode": "awsvpc",
    "requiresCompatibilities": ["FARGATE"],
    "containerDefinitions": [
      {
        "name": "pierre-server",
        "image": "your-account.dkr.ecr.region.amazonaws.com/pierre-mcp-server:latest",
        "portMappings": [
          {"containerPort": 8080, "protocol": "tcp"},
          {"containerPort": 8081, "protocol": "tcp"}
        ],
        "environment": [
          {"name": "STRAVA_CLIENT_ID", "value": "your_client_id"},
          {"name": "STRAVA_CLIENT_SECRET", "value": "your_client_secret"},
          {"name": "DATABASE_URL", "value": "postgresql://user:pass@rds-endpoint:5432/pierre"}
        ]
      }
    ]
  }
}
```

#### EC2 with Docker
```bash
# Install Docker on EC2 instance
sudo yum update -y
sudo yum install -y docker
sudo service docker start
sudo usermod -a -G docker ec2-user

# Deploy Pierre
docker run -d \
  --name pierre-production \
  -p 80:8081 \
  -p 8080:8080 \
  -e STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID} \
  -e STRAVA_CLIENT_SECRET=${STRAVA_CLIENT_SECRET} \
  -e DATABASE_URL=postgresql://user:pass@rds-endpoint:5432/pierre \
  -v /home/ec2-user/pierre-data:/app/data \
  pierre-mcp-server:latest
```

### Google Cloud Platform

#### Cloud Run
```yaml
# cloudrun.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: pierre-mcp-server
spec:
  template:
    spec:
      containers:
      - image: gcr.io/your-project/pierre-mcp-server:latest
        ports:
        - containerPort: 8081
        env:
        - name: STRAVA_CLIENT_ID
          value: "your_client_id"
        - name: STRAVA_CLIENT_SECRET
          value: "your_client_secret"
        - name: DATABASE_URL
          value: "postgresql://user:pass@db-ip:5432/pierre"
        resources:
          limits:
            memory: 1Gi
            cpu: 1000m
```

### Azure Container Instances

```bash
# Deploy to Azure Container Instances
az container create \
  --resource-group pierre-rg \
  --name pierre-server \
  --image your-registry.azurecr.io/pierre-mcp-server:latest \
  --ports 8080 8081 \
  --environment-variables \
    STRAVA_CLIENT_ID=your_client_id \
    STRAVA_CLIENT_SECRET=your_client_secret \
    DATABASE_URL=postgresql://user:pass@db-server:5432/pierre \
  --cpu 1 \
  --memory 2
```

## Kubernetes Deployment

### Deployment Configuration

```yaml
# pierre-deployment.yaml
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
      - name: pierre-server
        image: pierre-mcp-server:latest
        ports:
        - containerPort: 8080
          name: mcp
        - containerPort: 8081
          name: http
        env:
        - name: STRAVA_CLIENT_ID
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: strava-client-id
        - name: STRAVA_CLIENT_SECRET
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: strava-client-secret
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: database-url
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: pierre-service
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
  type: LoadBalancer
```

### Secrets Configuration

```yaml
# pierre-secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: pierre-secrets
type: Opaque
data:
  strava-client-id: <base64-encoded-client-id>
  strava-client-secret: <base64-encoded-client-secret>
  database-url: <base64-encoded-database-url>
  jwt-secret: <base64-encoded-jwt-secret>
```

## Monitoring and Observability

### Health Checks

Pierre includes built-in health check endpoints:

```bash
# Basic health check
curl http://localhost:8081/health

# Detailed system status
curl http://localhost:8081/health/detailed
```

### Logging

Configure structured logging for production:

```toml
# Add to fitness_config.toml
[logging]
level = "info"
format = "json"
features = {
  location = true,
  spans = true,
  thread = true
}
```

### Metrics and Monitoring

Pierre exposes metrics for monitoring tools:

```bash
# Prometheus metrics endpoint
curl http://localhost:8081/metrics

# WebSocket-based real-time metrics
ws://localhost:8081/ws/metrics
```

## Environment Variables

### Required Variables

```bash
# OAuth Configuration
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret

# Database
DATABASE_URL=sqlite:./data/users.db  # or PostgreSQL URL

# Security
JWT_SECRET=your_jwt_secret_here
ENCRYPTION_KEY=your_32_byte_key_here
```

### Optional Variables

```bash
# Server Configuration
MCP_PORT=8080
HTTP_PORT=8081
RUST_LOG=info

# Weather Integration
OPENWEATHER_API_KEY=your_weather_api_key

# CORS and Security
CORS_ORIGINS=https://yourdomain.com
ALLOWED_HOSTS=yourdomain.com,api.yourdomain.com

# Rate Limiting
RATE_LIMIT_REQUESTS=1000
RATE_LIMIT_WINDOW=3600
```

## Backup and Recovery

### Database Backup

```bash
# SQLite backup
sqlite3 ./data/users.db ".backup ./backups/users_$(date +%Y%m%d_%H%M%S).db"

# PostgreSQL backup
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d_%H%M%S).sql
```

### Automated Backup Script

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/app/backups"
RETENTION_DAYS=7

# Create backup
sqlite3 /app/data/users.db ".backup $BACKUP_DIR/users_$(date +%Y%m%d_%H%M%S).db"

# Clean old backups
find $BACKUP_DIR -name "users_*.db" -mtime +$RETENTION_DAYS -delete

echo "Backup completed: $(date)"
```

## Security Considerations

### Production Security Checklist

- [ ] Use strong JWT secrets (at least 32 characters)
- [ ] Configure HTTPS/TLS certificates
- [ ] Set up proper CORS origins
- [ ] Use encrypted database connections
- [ ] Regular security updates
- [ ] Monitor access logs
- [ ] Implement rate limiting
- [ ] Use secrets management (not environment variables in production)
- [ ] Regular backups with encryption
- [ ] Network security groups/firewalls

### TLS Configuration

```nginx
# nginx.conf for TLS termination
server {
    listen 443 ssl;
    server_name your-domain.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:8081;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    location /ws {
        proxy_pass http://localhost:8081;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }
}
```
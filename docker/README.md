# Docker Configuration

This directory contains all Docker-related files for the Pierre MCP Server.

## Directory Structure

```
docker/
├── README.md              # This file
├── compose/               # Docker Compose configurations
│   ├── compose.prod.yml       # Production deployment
│   ├── compose.postgres.yml   # PostgreSQL for integration tests
│   └── compose.redis.yml      # Redis for cache testing
├── images/                # Dockerfile definitions
│   ├── server/                # Main server image
│   │   ├── Dockerfile
│   │   └── entrypoint.sh
│   └── test/                  # Test runner image
│       └── Dockerfile
├── init/                  # Database initialization scripts
│   └── postgres/
│       └── 01-init.sql
└── scripts/               # Helper scripts
    └── compose-with-envrc.sh
```

## Quick Start

### Development (SQLite)

From the project root:
```bash
docker-compose up
```

### Production

```bash
docker-compose -f docker-compose.yml -f docker/compose/compose.prod.yml up -d
```

### PostgreSQL Integration Tests

```bash
docker-compose -f docker/compose/compose.postgres.yml up --build
```

### Redis Cache Tests

```bash
docker-compose -f docker/compose/compose.redis.yml up --build
```

## Using the Helper Script

The helper script loads environment variables from `.envrc`:

```bash
./docker/scripts/compose-with-envrc.sh up
./docker/scripts/compose-with-envrc.sh -f docker/compose/compose.prod.yml up -d
```

## Building Images

### Server Image

```bash
docker build -f docker/images/server/Dockerfile -t pierre-mcp-server:latest .
```

### Test Image

```bash
docker build -f docker/images/test/Dockerfile -t pierre-mcp-server-test:latest .
```

## Volumes

| Volume | Purpose |
|--------|---------|
| `pierre_data` | SQLite database and encryption keys (development) |
| `pierre_data_prod` | Production data persistence |
| `postgres_data` | PostgreSQL data (integration tests) |
| `redis_data` | Redis persistence (cache tests) |

## Ports

| Port | Service |
|------|---------|
| 8080 | MCP Server (stdio-over-HTTP) |
| 8081 | HTTP API and health checks |
| 5432 | PostgreSQL (when using compose.postgres.yml) |
| 6379 | Redis (when using compose.redis.yml) |

## Environment Variables

See `.env.example` in the project root for required environment variables.
Key variables:
- `STRAVA_CLIENT_ID` / `STRAVA_CLIENT_SECRET` - Strava OAuth
- `DATABASE_URL` - Database connection string
- `ENCRYPTION_KEY_PATH` - Path to encryption key file
- `JWT_SECRET_PATH` - Path to JWT secret file

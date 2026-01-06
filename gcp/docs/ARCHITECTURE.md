# Pierre MCP Server - GCP Architecture

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         USERS / MCP CLIENTS                             │
│              (Claude Desktop, ChatGPT, Custom Agents)                   │
└────────────────────────────┬────────────────────────────────────────────┘
                             │ HTTPS
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       CLOUD LOAD BALANCER                               │
│                    (Automatic with Cloud Run)                           │
│                  SSL/TLS Termination, DDoS Protection                   │
└────────────────────────────┬────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        CLOUD RUN SERVICE                                │
│                    (pierre-mcp-server container)                        │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │  Instance 1 (Min: 1-2, Max: 100, Auto-scaling)                    │ │
│  │  ┌─────────────────────────────────────────────────────────────┐  │ │
│  │  │  Pierre MCP Server (Rust Binary)                            │  │ │
│  │  │  - HTTP API (Port 8081)                                     │  │ │
│  │  │  - MCP Protocol Handler                                     │  │ │
│  │  │  - OAuth 2.0 Server                                         │  │ │
│  │  │  - JWT Authentication (RS256)                               │  │ │
│  │  │  - Multi-tenant Logic                                       │  │ │
│  │  │  - Intelligence Engine                                      │  │ │
│  │  └─────────────────────────────────────────────────────────────┘  │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                             │                                           │
│                    ┌────────┴────────┐                                  │
│                    │                 │                                  │
└────────────────────┼─────────────────┼──────────────────────────────────┘
                     │                 │
         ┌───────────▼────────┐       └──────────────┐
         │  Serverless VPC    │                      │
         │  Connector         │                      │
         │  (Private Network) │                      │
         └───────────┬────────┘                      │
                     │                               │
                     ▼                               ▼
    ┌────────────────────────────────┐   ┌──────────────────────────┐
    │      CLOUD SQL POSTGRES        │   │     SECRET MANAGER       │
    │    (PostgreSQL 16, HA)         │   │  (OAuth Secrets, Keys)   │
    │  ┌──────────────────────────┐  │   │  ┌────────────────────┐  │
    │  │  Database: pierre_mcp    │  │   │  │ Strava Secret      │  │
    │  │  Tables: 26+             │  │   │  │ Garmin Secret      │  │
    │  │  Users, Tenants,         │  │   │  │ Fitbit Secret      │  │
    │  │  Activities, Goals       │  │   │  │ OpenWeather Key    │  │
    │  │  OAuth Tokens (enc)      │  │   │  │ Encryption Key     │  │
    │  │  API Keys                │  │   │  │ DB Password        │  │
    │  └──────────────────────────┘  │   │  └────────────────────┘  │
    │                                │   └──────────────────────────┘
    │  Private IP: 10.0.0.x          │
    │  Automated Backups (Daily)     │
    │  Point-in-Time Recovery        │
    │  Regional HA (Production)      │
    └────────────────────────────────┘
                     │
                     │ (Backups)
                     ▼
         ┌──────────────────────────┐
         │   CLOUD STORAGE          │
         │   (Backup Retention)     │
         └──────────────────────────┘

                     ▲
                     │ (Outbound to External APIs)
                     │
    ┌────────────────┴───────────────┐
    │       CLOUD NAT GATEWAY        │
    │  (Outbound Internet Access)    │
    └────────────────┬───────────────┘
                     │
         ┌───────────┴───────────┐
         │                       │
         ▼                       ▼
┌─────────────────┐     ┌───────────────────┐
│ EXTERNAL APIs   │     │  MONITORING &     │
│                 │     │  OBSERVABILITY    │
│ • Strava API    │     │                   │
│ • Garmin API    │     │ • Cloud Logging   │
│ • Fitbit API    │     │ • Cloud Monitoring│
│ • OpenWeather   │     │ • Cloud Trace     │
│ • USDA FoodData │     │ • Uptime Checks   │
└─────────────────┘     │ • Alerts/PagerDuty│
                        └───────────────────┘
```

## Network Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         VPC NETWORK                             │
│                     (pierre-vpc-{env})                          │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Subnet: 10.0.0.0/24 (Regional)                        │    │
│  │                                                          │    │
│  │  ┌──────────────────────────────────────────────────┐  │    │
│  │  │  Cloud SQL Private IP Pool                       │  │    │
│  │  │  Reserved: 10.0.0.0/16 (VPC Peering)             │  │    │
│  │  └──────────────────────────────────────────────────┘  │    │
│  │                                                          │    │
│  │  ┌──────────────────────────────────────────────────┐  │    │
│  │  │  Serverless VPC Connector                        │  │    │
│  │  │  IP Range: 10.8.0.0/28                           │  │    │
│  │  │  Connects Cloud Run ↔ Cloud SQL                  │  │    │
│  │  └──────────────────────────────────────────────────┘  │    │
│  │                                                          │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  Cloud Router + Cloud NAT                              │    │
│  │  (Outbound connectivity for Cloud Run)                 │    │
│  │  • External API calls (Strava, Garmin, etc.)           │    │
│  │  • Static public IP for egress                         │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Traffic Flow

### 1. User Request Flow

```
User/MCP Client
    │
    │ 1. HTTPS Request (GET /health, POST /mcp, etc.)
    ▼
Cloud Run Service (Internet-facing)
    │
    │ 2. Authenticate (JWT Bearer Token from OAuth 2.0)
    ▼
Application Logic (Rust)
    │
    ├── 3a. Database Query ──┐
    │   (via Serverless VPC  │
    │    Connector)           │
    │                         ▼
    │                    Cloud SQL (Private IP)
    │                         │
    │   ┌─────────────────────┘
    │   │ 4. Query Result
    │   ▼
    ├── 3b. Secret Fetch ────┐
    │   (IAM Authentication)  │
    │                         ▼
    │                    Secret Manager
    │                         │
    │   ┌─────────────────────┘
    │   │ 5. Secret Value
    │   ▼
    └── 3c. External API ────┐
        (via Cloud NAT)      │
                             ▼
                        Strava/Garmin/Fitbit API
                             │
        ┌────────────────────┘
        │ 6. Provider Data
        ▼
Response JSON (with logging to Cloud Logging)
    │
    │ 7. HTTP Response
    ▼
User/MCP Client
```

### 2. OAuth Provider Flow

```
User → Cloud Run → OAuth Login Page
  │
  │ User clicks "Connect Strava"
  ▼
Cloud Run → Redirect to Strava OAuth
  │
  │ User authorizes
  ▼
Strava → Callback to Cloud Run (/api/oauth/callback/strava)
  │
  │ Exchange code for token
  ▼
Cloud Run:
  1. Decrypt existing tokens (if any)
  2. Store new token (encrypted with AAD)
  3. Save to Cloud SQL
  4. Emit SSE notification
  │
  ▼
User receives success notification
```

### 3. Database Migration Flow

```
Cloud Run Service Starts
    │
    │ 1. Read DATABASE_URL from env
    ▼
SQLx Migration System
    │
    │ 2. Check applied migrations (sqlx table)
    ├─── 3. Run pending migrations (CREATE TABLE IF NOT EXISTS)
    │    - users
    │    - user_oauth_tokens (encrypted columns)
    │    - tenants
    │    - api_keys
    │    - [26+ tables]
    │
    │ 4. Create indexes
    │ 5. Set up foreign keys
    ▼
Application Ready
    │
    └── Health check returns 200 OK
```

## Security Architecture

### 1. Network Security

```
┌──────────────────────────────────────────────────────────┐
│              SECURITY LAYERS                             │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Layer 1: Cloud Armor (WAF) [Production Only]           │
│  - DDoS protection                                       │
│  - Rate limiting                                         │
│  - Geo-fencing                                           │
│  - OWASP Top 10 rules                                    │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Layer 2: Cloud Run IAM                                 │
│  - JWT token validation (RS256)                          │
│  - Service account authentication                        │
│  - allUsers invoker (public API)                         │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Layer 3: Application-Level Auth                        │
│  - Bearer token required                                 │
│  - Tenant isolation (multi-tenancy)                      │
│  - API key authentication                                │
│  - Rate limiting (per user/tenant)                       │
│  - PII redaction middleware                              │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Layer 4: Database Security                             │
│  - Private IP only (no public access)                    │
│  - VPC Service Controls                                  │
│  - Encrypted at rest (AES-256)                           │
│  - Encrypted in transit (TLS 1.3)                        │
│  - OAuth tokens encrypted with AAD                       │
│                                                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Layer 5: Secret Management                             │
│  - Secret Manager (no env vars)                          │
│  - Automatic rotation (planned)                          │
│  - IAM-based access control                              │
│  - Audit logging                                         │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### 2. IAM Permissions Model

```
Service Account: pierre-mcp-server-runner-{env}@PROJECT.iam.gserviceaccount.com

Roles:
├── roles/cloudsql.client
│   └── Connect to Cloud SQL instances
│
├── roles/secretmanager.secretAccessor
│   └── Read secrets (OAuth, encryption keys)
│
├── roles/logging.logWriter
│   └── Write structured logs
│
├── roles/monitoring.metricWriter
│   └── Export custom metrics
│
└── roles/cloudtrace.agent
    └── Send distributed traces
```

## Data Flow Architecture

### Database Schema (26+ Tables)

```
┌────────────────────────────────────────────────────────────┐
│                     CORE TABLES                            │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  users                    Multi-tenant user accounts       │
│  ├── id (UUID)                                             │
│  ├── email                                                 │
│  ├── tenant_id (FK)                                        │
│  ├── tier (free/pro/enterprise)                            │
│  └── is_admin                                              │
│                                                            │
│  user_oauth_tokens        Encrypted OAuth credentials      │
│  ├── user_id + tenant_id + provider (PK)                   │
│  ├── access_token (ENCRYPTED with AAD)                     │
│  ├── refresh_token (ENCRYPTED with AAD)                    │
│  └── expires_at                                            │
│                                                            │
│  tenants                  Multi-tenant isolation           │
│  ├── id (UUID)                                             │
│  ├── name                                                  │
│  ├── slug                                                  │
│  └── subscription_tier                                     │
│                                                            │
│  api_keys                 Programmatic access              │
│  ├── id (UUID)                                             │
│  ├── key_hash (bcrypt)                                     │
│  ├── user_id (FK)                                          │
│  └── rate_limit                                            │
│                                                            │
└────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│                   FITNESS DATA                             │
├────────────────────────────────────────────────────────────┤
│  goals                    User fitness goals                │
│  insights                 AI-generated insights             │
│  fitness_configurations   Algorithm settings                │
└────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│                   OAUTH 2.0 SERVER                         │
├────────────────────────────────────────────────────────────┤
│  oauth2_clients           Registered MCP clients            │
│  oauth2_auth_codes        Authorization codes (PKCE)        │
│  oauth2_refresh_tokens    Refresh tokens                    │
│  rsa_keypairs             JWT signing keys (RS256)          │
└────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│                   A2A PROTOCOL                             │
├────────────────────────────────────────────────────────────┤
│  a2a_clients              Agent registrations               │
│  a2a_sessions             Active sessions                   │
│  a2a_tasks                Task execution                    │
└────────────────────────────────────────────────────────────┘
```

## Monitoring & Observability

### Metrics Collected

```
Cloud Run Metrics:
├── Request count (per minute)
├── Request latency (p50, p95, p99)
├── Error rate (4xx, 5xx)
├── Container CPU usage
├── Container memory usage
├── Instance count (current, min, max)
└── Cold start latency

Cloud SQL Metrics:
├── Connection count
├── Query latency
├── Disk usage
├── CPU utilization
├── Memory utilization
└── Replication lag (if HA enabled)

Application Metrics (Custom):
├── OAuth token refresh count
├── Provider API call latency
├── Cache hit/miss ratio
├── Database query performance
└── Multi-tenant request distribution
```

### Logging Strategy

```
Cloud Logging (Structured JSON):
├── HTTP Access Logs (Cloud Run automatic)
├── Application Logs (RUST_LOG)
│   ├── Level: ERROR, WARN, INFO, DEBUG
│   ├── Request ID correlation
│   └── PII redaction applied
├── SQL Query Logs (slow queries only)
├── OAuth Flow Logs (audit trail)
└── Security Events (failed auth, rate limits)

Log Retention:
├── Development: 7 days
├── Staging: 30 days
└── Production: 90 days (compliance)
```

### Alerting Rules

```
Critical Alerts (PagerDuty):
├── Service down (health check failed)
├── Error rate > 5% (5min window)
├── Database connection exhausted
└── External API failure (Strava, Garmin down)

Warning Alerts (Slack):
├── High latency (p95 > 1s)
├── Memory usage > 80%
├── Database disk > 85%
└── Unusual traffic spike (+50% baseline)
```

## Deployment Strategy

### Environments

```
┌──────────────────────────────────────────────────────────┐
│ Development (pierre-mcp-dev)                             │
├──────────────────────────────────────────────────────────┤
│ • db-f1-micro (0.6GB RAM)                                │
│ • Min instances: 0 (scale to zero)                       │
│ • Auto-deploy from main branch                           │
│ • Cost: ~$75/month                                       │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│ Staging (pierre-mcp-staging)                             │
├──────────────────────────────────────────────────────────┤
│ • db-custom-2-8192 (2 vCPU, 8GB)                         │
│ • Min instances: 1                                       │
│ • Auto-deploy from main branch                           │
│ • Production parity                                      │
│ • Cost: ~$200/month                                      │
└──────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────┐
│ Production (pierre-mcp-prod)                             │
├──────────────────────────────────────────────────────────┤
│ • db-custom-4-16384 (4 vCPU, 16GB)                       │
│ • High Availability (Regional HA)                        │
│ • Min instances: 2                                       │
│ • Manual approval required                               │
│ • Canary deployments (10% → 100%)                        │
│ • Cost: ~$500-1500/month                                 │
└──────────────────────────────────────────────────────────┘
```

### CI/CD Pipeline

```
GitHub Push (main branch)
    │
    ▼
GitHub Actions / Cloud Build Trigger
    │
    ├── 1. Run Tests (cargo test)
    ├── 2. Lint (cargo clippy)
    ├── 3. Security Scan (cargo deny)
    ├── 4. Build Docker Image
    │      └── Multi-stage: Rust build → Debian runtime
    │
    ├── 5. Push to Artifact Registry
    │      └── Tag: latest, {SHORT_SHA}, {ENV}
    │
    ├── 6. Deploy to Cloud Run
    │      └── Blue-green deployment (automatic)
    │
    ├── 7. Run Database Migrations
    │      └── SQLx embedded migrations
    │
    └── 8. Smoke Test
           └── curl /health (wait 60s for ready)

Production Deployment (git tag v1.0.0)
    │
    ├── Same steps as above
    ├── Deploy canary (10% traffic)
    ├── Monitor for 5 minutes
    ├── Manual approval to promote
    └── Rollback plan ready
```

## Disaster Recovery

### RTO/RPO Targets

```
┌────────────────────────────────────────────────┐
│ Recovery Objectives                            │
├────────────────────────────────────────────────┤
│ RTO (Recovery Time Objective):   15 minutes   │
│ RPO (Recovery Point Objective):   5 minutes   │
└────────────────────────────────────────────────┘
```

### Backup Strategy

```
Cloud SQL Automated Backups:
├── Daily backups at 3 AM UTC
├── Point-in-time recovery (7 days)
├── Transaction log retention (7 days)
└── Export to Cloud Storage (weekly)

Cloud Run Revisions:
├── Last 10 revisions retained
├── Instant rollback capability
└── Tagged releases (v1.0.0) kept indefinitely

Infrastructure as Code:
├── Terraform state in GCS
├── State versioning enabled
└── Git repository (version control)
```

## Scalability

### Auto-Scaling Configuration

```
Cloud Run:
├── Min Instances: 0 (dev), 1 (staging), 2 (prod)
├── Max Instances: 10 (dev), 50 (staging), 100 (prod)
├── Concurrency: 80 requests per instance
├── CPU Throttling: After request completion
└── Scale-to-zero: Enabled for dev only

Cloud SQL:
├── Vertical scaling: Change tier (manual)
├── Read replicas: Add up to 10 (manual)
├── Connection pooling: SQLx (10 connections)
└── Auto-increase storage: Enabled
```

### Expected Performance

```
Single Instance Capacity:
├── Throughput: ~500 RPS (simple GET)
├── Throughput: ~200 RPS (database queries)
├── Latency: p50=50ms, p95=200ms, p99=500ms
└── Cold start: <500ms (Rust binary)

100 Instances (Max Scale):
├── Throughput: 20,000+ RPS
├── Peak Load: 100,000+ daily active users
└── Database: Read replicas required
```

## Cost Breakdown (Production)

```
Monthly Estimate:
├── Cloud Run:             $100-300
│   ├── CPU time (vCPU-seconds)
│   ├── Memory (GB-seconds)
│   └── Requests (per million)
│
├── Cloud SQL:             $300-400
│   ├── db-custom-4-16384 tier
│   ├── HA configuration (+100%)
│   ├── Storage (100GB SSD)
│   └── Automated backups
│
├── Networking:            $60
│   ├── VPC connector ($9)
│   ├── Cloud NAT ($45)
│   └── Egress traffic ($5-10)
│
├── Secret Manager:        $5
│   └── Secret access operations
│
├── Logging & Monitoring:  $20-50
│   ├── Log ingestion
│   ├── Metrics
│   └── Traces
│
└── TOTAL:                 $485-825/month

Scale to 10x traffic: ~$2000-3000/month
```

# Pierre MCP Server - GCP Deployment Architecture

## Executive Summary

This document provides a comprehensive Google Cloud Platform (GCP) deployment architecture for the Pierre MCP Server, designed by SRE best practices for production workloads.

## Architecture Decision: Cloud Run + Cloud SQL

### Why Cloud Run?

**Recommended: Cloud Run** ✅

**Rationale:**
1. **Serverless Benefits**: Auto-scaling from 0 to N instances based on traffic
2. **Cost Efficiency**: Pay only for actual usage (billed per 100ms of CPU time)
3. **Container Native**: Direct Docker deployment, no Kubernetes complexity
4. **Built-in HTTPS**: Automatic SSL certificates and load balancing
5. **Global Distribution**: Deploy to multiple regions easily
6. **Fast Cold Starts**: Rust binary cold starts in <500ms
7. **Perfect Fit**: HTTP API workload with variable traffic patterns

**Cloud Run vs Alternatives:**
- **GKE (Google Kubernetes Engine)**: Overkill for single container, higher operational overhead
- **Compute Engine VMs**: Requires manual scaling, patching, and load balancing
- **App Engine**: Less flexible, Cloud Run is the modern replacement

### Why Cloud SQL for PostgreSQL?

**Recommended: Cloud SQL (PostgreSQL 16)** ✅

**Rationale:**
1. **Managed Service**: Automatic backups, patching, high availability
2. **Production Ready**: Point-in-time recovery, automated failover
3. **Performance**: SSD storage, read replicas, connection pooling
4. **Security**: Encrypted at rest and in transit, IAM authentication
5. **Monitoring**: Built-in metrics and logging integration
6. **Cost Effective**: Starts at ~$10/month for db-f1-micro

**Cloud SQL vs Alternatives:**
- **Cloud Spanner**: Overkill and expensive ($65/node/month) for this workload
- **Self-managed PostgreSQL on GCE**: High operational burden
- **AlloyDB**: More expensive, better for >10TB databases
- **SQLite on persistent disk**: Not recommended for production multi-tenant

## Infrastructure Components

### 1. Compute: Cloud Run Service
- **Service Name**: `pierre-mcp-server`
- **Container**: Custom Rust binary (~40MB)
- **CPU**: 1 vCPU (can burst to 2)
- **Memory**: 512Mi (can scale to 1Gi)
- **Concurrency**: 80 requests per instance
- **Min Instances**: 1 (avoid cold starts for critical path)
- **Max Instances**: 100 (adjust based on load testing)
- **Request Timeout**: 300s (5 minutes for long-running MCP operations)
- **Region**: `us-central1` (default, change as needed)

### 2. Database: Cloud SQL for PostgreSQL
- **Instance Name**: `pierre-postgres`
- **Version**: PostgreSQL 16
- **Tier**: `db-f1-micro` (dev/staging) or `db-custom-2-8192` (production)
- **Storage**: 20GB SSD (auto-increase enabled)
- **Backups**: Daily automated backups, 7-day retention
- **High Availability**: Regional HA configuration for production
- **Private IP**: VPC-native for security
- **Connection**: Via Cloud SQL Proxy or Private Service Connect

### 3. Networking
- **VPC**: Custom VPC with private subnets
- **Serverless VPC Connector**: Bridge Cloud Run to Cloud SQL private IP
- **Cloud NAT**: Outbound connectivity for external API calls (Strava, Garmin, etc.)
- **Cloud Armor**: WAF and DDoS protection (optional, for production)
- **Cloud CDN**: Not needed currently (no static assets)

### 4. Security & Secrets
- **Secret Manager**: Store sensitive credentials
  - `PIERRE_MASTER_ENCRYPTION_KEY`
  - `STRAVA_CLIENT_SECRET`
  - `GARMIN_CLIENT_SECRET`
  - `FITBIT_CLIENT_SECRET`
  - `OPENWEATHER_API_KEY`
  - Database connection strings
- **IAM Service Account**: Least-privilege access for Cloud Run
- **Workload Identity**: Secure authentication to GCP services

### 5. Monitoring & Observability
- **Cloud Logging**: Structured JSON logs from application
- **Cloud Monitoring**: Custom metrics, dashboards, alerts
- **Cloud Trace**: Distributed tracing with OpenTelemetry
- **Uptime Checks**: Monitor `/health` endpoint
- **Alerting**: PagerDuty/Slack integration for critical issues

### 6. CI/CD
- **Cloud Build**: Automated Docker builds and deployments
- **Artifact Registry**: Private container registry
- **GitHub Actions**: Trigger Cloud Build on push to main
- **Terraform Cloud**: Infrastructure state management (optional)

### 7. External API Access

The application requires outbound HTTPS access to:
- **Strava API**: `https://www.strava.com/api/v3/`
- **Garmin Connect**: `https://connectapi.garmin.com/`
- **Fitbit API**: `https://api.fitbit.com/`
- **OpenWeatherMap**: `https://api.openweathermap.org/`
- **USDA FoodData**: `https://api.nal.usda.gov/fdc/v1/`

**Network Configuration:**
- Cloud Run → Cloud NAT → Internet (outbound)
- Whitelisting: Not required (APIs use OAuth 2.0 tokens)

## Cost Estimation (Monthly)

### Development/Staging
- Cloud Run: $5-20 (low traffic)
- Cloud SQL (db-f1-micro): $10
- VPC Connector: $9
- Cloud NAT: $45
- Storage/Logs: $5
- **Total: ~$75-90/month**

### Production (Medium Scale)
- Cloud Run: $100-300 (moderate traffic, min instances)
- Cloud SQL (db-custom-2-8192): $150
- Cloud SQL HA: +$150
- VPC Connector: $9 (per connector)
- Cloud NAT: $45
- Storage/Logs: $20
- Cloud Monitoring: $10
- **Total: ~$485-685/month**

### Production (High Scale)
- Cloud Run: $500-1000 (high traffic)
- Cloud SQL (db-custom-4-16384): $300
- Read Replicas: +$300
- Cloud Armor: $5-50
- All other services: $100
- **Total: ~$1200-1750/month**

## Why Terraform (IaC Approach)

### Terraform vs Alternatives

**Recommended: Terraform** ✅

**Why Terraform:**
1. **Industry Standard**: Most popular IaC tool (40%+ market share)
2. **Multi-Cloud**: Works across GCP, AWS, Azure (future flexibility)
3. **Mature Ecosystem**: 3000+ providers, extensive community
4. **State Management**: Built-in state locking and remote backends
5. **Plan/Apply Workflow**: Preview changes before applying
6. **Module System**: Reusable components for consistency
7. **GitOps Ready**: Version control, code review, CI/CD integration

**Alternatives Considered:**

❌ **Google Cloud Deployment Manager**
- GCP-only, deprecated in favor of Terraform
- YAML/Jinja2 templates less expressive than HCL
- Limited community support

❌ **Pulumi**
- Uses real programming languages (Go, Python, TypeScript)
- Smaller community, less mature
- Overkill for this use case
- Requires developer expertise in specific language

❌ **gcloud CLI Scripts**
- Imperative, not declarative
- No state management
- Difficult to maintain
- No drift detection
- **Use case**: Quick prototypes only

❌ **Cloud Console (Manual Clicks)**
- Not reproducible
- No audit trail
- Human error prone
- Impossible to version control
- **Never use for production**

### Terraform Structure

```
gcp/terraform/
├── main.tf                 # Primary infrastructure definitions
├── variables.tf            # Input variables
├── outputs.tf              # Output values
├── versions.tf             # Provider versions
├── backend.tf              # Remote state configuration
├── modules/
│   ├── cloud-run/          # Cloud Run service module
│   ├── cloud-sql/          # Cloud SQL database module
│   ├── networking/         # VPC, subnets, NAT module
│   └── secrets/            # Secret Manager module
├── environments/
│   ├── dev/
│   │   └── terraform.tfvars
│   ├── staging/
│   │   └── terraform.tfvars
│   └── production/
│       └── terraform.tfvars
└── README.md
```

## Deployment Strategy

### Initial Deployment (One-Time Setup)
1. **Enable GCP APIs** (via Terraform or gcloud)
2. **Create Service Accounts** (least privilege)
3. **Create Terraform State Bucket** (GCS backend)
4. **Deploy Networking** (VPC, subnets, NAT)
5. **Deploy Cloud SQL** (database initialization)
6. **Store Secrets** (Secret Manager)
7. **Build Container** (Cloud Build)
8. **Deploy Cloud Run** (initial release)
9. **Run Database Migrations** (Cloud Run job)
10. **Verify Health Checks** (smoke tests)

### Continuous Deployment (Every Commit)
1. **GitHub Actions** triggers on push to main
2. **Run Tests** (cargo test, linting)
3. **Build Docker Image** (Cloud Build)
4. **Push to Artifact Registry** (tagged with git SHA)
5. **Deploy to Staging** (auto-deploy)
6. **Run E2E Tests** (smoke tests against staging)
7. **Manual Approval** (for production)
8. **Deploy to Production** (blue-green deployment)
9. **Health Check** (automatic rollback on failure)

### Database Migration Strategy
- **SQLx Migrations**: Embedded in application binary
- **Init Container**: Run migrations before app starts
- **Cloud Run Jobs**: Separate job for migrations
- **Rollback Plan**: Keep previous revision ready

## High Availability & Disaster Recovery

### High Availability
- **Cloud Run**: Multi-zone by default (no config needed)
- **Cloud SQL**: Regional HA with automatic failover
- **Read Replicas**: For read-heavy workloads
- **Health Checks**: Automatic instance replacement

### Disaster Recovery
- **RTO (Recovery Time Objective)**: 15 minutes
- **RPO (Recovery Point Objective)**: 5 minutes
- **Backup Strategy**:
  - Automated daily backups (Cloud SQL)
  - Point-in-time recovery (7 days)
  - Cross-region backup replication
  - Export to Cloud Storage (weekly)

### Monitoring & Alerting
- **Uptime SLI**: 99.9% availability
- **Latency SLI**: p95 < 500ms
- **Error Rate SLI**: < 0.1%
- **Alerts**:
  - Service down (critical)
  - Error rate spike (critical)
  - Database connections exhausted (warning)
  - Memory/CPU high (warning)

## Security Best Practices

### Network Security
- ✅ Private Cloud SQL (no public IP)
- ✅ VPC Service Controls (optional, for compliance)
- ✅ Cloud Armor for DDoS protection
- ✅ HTTPS only (Cloud Run enforces)

### Identity & Access
- ✅ Service accounts with least privilege
- ✅ Workload Identity Federation
- ✅ Secret Manager for credentials
- ✅ IAM audit logging

### Application Security
- ✅ JWT with RS256 signing
- ✅ Rate limiting (application-level)
- ✅ Input validation
- ✅ OWASP Top 10 compliance

### Compliance
- ✅ Encryption at rest (Cloud SQL, Secret Manager)
- ✅ Encryption in transit (TLS 1.3)
- ✅ Audit logs (Cloud Audit Logs)
- ✅ PII redaction middleware (already implemented)

## Scalability Plan

### Vertical Scaling
- **Cloud Run**: Increase CPU/memory per instance
- **Cloud SQL**: Upgrade tier (db-custom-X-YYYY)

### Horizontal Scaling
- **Cloud Run**: Increase max instances (auto-scaling)
- **Cloud SQL**: Add read replicas

### Performance Optimization
- **Database Connection Pooling**: SQLx pool (already implemented)
- **Caching**: Redis (Cloud Memorystore) for session cache
- **CDN**: Cloud CDN for MCP SDK static files (future)

### Load Testing
- **Tool**: k6 or Locust
- **Scenarios**:
  - Baseline: 100 RPS sustained
  - Peak: 1000 RPS burst
  - Soak: 200 RPS for 6 hours
- **Metrics**: Latency, error rate, resource utilization

## Migration from Current Setup

### From SQLite (Local Dev)
1. Export SQLite data: `.dump` command
2. Convert to PostgreSQL: `pgloader` tool
3. Import to Cloud SQL: `psql` command
4. Verify data integrity: checksums

### From Self-Managed PostgreSQL
1. Use `pg_dump` for full backup
2. Restore to Cloud SQL: `pg_restore`
3. Set up replication (optional): logical replication
4. Cutover: DNS/load balancer switch

## Next Steps

1. **Review Architecture**: Team approval
2. **Create GCP Project**: Separate projects for dev/staging/prod
3. **Set Up Terraform**: Initialize backend, write modules
4. **Deploy to Dev**: Test infrastructure code
5. **Deploy to Staging**: Full E2E testing
6. **Deploy to Production**: Gradual rollout
7. **Document Runbooks**: Incident response procedures

## References

- [Cloud Run Documentation](https://cloud.google.com/run/docs)
- [Cloud SQL for PostgreSQL](https://cloud.google.com/sql/docs/postgres)
- [Terraform GCP Provider](https://registry.terraform.io/providers/hashicorp/google/latest/docs)
- [GCP Best Practices](https://cloud.google.com/docs/enterprise/best-practices-for-enterprise-organizations)

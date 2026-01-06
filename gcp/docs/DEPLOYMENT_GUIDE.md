# Pierre MCP Server - GCP Deployment Guide

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Initial Setup](#initial-setup)
3. [Development Deployment](#development-deployment)
4. [Staging Deployment](#staging-deployment)
5. [Production Deployment](#production-deployment)
6. [Continuous Deployment](#continuous-deployment)
7. [Troubleshooting](#troubleshooting)
8. [Operations Runbook](#operations-runbook)

## Prerequisites

### Required Tools

```bash
# Install gcloud SDK
curl https://sdk.cloud.google.com | bash
exec -l $SHELL
gcloud init

# Install Terraform (v1.6+)
brew install terraform  # macOS
# or
wget https://releases.hashicorp.com/terraform/1.6.0/terraform_1.6.0_linux_amd64.zip
unzip terraform_1.6.0_linux_amd64.zip
sudo mv terraform /usr/local/bin/

# Install jq (for JSON parsing)
brew install jq  # macOS
sudo apt-get install jq  # Ubuntu/Debian

# Verify installations
gcloud version
terraform version
jq --version
```

### GCP Account Setup

1. **Create GCP Projects** (one per environment):
   ```bash
   # Development
   gcloud projects create pierre-mcp-dev --name="Pierre MCP Dev"

   # Staging
   gcloud projects create pierre-mcp-staging --name="Pierre MCP Staging"

   # Production
   gcloud projects create pierre-mcp-prod --name="Pierre MCP Production"
   ```

2. **Link Billing Account**:
   ```bash
   # List billing accounts
   gcloud billing accounts list

   # Link to projects
   gcloud billing projects link pierre-mcp-dev \
     --billing-account=BILLING_ACCOUNT_ID
   ```

3. **Configure OAuth Applications**:
   - **Strava**: https://www.strava.com/settings/api
   - **Garmin**: https://developer.garmin.com/
   - **Fitbit**: https://dev.fitbit.com/apps
   - **OpenWeatherMap**: https://openweathermap.org/api

## Initial Setup

### Step 1: Run GCP Project Setup Script

```bash
cd gcp/scripts

# Development environment
./setup-gcp-project.sh pierre-mcp-dev dev us-central1

# Staging environment
./setup-gcp-project.sh pierre-mcp-staging staging us-central1

# Production environment
./setup-gcp-project.sh pierre-mcp-prod production us-central1
```

This script:
- ✅ Enables required GCP APIs
- ✅ Creates service accounts
- ✅ Configures IAM permissions
- ✅ Creates Artifact Registry
- ✅ Creates Terraform state bucket
- ✅ Generates master encryption key

### Step 2: Store OAuth Secrets

```bash
# Set active project
gcloud config set project pierre-mcp-dev

# Store Strava credentials
echo "your-strava-client-secret" | \
  gcloud secrets create pierre-mcp-server-strava-client-secret-dev --data-file=-

# Store Garmin credentials
echo "your-garmin-client-secret" | \
  gcloud secrets create pierre-mcp-server-garmin-client-secret-dev --data-file=-

# Store Fitbit credentials
echo "your-fitbit-client-secret" | \
  gcloud secrets create pierre-mcp-server-fitbit-client-secret-dev --data-file=-

# Store OpenWeather API key
echo "your-openweather-api-key" | \
  gcloud secrets create pierre-mcp-server-openweather-api-key-dev --data-file=-

# Verify secrets
gcloud secrets list
```

### Step 3: Configure Terraform Variables

Edit the environment-specific tfvars files:

```bash
cd gcp/terraform/environments

# Edit dev environment
vim dev/terraform.tfvars
# Update:
#   - project_id
#   - container_image URL
#   - OAuth client IDs
#   - alert_email

# Repeat for staging and production
vim staging/terraform.tfvars
vim production/terraform.tfvars
```

### Step 4: Build Initial Container Image

```bash
# Build and push to Artifact Registry
cd /path/to/pierre_mcp_server

# Configure Docker authentication
gcloud auth configure-docker us-central1-docker.pkg.dev

# Build image
docker build -t us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.1.0 .

# Push image
docker push us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.1.0
```

## Development Deployment

### Deploy with Terraform

```bash
cd gcp/scripts

# Generate plan
./deploy-terraform.sh dev plan

# Review the plan, then apply
./deploy-terraform.sh dev apply
```

### Verify Deployment

```bash
# Get service URL
SERVICE_URL=$(cd ../terraform && terraform output -raw cloud_run_service_url)

# Test health endpoint
curl $SERVICE_URL/health

# Expected response: {"status":"ok"}

# View logs
gcloud logging read \
  "resource.type=cloud_run_revision AND resource.labels.service_name=pierre-mcp-server" \
  --limit 20 \
  --format json
```

### Deploy New Version

```bash
# Build new image
docker build -t us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.2.0 .
docker push us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.2.0

# Update Cloud Run
gcloud run deploy pierre-mcp-server \
  --image us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.2.0 \
  --region us-central1
```

## Staging Deployment

Staging mirrors production configuration for realistic testing.

```bash
# Deploy infrastructure
cd gcp/scripts
./deploy-terraform.sh staging plan
./deploy-terraform.sh staging apply

# Set up Cloud Build trigger (one-time)
gcloud builds triggers create github \
  --repo-name=pierre_mcp_server \
  --repo-owner=Async-IO \
  --branch-pattern="^main$" \
  --build-config=gcp/cloudbuild/cloudbuild.yaml \
  --substitutions=_ENVIRONMENT=staging

# Trigger manual build
gcloud builds submit \
  --config gcp/cloudbuild/cloudbuild.yaml \
  --substitutions=_ENVIRONMENT=staging
```

## Production Deployment

⚠️ **Production deployments require extra caution and approvals.**

### Pre-Deployment Checklist

- [ ] Code reviewed and approved
- [ ] Staging deployment successful
- [ ] Load testing completed
- [ ] Database migrations tested
- [ ] Rollback plan documented
- [ ] On-call engineer notified
- [ ] Monitoring dashboards reviewed

### Deploy Infrastructure (First Time)

```bash
cd gcp/scripts

# Review plan carefully
./deploy-terraform.sh production plan

# Get approval from team lead
# Then apply
./deploy-terraform.sh production apply
```

### Deploy Application (Canary Release)

```bash
# Tag release in git
git tag -a v1.0.0 -m "Production release v1.0.0"
git push origin v1.0.0

# Trigger production build with canary deployment
gcloud builds submit \
  --config gcp/cloudbuild/cloudbuild-production.yaml \
  --substitutions=TAG_NAME=v1.0.0,_ENVIRONMENT=production
```

The canary deployment:
1. Deploys new revision with 10% traffic
2. Monitors for 5 minutes
3. Provides commands for full rollout or rollback

### Complete Rollout (After Canary Success)

```bash
# Promote canary to 100% traffic
gcloud run services update-traffic pierre-mcp-server \
  --region us-central1 \
  --to-latest

# Monitor for 30 minutes
# Check error rates, latency, logs
```

### Rollback (If Issues Detected)

```bash
# List revisions
gcloud run revisions list \
  --service pierre-mcp-server \
  --region us-central1

# Rollback to previous revision
PREVIOUS_REVISION="pierre-mcp-server-00042-abc"  # Replace with actual
gcloud run services update-traffic pierre-mcp-server \
  --region us-central1 \
  --to-revisions=$PREVIOUS_REVISION=100
```

## Continuous Deployment

### GitHub Actions Integration

Create `.github/workflows/deploy-gcp.yml`:

```yaml
name: Deploy to GCP

on:
  push:
    branches:
      - main
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - id: auth
        uses: google-github-actions/auth@v2
        with:
          credentials_json: ${{ secrets.GCP_CREDENTIALS }}

      - name: Build and Deploy
        run: |
          ENV="staging"
          if [[ "${{ github.event_name }}" == "release" ]]; then
            ENV="production"
          fi

          gcloud builds submit \
            --config gcp/cloudbuild/cloudbuild.yaml \
            --substitutions=_ENVIRONMENT=$ENV
```

### Cloud Build Trigger Setup

```bash
# Connect GitHub repository
gcloud builds triggers create github \
  --name=pierre-mcp-staging-deploy \
  --repo-name=pierre_mcp_server \
  --repo-owner=Async-IO \
  --branch-pattern="^main$" \
  --build-config=gcp/cloudbuild/cloudbuild.yaml

# Production trigger (manual only)
gcloud builds triggers create github \
  --name=pierre-mcp-production-deploy \
  --repo-name=pierre_mcp_server \
  --repo-owner=Async-IO \
  --tag-pattern="^v[0-9]+\.[0-9]+\.[0-9]+$" \
  --build-config=gcp/cloudbuild/cloudbuild-production.yaml \
  --require-approval
```

## Troubleshooting

### Service Won't Start

```bash
# Check Cloud Run logs
gcloud logging read \
  "resource.type=cloud_run_revision" \
  --limit 50 \
  --format json | jq -r '.[] | .textPayload'

# Common issues:
# 1. Database connection failed
#    → Check Cloud SQL instance is running
#    → Verify VPC connector is attached

# 2. Secret access denied
#    → Check service account has secretAccessor role

# 3. Port mismatch
#    → Ensure container listens on port 8081
```

### Database Connection Issues

```bash
# Test Cloud SQL connectivity
gcloud sql instances describe pierre-mcp-server-postgres-dev \
  --format="value(connectionName)"

# Check VPC connector
gcloud compute networks vpc-access connectors describe \
  serverless-connector-dev \
  --region us-central1

# Verify private IP
gcloud sql instances describe pierre-mcp-server-postgres-dev \
  --format="value(ipAddresses[0].ipAddress)"
```

### Secret Manager Issues

```bash
# List secrets
gcloud secrets list

# Check secret value
gcloud secrets versions access latest \
  --secret=pierre-mcp-server-db-password-dev

# Verify IAM permissions
gcloud secrets get-iam-policy \
  pierre-mcp-server-db-password-dev
```

### Terraform State Issues

```bash
# Unlock stuck state
terraform force-unlock LOCK_ID

# Import existing resource
terraform import google_cloud_run_service.pierre_mcp_server \
  projects/PROJECT_ID/locations/REGION/services/SERVICE_NAME

# Refresh state
cd gcp/scripts
./deploy-terraform.sh dev refresh
```

## Operations Runbook

### Daily Operations

**Monitoring Dashboard**:
```bash
# Open Cloud Console monitoring
gcloud monitoring dashboards list
```

**Check Service Health**:
```bash
curl https://pierre-mcp-server-xxxxx.run.app/health
```

**View Recent Logs**:
```bash
gcloud logging read \
  "resource.type=cloud_run_revision" \
  --limit 100 \
  --format json | jq -r '.[] | "\(.timestamp) \(.textPayload)"'
```

### Incident Response

#### Service Down

1. **Check uptime**:
   ```bash
   gcloud monitoring uptime-checks list
   ```

2. **View error logs**:
   ```bash
   gcloud logging read \
     "resource.type=cloud_run_revision AND severity>=ERROR" \
     --limit 50
   ```

3. **Rollback if needed** (see rollback section above)

#### High Error Rate

1. **Check error distribution**:
   ```bash
   gcloud logging read \
     "resource.type=cloud_run_revision AND httpRequest.status>=400" \
     --limit 100 \
     --format json | jq -r '.[] | .httpRequest.status' | sort | uniq -c
   ```

2. **Identify failing endpoint**:
   ```bash
   gcloud logging read \
     "resource.type=cloud_run_revision AND httpRequest.status>=500" \
     --limit 20 \
     --format json | jq -r '.[] | .httpRequest.requestUrl'
   ```

#### Database Performance Issues

1. **Check Cloud SQL metrics**:
   ```bash
   gcloud sql operations list \
     --instance pierre-mcp-server-postgres-prod
   ```

2. **View slow queries**:
   ```bash
   gcloud logging read \
     "resource.type=cloudsql_database AND log_name=~postgres.log" \
     --limit 50
   ```

3. **Scale database if needed**:
   ```bash
   gcloud sql instances patch pierre-mcp-server-postgres-prod \
     --tier db-custom-8-32768
   ```

### Scaling Operations

**Increase Cloud Run instances**:
```bash
gcloud run services update pierre-mcp-server \
  --region us-central1 \
  --max-instances 200 \
  --min-instances 5
```

**Scale database**:
```bash
gcloud sql instances patch INSTANCE_NAME \
  --tier db-custom-4-16384
```

### Backup and Recovery

**Manual backup**:
```bash
gcloud sql backups create \
  --instance pierre-mcp-server-postgres-prod \
  --description "Manual backup before migration"
```

**Restore from backup**:
```bash
# List backups
gcloud sql backups list --instance INSTANCE_NAME

# Restore
gcloud sql backups restore BACKUP_ID \
  --backup-instance SOURCE_INSTANCE \
  --backup-id BACKUP_ID \
  --instance TARGET_INSTANCE
```

### Cost Optimization

**View current costs**:
```bash
gcloud billing accounts get-iam-policy BILLING_ACCOUNT_ID
```

**Optimize Cloud Run**:
- Scale to zero when not in use (dev/staging)
- Reduce min instances
- Right-size CPU/memory

**Optimize Cloud SQL**:
- Use smaller tier for dev/staging
- Disable HA for non-production
- Clean up old backups

## Support

- **Documentation**: `gcp/docs/`
- **GitHub Issues**: https://github.com/Async-IO/pierre_mcp_server/issues
- **On-Call**: platform-oncall@example.com

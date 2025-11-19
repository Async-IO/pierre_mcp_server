# Pierre MCP Server - GCP Quick Start

Get your Pierre MCP Server running on Google Cloud Platform in 30 minutes.

## Prerequisites

- GCP account with billing enabled
- `gcloud` CLI installed and authenticated
- Terraform 1.6+ installed
- Docker installed (for local builds)
- OAuth credentials from Strava/Garmin/Fitbit

## Quick Start (Development Environment)

### Step 1: Clone Repository

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
```

### Step 2: Create GCP Project

```bash
# Create project
gcloud projects create pierre-mcp-dev --name="Pierre MCP Dev"

# Set as active
gcloud config set project pierre-mcp-dev

# Link billing (replace with your billing account)
gcloud billing projects link pierre-mcp-dev \
  --billing-account=XXXXXX-XXXXXX-XXXXXX
```

### Step 3: Run Setup Script

```bash
cd gcp/scripts
./setup-gcp-project.sh pierre-mcp-dev dev us-central1
```

This script will:
- ✅ Enable 12 required GCP APIs (2-3 minutes)
- ✅ Create Artifact Registry for Docker images
- ✅ Create service account with IAM roles
- ✅ Create GCS bucket for Terraform state
- ✅ Generate master encryption key

### Step 4: Store OAuth Secrets

```bash
# Strava (get from https://www.strava.com/settings/api)
echo "YOUR_STRAVA_CLIENT_SECRET" | \
  gcloud secrets create pierre-mcp-server-strava-client-secret-dev \
  --data-file=-

# Garmin (optional)
echo "YOUR_GARMIN_CLIENT_SECRET" | \
  gcloud secrets create pierre-mcp-server-garmin-client-secret-dev \
  --data-file=-

# Fitbit (optional)
echo "YOUR_FITBIT_CLIENT_SECRET" | \
  gcloud secrets create pierre-mcp-server-fitbit-client-secret-dev \
  --data-file=-

# OpenWeather (optional, get from https://openweathermap.org/api)
echo "YOUR_OPENWEATHER_API_KEY" | \
  gcloud secrets create pierre-mcp-server-openweather-api-key-dev \
  --data-file=-
```

### Step 5: Build and Push Docker Image

```bash
cd /path/to/pierre_mcp_server

# Configure Docker auth
gcloud auth configure-docker us-central1-docker.pkg.dev

# Build image
docker build \
  -t us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.1.0 \
  -f Dockerfile \
  .

# Push to Artifact Registry
docker push us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.1.0
```

### Step 6: Configure Terraform

```bash
cd gcp/terraform/environments/dev

# Edit terraform.tfvars
vim terraform.tfvars
```

Update these values:
```hcl
project_id      = "pierre-mcp-dev"
container_image = "us-central1-docker.pkg.dev/pierre-mcp-dev/pierre-mcp/pierre-mcp-server:v0.1.0"

# OAuth client IDs (get from provider developer portals)
strava_client_id = "YOUR_STRAVA_CLIENT_ID"
garmin_client_id = "YOUR_GARMIN_CLIENT_ID"
fitbit_client_id = "YOUR_FITBIT_CLIENT_ID"

# Alert email
alert_email = "your-email@example.com"

# Secrets (managed via Secret Manager above)
secrets = {
  strava_client_secret = ""  # Already in Secret Manager
  garmin_client_secret = ""
  fitbit_client_secret = ""
  openweather_api_key  = ""
}
```

### Step 7: Deploy with Terraform

```bash
cd gcp/scripts

# Initialize Terraform (first time only)
cd ../terraform
terraform init -backend-config="bucket=pierre-mcp-dev-terraform-state"

# Back to scripts directory
cd ../scripts

# Generate plan
./deploy-terraform.sh dev plan

# Review the plan output, then apply
./deploy-terraform.sh dev apply
```

### Step 8: Verify Deployment

```bash
# Get service URL
cd ../terraform
SERVICE_URL=$(terraform output -raw cloud_run_service_url)
echo "Service URL: $SERVICE_URL"

# Test health endpoint
curl $SERVICE_URL/health
# Expected: {"status":"ok"}

# Test MCP tools endpoint
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" $SERVICE_URL/mcp/tools
```

### Step 9: View Logs

```bash
# Real-time logs
gcloud logging tail "resource.type=cloud_run_revision"

# Last 50 logs
gcloud logging read \
  "resource.type=cloud_run_revision AND resource.labels.service_name=pierre-mcp-server" \
  --limit 50 \
  --format json
```

## Next Steps

### 1. Register MCP Client (Claude Desktop)

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "YOUR_CLOUD_RUN_URL"
      ]
    }
  }
}
```

### 2. Create Admin User

```bash
SERVICE_URL=$(cd ../terraform && terraform output -raw cloud_run_service_url)

curl -X POST $SERVICE_URL/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "Admin User"
  }'
```

### 3. Test OAuth Flow

```bash
# Navigate to OAuth initiation endpoint
open "$SERVICE_URL/api/oauth/auth/strava/YOUR_USER_ID"

# Complete authorization in browser
# Check status
curl "$SERVICE_URL/api/oauth/status" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### 4. Set Up Continuous Deployment

```bash
# Create Cloud Build trigger
gcloud builds triggers create github \
  --name=pierre-mcp-dev-deploy \
  --repo-name=pierre_mcp_server \
  --repo-owner=Async-IO \
  --branch-pattern="^main$" \
  --build-config=gcp/cloudbuild/cloudbuild.yaml \
  --substitutions=_ENVIRONMENT=dev
```

Now every push to `main` branch will automatically deploy to dev!

## Troubleshooting

### Container fails to start

```bash
# Check logs
gcloud logging read \
  "resource.type=cloud_run_revision AND severity>=ERROR" \
  --limit 20

# Common issues:
# 1. Database connection failed → Check VPC connector
# 2. Secret access denied → Verify IAM permissions
# 3. Port mismatch → Ensure HTTP_PORT=8081
```

### Database connection issues

```bash
# Check Cloud SQL status
gcloud sql instances describe pierre-mcp-server-postgres-dev

# Check VPC connector
gcloud compute networks vpc-access connectors describe \
  serverless-connector-dev \
  --region us-central1
```

### Terraform errors

```bash
# Refresh state
cd gcp/scripts
./deploy-terraform.sh dev refresh

# Force unlock (if stuck)
cd ../terraform
terraform force-unlock LOCK_ID

# Destroy and recreate (dev only!)
cd ../scripts
./deploy-terraform.sh dev destroy
./deploy-terraform.sh dev apply
```

## Clean Up

To avoid charges, destroy resources when not needed:

```bash
cd gcp/scripts
./deploy-terraform.sh dev destroy
```

This will:
- Delete Cloud Run service
- Delete Cloud SQL instance
- Delete VPC and networking
- Keep Terraform state and Artifact Registry

To delete everything:
```bash
# Delete project (CAUTION: Irreversible!)
gcloud projects delete pierre-mcp-dev
```

## Cost Monitoring

```bash
# View current month costs
gcloud billing projects describe pierre-mcp-dev \
  --format="value(billingAccountName)"

# Set budget alert (via console)
# Navigate to: Billing → Budgets & alerts
# Set alert at $50/month for dev
```

## Support

- **Documentation**: [Full Deployment Guide](./DEPLOYMENT_GUIDE.md)
- **Architecture**: [Architecture Overview](./ARCHITECTURE.md)
- **Issues**: https://github.com/Async-IO/pierre_mcp_server/issues
- **Discussions**: https://github.com/Async-IO/pierre_mcp_server/discussions

## What's Next?

1. **Staging Environment**: Repeat steps for `pierre-mcp-staging`
2. **Custom Domain**: Configure Cloud Run custom domain
3. **Monitoring Dashboards**: Set up Cloud Monitoring dashboards
4. **Load Testing**: Test with k6 or Locust
5. **Production Deployment**: Follow [Production Deployment Guide](./DEPLOYMENT_GUIDE.md#production-deployment)

# Pierre MCP Server - Terraform Infrastructure

Infrastructure as Code (IaC) for deploying Pierre MCP Server on Google Cloud Platform.

## Directory Structure

```
terraform/
├── main.tf              # Primary infrastructure resources
├── variables.tf         # Input variable definitions
├── outputs.tf           # Output value definitions
├── versions.tf          # Terraform and provider versions
├── backend.tf           # Remote state configuration
├── environments/        # Environment-specific configurations
│   ├── dev/
│   │   └── terraform.tfvars
│   ├── staging/
│   │   └── terraform.tfvars
│   └── production/
│       └── terraform.tfvars
└── README.md           # This file
```

## Prerequisites

1. **Terraform CLI** (v1.6+):
   ```bash
   brew install terraform  # macOS
   # or download from https://www.terraform.io/downloads
   ```

2. **Google Cloud SDK**:
   ```bash
   curl https://sdk.cloud.google.com | bash
   gcloud auth application-default login
   ```

3. **GCP Project Setup**:
   ```bash
   # Run the setup script first
   ../scripts/setup-gcp-project.sh PROJECT_ID ENVIRONMENT REGION
   ```

## Quick Start

### 1. Initialize Terraform

```bash
cd terraform

# Initialize with remote state backend
terraform init -backend-config="bucket=YOUR_PROJECT_ID-terraform-state"
```

### 2. Configure Variables

Edit the appropriate environment file:

```bash
vim environments/dev/terraform.tfvars
```

Required variables:
- `project_id`: GCP project ID
- `container_image`: Docker image URL from Artifact Registry
- `strava_client_id`, `garmin_client_id`, `fitbit_client_id`: OAuth app IDs

### 3. Plan Deployment

```bash
terraform plan -var-file=environments/dev/terraform.tfvars -out=dev.tfplan
```

Review the plan output carefully!

### 4. Apply Changes

```bash
terraform apply dev.tfplan
```

Or use the deployment script:

```bash
cd ../scripts
./deploy-terraform.sh dev apply
```

## Resources Created

### Networking
- **VPC Network**: Custom VPC with private subnets
- **Subnet**: Regional subnet (10.0.0.0/24)
- **Serverless VPC Connector**: Bridge Cloud Run ↔ Cloud SQL
- **Cloud Router + NAT**: Outbound connectivity for external APIs
- **Private VPC Connection**: For Cloud SQL private IP

### Compute
- **Cloud Run Service**: Serverless container deployment
  - Auto-scaling (0-100 instances)
  - CPU: 1-2 vCPU
  - Memory: 512Mi-2Gi
  - Concurrency: 80 requests/instance
  - Health checks: `/health` endpoint

### Database
- **Cloud SQL Instance**: PostgreSQL 16
  - Tier: db-f1-micro (dev) to db-custom-4-16384 (prod)
  - Private IP only
  - Automated backups (daily at 3 AM UTC)
  - Point-in-time recovery
  - High availability (production only)

### Security
- **Service Account**: `pierre-mcp-server-runner-{env}`
  - IAM roles for Cloud SQL, Secret Manager, Logging
- **Secret Manager Secrets**:
  - Database password (auto-generated)
  - Master encryption key
  - OAuth client secrets
  - External API keys

### Monitoring
- **Uptime Check**: `/health` endpoint monitoring
- **Alert Policy**: Service downtime notifications
- **Notification Channel**: Email alerts

## Variables Reference

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `project_id` | GCP project ID | `pierre-mcp-dev` |
| `container_image` | Docker image URL | `gcr.io/PROJECT/image:tag` |
| `environment` | Environment name | `dev`, `staging`, `production` |

### Optional Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `region` | `us-central1` | GCP region |
| `cloud_run_cpu` | `"1"` | CPU allocation |
| `cloud_run_memory` | `"512Mi"` | Memory allocation |
| `cloud_run_min_instances` | `0` | Minimum instances |
| `cloud_run_max_instances` | `100` | Maximum instances |
| `database_tier` | `db-f1-micro` | Cloud SQL tier |
| `database_high_availability` | `false` | Enable HA |
| `enable_uptime_checks` | `true` | Enable monitoring |
| `alert_email` | `""` | Alert recipient |

See `variables.tf` for complete list.

## Outputs

After successful deployment:

```bash
terraform output
```

Key outputs:
- `cloud_run_service_url`: Public URL for the service
- `database_connection_name`: Cloud SQL connection string
- `health_check_url`: Health endpoint URL
- `deployment_instructions`: Quick reference guide

## Managing Secrets

### Add Secrets via Terraform

Edit `environments/{env}/terraform.tfvars`:

```hcl
secrets = {
  strava_client_secret  = "your-secret"
  garmin_client_secret  = "your-secret"
  fitbit_client_secret  = "your-secret"
  openweather_api_key   = "your-key"
}
```

**Warning**: Never commit secrets to git! Use `.gitignore` or environment variables.

### Add Secrets via gcloud CLI (Recommended)

```bash
echo "your-secret" | \
  gcloud secrets create pierre-mcp-server-strava-client-secret-dev \
  --data-file=-
```

Terraform will automatically detect and use existing secrets.

## State Management

### Remote State Backend

Terraform state is stored in Google Cloud Storage:

```hcl
# backend.tf
terraform {
  backend "gcs" {
    bucket = "PROJECT_ID-terraform-state"
    prefix = "pierre-mcp-server"
  }
}
```

### State Locking

GCS backend provides automatic state locking to prevent concurrent modifications.

### Viewing State

```bash
# List all resources
terraform state list

# Show specific resource
terraform state show google_cloud_run_service.pierre_mcp_server

# Pull state locally (read-only)
terraform state pull > current-state.json
```

### Importing Existing Resources

If resources were created manually:

```bash
terraform import \
  google_cloud_run_service.pierre_mcp_server \
  projects/PROJECT_ID/locations/REGION/services/SERVICE_NAME
```

## Multi-Environment Management

### Workspace Strategy (Not Recommended)

We use separate state files per environment instead of Terraform workspaces.

### Environment Isolation

Each environment has:
- ✅ Separate GCP project
- ✅ Separate state file (via `terraform.tfvars`)
- ✅ Separate resource naming (`{resource}-{env}`)
- ✅ Separate IAM permissions

### Deploying Multiple Environments

```bash
# Development
terraform apply -var-file=environments/dev/terraform.tfvars

# Staging
terraform apply -var-file=environments/staging/terraform.tfvars

# Production
terraform apply -var-file=environments/production/terraform.tfvars
```

## Common Operations

### Update Container Image

```bash
# Option 1: Update tfvars and re-apply
vim environments/dev/terraform.tfvars
# Change: container_image = "...new-tag"
terraform apply -var-file=environments/dev/terraform.tfvars

# Option 2: Use gcloud directly (faster)
gcloud run deploy pierre-mcp-server \
  --image=NEW_IMAGE_URL \
  --region=us-central1
```

### Scale Cloud Run

```bash
# Via Terraform: Edit tfvars
cloud_run_min_instances = 2
cloud_run_max_instances = 200

# Via gcloud (immediate)
gcloud run services update pierre-mcp-server \
  --region=us-central1 \
  --min-instances=2 \
  --max-instances=200
```

### Upgrade Database Tier

```bash
# Edit tfvars
database_tier = "db-custom-4-16384"

# Apply (will cause brief downtime)
terraform apply -var-file=environments/production/terraform.tfvars
```

### Add New Secret

```bash
# Create secret
gcloud secrets create pierre-mcp-server-new-secret-dev \
  --data-file=-

# Grant access
gcloud secrets add-iam-policy-binding \
  pierre-mcp-server-new-secret-dev \
  --member="serviceAccount:SERVICE_ACCOUNT@PROJECT.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"
```

## Troubleshooting

### Plan Fails with API Errors

```bash
# Enable required APIs
gcloud services enable run.googleapis.com sqladmin.googleapis.com ...

# Or run setup script
../scripts/setup-gcp-project.sh PROJECT_ID ENV REGION
```

### State Lock Stuck

```bash
terraform force-unlock LOCK_ID
```

### Resource Already Exists

```bash
# Import existing resource
terraform import RESOURCE_TYPE.NAME RESOURCE_ID

# Or delete manually
gcloud run services delete SERVICE_NAME --region=REGION
```

### Database Won't Delete (Protection)

For production, `deletion_protection = true` prevents accidental deletion.

To delete:
```bash
# Option 1: Disable protection
database_deletion_protection = false
terraform apply

# Option 2: Delete manually
gcloud sql instances delete INSTANCE_NAME
```

## Best Practices

### 1. Never Commit Secrets

Add to `.gitignore`:
```
*.tfvars
!environments/*/terraform.tfvars.example
*.tfstate
*.tfstate.backup
.terraform/
```

### 2. Use Separate Projects per Environment

```
pierre-mcp-dev
pierre-mcp-staging
pierre-mcp-prod
```

### 3. Tag Resources

```hcl
labels = {
  environment = "production"
  managed_by  = "terraform"
  team        = "platform"
  cost_center = "engineering"
}
```

### 4. Enable Deletion Protection

```hcl
deletion_protection = var.environment == "production" ? true : false
```

### 5. Use Variables for Everything

Never hardcode values in `main.tf`.

### 6. Document Changes

```bash
git commit -m "infra: increase Cloud Run max instances to 200"
```

## Cost Estimation

Before applying:

```bash
# Install cost estimation tool
terraform plan -out=plan.tfplan
terraform show -json plan.tfplan | infracost breakdown --path -
```

## Terraform Modules (Future)

For reusability across projects, consider extracting into modules:

```
modules/
├── cloud-run/
├── cloud-sql/
├── networking/
└── monitoring/
```

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Terraform Apply
  run: |
    cd gcp/terraform
    terraform init -backend-config="bucket=${{ secrets.TF_STATE_BUCKET }}"
    terraform apply -var-file=environments/${{ matrix.env }}/terraform.tfvars -auto-approve
```

### Cloud Build Integration

```yaml
steps:
  - name: 'hashicorp/terraform'
    args: ['init', '-backend-config=bucket=$_TF_STATE_BUCKET']
    dir: 'gcp/terraform'

  - name: 'hashicorp/terraform'
    args: ['apply', '-var-file=environments/$_ENVIRONMENT/terraform.tfvars', '-auto-approve']
    dir: 'gcp/terraform'
```

## Support

- **Terraform Registry**: https://registry.terraform.io/providers/hashicorp/google/latest/docs
- **GCP Terraform Examples**: https://github.com/terraform-google-modules
- **Project Issues**: https://github.com/Async-IO/pierre_mcp_server/issues

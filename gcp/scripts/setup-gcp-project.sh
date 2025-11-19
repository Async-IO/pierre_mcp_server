#!/usr/bin/env bash
# Setup GCP Project for Pierre MCP Server
# Purpose: One-time setup of GCP project, APIs, service accounts, and permissions
# Usage: ./setup-gcp-project.sh <project-id> <environment>

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script arguments
PROJECT_ID="${1:-}"
ENVIRONMENT="${2:-dev}"
REGION="${3:-us-central1}"

if [ -z "$PROJECT_ID" ]; then
    echo -e "${RED}❌ Error: Project ID required${NC}"
    echo "Usage: $0 <project-id> [environment] [region]"
    echo "Example: $0 pierre-mcp-dev dev us-central1"
    exit 1
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Pierre MCP Server - GCP Project Setup${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Project ID:  $PROJECT_ID"
echo "Environment: $ENVIRONMENT"
echo "Region:      $REGION"
echo ""

# Confirm with user
read -p "Continue with setup? (y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Setup cancelled"
    exit 1
fi

# Set active project
echo -e "${YELLOW}📋 Setting active GCP project...${NC}"
gcloud config set project "$PROJECT_ID"

# Enable required APIs
echo -e "${YELLOW}🔌 Enabling required GCP APIs (this may take 2-3 minutes)...${NC}"
gcloud services enable \
    run.googleapis.com \
    sqladmin.googleapis.com \
    compute.googleapis.com \
    vpcaccess.googleapis.com \
    servicenetworking.googleapis.com \
    secretmanager.googleapis.com \
    cloudresourcemanager.googleapis.com \
    iam.googleapis.com \
    logging.googleapis.com \
    monitoring.googleapis.com \
    artifactregistry.googleapis.com \
    cloudbuild.googleapis.com

echo -e "${GREEN}✅ APIs enabled${NC}"

# Create Artifact Registry repository
echo -e "${YELLOW}📦 Creating Artifact Registry repository...${NC}"
if ! gcloud artifacts repositories describe pierre-mcp --location="$REGION" &>/dev/null; then
    gcloud artifacts repositories create pierre-mcp \
        --repository-format=docker \
        --location="$REGION" \
        --description="Pierre MCP Server container images"
    echo -e "${GREEN}✅ Artifact Registry repository created${NC}"
else
    echo -e "${YELLOW}ℹ️  Artifact Registry repository already exists${NC}"
fi

# Create service account for Cloud Run
echo -e "${YELLOW}👤 Creating Cloud Run service account...${NC}"
SA_NAME="pierre-mcp-server-runner-$ENVIRONMENT"
SA_EMAIL="$SA_NAME@$PROJECT_ID.iam.gserviceaccount.com"

if ! gcloud iam service-accounts describe "$SA_EMAIL" &>/dev/null; then
    gcloud iam service-accounts create "$SA_NAME" \
        --display-name="Cloud Run service account for Pierre MCP ($ENVIRONMENT)" \
        --description="Service account with least-privilege access"
    echo -e "${GREEN}✅ Service account created: $SA_EMAIL${NC}"
else
    echo -e "${YELLOW}ℹ️  Service account already exists${NC}"
fi

# Grant IAM roles to service account
echo -e "${YELLOW}🔐 Granting IAM roles to service account...${NC}"
for role in \
    "roles/cloudsql.client" \
    "roles/secretmanager.secretAccessor" \
    "roles/logging.logWriter" \
    "roles/monitoring.metricWriter" \
    "roles/cloudtrace.agent"; do

    gcloud projects add-iam-policy-binding "$PROJECT_ID" \
        --member="serviceAccount:$SA_EMAIL" \
        --role="$role" \
        --condition=None \
        --quiet
done
echo -e "${GREEN}✅ IAM roles granted${NC}"

# Configure Cloud Build service account permissions
echo -e "${YELLOW}🏗️  Configuring Cloud Build permissions...${NC}"
PROJECT_NUMBER=$(gcloud projects describe "$PROJECT_ID" --format="value(projectNumber)")
CLOUD_BUILD_SA="$PROJECT_NUMBER@cloudbuild.gserviceaccount.com"

for role in \
    "roles/run.admin" \
    "roles/iam.serviceAccountUser"; do

    gcloud projects add-iam-policy-binding "$PROJECT_ID" \
        --member="serviceAccount:$CLOUD_BUILD_SA" \
        --role="$role" \
        --condition=None \
        --quiet
done
echo -e "${GREEN}✅ Cloud Build permissions configured${NC}"

# Create GCS bucket for Terraform state
echo -e "${YELLOW}🪣 Creating GCS bucket for Terraform state...${NC}"
BUCKET_NAME="$PROJECT_ID-terraform-state"

if ! gsutil ls -b "gs://$BUCKET_NAME" &>/dev/null; then
    gsutil mb -p "$PROJECT_ID" -l "$REGION" "gs://$BUCKET_NAME"
    gsutil versioning set on "gs://$BUCKET_NAME"
    gsutil uniformbucketlevelaccess set on "gs://$BUCKET_NAME"
    echo -e "${GREEN}✅ Terraform state bucket created: gs://$BUCKET_NAME${NC}"
else
    echo -e "${YELLOW}ℹ️  Terraform state bucket already exists${NC}"
fi

# Create initial secrets in Secret Manager
echo -e "${YELLOW}🔑 Creating Secret Manager secrets...${NC}"

create_secret_if_not_exists() {
    local secret_name=$1
    local secret_value=$2

    if ! gcloud secrets describe "$secret_name" &>/dev/null; then
        echo "$secret_value" | gcloud secrets create "$secret_name" \
            --data-file=- \
            --replication-policy="automatic"
        echo -e "${GREEN}✅ Created secret: $secret_name${NC}"
    else
        echo -e "${YELLOW}ℹ️  Secret already exists: $secret_name${NC}"
    fi
}

# Generate master encryption key (base64-encoded 32-byte key)
MASTER_KEY=$(openssl rand -base64 32)
create_secret_if_not_exists "pierre-mcp-server-master-encryption-key-$ENVIRONMENT" "$MASTER_KEY"

echo -e "${YELLOW}ℹ️  Note: OAuth secrets (Strava, Garmin, Fitbit, OpenWeather) should be added manually:${NC}"
echo ""
echo "  gcloud secrets create pierre-mcp-server-strava-client-secret-$ENVIRONMENT --data-file=- <<< 'YOUR_SECRET'"
echo "  gcloud secrets create pierre-mcp-server-garmin-client-secret-$ENVIRONMENT --data-file=- <<< 'YOUR_SECRET'"
echo "  gcloud secrets create pierre-mcp-server-fitbit-client-secret-$ENVIRONMENT --data-file=- <<< 'YOUR_SECRET'"
echo "  gcloud secrets create pierre-mcp-server-openweather-api-key-$ENVIRONMENT --data-file=- <<< 'YOUR_KEY'"
echo ""

# Summary
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}✅ GCP Project Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Project ID:         $PROJECT_ID"
echo "Region:             $REGION"
echo "Service Account:    $SA_EMAIL"
echo "Artifact Registry:  $REGION-docker.pkg.dev/$PROJECT_ID/pierre-mcp"
echo "Terraform Bucket:   gs://$BUCKET_NAME"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "1. Add OAuth secrets to Secret Manager (see commands above)"
echo "2. Configure Terraform backend:"
echo "   cd gcp/terraform"
echo "   terraform init -backend-config=\"bucket=$BUCKET_NAME\""
echo "3. Review and update terraform.tfvars in gcp/terraform/environments/$ENVIRONMENT/"
echo "4. Run Terraform:"
echo "   terraform plan -var-file=environments/$ENVIRONMENT/terraform.tfvars"
echo "   terraform apply -var-file=environments/$ENVIRONMENT/terraform.tfvars"
echo ""

#!/usr/bin/env bash
# ABOUTME: One-time GCP project setup script
# ABOUTME: Enables APIs, creates Cloud SQL instance, Artifact Registry, and VPC connector

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/../config.sh"

echo "=== Pierre MCP Server - GCP Setup ==="
echo "Project: ${GCP_PROJECT_ID}"
echo "Region:  ${GCP_REGION}"
echo ""

# Ensure we're using the right project
gcloud config set project "${GCP_PROJECT_ID}"

# Step 1: Enable required APIs
echo ">>> Enabling required APIs..."
gcloud services enable \
    run.googleapis.com \
    sqladmin.googleapis.com \
    secretmanager.googleapis.com \
    artifactregistry.googleapis.com \
    cloudbuild.googleapis.com \
    compute.googleapis.com \
    vpcaccess.googleapis.com \
    aiplatform.googleapis.com \
    --quiet

echo "APIs enabled (including Vertex AI for LLM)."

# Step 2: Create Artifact Registry repository
echo ""
echo ">>> Creating Artifact Registry repository..."
if gcloud artifacts repositories describe "${REGISTRY_NAME}" \
    --location="${GCP_REGION}" &>/dev/null; then
    echo "Repository '${REGISTRY_NAME}' already exists."
else
    gcloud artifacts repositories create "${REGISTRY_NAME}" \
        --repository-format=docker \
        --location="${GCP_REGION}" \
        --description="Pierre MCP Server container images"
    echo "Repository created."
fi

# Step 3: Create Cloud SQL instance
echo ""
echo ">>> Creating Cloud SQL PostgreSQL instance..."
if gcloud sql instances describe "${DB_INSTANCE_NAME}" &>/dev/null; then
    echo "Instance '${DB_INSTANCE_NAME}' already exists."
else
    gcloud sql instances create "${DB_INSTANCE_NAME}" \
        --database-version="${DB_VERSION}" \
        --tier="${DB_TIER}" \
        --region="${GCP_REGION}" \
        --storage-type=SSD \
        --storage-size=10GB \
        --storage-auto-increase \
        --backup-start-time="03:00" \
        --maintenance-window-day=SUN \
        --maintenance-window-hour=3 \
        --deletion-protection
    echo "Instance created."
fi

# Step 4: Create database
echo ""
echo ">>> Creating database..."
if gcloud sql databases describe "${DB_NAME}" --instance="${DB_INSTANCE_NAME}" &>/dev/null; then
    echo "Database '${DB_NAME}' already exists."
else
    gcloud sql databases create "${DB_NAME}" --instance="${DB_INSTANCE_NAME}"
    echo "Database created."
fi

# Step 5: Create database user (password will be stored in Secret Manager)
echo ""
echo ">>> Creating database user..."
DB_PASSWORD=$(openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c 32)
if gcloud sql users describe "${DB_USER}" --instance="${DB_INSTANCE_NAME}" &>/dev/null; then
    echo "User '${DB_USER}' already exists. Updating password..."
    gcloud sql users set-password "${DB_USER}" \
        --instance="${DB_INSTANCE_NAME}" \
        --password="${DB_PASSWORD}"
else
    gcloud sql users create "${DB_USER}" \
        --instance="${DB_INSTANCE_NAME}" \
        --password="${DB_PASSWORD}"
    echo "User created."
fi

# Step 6: Store database password in Secret Manager
echo ""
echo ">>> Storing database password in Secret Manager..."
SECRET_NAME="${SERVICE_NAME}-db-password"
if gcloud secrets describe "${SECRET_NAME}" &>/dev/null; then
    echo "Secret exists. Adding new version..."
    echo -n "${DB_PASSWORD}" | gcloud secrets versions add "${SECRET_NAME}" --data-file=-
else
    echo -n "${DB_PASSWORD}" | gcloud secrets create "${SECRET_NAME}" \
        --replication-policy="automatic" \
        --data-file=-
fi
echo "Password stored in Secret Manager."

# Step 7: Generate and store master encryption key
echo ""
echo ">>> Creating master encryption key..."
ENCRYPTION_KEY=$(openssl rand -base64 32)
SECRET_NAME="${SERVICE_NAME}-encryption-key"
if gcloud secrets describe "${SECRET_NAME}" &>/dev/null; then
    echo "Encryption key secret exists."
else
    echo -n "${ENCRYPTION_KEY}" | gcloud secrets create "${SECRET_NAME}" \
        --replication-policy="automatic" \
        --data-file=-
    echo "Encryption key created and stored."
fi

# Step 8: Create VPC connector for Cloud Run -> Cloud SQL
echo ""
echo ">>> Creating Serverless VPC Access connector..."
CONNECTOR_NAME="pierre-vpc-connector"
if gcloud compute networks vpc-access connectors describe "${CONNECTOR_NAME}" \
    --region="${GCP_REGION}" &>/dev/null; then
    echo "VPC connector '${CONNECTOR_NAME}' already exists."
else
    gcloud compute networks vpc-access connectors create "${CONNECTOR_NAME}" \
        --region="${GCP_REGION}" \
        --range="10.8.0.0/28" \
        --min-instances=2 \
        --max-instances=3 \
        --machine-type=e2-micro
    echo "VPC connector created."
fi

# Step 9: Get Cloud SQL connection name for later
echo ""
echo ">>> Getting Cloud SQL connection info..."
CONNECTION_NAME=$(gcloud sql instances describe "${DB_INSTANCE_NAME}" \
    --format="value(connectionName)")
echo "Cloud SQL connection name: ${CONNECTION_NAME}"

# Step 10: Grant Vertex AI access to Cloud Run service account
echo ""
echo ">>> Granting Vertex AI access to Cloud Run service account..."
PROJECT_NUMBER=$(gcloud projects describe "${GCP_PROJECT_ID}" --format="value(projectNumber)")
CLOUD_RUN_SA="${PROJECT_NUMBER}-compute@developer.gserviceaccount.com"

gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${CLOUD_RUN_SA}" \
    --role="roles/aiplatform.user" \
    --quiet

echo "Vertex AI access granted to Cloud Run service account."

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Add OAuth secrets:  ./scripts/secrets.sh add strava-client-id YOUR_ID"
echo "2. Build the image:    ./scripts/build.sh"
echo "3. Deploy:             ./scripts/deploy.sh"
echo ""
echo "Cloud SQL Connection: ${CONNECTION_NAME}"

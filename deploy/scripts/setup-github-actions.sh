#!/usr/bin/env bash
# ABOUTME: Set up Workload Identity Federation for GitHub Actions
# ABOUTME: Allows GitHub to deploy to GCP without storing service account keys

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/../config.sh"

# GitHub repository (change if different)
GITHUB_ORG="Async-IO"
GITHUB_REPO="pierre_mcp_server"

echo "=== GitHub Actions - GCP Authentication Setup ==="
echo ""
echo "This creates Workload Identity Federation, which lets GitHub Actions"
echo "authenticate to GCP without service account JSON keys."
echo ""
echo "Project:    ${GCP_PROJECT_ID}"
echo "Repository: ${GITHUB_ORG}/${GITHUB_REPO}"
echo ""

# Step 1: Enable required APIs
echo ">>> Enabling IAM APIs..."
gcloud services enable \
    iamcredentials.googleapis.com \
    iam.googleapis.com \
    --quiet

# Step 2: Create Workload Identity Pool
POOL_NAME="github-pool"
echo ""
echo ">>> Creating Workload Identity Pool..."
if gcloud iam workload-identity-pools describe "${POOL_NAME}" \
    --location="global" &>/dev/null; then
    echo "Pool '${POOL_NAME}' already exists."
else
    gcloud iam workload-identity-pools create "${POOL_NAME}" \
        --location="global" \
        --display-name="GitHub Actions Pool" \
        --description="Identity pool for GitHub Actions CI/CD"
fi

# Step 3: Create Workload Identity Provider
PROVIDER_NAME="github-provider"
echo ""
echo ">>> Creating Workload Identity Provider..."
if gcloud iam workload-identity-pools providers describe "${PROVIDER_NAME}" \
    --location="global" \
    --workload-identity-pool="${POOL_NAME}" &>/dev/null; then
    echo "Provider '${PROVIDER_NAME}' already exists."
else
    gcloud iam workload-identity-pools providers create-oidc "${PROVIDER_NAME}" \
        --location="global" \
        --workload-identity-pool="${POOL_NAME}" \
        --display-name="GitHub Provider" \
        --issuer-uri="https://token.actions.githubusercontent.com" \
        --attribute-mapping="google.subject=assertion.sub,attribute.actor=assertion.actor,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
        --attribute-condition="assertion.repository_owner == '${GITHUB_ORG}'"
fi

# Step 4: Create Service Account for GitHub Actions
SA_NAME="github-actions-deployer"
SA_EMAIL="${SA_NAME}@${GCP_PROJECT_ID}.iam.gserviceaccount.com"
echo ""
echo ">>> Creating Service Account..."
if gcloud iam service-accounts describe "${SA_EMAIL}" &>/dev/null; then
    echo "Service account '${SA_NAME}' already exists."
else
    gcloud iam service-accounts create "${SA_NAME}" \
        --display-name="GitHub Actions Deployer" \
        --description="Service account for GitHub Actions CI/CD deployments"
fi

# Step 5: Grant permissions to Service Account
echo ""
echo ">>> Granting permissions to service account..."

# Cloud Run Admin (deploy services)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/run.admin" \
    --quiet

# Cloud Build Editor (submit builds)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/cloudbuild.builds.editor" \
    --quiet

# Artifact Registry Writer (push images)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/artifactregistry.writer" \
    --quiet

# Secret Manager Secret Accessor (read secrets for deployment)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/secretmanager.secretAccessor" \
    --quiet

# Cloud SQL Client (for connection info)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/cloudsql.client" \
    --quiet

# Service Account User (to deploy as the Cloud Run service account)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/iam.serviceAccountUser" \
    --quiet

# Storage Admin (for Cloud Build logs)
gcloud projects add-iam-policy-binding "${GCP_PROJECT_ID}" \
    --member="serviceAccount:${SA_EMAIL}" \
    --role="roles/storage.admin" \
    --quiet

echo "Permissions granted."

# Step 6: Allow GitHub to impersonate the service account
echo ""
echo ">>> Allowing GitHub to impersonate service account..."
POOL_ID=$(gcloud iam workload-identity-pools describe "${POOL_NAME}" \
    --location="global" \
    --format="value(name)")

gcloud iam service-accounts add-iam-policy-binding "${SA_EMAIL}" \
    --role="roles/iam.workloadIdentityUser" \
    --member="principalSet://iam.googleapis.com/${POOL_ID}/attribute.repository/${GITHUB_ORG}/${GITHUB_REPO}" \
    --quiet

# Step 7: Get the values needed for GitHub Secrets
echo ""
echo "=== Setup Complete ==="
echo ""
echo "Add these secrets to your GitHub repository:"
echo "  Settings → Secrets and variables → Actions → New repository secret"
echo ""
echo "┌─────────────────────────────────────────────────────────────────────┐"
echo "│ GCP_WORKLOAD_IDENTITY_PROVIDER                                      │"
echo "├─────────────────────────────────────────────────────────────────────┤"

PROVIDER_ID=$(gcloud iam workload-identity-pools providers describe "${PROVIDER_NAME}" \
    --location="global" \
    --workload-identity-pool="${POOL_NAME}" \
    --format="value(name)")
echo "│ ${PROVIDER_ID}"
echo "└─────────────────────────────────────────────────────────────────────┘"
echo ""
echo "┌─────────────────────────────────────────────────────────────────────┐"
echo "│ GCP_SERVICE_ACCOUNT                                                 │"
echo "├─────────────────────────────────────────────────────────────────────┤"
echo "│ ${SA_EMAIL}"
echo "└─────────────────────────────────────────────────────────────────────┘"
echo ""
echo "Also add these secrets for OAuth/Firebase:"
echo "  - STRAVA_CLIENT_ID"
echo "  - FITBIT_CLIENT_ID"
echo "  - GARMIN_CLIENT_ID"
echo "  - VITE_FIREBASE_API_KEY"
echo "  - VITE_FIREBASE_MESSAGING_SENDER_ID"
echo "  - VITE_FIREBASE_APP_ID"
echo "  - VITE_FIREBASE_MEASUREMENT_ID"
echo "  - FIREBASE_SERVICE_ACCOUNT (JSON key for Firebase Hosting)"
echo ""
echo "Note: OAuth client SECRETS are stored in GCP Secret Manager, not GitHub."
echo "      Only the client IDs (not secrets) go in GitHub secrets."

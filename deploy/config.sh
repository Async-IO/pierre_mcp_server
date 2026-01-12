# ABOUTME: GCP deployment configuration variables
# ABOUTME: Source this file before running deployment scripts

# Project Configuration
export GCP_PROJECT_ID="pierrefitnessplatform"
export GCP_REGION="northamerica-northeast1"
export GCP_ZONE="${GCP_REGION}-a"

# Service Names
export SERVICE_NAME="pierre-mcp-server"
export DB_INSTANCE_NAME="pierre-postgres"
export DB_NAME="pierre"
export DB_USER="pierre"

# Cloud Run Configuration
export CLOUD_RUN_MEMORY="512Mi"
export CLOUD_RUN_CPU="1"
export CLOUD_RUN_MIN_INSTANCES="0"
export CLOUD_RUN_MAX_INSTANCES="10"
export CLOUD_RUN_CONCURRENCY="80"
export CLOUD_RUN_TIMEOUT="300"

# Cloud SQL Configuration (db-f1-micro is cheapest ~$8/mo)
export DB_TIER="db-f1-micro"
export DB_VERSION="POSTGRES_16"

# Artifact Registry
export REGISTRY_NAME="pierre-images"
export IMAGE_NAME="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT_ID}/${REGISTRY_NAME}/${SERVICE_NAME}"

# Application Configuration
export HTTP_PORT="8081"

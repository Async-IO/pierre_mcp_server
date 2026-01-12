#!/usr/bin/env bash
# ABOUTME: Deploy Pierre MCP Server to Cloud Run
# ABOUTME: Connects to Cloud SQL, configures all environment variables and secrets

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/../config.sh"
source "${SCRIPT_DIR}/../env.production.sh"

# Optional: specify image tag, default to latest
TAG="${1:-latest}"
FULL_IMAGE="${IMAGE_NAME}:${TAG}"

echo "=== Pierre MCP Server - Deploy ==="
echo "Image:   ${FULL_IMAGE}"
echo "Service: ${SERVICE_NAME}"
echo "Region:  ${GCP_REGION}"
echo ""

# Get Cloud SQL connection name
CONNECTION_NAME=$(gcloud sql instances describe "${DB_INSTANCE_NAME}" \
    --format="value(connectionName)")
echo "Cloud SQL: ${CONNECTION_NAME}"

# Get VPC connector
CONNECTOR_NAME="pierre-vpc-connector"
VPC_CONNECTOR="projects/${GCP_PROJECT_ID}/locations/${GCP_REGION}/connectors/${CONNECTOR_NAME}"

# Cloud Run uses Unix socket connection to Cloud SQL
DB_SOCKET_PATH="/cloudsql/${CONNECTION_NAME}"

# Check if this is first deployment (service doesn't exist yet)
FIRST_DEPLOY=false
if ! gcloud run services describe "${SERVICE_NAME}" --region="${GCP_REGION}" &>/dev/null; then
    FIRST_DEPLOY=true
    echo "First deployment detected."
fi

# Build secrets string - only include secrets that exist
SECRETS_ARGS=""
add_secret_if_exists() {
    local env_var="$1"
    local secret_name="$2"
    if gcloud secrets describe "${secret_name}" &>/dev/null; then
        SECRETS_ARGS="${SECRETS_ARGS} --set-secrets=${env_var}=${secret_name}:latest"
    fi
}

add_secret_if_exists "DB_PASSWORD" "${SERVICE_NAME}-db-password"
add_secret_if_exists "PIERRE_MASTER_ENCRYPTION_KEY" "${SERVICE_NAME}-encryption-key"
add_secret_if_exists "STRAVA_CLIENT_SECRET" "${SERVICE_NAME}-strava-client-secret"
add_secret_if_exists "FITBIT_CLIENT_SECRET" "${SERVICE_NAME}-fitbit-client-secret"
add_secret_if_exists "GARMIN_CLIENT_SECRET" "${SERVICE_NAME}-garmin-client-secret"
add_secret_if_exists "COROS_CLIENT_SECRET" "${SERVICE_NAME}-coros-client-secret"
add_secret_if_exists "GROQ_API_KEY" "${SERVICE_NAME}-groq-api-key"
add_secret_if_exists "GEMINI_API_KEY" "${SERVICE_NAME}-gemini-api-key"
add_secret_if_exists "OPENWEATHER_API_KEY" "${SERVICE_NAME}-openweather-api-key"
add_secret_if_exists "USDA_API_KEY" "${SERVICE_NAME}-usda-api-key"

echo ""
echo ">>> Deploying to Cloud Run..."

# For first deploy, we can't reference our own URL for redirects
# After deploy, we'll update with the correct redirect URIs
gcloud run deploy "${SERVICE_NAME}" \
    --image="${FULL_IMAGE}" \
    --region="${GCP_REGION}" \
    --platform=managed \
    --allow-unauthenticated \
    --port="${HTTP_PORT}" \
    --memory="${CLOUD_RUN_MEMORY}" \
    --cpu="${CLOUD_RUN_CPU}" \
    --min-instances="${CLOUD_RUN_MIN_INSTANCES}" \
    --max-instances="${CLOUD_RUN_MAX_INSTANCES}" \
    --concurrency="${CLOUD_RUN_CONCURRENCY}" \
    --timeout="${CLOUD_RUN_TIMEOUT}" \
    --add-cloudsql-instances="${CONNECTION_NAME}" \
    --vpc-connector="${VPC_CONNECTOR}" \
    --vpc-egress=private-ranges-only \
    --set-env-vars="^##^RUST_LOG=${RUST_LOG}" \
    --set-env-vars="HTTP_PORT=${HTTP_PORT}" \
    --set-env-vars="DATABASE_URL=postgresql://${DB_USER}:\${DB_PASSWORD}@/${DB_NAME}?host=${DB_SOCKET_PATH}" \
    --set-env-vars="POSTGRES_MAX_CONNECTIONS=${POSTGRES_MAX_CONNECTIONS}" \
    --set-env-vars="POSTGRES_MIN_CONNECTIONS=${POSTGRES_MIN_CONNECTIONS}" \
    --set-env-vars="POSTGRES_ACQUIRE_TIMEOUT=${POSTGRES_ACQUIRE_TIMEOUT}" \
    --set-env-vars="PIERRE_RSA_KEY_SIZE=${PIERRE_RSA_KEY_SIZE}" \
    --set-env-vars="JWT_EXPIRY_HOURS=${JWT_EXPIRY_HOURS}" \
    --set-env-vars="FIREBASE_PROJECT_ID=${FIREBASE_PROJECT_ID}" \
    --set-env-vars="PIERRE_LLM_PROVIDER=${PIERRE_LLM_PROVIDER}" \
    --set-env-vars="GCP_PROJECT_ID=${GCP_PROJECT_ID}" \
    --set-env-vars="GCP_REGION=${GCP_REGION}" \
    --set-env-vars="CACHE_MAX_ENTRIES=${CACHE_MAX_ENTRIES}" \
    --set-env-vars="CACHE_CLEANUP_INTERVAL_SECS=${CACHE_CLEANUP_INTERVAL_SECS}" \
    --set-env-vars="RATE_LIMIT_ENABLED=${RATE_LIMIT_ENABLED}" \
    --set-env-vars="RATE_LIMIT_REQUESTS=${RATE_LIMIT_REQUESTS}" \
    --set-env-vars="RATE_LIMIT_WINDOW=${RATE_LIMIT_WINDOW}" \
    --set-env-vars="MAX_ACTIVITIES_FETCH=${MAX_ACTIVITIES_FETCH}" \
    --set-env-vars="DEFAULT_ACTIVITIES_LIMIT=${DEFAULT_ACTIVITIES_LIMIT}" \
    --set-env-vars="FITNESS_EFFORT_LIGHT_MAX=${FITNESS_EFFORT_LIGHT_MAX}" \
    --set-env-vars="FITNESS_EFFORT_MODERATE_MAX=${FITNESS_EFFORT_MODERATE_MAX}" \
    --set-env-vars="FITNESS_EFFORT_HARD_MAX=${FITNESS_EFFORT_HARD_MAX}" \
    --set-env-vars="FITNESS_ZONE_RECOVERY_MAX=${FITNESS_ZONE_RECOVERY_MAX}" \
    --set-env-vars="FITNESS_ZONE_ENDURANCE_MAX=${FITNESS_ZONE_ENDURANCE_MAX}" \
    --set-env-vars="FITNESS_ZONE_TEMPO_MAX=${FITNESS_ZONE_TEMPO_MAX}" \
    --set-env-vars="FITNESS_ZONE_THRESHOLD_MAX=${FITNESS_ZONE_THRESHOLD_MAX}" \
    --set-env-vars="FITNESS_WEATHER_ENABLED=${FITNESS_WEATHER_ENABLED}" \
    --set-env-vars="FITNESS_WEATHER_CACHE_DURATION_HOURS=${FITNESS_WEATHER_CACHE_DURATION_HOURS}" \
    --set-env-vars="FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS=${FITNESS_WEATHER_REQUEST_TIMEOUT_SECONDS}" \
    --set-env-vars="FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE=${FITNESS_WEATHER_RATE_LIMIT_PER_MINUTE}" \
    --set-env-vars="FITNESS_WEATHER_WIND_THRESHOLD=${FITNESS_WEATHER_WIND_THRESHOLD}" \
    --set-env-vars="FITNESS_PR_PACE_IMPROVEMENT_THRESHOLD=${FITNESS_PR_PACE_IMPROVEMENT_THRESHOLD}" \
    --set-env-vars="STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID:-}" \
    --set-env-vars="FITBIT_CLIENT_ID=${FITBIT_CLIENT_ID:-}" \
    --set-env-vars="GARMIN_CLIENT_ID=${GARMIN_CLIENT_ID:-}" \
    --set-env-vars="COROS_CLIENT_ID=${COROS_CLIENT_ID:-}" \
    ${SECRETS_ARGS} \
    --update-labels="app=pierre,env=production"

# Get the service URL
SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}" \
    --region="${GCP_REGION}" \
    --format="value(status.url)")

# Update OAuth redirect URIs now that we have the service URL
echo ""
echo ">>> Updating OAuth redirect URIs..."
gcloud run services update "${SERVICE_NAME}" \
    --region="${GCP_REGION}" \
    --set-env-vars="STRAVA_REDIRECT_URI=${SERVICE_URL}/api/oauth/callback/strava" \
    --set-env-vars="FITBIT_REDIRECT_URI=${SERVICE_URL}/api/oauth/callback/fitbit" \
    --set-env-vars="GARMIN_REDIRECT_URI=${SERVICE_URL}/api/oauth/callback/garmin" \
    --set-env-vars="COROS_REDIRECT_URI=${SERVICE_URL}/api/oauth/callback/coros" \
    --set-env-vars="FRONTEND_URL=${SERVICE_URL}"

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Service URL: ${SERVICE_URL}"
echo ""
echo "Endpoints:"
echo "  Health:   ${SERVICE_URL}/health"
echo "  API:      ${SERVICE_URL}/api"
echo "  MCP:      ${SERVICE_URL}/mcp"
echo "  Frontend: ${SERVICE_URL}/"
echo ""
echo "Next steps:"
echo "  1. Update env.production.sh with VITE_API_BASE_URL=${SERVICE_URL}"
echo "  2. Deploy frontend: ./scripts/deploy-frontend.sh"
echo "  3. Register OAuth redirect URIs with providers"

#!/usr/bin/env bash
# ABOUTME: Build and push container image to Artifact Registry
# ABOUTME: Uses Cloud Build for remote building (no local Docker needed)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/../config.sh"

# Optional: specify tag as argument, default to git SHA
TAG="${1:-$(git rev-parse --short HEAD)}"
FULL_IMAGE="${IMAGE_NAME}:${TAG}"
LATEST_IMAGE="${IMAGE_NAME}:latest"

echo "=== Pierre MCP Server - Build ==="
echo "Image: ${FULL_IMAGE}"
echo ""

# Configure Docker auth for Artifact Registry
echo ">>> Configuring Docker authentication..."
gcloud auth configure-docker "${GCP_REGION}-docker.pkg.dev" --quiet

# Build using Cloud Build (remote build, no local Docker needed)
echo ">>> Building image with Cloud Build..."
gcloud builds submit \
    --project="${GCP_PROJECT_ID}" \
    --region="${GCP_REGION}" \
    --tag="${FULL_IMAGE}" \
    --timeout=20m \
    .

# Tag as latest
echo ">>> Tagging as latest..."
gcloud artifacts docker tags add "${FULL_IMAGE}" "${LATEST_IMAGE}" --quiet

echo ""
echo "=== Build Complete ==="
echo "Image: ${FULL_IMAGE}"
echo "Latest: ${LATEST_IMAGE}"

#!/bin/bash
# Pierre MCP Server - Build Script

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
IMAGE_NAME="pierre-mcp-server"
TAG="${1:-latest}"
BUILD_ARGS=""

echo -e "${GREEN}🚀 Building Pierre MCP Server Docker Image${NC}"
echo -e "${YELLOW}Tag: ${TAG}${NC}"

# Check if Dockerfile exists
if [[ ! -f "Dockerfile" ]]; then
    echo -e "${RED}❌ Dockerfile not found in current directory${NC}"
    exit 1
fi

# Build the image
echo -e "${YELLOW}📦 Building Docker image...${NC}"
docker build \
    ${BUILD_ARGS} \
    -t "${IMAGE_NAME}:${TAG}" \
    -t "${IMAGE_NAME}:latest" \
    .

# Verify the build
echo -e "${YELLOW}🔍 Verifying build...${NC}"
docker images "${IMAGE_NAME}:${TAG}"

# Test the image
echo -e "${YELLOW}🧪 Testing image...${NC}"
docker run --rm "${IMAGE_NAME}:${TAG}" --help

echo -e "${GREEN}✅ Build completed successfully!${NC}"
echo -e "${GREEN}Image: ${IMAGE_NAME}:${TAG}${NC}"

# Show image size
IMAGE_SIZE=$(docker images "${IMAGE_NAME}:${TAG}" --format "{{.Size}}")
echo -e "${GREEN}Size: ${IMAGE_SIZE}${NC}"

# Optional: Run security scan if trivy is available
if command -v trivy &> /dev/null; then
    echo -e "${YELLOW}🔒 Running security scan...${NC}"
    trivy image "${IMAGE_NAME}:${TAG}"
else
    echo -e "${YELLOW}⚠️  Trivy not found. Consider installing for security scanning.${NC}"
fi

echo -e "${GREEN}🎉 Build process complete!${NC}"
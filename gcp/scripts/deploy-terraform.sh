#!/usr/bin/env bash
# Deploy Infrastructure with Terraform
# Purpose: Simplified Terraform deployment wrapper with safety checks
# Usage: ./deploy-terraform.sh <environment> <action>

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Script arguments
ENVIRONMENT="${1:-}"
ACTION="${2:-plan}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="$(cd "$SCRIPT_DIR/../terraform" && pwd)"

# Validate arguments
if [ -z "$ENVIRONMENT" ]; then
    echo -e "${RED}❌ Error: Environment required${NC}"
    echo "Usage: $0 <environment> [action]"
    echo ""
    echo "Environments: dev, staging, production"
    echo "Actions: plan, apply, destroy, output"
    echo ""
    echo "Examples:"
    echo "  $0 dev plan       # Preview changes for dev"
    echo "  $0 staging apply  # Apply changes to staging"
    echo "  $0 production output  # Show production outputs"
    exit 1
fi

if [[ ! "$ENVIRONMENT" =~ ^(dev|staging|production)$ ]]; then
    echo -e "${RED}❌ Error: Invalid environment '$ENVIRONMENT'${NC}"
    echo "Valid environments: dev, staging, production"
    exit 1
fi

if [[ ! "$ACTION" =~ ^(plan|apply|destroy|output|refresh)$ ]]; then
    echo -e "${RED}❌ Error: Invalid action '$ACTION'${NC}"
    echo "Valid actions: plan, apply, destroy, output, refresh"
    exit 1
fi

TFVARS_FILE="$TERRAFORM_DIR/environments/$ENVIRONMENT/terraform.tfvars"

if [ ! -f "$TFVARS_FILE" ]; then
    echo -e "${RED}❌ Error: Terraform variables file not found${NC}"
    echo "Expected: $TFVARS_FILE"
    exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Pierre MCP Server - Terraform Deployment${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Environment:  $ENVIRONMENT"
echo "Action:       $ACTION"
echo "Variables:    $TFVARS_FILE"
echo "Directory:    $TERRAFORM_DIR"
echo ""

# Change to Terraform directory
cd "$TERRAFORM_DIR"

# Check Terraform version
TERRAFORM_VERSION=$(terraform version -json | jq -r '.terraform_version')
echo -e "${YELLOW}📦 Terraform version: $TERRAFORM_VERSION${NC}"

# Initialize Terraform if needed
if [ ! -d ".terraform" ]; then
    echo -e "${YELLOW}🔧 Initializing Terraform...${NC}"
    terraform init
fi

# Extract project ID from tfvars
PROJECT_ID=$(grep '^project_id' "$TFVARS_FILE" | sed 's/.*=\s*"\(.*\)"/\1/')
echo -e "${YELLOW}📋 GCP Project: $PROJECT_ID${NC}"

# Set active GCP project
gcloud config set project "$PROJECT_ID" --quiet

# Validate Terraform configuration
echo -e "${YELLOW}✅ Validating Terraform configuration...${NC}"
terraform validate

# Execute Terraform action
case "$ACTION" in
    plan)
        echo -e "${YELLOW}📊 Generating execution plan...${NC}"
        terraform plan \
            -var-file="$TFVARS_FILE" \
            -out="terraform-$ENVIRONMENT.tfplan"

        echo ""
        echo -e "${GREEN}✅ Plan generated successfully!${NC}"
        echo ""
        echo "To apply this plan:"
        echo "  $0 $ENVIRONMENT apply"
        ;;

    apply)
        # Safety check for production
        if [ "$ENVIRONMENT" == "production" ]; then
            echo -e "${RED}⚠️  WARNING: You are about to modify PRODUCTION infrastructure!${NC}"
            read -p "Type 'production' to confirm: " confirm
            if [ "$confirm" != "production" ]; then
                echo "Deployment cancelled"
                exit 1
            fi
        fi

        echo -e "${YELLOW}🚀 Applying infrastructure changes...${NC}"

        # Check if plan file exists
        if [ -f "terraform-$ENVIRONMENT.tfplan" ]; then
            echo "Using existing plan file..."
            terraform apply "terraform-$ENVIRONMENT.tfplan"
            rm -f "terraform-$ENVIRONMENT.tfplan"
        else
            echo "No plan file found, running apply with auto-approve..."
            terraform apply \
                -var-file="$TFVARS_FILE" \
                -auto-approve
        fi

        echo ""
        echo -e "${GREEN}========================================${NC}"
        echo -e "${GREEN}✅ Infrastructure deployed successfully!${NC}"
        echo -e "${GREEN}========================================${NC}"
        echo ""

        # Show important outputs
        echo -e "${YELLOW}📋 Deployment Outputs:${NC}"
        terraform output -json | jq -r 'to_entries[] | "\(.key) = \(.value.value)"' | grep -E '(service_url|database_connection|health_check)' || true

        echo ""
        echo -e "${YELLOW}Next steps:${NC}"
        echo "1. Test the deployment:"
        echo "   SERVICE_URL=\$(terraform output -raw cloud_run_service_url)"
        echo "   curl \$SERVICE_URL/health"
        echo ""
        echo "2. View logs:"
        echo "   gcloud logging read \"resource.type=cloud_run_revision\" --limit 50"
        echo ""
        echo "3. Deploy new version:"
        echo "   gcloud builds submit --config=../cloudbuild/cloudbuild.yaml"
        echo ""
        ;;

    destroy)
        echo -e "${RED}⚠️  WARNING: You are about to DESTROY infrastructure!${NC}"
        echo "Environment: $ENVIRONMENT"
        echo ""

        if [ "$ENVIRONMENT" == "production" ]; then
            echo -e "${RED}🚨 PRODUCTION DESTRUCTION BLOCKED${NC}"
            echo "Destroying production requires manual intervention."
            echo "If you really need to destroy production:"
            echo "1. Remove deletion_protection from terraform.tfvars"
            echo "2. Run: terraform destroy -var-file=$TFVARS_FILE"
            exit 1
        fi

        read -p "Type '$ENVIRONMENT' to confirm destruction: " confirm
        if [ "$confirm" != "$ENVIRONMENT" ]; then
            echo "Destruction cancelled"
            exit 1
        fi

        echo -e "${YELLOW}💥 Destroying infrastructure...${NC}"
        terraform destroy \
            -var-file="$TFVARS_FILE" \
            -auto-approve

        echo ""
        echo -e "${GREEN}✅ Infrastructure destroyed${NC}"
        ;;

    output)
        echo -e "${YELLOW}📋 Infrastructure Outputs:${NC}"
        terraform output
        ;;

    refresh)
        echo -e "${YELLOW}🔄 Refreshing Terraform state...${NC}"
        terraform refresh -var-file="$TFVARS_FILE"
        echo -e "${GREEN}✅ State refreshed${NC}"
        ;;
esac

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Deployment complete!${NC}"
echo -e "${BLUE}========================================${NC}"

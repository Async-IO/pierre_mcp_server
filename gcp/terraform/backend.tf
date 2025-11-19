# Terraform remote state configuration
# Stores state in GCS bucket for team collaboration and state locking
#
# IMPORTANT: Create the bucket manually before running terraform init:
#   gsutil mb -p PROJECT_ID -l REGION gs://PROJECT_ID-terraform-state
#   gsutil versioning set on gs://PROJECT_ID-terraform-state
#
# Then initialize with:
#   terraform init -backend-config="bucket=PROJECT_ID-terraform-state"

terraform {
  backend "gcs" {
    # bucket = "REPLACE_WITH_YOUR_PROJECT_ID-terraform-state"  # Set via -backend-config
    prefix = "pierre-mcp-server"
  }
}

# Alternative: Local backend for testing (NOT for production)
# terraform {
#   backend "local" {
#     path = "terraform.tfstate"
#   }
# }

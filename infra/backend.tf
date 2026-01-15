# ABOUTME: Configures GCS remote state backend for Terraform
# ABOUTME: State is stored in a versioned bucket for safety and collaboration

terraform {
  backend "gcs" {
    bucket = "pierre-terraform-state"
    prefix = "pierre-mcp-server"
  }
}

# ABOUTME: Outputs from Pierre MCP Server infrastructure
# ABOUTME: Provides values needed for Cloud Run deployment and GitHub secrets

# -----------------------------------------------------------------------------
# Database Outputs
# -----------------------------------------------------------------------------

output "database_connection_name" {
  description = "Cloud SQL connection name (for Cloud Run --add-cloudsql-instances)"
  value       = module.database.connection_name
}

output "database_private_ip" {
  description = "Private IP address of the Cloud SQL instance"
  value       = module.database.private_ip_address
}

output "database_name" {
  description = "Name of the PostgreSQL database"
  value       = module.database.database_name
}

output "database_user" {
  description = "Name of the PostgreSQL user"
  value       = module.database.database_user
}

# -----------------------------------------------------------------------------
# Networking Outputs
# -----------------------------------------------------------------------------

output "vpc_connector_id" {
  description = "Serverless VPC connector ID (for Cloud Run --vpc-connector)"
  value       = module.networking.vpc_connector_id
}

output "vpc_name" {
  description = "Name of the VPC network"
  value       = module.networking.vpc_name
}

# -----------------------------------------------------------------------------
# Service Account Outputs
# -----------------------------------------------------------------------------

output "app_service_account_email" {
  description = "App service account email (for Cloud Run --service-account)"
  value       = module.service_accounts.app_service_account_email
}

output "deployer_service_account_email" {
  description = "Deployer service account email (for GitHub GCP_SERVICE_ACCOUNT secret)"
  value       = module.service_accounts.deployer_service_account_email
}

# -----------------------------------------------------------------------------
# Workload Identity Outputs
# -----------------------------------------------------------------------------

output "workload_identity_provider" {
  description = "Workload Identity Provider name (for GitHub GCP_WORKLOAD_IDENTITY_PROVIDER secret)"
  value       = module.workload_identity.provider_name
}

# -----------------------------------------------------------------------------
# Artifact Registry Outputs
# -----------------------------------------------------------------------------

output "artifact_registry_url" {
  description = "Artifact Registry URL (for docker push)"
  value       = module.artifact_registry.registry_url
}

# -----------------------------------------------------------------------------
# Secret Outputs
# -----------------------------------------------------------------------------

output "secret_ids" {
  description = "Map of secret names to their Secret Manager IDs"
  value       = module.secrets.secret_ids
}

# -----------------------------------------------------------------------------
# GitHub Actions Configuration Summary
# -----------------------------------------------------------------------------

output "github_secrets_summary" {
  description = "Summary of values to add as GitHub repository secrets"
  value = {
    GCP_WORKLOAD_IDENTITY_PROVIDER = module.workload_identity.provider_name
    GCP_SERVICE_ACCOUNT            = module.service_accounts.deployer_service_account_email
  }
}

# -----------------------------------------------------------------------------
# Cloud Run Deployment Configuration
# -----------------------------------------------------------------------------

output "cloud_run_config" {
  description = "Configuration values for Cloud Run deployment"
  value = {
    service_account      = module.service_accounts.app_service_account_email
    vpc_connector        = module.networking.vpc_connector_id
    cloudsql_instance    = module.database.connection_name
    artifact_registry    = module.artifact_registry.registry_url
    database_url_pattern = "postgresql://${module.database.database_user}:$${DB_PASSWORD}@/pierre?host=/cloudsql/${module.database.connection_name}"
  }
  sensitive = true
}

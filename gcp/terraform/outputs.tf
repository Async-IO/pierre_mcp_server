# Cloud Run Outputs
output "cloud_run_service_url" {
  description = "URL of the deployed Cloud Run service"
  value       = google_cloud_run_service.pierre_mcp_server.status[0].url
}

output "cloud_run_service_id" {
  description = "Cloud Run service ID"
  value       = google_cloud_run_service.pierre_mcp_server.id
}

output "cloud_run_service_name" {
  description = "Cloud Run service name"
  value       = google_cloud_run_service.pierre_mcp_server.name
}

# Cloud SQL Outputs
output "database_instance_name" {
  description = "Cloud SQL instance name"
  value       = google_sql_database_instance.postgres.name
}

output "database_connection_name" {
  description = "Cloud SQL instance connection name (for Cloud SQL Proxy)"
  value       = google_sql_database_instance.postgres.connection_name
}

output "database_private_ip" {
  description = "Cloud SQL private IP address"
  value       = google_sql_database_instance.postgres.private_ip_address
  sensitive   = true
}

output "database_public_ip" {
  description = "Cloud SQL public IP address (if enabled)"
  value       = length(google_sql_database_instance.postgres.ip_address) > 0 ? google_sql_database_instance.postgres.ip_address[0].ip_address : "N/A"
}

output "database_name" {
  description = "PostgreSQL database name"
  value       = google_sql_database.pierre_db.name
}

output "database_user" {
  description = "PostgreSQL database user"
  value       = google_sql_user.pierre_user.name
}

# Networking Outputs
output "vpc_network_name" {
  description = "VPC network name"
  value       = google_compute_network.vpc.name
}

output "vpc_network_id" {
  description = "VPC network ID"
  value       = google_compute_network.vpc.id
}

output "subnet_name" {
  description = "Subnet name"
  value       = google_compute_subnetwork.subnet.name
}

output "serverless_vpc_connector_name" {
  description = "Serverless VPC Access connector name"
  value       = google_vpc_access_connector.connector.name
}

output "cloud_nat_name" {
  description = "Cloud NAT gateway name"
  value       = google_compute_router_nat.nat.name
}

# Secret Manager Outputs
output "secret_ids" {
  description = "Map of secret names to their Secret Manager IDs"
  value = {
    for k, v in google_secret_manager_secret.secrets : k => v.id
  }
  sensitive = true
}

# Service Account Outputs
output "cloud_run_service_account_email" {
  description = "Email of the Cloud Run service account"
  value       = google_service_account.cloud_run_sa.email
}

output "cloud_run_service_account_name" {
  description = "Name of the Cloud Run service account"
  value       = google_service_account.cloud_run_sa.name
}

# Project Configuration
output "project_id" {
  description = "GCP project ID"
  value       = var.project_id
}

output "region" {
  description = "GCP region"
  value       = var.region
}

output "environment" {
  description = "Environment name"
  value       = var.environment
}

# Database Connection String (for application configuration)
output "database_url" {
  description = "PostgreSQL connection URL (use with Cloud SQL Proxy or private IP)"
  value       = "postgresql://${google_sql_user.pierre_user.name}:GENERATED_PASSWORD@${google_sql_database_instance.postgres.private_ip_address}:5432/${google_sql_database.pierre_db.name}"
  sensitive   = true
}

# Health Check Endpoint
output "health_check_url" {
  description = "URL for health check endpoint"
  value       = "${google_cloud_run_service.pierre_mcp_server.status[0].url}/health"
}

# Deployment Instructions
output "deployment_instructions" {
  description = "Quick start deployment instructions"
  value       = <<-EOT
    ===================================================================
    Pierre MCP Server Deployment Complete!
    ===================================================================

    Service URL: ${google_cloud_run_service.pierre_mcp_server.status[0].url}
    Health Check: ${google_cloud_run_service.pierre_mcp_server.status[0].url}/health

    Database Connection:
      Instance: ${google_sql_database_instance.postgres.connection_name}
      Private IP: ${google_sql_database_instance.postgres.private_ip_address}
      Database: ${google_sql_database.pierre_db.name}
      User: ${google_sql_user.pierre_user.name}

    Next Steps:
    1. Retrieve database password from Secret Manager:
       gcloud secrets versions access latest --secret="${var.service_name}-db-password"

    2. Test the health endpoint:
       curl ${google_cloud_run_service.pierre_mcp_server.status[0].url}/health

    3. View logs:
       gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=${google_cloud_run_service.pierre_mcp_server.name}" --limit 50

    4. Deploy new version:
       gcloud run deploy ${google_cloud_run_service.pierre_mcp_server.name} \
         --image=NEW_IMAGE_URL \
         --region=${var.region}

    ===================================================================
  EOT
}

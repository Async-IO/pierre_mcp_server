# ABOUTME: Outputs from the database module
# ABOUTME: Provides connection info for Cloud Run deployment

output "instance_name" {
  description = "Name of the Cloud SQL instance"
  value       = google_sql_database_instance.postgres.name
}

output "connection_name" {
  description = "Connection name for Cloud SQL (project:region:instance)"
  value       = google_sql_database_instance.postgres.connection_name
}

output "private_ip_address" {
  description = "Private IP address of the Cloud SQL instance"
  value       = google_sql_database_instance.postgres.private_ip_address
}

output "database_name" {
  description = "Name of the database"
  value       = google_sql_database.database.name
}

output "database_user" {
  description = "Name of the database user"
  value       = google_sql_user.user.name
}

output "database_url" {
  description = "Database connection URL (without password)"
  value       = "postgresql://${google_sql_user.user.name}@/pierre?host=/cloudsql/${google_sql_database_instance.postgres.connection_name}"
  sensitive   = true
}

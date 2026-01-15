# ABOUTME: Outputs from the artifact_registry module
# ABOUTME: Provides registry URLs for Docker push and Cloud Run deployment

output "repository_id" {
  description = "ID of the Artifact Registry repository"
  value       = google_artifact_registry_repository.docker.id
}

output "repository_name" {
  description = "Name of the Artifact Registry repository"
  value       = google_artifact_registry_repository.docker.name
}

output "registry_url" {
  description = "URL for Docker push/pull operations"
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${var.registry_name}"
}

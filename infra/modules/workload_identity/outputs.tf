# ABOUTME: Outputs from the workload_identity module
# ABOUTME: Provides the provider name needed for GitHub Actions authentication

output "pool_id" {
  description = "ID of the Workload Identity Pool"
  value       = google_iam_workload_identity_pool.github.workload_identity_pool_id
}

output "pool_name" {
  description = "Full resource name of the Workload Identity Pool"
  value       = google_iam_workload_identity_pool.github.name
}

output "provider_name" {
  description = "Full resource name of the Workload Identity Provider (for GitHub secrets)"
  value       = google_iam_workload_identity_pool_provider.github.name
}

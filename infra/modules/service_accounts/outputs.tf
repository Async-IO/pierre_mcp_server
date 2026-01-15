# ABOUTME: Outputs from the service_accounts module
# ABOUTME: Provides service account emails for Cloud Run and GitHub Actions

output "app_service_account_email" {
  description = "Email of the app service account"
  value       = google_service_account.app.email
}

output "app_service_account_name" {
  description = "Full resource name of the app service account"
  value       = google_service_account.app.name
}

output "deployer_service_account_email" {
  description = "Email of the deployer service account"
  value       = google_service_account.deployer.email
}

output "deployer_service_account_name" {
  description = "Full resource name of the deployer service account"
  value       = google_service_account.deployer.name
}

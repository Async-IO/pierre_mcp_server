# ABOUTME: Outputs from the storage module
# ABOUTME: Provides bucket names and URLs

output "app_data_bucket_name" {
  description = "Name of the application data bucket"
  value       = var.create_app_bucket ? google_storage_bucket.app_data[0].name : null
}

output "app_data_bucket_url" {
  description = "URL of the application data bucket"
  value       = var.create_app_bucket ? google_storage_bucket.app_data[0].url : null
}

output "terraform_state_bucket_name" {
  description = "Name of the Terraform state bucket"
  value       = var.create_terraform_state_bucket ? google_storage_bucket.terraform_state[0].name : null
}

# ABOUTME: Variables for the service_accounts module
# ABOUTME: Configures service account naming

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "service_name" {
  description = "Name of the service (used as prefix for SA account IDs)"
  type        = string
}

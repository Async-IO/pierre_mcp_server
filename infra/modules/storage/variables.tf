# ABOUTME: Variables for the storage module
# ABOUTME: Configures bucket creation options

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
}

variable "service_name" {
  description = "Name of the service (used in bucket naming)"
  type        = string
}

variable "create_app_bucket" {
  description = "Whether to create an application data bucket"
  type        = bool
  default     = false
}

variable "create_terraform_state_bucket" {
  description = "Whether to create a Terraform state bucket (for bootstrapping)"
  type        = bool
  default     = false
}

variable "labels" {
  description = "Labels to apply to buckets"
  type        = map(string)
  default     = {}
}

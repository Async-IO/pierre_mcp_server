# ABOUTME: Variables for the secrets module
# ABOUTME: Configures secret naming and labels

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "service_name" {
  description = "Name of the service (used as prefix for secret IDs)"
  type        = string
}

variable "labels" {
  description = "Labels to apply to secrets"
  type        = map(string)
  default     = {}
}

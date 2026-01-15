# ABOUTME: Variables for the artifact_registry module
# ABOUTME: Configures Docker repository name and location

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
}

variable "registry_name" {
  description = "Name of the Artifact Registry repository"
  type        = string
}

variable "labels" {
  description = "Labels to apply to the repository"
  type        = map(string)
  default     = {}
}

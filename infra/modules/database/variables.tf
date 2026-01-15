# ABOUTME: Variables for the database module
# ABOUTME: Configures Cloud SQL instance, database, and user settings

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
}

variable "service_name" {
  description = "Name of the service (used as prefix for instance name)"
  type        = string
}

variable "environment" {
  description = "Environment (production, staging, development)"
  type        = string
}

variable "vpc_self_link" {
  description = "Self-link of the VPC network"
  type        = string
}

variable "private_vpc_connection_id" {
  description = "ID of the private VPC connection"
  type        = string
}

variable "database_version" {
  description = "PostgreSQL version"
  type        = string
  default     = "POSTGRES_15"
}

variable "database_tier" {
  description = "Cloud SQL machine tier"
  type        = string
  default     = "db-f1-micro"
}

variable "database_name" {
  description = "Name of the database"
  type        = string
  default     = "pierre"
}

variable "database_user" {
  description = "Name of the database user"
  type        = string
  default     = "pierre"
}

variable "database_password" {
  description = "Password for the database user"
  type        = string
  sensitive   = true
}

variable "disk_size_gb" {
  description = "Initial disk size in GB"
  type        = number
  default     = 10
}

variable "deletion_protection" {
  description = "Enable deletion protection"
  type        = bool
  default     = true
}

variable "backup_enabled" {
  description = "Enable automated backups"
  type        = bool
  default     = true
}

variable "backup_start_time" {
  description = "Start time for backups (HH:MM format, UTC)"
  type        = string
  default     = "03:00"
}

variable "labels" {
  description = "Labels to apply to resources"
  type        = map(string)
  default     = {}
}

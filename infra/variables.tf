# ABOUTME: Defines all configurable variables for Pierre MCP Server infrastructure
# ABOUTME: Includes project settings, database config, and GitHub integration

# -----------------------------------------------------------------------------
# Project Configuration
# -----------------------------------------------------------------------------

variable "project_id" {
  description = "GCP project ID where resources will be created"
  type        = string

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{4,28}[a-z0-9]$", var.project_id))
    error_message = "Project ID must be 6-30 lowercase letters, digits, or hyphens."
  }
}

variable "region" {
  description = "GCP region for resource deployment"
  type        = string
  default     = "northamerica-northeast1"
}

variable "environment" {
  description = "Environment name (e.g., production, staging)"
  type        = string
  default     = "production"

  validation {
    condition     = contains(["production", "staging", "development"], var.environment)
    error_message = "Environment must be production, staging, or development."
  }
}

# -----------------------------------------------------------------------------
# Service Configuration
# -----------------------------------------------------------------------------

variable "service_name" {
  description = "Name of the Cloud Run service"
  type        = string
  default     = "pierre-mcp-server"
}

# -----------------------------------------------------------------------------
# Database Configuration
# -----------------------------------------------------------------------------

variable "database_tier" {
  description = "Cloud SQL machine tier (e.g., db-f1-micro, db-custom-1-3840)"
  type        = string
  default     = "db-f1-micro"
}

variable "database_version" {
  description = "PostgreSQL version for Cloud SQL"
  type        = string
  default     = "POSTGRES_15"
}

variable "database_name" {
  description = "Name of the PostgreSQL database"
  type        = string
  default     = "pierre"
}

variable "database_user" {
  description = "Name of the PostgreSQL user"
  type        = string
  default     = "pierre"
}

variable "database_deletion_protection" {
  description = "Enable deletion protection for the database instance"
  type        = bool
  default     = true
}

variable "database_backup_enabled" {
  description = "Enable automated backups for the database"
  type        = bool
  default     = true
}

variable "database_backup_start_time" {
  description = "Start time for database backups (HH:MM format, UTC)"
  type        = string
  default     = "03:00"
}

# -----------------------------------------------------------------------------
# Networking Configuration
# -----------------------------------------------------------------------------

variable "vpc_name" {
  description = "Name of the VPC network"
  type        = string
  default     = "pierre-vpc"
}

variable "subnet_cidr" {
  description = "CIDR range for the VPC subnet"
  type        = string
  default     = "10.0.0.0/24"
}

variable "vpc_connector_cidr" {
  description = "CIDR range for the serverless VPC connector"
  type        = string
  default     = "10.8.0.0/28"
}

# -----------------------------------------------------------------------------
# GitHub Integration
# -----------------------------------------------------------------------------

variable "github_org" {
  description = "GitHub organization or username"
  type        = string
  default     = "Async-IO"
}

variable "github_repo" {
  description = "GitHub repository name"
  type        = string
  default     = "pierre_mcp_server"
}

# -----------------------------------------------------------------------------
# Artifact Registry
# -----------------------------------------------------------------------------

variable "registry_name" {
  description = "Name of the Artifact Registry Docker repository"
  type        = string
  default     = "pierre-images"
}

# -----------------------------------------------------------------------------
# Labels
# -----------------------------------------------------------------------------

variable "labels" {
  description = "Common labels to apply to all resources"
  type        = map(string)
  default = {
    app        = "pierre"
    managed_by = "terraform"
  }
}

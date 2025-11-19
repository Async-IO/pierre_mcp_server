# GCP Project Configuration
variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "region" {
  description = "GCP region for resources"
  type        = string
  default     = "us-central1"
}

variable "zone" {
  description = "GCP zone for zonal resources"
  type        = string
  default     = "us-central1-a"
}

# Environment Configuration
variable "environment" {
  description = "Environment name (dev, staging, production)"
  type        = string
  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be dev, staging, or production"
  }
}

# Cloud Run Configuration
variable "service_name" {
  description = "Cloud Run service name"
  type        = string
  default     = "pierre-mcp-server"
}

variable "container_image" {
  description = "Container image URL (e.g., gcr.io/PROJECT/pierre-mcp-server:latest)"
  type        = string
}

variable "cloud_run_cpu" {
  description = "CPU allocation for Cloud Run (e.g., '1', '2', '4')"
  type        = string
  default     = "1"
}

variable "cloud_run_memory" {
  description = "Memory allocation for Cloud Run (e.g., '512Mi', '1Gi', '2Gi')"
  type        = string
  default     = "512Mi"
}

variable "cloud_run_min_instances" {
  description = "Minimum number of Cloud Run instances"
  type        = number
  default     = 0
}

variable "cloud_run_max_instances" {
  description = "Maximum number of Cloud Run instances"
  type        = number
  default     = 100
}

variable "cloud_run_concurrency" {
  description = "Maximum concurrent requests per instance"
  type        = number
  default     = 80
}

variable "cloud_run_timeout" {
  description = "Request timeout in seconds"
  type        = number
  default     = 300
}

# Cloud SQL Configuration
variable "database_name" {
  description = "Cloud SQL database name"
  type        = string
  default     = "pierre_mcp_server"
}

variable "database_user" {
  description = "Cloud SQL database user"
  type        = string
  default     = "pierre"
}

variable "database_tier" {
  description = "Cloud SQL tier (db-f1-micro, db-custom-2-8192, etc.)"
  type        = string
  default     = "db-f1-micro"
}

variable "database_disk_size" {
  description = "Cloud SQL disk size in GB"
  type        = number
  default     = 20
}

variable "database_disk_type" {
  description = "Cloud SQL disk type (PD_SSD or PD_HDD)"
  type        = string
  default     = "PD_SSD"
}

variable "database_backup_enabled" {
  description = "Enable automated backups"
  type        = bool
  default     = true
}

variable "database_backup_retention_days" {
  description = "Number of days to retain backups"
  type        = number
  default     = 7
}

variable "database_high_availability" {
  description = "Enable high availability (regional) configuration"
  type        = bool
  default     = false
}

variable "database_private_network" {
  description = "Enable private IP for Cloud SQL (recommended for production)"
  type        = bool
  default     = true
}

# Networking Configuration
variable "vpc_name" {
  description = "VPC network name"
  type        = string
  default     = "pierre-vpc"
}

variable "subnet_cidr" {
  description = "Subnet CIDR range"
  type        = string
  default     = "10.0.0.0/24"
}

variable "serverless_connector_cidr" {
  description = "CIDR range for Serverless VPC Access connector"
  type        = string
  default     = "10.8.0.0/28"
}

# Secret Manager Secrets
variable "secrets" {
  description = "Map of secret names to their values (will be stored in Secret Manager)"
  type        = map(string)
  sensitive   = true
  default     = {}
}

# External API Configuration (OAuth Providers)
variable "strava_client_id" {
  description = "Strava OAuth client ID"
  type        = string
  default     = ""
}

variable "strava_redirect_uri" {
  description = "Strava OAuth redirect URI"
  type        = string
  default     = ""
}

variable "garmin_client_id" {
  description = "Garmin OAuth client ID"
  type        = string
  default     = ""
}

variable "garmin_redirect_uri" {
  description = "Garmin OAuth redirect URI"
  type        = string
  default     = ""
}

variable "fitbit_client_id" {
  description = "Fitbit OAuth client ID"
  type        = string
  default     = ""
}

variable "fitbit_redirect_uri" {
  description = "Fitbit OAuth redirect URI"
  type        = string
  default     = ""
}

# Monitoring & Alerting
variable "enable_uptime_checks" {
  description = "Enable Cloud Monitoring uptime checks"
  type        = bool
  default     = true
}

variable "alert_email" {
  description = "Email address for critical alerts"
  type        = string
  default     = ""
}

# Labels (for cost tracking and organization)
variable "labels" {
  description = "Labels to apply to all resources"
  type        = map(string)
  default = {
    managed_by = "terraform"
    application = "pierre-mcp-server"
  }
}

# Security
variable "enable_cloud_armor" {
  description = "Enable Cloud Armor WAF protection"
  type        = bool
  default     = false
}

variable "allowed_ingress_cidrs" {
  description = "CIDR ranges allowed to access the service"
  type        = list(string)
  default     = ["0.0.0.0/0"]  # Open to internet by default, restrict in production
}

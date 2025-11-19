# Development Environment Configuration
# Purpose: Local development testing, rapid iteration
# Cost: ~$75-90/month

# GCP Project Configuration
project_id  = "pierre-mcp-dev"  # REPLACE with your actual GCP project ID
region      = "us-central1"
zone        = "us-central1-a"
environment = "dev"

# Cloud Run Configuration (Minimal resources for dev)
service_name            = "pierre-mcp-server"
container_image         = "gcr.io/pierre-mcp-dev/pierre-mcp-server:latest"  # REPLACE
cloud_run_cpu           = "1"
cloud_run_memory        = "512Mi"
cloud_run_min_instances = 0  # Scale to zero when not in use
cloud_run_max_instances = 10
cloud_run_concurrency   = 80
cloud_run_timeout       = 300

# Cloud SQL Configuration (Smallest tier for dev)
database_name                    = "pierre_mcp_server"
database_user                    = "pierre"
database_tier                    = "db-f1-micro"  # Shared CPU, 0.6GB RAM
database_disk_size               = 10             # 10GB minimum
database_disk_type               = "PD_SSD"
database_backup_enabled          = true
database_backup_retention_days   = 3   # Keep 3 days of backups
database_high_availability       = false  # No HA for dev
database_private_network         = true

# Networking Configuration
vpc_name                   = "pierre-vpc"
subnet_cidr                = "10.0.0.0/24"
serverless_connector_cidr  = "10.8.0.0/28"

# OAuth Provider Configuration (Use test credentials)
strava_client_id    = "your-dev-strava-client-id"
strava_redirect_uri = ""  # Will auto-generate from Cloud Run URL

garmin_client_id    = ""
garmin_redirect_uri = ""

fitbit_client_id    = ""
fitbit_redirect_uri = ""

# Secrets (Store in Secret Manager via terraform apply)
# secrets = {
#   strava_client_secret  = "your-strava-secret"
#   garmin_client_secret  = "your-garmin-secret"
#   fitbit_client_secret  = "your-fitbit-secret"
#   openweather_api_key   = "your-openweather-key"
# }

# Monitoring & Alerting
enable_uptime_checks = true
alert_email          = "devteam@example.com"  # REPLACE

# Security
enable_cloud_armor     = false  # Not needed for dev
allowed_ingress_cidrs  = ["0.0.0.0/0"]  # Open to internet for testing

# Resource Labels
labels = {
  environment = "dev"
  managed_by  = "terraform"
  application = "pierre-mcp-server"
  team        = "platform"
  cost_center = "engineering"
}

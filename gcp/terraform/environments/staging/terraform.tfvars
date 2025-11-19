# Staging Environment Configuration
# Purpose: Pre-production testing, integration testing, QA validation
# Cost: ~$200-300/month

# GCP Project Configuration
project_id  = "pierre-mcp-staging"  # REPLACE with your actual GCP project ID
region      = "us-central1"
zone        = "us-central1-a"
environment = "staging"

# Cloud Run Configuration (Production-like sizing)
service_name            = "pierre-mcp-server"
container_image         = "gcr.io/pierre-mcp-staging/pierre-mcp-server:latest"  # REPLACE
cloud_run_cpu           = "2"
cloud_run_memory        = "1Gi"
cloud_run_min_instances = 1  # Always have 1 instance warm
cloud_run_max_instances = 50
cloud_run_concurrency   = 80
cloud_run_timeout       = 300

# Cloud SQL Configuration (Mid-tier for staging)
database_name                    = "pierre_mcp_server"
database_user                    = "pierre"
database_tier                    = "db-custom-2-8192"  # 2 vCPU, 8GB RAM
database_disk_size               = 20
database_disk_type               = "PD_SSD"
database_backup_enabled          = true
database_backup_retention_days   = 7
database_high_availability       = false  # Single zone for staging
database_private_network         = true

# Networking Configuration
vpc_name                   = "pierre-vpc"
subnet_cidr                = "10.0.0.0/24"
serverless_connector_cidr  = "10.8.0.0/28"

# OAuth Provider Configuration (Staging OAuth apps)
strava_client_id    = "your-staging-strava-client-id"
strava_redirect_uri = ""  # Will auto-generate

garmin_client_id    = "your-staging-garmin-client-id"
garmin_redirect_uri = ""

fitbit_client_id    = "your-staging-fitbit-client-id"
fitbit_redirect_uri = ""

# Secrets (Managed separately via Secret Manager)
# secrets = {
#   strava_client_secret  = "staging-strava-secret"
#   garmin_client_secret  = "staging-garmin-secret"
#   fitbit_client_secret  = "staging-fitbit-secret"
#   openweather_api_key   = "staging-openweather-key"
# }

# Monitoring & Alerting
enable_uptime_checks = true
alert_email          = "platform-staging-alerts@example.com"  # REPLACE

# Security
enable_cloud_armor     = false  # Can enable if testing WAF rules
allowed_ingress_cidrs  = ["0.0.0.0/0"]  # Open for QA team testing

# Resource Labels
labels = {
  environment = "staging"
  managed_by  = "terraform"
  application = "pierre-mcp-server"
  team        = "platform"
  cost_center = "engineering"
}

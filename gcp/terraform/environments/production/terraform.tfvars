# Production Environment Configuration
# Purpose: Live production workload serving real users
# Cost: ~$500-1500/month (scales with usage)

# GCP Project Configuration
project_id  = "pierre-mcp-prod"  # REPLACE with your actual GCP project ID
region      = "us-central1"
zone        = "us-central1-a"
environment = "production"

# Cloud Run Configuration (High availability, auto-scaling)
service_name            = "pierre-mcp-server"
container_image         = "gcr.io/pierre-mcp-prod/pierre-mcp-server:v1.0.0"  # REPLACE with tagged version
cloud_run_cpu           = "2"
cloud_run_memory        = "2Gi"
cloud_run_min_instances = 2  # Always have 2 instances for redundancy
cloud_run_max_instances = 100
cloud_run_concurrency   = 80
cloud_run_timeout       = 300

# Cloud SQL Configuration (Production-grade)
database_name                    = "pierre_mcp_server"
database_user                    = "pierre"
database_tier                    = "db-custom-4-16384"  # 4 vCPU, 16GB RAM
database_disk_size               = 100                  # 100GB with auto-resize
database_disk_type               = "PD_SSD"
database_backup_enabled          = true
database_backup_retention_days   = 30  # 30 days for compliance
database_high_availability       = true  # Regional HA with automatic failover
database_private_network         = true

# Networking Configuration
vpc_name                   = "pierre-vpc"
subnet_cidr                = "10.0.0.0/24"
serverless_connector_cidr  = "10.8.0.0/28"

# OAuth Provider Configuration (Production OAuth apps)
strava_client_id    = "your-production-strava-client-id"
strava_redirect_uri = "https://api.pierre-fitness.com/api/oauth/callback/strava"  # REPLACE with your domain

garmin_client_id    = "your-production-garmin-client-id"
garmin_redirect_uri = "https://api.pierre-fitness.com/api/oauth/callback/garmin"

fitbit_client_id    = "your-production-fitbit-client-id"
fitbit_redirect_uri = "https://api.pierre-fitness.com/api/oauth/callback/fitbit"

# Secrets (NEVER commit these! Manage via Secret Manager CLI or console)
# secrets = {
#   strava_client_secret  = ""  # Set via: gcloud secrets versions add ... --data-file=-
#   garmin_client_secret  = ""
#   fitbit_client_secret  = ""
#   openweather_api_key   = ""
# }

# Monitoring & Alerting (Critical for production)
enable_uptime_checks = true
alert_email          = "platform-oncall@example.com"  # REPLACE with PagerDuty/OpsGenie email

# Security (Production hardening)
enable_cloud_armor     = true  # Enable WAF and DDoS protection
allowed_ingress_cidrs  = ["0.0.0.0/0"]  # Public API, can restrict to known IPs if needed

# Resource Labels (For cost tracking and governance)
labels = {
  environment = "production"
  managed_by  = "terraform"
  application = "pierre-mcp-server"
  team        = "platform"
  cost_center = "product"
  compliance  = "gdpr-compliant"
  sla         = "99.9"
}

# ============================================================================
# PRODUCTION DEPLOYMENT CHECKLIST
# ============================================================================
# Before deploying to production:
#
# [ ] Domain configured and DNS pointed to Cloud Run URL
# [ ] SSL certificate provisioned (automatic with Cloud Run custom domains)
# [ ] OAuth apps registered with production callback URLs
# [ ] Secrets stored in Secret Manager (not in tfvars!)
# [ ] Database backups tested and verified
# [ ] Monitoring dashboards created
# [ ] Alert notification channels configured (PagerDuty, Slack)
# [ ] Runbooks documented for incident response
# [ ] Load testing completed (1000+ RPS sustained)
# [ ] Security scan passed (OWASP, dependency audit)
# [ ] GDPR/compliance requirements reviewed
# [ ] Disaster recovery plan documented
# [ ] Team trained on deployment procedures
# ============================================================================

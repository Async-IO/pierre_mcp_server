# Pierre MCP Server - Main Terraform Configuration
# Deploys Cloud Run + Cloud SQL + Networking infrastructure on GCP

# Enable required GCP APIs
resource "google_project_service" "required_apis" {
  for_each = toset([
    "run.googleapis.com",                  # Cloud Run
    "sqladmin.googleapis.com",             # Cloud SQL
    "compute.googleapis.com",              # Compute Engine (for VPC)
    "vpcaccess.googleapis.com",            # Serverless VPC Access
    "servicenetworking.googleapis.com",    # Service Networking (for private IP)
    "secretmanager.googleapis.com",        # Secret Manager
    "cloudresourcemanager.googleapis.com", # Resource Manager
    "iam.googleapis.com",                  # IAM
    "logging.googleapis.com",              # Cloud Logging
    "monitoring.googleapis.com",           # Cloud Monitoring
    "artifactregistry.googleapis.com",     # Artifact Registry
    "cloudbuild.googleapis.com",           # Cloud Build
  ])

  service            = each.key
  disable_on_destroy = false

  # Prevent accidental API disabling
  lifecycle {
    prevent_destroy = false
  }
}

# ============================================================================
# NETWORKING
# ============================================================================

# VPC Network
resource "google_compute_network" "vpc" {
  name                    = "${var.vpc_name}-${var.environment}"
  auto_create_subnetworks = false
  routing_mode            = "REGIONAL"

  depends_on = [google_project_service.required_apis]
}

# Subnet for Cloud SQL and other resources
resource "google_compute_subnetwork" "subnet" {
  name          = "${var.vpc_name}-subnet-${var.environment}"
  ip_cidr_range = var.subnet_cidr
  region        = var.region
  network       = google_compute_network.vpc.id

  private_ip_google_access = true

  log_config {
    aggregation_interval = "INTERVAL_10_MIN"
    flow_sampling        = 0.5
    metadata             = "INCLUDE_ALL_METADATA"
  }
}

# Global address for Cloud SQL private IP
resource "google_compute_global_address" "private_ip_address" {
  count         = var.database_private_network ? 1 : 0
  name          = "private-ip-address-${var.environment}"
  purpose       = "VPC_PEERING"
  address_type  = "INTERNAL"
  prefix_length = 16
  network       = google_compute_network.vpc.id
}

# Private VPC connection for Cloud SQL
resource "google_service_networking_connection" "private_vpc_connection" {
  count                   = var.database_private_network ? 1 : 0
  network                 = google_compute_network.vpc.id
  service                 = "servicenetworking.googleapis.com"
  reserved_peering_ranges = [google_compute_global_address.private_ip_address[0].name]

  depends_on = [google_project_service.required_apis]
}

# Serverless VPC Access Connector (Cloud Run → Cloud SQL)
resource "google_vpc_access_connector" "connector" {
  name          = "serverless-connector-${var.environment}"
  region        = var.region
  network       = google_compute_network.vpc.name
  ip_cidr_range = var.serverless_connector_cidr

  min_instances = 2
  max_instances = 3
  machine_type  = "e2-micro"

  depends_on = [
    google_compute_subnetwork.subnet,
    google_project_service.required_apis
  ]
}

# Cloud Router for NAT
resource "google_compute_router" "router" {
  name    = "cloud-router-${var.environment}"
  region  = var.region
  network = google_compute_network.vpc.id

  bgp {
    asn = 64514
  }
}

# Cloud NAT (for outbound connectivity to Strava, Garmin, Fitbit, etc.)
resource "google_compute_router_nat" "nat" {
  name   = "cloud-nat-${var.environment}"
  router = google_compute_router.router.name
  region = var.region

  nat_ip_allocate_option             = "AUTO_ONLY"
  source_subnetwork_ip_ranges_to_nat = "ALL_SUBNETWORKS_ALL_IP_RANGES"

  log_config {
    enable = true
    filter = "ERRORS_ONLY"
  }
}

# ============================================================================
# CLOUD SQL (PostgreSQL)
# ============================================================================

# Generate random database password
resource "random_password" "db_password" {
  length  = 32
  special = true
}

# Cloud SQL Instance
resource "google_sql_database_instance" "postgres" {
  name             = "${var.service_name}-postgres-${var.environment}"
  database_version = "POSTGRES_16"
  region           = var.region

  settings {
    tier              = var.database_tier
    availability_type = var.database_high_availability ? "REGIONAL" : "ZONAL"
    disk_type         = var.database_disk_type
    disk_size         = var.database_disk_size
    disk_autoresize   = true

    ip_configuration {
      ipv4_enabled                                  = !var.database_private_network
      private_network                               = var.database_private_network ? google_compute_network.vpc.id : null
      enable_private_path_for_google_cloud_services = var.database_private_network
    }

    backup_configuration {
      enabled                        = var.database_backup_enabled
      start_time                     = "03:00"
      point_in_time_recovery_enabled = true
      transaction_log_retention_days = 7
      backup_retention_settings {
        retained_backups = var.database_backup_retention_days
      }
    }

    maintenance_window {
      day          = 7  # Sunday
      hour         = 3  # 3 AM
      update_track = "stable"
    }

    insights_config {
      query_insights_enabled  = true
      query_plans_per_minute  = 5
      query_string_length     = 1024
      record_application_tags = true
    }

    database_flags {
      name  = "max_connections"
      value = "100"
    }

    database_flags {
      name  = "shared_buffers"
      value = "32768"  # 256MB for db-custom-2-8192
    }

    database_flags {
      name  = "log_checkpoints"
      value = "on"
    }

    database_flags {
      name  = "log_connections"
      value = "on"
    }

    database_flags {
      name  = "log_disconnections"
      value = "on"
    }
  }

  deletion_protection = var.environment == "production" ? true : false

  depends_on = [
    google_service_networking_connection.private_vpc_connection,
    google_project_service.required_apis
  ]
}

# PostgreSQL Database
resource "google_sql_database" "pierre_db" {
  name     = var.database_name
  instance = google_sql_database_instance.postgres.name
}

# Database User
resource "google_sql_user" "pierre_user" {
  name     = var.database_user
  instance = google_sql_database_instance.postgres.name
  password = random_password.db_password.result
}

# ============================================================================
# SECRET MANAGER
# ============================================================================

# Database password secret
resource "google_secret_manager_secret" "db_password" {
  secret_id = "${var.service_name}-db-password-${var.environment}"

  replication {
    auto {}
  }

  labels = merge(var.labels, {
    environment = var.environment
    secret_type = "database"
  })

  depends_on = [google_project_service.required_apis]
}

resource "google_secret_manager_secret_version" "db_password_version" {
  secret      = google_secret_manager_secret.db_password.id
  secret_data = random_password.db_password.result
}

# Master encryption key secret
resource "google_secret_manager_secret" "master_encryption_key" {
  secret_id = "${var.service_name}-master-encryption-key-${var.environment}"

  replication {
    auto {}
  }

  labels = merge(var.labels, {
    environment = var.environment
    secret_type = "encryption"
  })

  depends_on = [google_project_service.required_apis]
}

resource "google_secret_manager_secret_version" "master_encryption_key_version" {
  secret = google_secret_manager_secret.master_encryption_key.id
  secret_data = base64encode(random_password.db_password.result)  # Generate secure random key
}

# OAuth provider secrets (conditional)
locals {
  oauth_secrets = {
    strava_client_secret  = var.secrets["strava_client_secret"]
    garmin_client_secret  = var.secrets["garmin_client_secret"]
    fitbit_client_secret  = var.secrets["fitbit_client_secret"]
    openweather_api_key   = var.secrets["openweather_api_key"]
  }
}

resource "google_secret_manager_secret" "secrets" {
  for_each = { for k, v in local.oauth_secrets : k => v if v != "" && v != null }

  secret_id = "${var.service_name}-${each.key}-${var.environment}"

  replication {
    auto {}
  }

  labels = merge(var.labels, {
    environment = var.environment
    secret_type = "oauth"
  })

  depends_on = [google_project_service.required_apis]
}

resource "google_secret_manager_secret_version" "secret_versions" {
  for_each = google_secret_manager_secret.secrets

  secret      = each.value.id
  secret_data = local.oauth_secrets[each.key]
}

# ============================================================================
# IAM & SERVICE ACCOUNTS
# ============================================================================

# Service account for Cloud Run
resource "google_service_account" "cloud_run_sa" {
  account_id   = "${var.service_name}-runner-${var.environment}"
  display_name = "Cloud Run service account for Pierre MCP Server (${var.environment})"
  description  = "Service account with least-privilege access for Cloud Run workload"
}

# Grant Cloud SQL Client role to service account
resource "google_project_iam_member" "cloud_sql_client" {
  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

# Grant Secret Manager Secret Accessor role
resource "google_secret_manager_secret_iam_member" "db_password_access" {
  secret_id = google_secret_manager_secret.db_password.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

resource "google_secret_manager_secret_iam_member" "master_key_access" {
  secret_id = google_secret_manager_secret.master_encryption_key.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

resource "google_secret_manager_secret_iam_member" "oauth_secrets_access" {
  for_each = google_secret_manager_secret.secrets

  secret_id = each.value.id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

# Grant logging and monitoring permissions
resource "google_project_iam_member" "log_writer" {
  project = var.project_id
  role    = "roles/logging.logWriter"
  member  = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

resource "google_project_iam_member" "monitoring_metric_writer" {
  project = var.project_id
  role    = "roles/monitoring.metricWriter"
  member  = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

resource "google_project_iam_member" "trace_agent" {
  project = var.project_id
  role    = "roles/cloudtrace.agent"
  member  = "serviceAccount:${google_service_account.cloud_run_sa.email}"
}

# ============================================================================
# CLOUD RUN SERVICE
# ============================================================================

resource "google_cloud_run_service" "pierre_mcp_server" {
  name     = var.service_name
  location = var.region

  template {
    spec {
      service_account_name = google_service_account.cloud_run_sa.email

      containers {
        image = var.container_image

        ports {
          name           = "http1"
          container_port = 8081
        }

        resources {
          limits = {
            cpu    = var.cloud_run_cpu
            memory = var.cloud_run_memory
          }
        }

        env {
          name  = "RUST_LOG"
          value = var.environment == "production" ? "info" : "debug"
        }

        env {
          name  = "HTTP_PORT"
          value = "8081"
        }

        env {
          name  = "DATABASE_URL"
          value = "postgresql://${google_sql_user.pierre_user.name}:$(DATABASE_PASSWORD)@${google_sql_database_instance.postgres.private_ip_address}:5432/${google_sql_database.pierre_db.name}?sslmode=require"
        }

        env {
          name = "DATABASE_PASSWORD"
          value_from {
            secret_key_ref {
              name = google_secret_manager_secret.db_password.secret_id
              key  = "latest"
            }
          }
        }

        env {
          name = "PIERRE_MASTER_ENCRYPTION_KEY"
          value_from {
            secret_key_ref {
              name = google_secret_manager_secret.master_encryption_key.secret_id
              key  = "latest"
            }
          }
        }

        env {
          name  = "PIERRE_RSA_KEY_SIZE"
          value = "4096"
        }

        env {
          name  = "JWT_EXPIRY_HOURS"
          value = "24"
        }

        # OAuth Provider Configuration
        env {
          name  = "STRAVA_CLIENT_ID"
          value = var.strava_client_id
        }

        dynamic "env" {
          for_each = contains(keys(google_secret_manager_secret.secrets), "strava_client_secret") ? [1] : []
          content {
            name = "STRAVA_CLIENT_SECRET"
            value_from {
              secret_key_ref {
                name = google_secret_manager_secret.secrets["strava_client_secret"].secret_id
                key  = "latest"
              }
            }
          }
        }

        env {
          name  = "STRAVA_REDIRECT_URI"
          value = var.strava_redirect_uri != "" ? var.strava_redirect_uri : "${google_cloud_run_service.pierre_mcp_server.status[0].url}/api/oauth/callback/strava"
        }

        env {
          name  = "GARMIN_CLIENT_ID"
          value = var.garmin_client_id
        }

        dynamic "env" {
          for_each = contains(keys(google_secret_manager_secret.secrets), "garmin_client_secret") ? [1] : []
          content {
            name = "GARMIN_CLIENT_SECRET"
            value_from {
              secret_key_ref {
                name = google_secret_manager_secret.secrets["garmin_client_secret"].secret_id
                key  = "latest"
              }
            }
          }
        }

        env {
          name  = "GARMIN_REDIRECT_URI"
          value = var.garmin_redirect_uri != "" ? var.garmin_redirect_uri : "${google_cloud_run_service.pierre_mcp_server.status[0].url}/api/oauth/callback/garmin"
        }

        env {
          name  = "FITBIT_CLIENT_ID"
          value = var.fitbit_client_id
        }

        dynamic "env" {
          for_each = contains(keys(google_secret_manager_secret.secrets), "fitbit_client_secret") ? [1] : []
          content {
            name = "FITBIT_CLIENT_SECRET"
            value_from {
              secret_key_ref {
                name = google_secret_manager_secret.secrets["fitbit_client_secret"].secret_id
                key  = "latest"
              }
            }
          }
        }

        env {
          name  = "FITBIT_REDIRECT_URI"
          value = var.fitbit_redirect_uri != "" ? var.fitbit_redirect_uri : "${google_cloud_run_service.pierre_mcp_server.status[0].url}/api/oauth/callback/fitbit"
        }

        # OpenWeather API Key
        dynamic "env" {
          for_each = contains(keys(google_secret_manager_secret.secrets), "openweather_api_key") ? [1] : []
          content {
            name = "OPENWEATHER_API_KEY"
            value_from {
              secret_key_ref {
                name = google_secret_manager_secret.secrets["openweather_api_key"].secret_id
                key  = "latest"
              }
            }
          }
        }

        # Database connection pool settings
        env {
          name  = "POSTGRES_MAX_CONNECTIONS"
          value = "10"
        }

        env {
          name  = "POSTGRES_MIN_CONNECTIONS"
          value = "2"
        }

        env {
          name  = "POSTGRES_ACQUIRE_TIMEOUT"
          value = "30"
        }

        # Health check configuration
        startup_probe {
          http_get {
            path = "/health"
            port = 8081
          }
          initial_delay_seconds = 10
          timeout_seconds       = 3
          period_seconds        = 10
          failure_threshold     = 3
        }

        liveness_probe {
          http_get {
            path = "/health"
            port = 8081
          }
          initial_delay_seconds = 30
          timeout_seconds       = 3
          period_seconds        = 30
          failure_threshold     = 3
        }
      }

      container_concurrency = var.cloud_run_concurrency
      timeout_seconds       = var.cloud_run_timeout
    }

    metadata {
      annotations = {
        "autoscaling.knative.dev/minScale"        = tostring(var.cloud_run_min_instances)
        "autoscaling.knative.dev/maxScale"        = tostring(var.cloud_run_max_instances)
        "run.googleapis.com/vpc-access-connector" = google_vpc_access_connector.connector.name
        "run.googleapis.com/vpc-access-egress"    = "private-ranges-only"
        "run.googleapis.com/startup-cpu-boost"    = "true"
        "run.googleapis.com/execution-environment" = "gen2"
      }

      labels = merge(var.labels, {
        environment = var.environment
      })
    }
  }

  traffic {
    percent         = 100
    latest_revision = true
  }

  autogenerate_revision_name = true

  depends_on = [
    google_vpc_access_connector.connector,
    google_sql_database_instance.postgres,
    google_project_service.required_apis
  ]

  lifecycle {
    ignore_changes = [
      template[0].metadata[0].annotations["client.knative.dev/user-image"],
      template[0].metadata[0].annotations["run.googleapis.com/client-name"],
      template[0].metadata[0].annotations["run.googleapis.com/client-version"],
    ]
  }
}

# IAM policy for Cloud Run (allow public access or restrict)
resource "google_cloud_run_service_iam_member" "public_access" {
  count = length(var.allowed_ingress_cidrs) > 0 && contains(var.allowed_ingress_cidrs, "0.0.0.0/0") ? 1 : 0

  service  = google_cloud_run_service.pierre_mcp_server.name
  location = google_cloud_run_service.pierre_mcp_server.location
  role     = "roles/run.invoker"
  member   = "allUsers"
}

# ============================================================================
# MONITORING & ALERTING
# ============================================================================

# Uptime check for health endpoint
resource "google_monitoring_uptime_check_config" "health_check" {
  count = var.enable_uptime_checks ? 1 : 0

  display_name = "${var.service_name}-health-check-${var.environment}"
  timeout      = "10s"
  period       = "60s"

  http_check {
    path         = "/health"
    port         = "443"
    use_ssl      = true
    validate_ssl = true
  }

  monitored_resource {
    type = "uptime_url"
    labels = {
      project_id = var.project_id
      host       = replace(google_cloud_run_service.pierre_mcp_server.status[0].url, "https://", "")
    }
  }

  content_matchers {
    content = "ok"
    matcher = "CONTAINS_STRING"
  }
}

# Alert policy for service downtime
resource "google_monitoring_alert_policy" "service_down" {
  count = var.enable_uptime_checks && var.alert_email != "" ? 1 : 0

  display_name = "${var.service_name} Service Down (${var.environment})"
  combiner     = "OR"

  conditions {
    display_name = "Uptime check failed"

    condition_threshold {
      filter          = "metric.type=\"monitoring.googleapis.com/uptime_check/check_passed\" AND resource.type=\"uptime_url\" AND metric.label.check_id=\"${google_monitoring_uptime_check_config.health_check[0].uptime_check_id}\""
      duration        = "300s"
      comparison      = "COMPARISON_LT"
      threshold_value = 1

      aggregations {
        alignment_period   = "60s"
        per_series_aligner = "ALIGN_NEXT_OLDER"
      }
    }
  }

  notification_channels = [
    google_monitoring_notification_channel.email[0].id
  ]

  alert_strategy {
    auto_close = "1800s"
  }

  documentation {
    content = <<-EOT
      The Pierre MCP Server (${var.environment}) health check has failed.

      Runbook:
      1. Check Cloud Run logs: gcloud logging read "resource.type=cloud_run_revision AND resource.labels.service_name=${var.service_name}" --limit 50
      2. Check database connectivity: Verify Cloud SQL instance is running
      3. Check recent deployments: Review last Cloud Run revision
      4. Manual verification: curl ${google_cloud_run_service.pierre_mcp_server.status[0].url}/health
    EOT
  }
}

# Email notification channel
resource "google_monitoring_notification_channel" "email" {
  count = var.alert_email != "" ? 1 : 0

  display_name = "Email - ${var.alert_email}"
  type         = "email"

  labels = {
    email_address = var.alert_email
  }
}

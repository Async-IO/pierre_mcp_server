# ABOUTME: Creates Cloud SQL PostgreSQL instance for Pierre MCP Server
# ABOUTME: Configured with private IP, SSL, backups, and high availability options

# -----------------------------------------------------------------------------
# Cloud SQL Instance
# -----------------------------------------------------------------------------

resource "google_sql_database_instance" "postgres" {
  name                = "${var.service_name}-postgres"
  project             = var.project_id
  region              = var.region
  database_version    = var.database_version
  deletion_protection = var.deletion_protection

  settings {
    tier              = var.database_tier
    availability_type = var.environment == "production" ? "REGIONAL" : "ZONAL"
    disk_size         = var.disk_size_gb
    disk_autoresize   = true
    disk_type         = "PD_SSD"

    ip_configuration {
      ipv4_enabled                                  = false
      private_network                               = var.vpc_self_link
      enable_private_path_for_google_cloud_services = true
      require_ssl                                   = true

      ssl_mode = "ENCRYPTED_ONLY"
    }

    backup_configuration {
      enabled                        = var.backup_enabled
      start_time                     = var.backup_start_time
      point_in_time_recovery_enabled = var.backup_enabled
      transaction_log_retention_days = var.backup_enabled ? 7 : null

      backup_retention_settings {
        retained_backups = var.backup_enabled ? 7 : 1
        retention_unit   = "COUNT"
      }
    }

    maintenance_window {
      day          = 7 # Sunday
      hour         = 4 # 4 AM UTC
      update_track = "stable"
    }

    insights_config {
      query_insights_enabled  = true
      query_plans_per_minute  = 5
      query_string_length     = 1024
      record_application_tags = true
      record_client_address   = true
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

    user_labels = var.labels
  }

  depends_on = [var.private_vpc_connection_id]
}

# -----------------------------------------------------------------------------
# Database
# -----------------------------------------------------------------------------

resource "google_sql_database" "database" {
  name      = var.database_name
  project   = var.project_id
  instance  = google_sql_database_instance.postgres.name
  charset   = "UTF8"
  collation = "en_US.UTF8"
}

# -----------------------------------------------------------------------------
# Database User
# -----------------------------------------------------------------------------

resource "google_sql_user" "user" {
  name     = var.database_user
  project  = var.project_id
  instance = google_sql_database_instance.postgres.name
  password = var.database_password
  type     = "BUILT_IN"
}

# ABOUTME: Creates Secret Manager secrets for Pierre MCP Server
# ABOUTME: Auto-generates critical secrets, creates placeholders for OAuth

# -----------------------------------------------------------------------------
# Auto-Generated Secrets
# -----------------------------------------------------------------------------

# Database password
resource "random_password" "db_password" {
  length           = 32
  special          = true
  override_special = "!@#$%^&*()_+-="
}

resource "google_secret_manager_secret" "db_password" {
  project   = var.project_id
  secret_id = "${var.service_name}-db-password"

  labels = var.labels

  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "db_password" {
  secret      = google_secret_manager_secret.db_password.id
  secret_data = random_password.db_password.result
}

# Master encryption key
resource "random_password" "encryption_key" {
  length  = 32
  special = false
}

resource "google_secret_manager_secret" "encryption_key" {
  project   = var.project_id
  secret_id = "${var.service_name}-encryption-key"

  labels = var.labels

  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "encryption_key" {
  secret      = google_secret_manager_secret.encryption_key.id
  secret_data = base64encode(random_password.encryption_key.result)
}

# -----------------------------------------------------------------------------
# OAuth Placeholder Secrets (to be filled manually)
# -----------------------------------------------------------------------------

resource "google_secret_manager_secret" "strava_client_secret" {
  project   = var.project_id
  secret_id = "${var.service_name}-strava-client-secret"

  labels = var.labels

  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "strava_client_secret_placeholder" {
  secret      = google_secret_manager_secret.strava_client_secret.id
  secret_data = "PLACEHOLDER_FILL_MANUALLY"

  lifecycle {
    ignore_changes = [secret_data]
  }
}

resource "google_secret_manager_secret" "fitbit_client_secret" {
  project   = var.project_id
  secret_id = "${var.service_name}-fitbit-client-secret"

  labels = var.labels

  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "fitbit_client_secret_placeholder" {
  secret      = google_secret_manager_secret.fitbit_client_secret.id
  secret_data = "PLACEHOLDER_FILL_MANUALLY"

  lifecycle {
    ignore_changes = [secret_data]
  }
}

resource "google_secret_manager_secret" "garmin_client_secret" {
  project   = var.project_id
  secret_id = "${var.service_name}-garmin-client-secret"

  labels = var.labels

  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "garmin_client_secret_placeholder" {
  secret      = google_secret_manager_secret.garmin_client_secret.id
  secret_data = "PLACEHOLDER_FILL_MANUALLY"

  lifecycle {
    ignore_changes = [secret_data]
  }
}

resource "google_secret_manager_secret" "openweather_api_key" {
  project   = var.project_id
  secret_id = "${var.service_name}-openweather-api-key"

  labels = var.labels

  replication {
    auto {}
  }
}

resource "google_secret_manager_secret_version" "openweather_api_key_placeholder" {
  secret      = google_secret_manager_secret.openweather_api_key.id
  secret_data = "PLACEHOLDER_FILL_MANUALLY"

  lifecycle {
    ignore_changes = [secret_data]
  }
}

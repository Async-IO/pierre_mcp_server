# ABOUTME: Outputs from the secrets module
# ABOUTME: Provides secret IDs for Cloud Run deployment configuration

output "db_password" {
  description = "Generated database password"
  value       = random_password.db_password.result
  sensitive   = true
}

output "encryption_key" {
  description = "Generated encryption key (base64 encoded)"
  value       = base64encode(random_password.encryption_key.result)
  sensitive   = true
}

output "secret_ids" {
  description = "Map of secret names to their Secret Manager IDs"
  value = {
    db_password          = google_secret_manager_secret.db_password.secret_id
    encryption_key       = google_secret_manager_secret.encryption_key.secret_id
    strava_client_secret = google_secret_manager_secret.strava_client_secret.secret_id
    fitbit_client_secret = google_secret_manager_secret.fitbit_client_secret.secret_id
    garmin_client_secret = google_secret_manager_secret.garmin_client_secret.secret_id
    openweather_api_key  = google_secret_manager_secret.openweather_api_key.secret_id
  }
}

output "secret_names" {
  description = "Map of secret names to their full resource names"
  value = {
    db_password          = google_secret_manager_secret.db_password.name
    encryption_key       = google_secret_manager_secret.encryption_key.name
    strava_client_secret = google_secret_manager_secret.strava_client_secret.name
    fitbit_client_secret = google_secret_manager_secret.fitbit_client_secret.name
    garmin_client_secret = google_secret_manager_secret.garmin_client_secret.name
    openweather_api_key  = google_secret_manager_secret.openweather_api_key.name
  }
}

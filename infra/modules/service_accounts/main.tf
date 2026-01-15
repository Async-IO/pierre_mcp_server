# ABOUTME: Creates service accounts for Pierre MCP Server
# ABOUTME: Includes app SA for Cloud Run and deployer SA for GitHub Actions

# -----------------------------------------------------------------------------
# App Service Account (used by Cloud Run)
# -----------------------------------------------------------------------------

resource "google_service_account" "app" {
  account_id   = "${var.service_name}-app"
  project      = var.project_id
  display_name = "Pierre App Service Account"
  description  = "Service account for Pierre MCP Server Cloud Run service"
}

# Cloud SQL Client (connect to database)
resource "google_project_iam_member" "app_cloudsql_client" {
  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.app.email}"
}

# Secret Manager Secret Accessor (read secrets)
resource "google_project_iam_member" "app_secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.app.email}"
}

# Storage Object Admin (for any storage operations)
resource "google_project_iam_member" "app_storage_admin" {
  project = var.project_id
  role    = "roles/storage.objectAdmin"
  member  = "serviceAccount:${google_service_account.app.email}"
}

# AI Platform User (for Vertex AI access)
resource "google_project_iam_member" "app_aiplatform_user" {
  project = var.project_id
  role    = "roles/aiplatform.user"
  member  = "serviceAccount:${google_service_account.app.email}"
}

# -----------------------------------------------------------------------------
# Deployer Service Account (used by GitHub Actions)
# -----------------------------------------------------------------------------

resource "google_service_account" "deployer" {
  account_id   = "github-actions-deployer"
  project      = var.project_id
  display_name = "GitHub Actions Deployer"
  description  = "Service account for GitHub Actions CI/CD deployments"
}

# Cloud Run Admin (deploy services)
resource "google_project_iam_member" "deployer_run_admin" {
  project = var.project_id
  role    = "roles/run.admin"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Cloud Build Editor (submit builds)
resource "google_project_iam_member" "deployer_cloudbuild_editor" {
  project = var.project_id
  role    = "roles/cloudbuild.builds.editor"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Artifact Registry Writer (push images)
resource "google_project_iam_member" "deployer_artifact_writer" {
  project = var.project_id
  role    = "roles/artifactregistry.writer"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Secret Manager Secret Accessor (read secrets during deployment)
resource "google_project_iam_member" "deployer_secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Cloud SQL Client (for connection info)
resource "google_project_iam_member" "deployer_cloudsql_client" {
  project = var.project_id
  role    = "roles/cloudsql.client"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Service Account User (deploy as the app service account)
resource "google_project_iam_member" "deployer_sa_user" {
  project = var.project_id
  role    = "roles/iam.serviceAccountUser"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Storage Admin (for Cloud Build logs)
resource "google_project_iam_member" "deployer_storage_admin" {
  project = var.project_id
  role    = "roles/storage.admin"
  member  = "serviceAccount:${google_service_account.deployer.email}"
}

# Allow deployer to act as app service account
resource "google_service_account_iam_member" "deployer_can_act_as_app" {
  service_account_id = google_service_account.app.name
  role               = "roles/iam.serviceAccountUser"
  member             = "serviceAccount:${google_service_account.deployer.email}"
}

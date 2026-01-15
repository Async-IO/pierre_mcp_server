# ABOUTME: Enables required GCP APIs for Pierre MCP Server
# ABOUTME: APIs are enabled with disable_on_destroy = false for safety

resource "google_project_service" "apis" {
  for_each = toset([
    "compute.googleapis.com",              # VPC, networking
    "sqladmin.googleapis.com",             # Cloud SQL
    "secretmanager.googleapis.com",        # Secrets
    "run.googleapis.com",                  # Cloud Run
    "cloudbuild.googleapis.com",           # Cloud Build
    "artifactregistry.googleapis.com",     # Artifact Registry
    "iam.googleapis.com",                  # IAM
    "iamcredentials.googleapis.com",       # Workload Identity
    "servicenetworking.googleapis.com",    # Private Service Connect
    "vpcaccess.googleapis.com",            # Serverless VPC Access
    "cloudresourcemanager.googleapis.com", # Resource Manager
  ])

  project            = var.project_id
  service            = each.value
  disable_on_destroy = false

  timeouts {
    create = "10m"
    update = "10m"
  }
}

# Wait for APIs to be enabled before other resources can use them
resource "time_sleep" "api_propagation" {
  depends_on = [google_project_service.apis]

  create_duration = "30s"
}

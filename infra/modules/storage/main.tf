# ABOUTME: Creates GCS buckets for Pierre MCP Server
# ABOUTME: Includes buckets for backups and any application storage needs

# -----------------------------------------------------------------------------
# Application Data Bucket (optional, for file storage)
# -----------------------------------------------------------------------------

resource "google_storage_bucket" "app_data" {
  count = var.create_app_bucket ? 1 : 0

  name          = "${var.project_id}-${var.service_name}-data"
  project       = var.project_id
  location      = var.region
  storage_class = "STANDARD"

  uniform_bucket_level_access = true

  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      age = 90
    }
    action {
      type          = "SetStorageClass"
      storage_class = "NEARLINE"
    }
  }

  lifecycle_rule {
    condition {
      age = 365
    }
    action {
      type          = "SetStorageClass"
      storage_class = "COLDLINE"
    }
  }

  labels = var.labels
}

# -----------------------------------------------------------------------------
# Terraform State Bucket (for bootstrapping)
# -----------------------------------------------------------------------------

resource "google_storage_bucket" "terraform_state" {
  count = var.create_terraform_state_bucket ? 1 : 0

  name          = "pierre-terraform-state"
  project       = var.project_id
  location      = var.region
  storage_class = "STANDARD"

  uniform_bucket_level_access = true

  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      num_newer_versions = 5
    }
    action {
      type = "Delete"
    }
  }

  labels = var.labels
}

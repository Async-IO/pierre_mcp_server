# ABOUTME: Creates Artifact Registry Docker repository for Pierre images
# ABOUTME: Stores container images for Cloud Run deployment

resource "google_artifact_registry_repository" "docker" {
  location      = var.region
  project       = var.project_id
  repository_id = var.registry_name
  description   = "Docker repository for Pierre MCP Server container images"
  format        = "DOCKER"

  cleanup_policies {
    id     = "keep-recent-versions"
    action = "KEEP"

    most_recent_versions {
      keep_count = 10
    }
  }

  cleanup_policies {
    id     = "delete-old-untagged"
    action = "DELETE"

    condition {
      tag_state  = "UNTAGGED"
      older_than = "604800s" # 7 days
    }
  }

  labels = var.labels
}

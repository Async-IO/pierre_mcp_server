# ABOUTME: Outputs from the project module
# ABOUTME: Provides dependency marker for other modules

output "apis_enabled" {
  description = "Map of enabled API names"
  value       = { for k, v in google_project_service.apis : k => v.service }
}

output "ready" {
  description = "Marker indicating APIs are ready to use"
  value       = time_sleep.api_propagation.id
}

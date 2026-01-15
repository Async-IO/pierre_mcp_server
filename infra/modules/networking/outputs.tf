# ABOUTME: Outputs from the networking module
# ABOUTME: Provides VPC, subnet, and connector IDs for other modules

output "vpc_id" {
  description = "ID of the VPC network"
  value       = google_compute_network.vpc.id
}

output "vpc_name" {
  description = "Name of the VPC network"
  value       = google_compute_network.vpc.name
}

output "vpc_self_link" {
  description = "Self-link of the VPC network"
  value       = google_compute_network.vpc.self_link
}

output "subnet_id" {
  description = "ID of the subnet"
  value       = google_compute_subnetwork.subnet.id
}

output "subnet_name" {
  description = "Name of the subnet"
  value       = google_compute_subnetwork.subnet.name
}

output "vpc_connector_id" {
  description = "ID of the serverless VPC connector"
  value       = google_vpc_access_connector.connector.id
}

output "vpc_connector_name" {
  description = "Name of the serverless VPC connector"
  value       = google_vpc_access_connector.connector.name
}

output "private_vpc_connection_id" {
  description = "ID of the private VPC connection for Cloud SQL"
  value       = google_service_networking_connection.private_vpc_connection.id
}

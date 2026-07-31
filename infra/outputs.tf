output "server_image_repository" {
  description = "Docker repository path for immutable game-server release images."
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.server_images.repository_id}"
}

output "server_endpoint" {
  description = "Public game-server endpoint for client release configuration."
  value       = "${google_compute_address.game_server.address}:${var.game_port}"
}

output "game_port" {
  description = "Public TCP port configured for the game server."
  value       = var.game_port
}

output "vm_name" {
  description = "Name of the public game-server VM."
  value       = google_compute_instance.game_server.name
}

output "vm_zone" {
  description = "Zone containing the public game-server VM."
  value       = google_compute_instance.game_server.zone
}

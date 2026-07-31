output "server_image_repository" {
  description = "Docker repository path for immutable game-server release images."
  value       = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.server_images.repository_id}"
}

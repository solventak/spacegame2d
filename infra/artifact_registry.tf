resource "google_project_service" "artifact_registry" {
  project            = var.project_id
  service            = "artifactregistry.googleapis.com"
  disable_on_destroy = false
}

resource "google_artifact_registry_repository" "server_images" {
  location      = var.region
  repository_id = var.server_image_repository_id
  format        = "DOCKER"
  description   = "Immutable production images for the Relay Operations server."

  depends_on = [google_project_service.artifact_registry]
}

resource "google_artifact_registry_repository_iam_member" "server_release_writer" {
  project    = var.project_id
  location   = google_artifact_registry_repository.server_images.location
  repository = google_artifact_registry_repository.server_images.name
  role       = "roles/artifactregistry.writer"
  member     = "serviceAccount:gha-server-release@${var.project_id}.iam.gserviceaccount.com"
}

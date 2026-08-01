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

  cleanup_policies {
    id     = "keep-two-most-recent"
    action = "KEEP"

    most_recent_versions {
      keep_count = 2
    }
  }

  cleanup_policies {
    id     = "delete-older-than-thirty-days"
    action = "DELETE"

    condition {
      older_than = "2592000s"
    }
  }

  depends_on = [google_project_service.artifact_registry]
}

resource "google_artifact_registry_repository_iam_member" "server_release_writer" {
  project    = var.project_id
  location   = google_artifact_registry_repository.server_images.location
  repository = google_artifact_registry_repository.server_images.name
  role       = "roles/artifactregistry.writer"
  member     = "serviceAccount:gha-server-release@${var.project_id}.iam.gserviceaccount.com"
}

resource "google_artifact_registry_repository_iam_member" "game_server_runtime_reader" {
  project    = var.project_id
  location   = google_artifact_registry_repository.server_images.location
  repository = google_artifact_registry_repository.server_images.name
  role       = "roles/artifactregistry.reader"
  member     = "serviceAccount:${local.runtime_identity}"
}

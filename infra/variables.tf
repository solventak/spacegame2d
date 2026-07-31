variable "project_id" {
  type        = string
  description = "Google Cloud project that owns the playtest infrastructure."
}

variable "region" {
  type        = string
  description = "Google Cloud region for playtest infrastructure."
  default     = "us-west1"
}

variable "zone" {
  type        = string
  description = "Default Google Cloud zone for the game-server host."
  default     = "us-west1-a"
}

variable "server_image_repository_id" {
  type        = string
  description = "Artifact Registry repository ID for immutable game-server images."
  default     = "spacegame2d-server"
}

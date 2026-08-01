variable "project_id" {
  type        = string
  description = "Google Cloud project that owns the playtest infrastructure."
}

variable "billing_account_id" {
  type        = string
  default     = null
  description = "Billing account ID associated with the playtest project, supplied by the deployment environment."
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

  validation {
    condition     = startswith(var.zone, "${var.region}-")
    error_message = "zone must belong to the configured region."
  }
}

variable "game_port" {
  type        = number
  description = "Public TCP port reserved for the game server."
  default     = 4000

  validation {
    condition     = var.game_port >= 1 && var.game_port <= 65535
    error_message = "game_port must be a valid TCP port number."
  }
}

variable "game_server_name" {
  type        = string
  description = "Name of the public game-server VM."
  default     = "relay-operations-server"

  validation {
    condition     = can(regex("^[a-z]([-a-z0-9]*[a-z0-9])?$", var.game_server_name))
    error_message = "game_server_name must be a valid RFC 1035 resource name."
  }
}

variable "game_server_subnet_cidr" {
  type        = string
  description = "IPv4 range for the dedicated public game-server subnet."
  default     = "10.42.0.0/24"
}

variable "server_image_repository_id" {
  type        = string
  description = "Artifact Registry repository ID for immutable game-server images."
  default     = "spacegame2d-server"
}

variable "server_image_name" {
  type        = string
  description = "Artifact Registry image name for the game-server container."
  default     = "spacegame2d-server"

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9._-]*$", var.server_image_name))
    error_message = "server_image_name must be a valid Artifact Registry image name."
  }
}

variable "runtime_service_account_id" {
  type        = string
  description = "Existing bootstrap-created service-account ID attached to the game-server VM."
  default     = "relay-server-runtime"

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{4,28}[a-z0-9]$", var.runtime_service_account_id))
    error_message = "runtime_service_account_id must be a valid Google service-account ID."
  }
}

variable "operator_identity" {
  type        = string
  description = "Alex's Google identity in IAM member form."
  default     = "user:akennedy4155@gmail.com"

  validation {
    condition     = can(regex("^(user|group):[^ ]+$", var.operator_identity))
    error_message = "operator_identity must be an IAM user:/group: member."
  }
}

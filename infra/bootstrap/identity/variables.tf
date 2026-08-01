variable "project_id" {
  type        = string
  description = "Google Cloud project that owns the playtest infrastructure."
}

variable "project_number" {
  type        = string
  description = "Immutable numeric identifier for the Google Cloud project."
}

variable "github_owner" {
  type        = string
  description = "GitHub owner of the trusted repository."
}

variable "github_owner_id" {
  type        = string
  description = "Immutable numeric GitHub owner identifier."
}

variable "github_repository" {
  type        = string
  description = "Trusted GitHub repository in owner/name form."
}

variable "github_repository_id" {
  type        = string
  description = "Immutable numeric GitHub repository identifier."
}

variable "production_state_bucket" {
  type        = string
  description = "Production Terraform state bucket readable by the client release identity."
  default     = "relayoperations-terraform-state-926404861741"
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

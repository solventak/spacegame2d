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

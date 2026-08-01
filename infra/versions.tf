terraform {
  required_version = "~> 1.14.0"

  required_providers {
    external = {
      source  = "hashicorp/external"
      version = "= 2.4.0"
    }

    google = {
      source  = "hashicorp/google"
      version = "= 7.41.0"
    }
  }
}

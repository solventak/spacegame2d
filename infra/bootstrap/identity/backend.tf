terraform {
  backend "gcs" {
    bucket = "relayoperations-terraform-state-926404861741"
    prefix = "bootstrap/identity"
  }
}

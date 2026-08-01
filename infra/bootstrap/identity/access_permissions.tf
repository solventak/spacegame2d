locals {
  game_server_access_members = {
    operator = var.operator_identity
    release  = "serviceAccount:${google_service_account.github_actions["release"].email}"
  }
}

resource "google_project_iam_custom_role" "game_server_compute_cli_lookup" {
  role_id     = "relayOperationsComputeCliLookup"
  title       = "Relay Operations Compute CLI lookup"
  description = "Minimal project lookup permission required by gcloud compute ssh."
  stage       = "GA"
  permissions = ["compute.projects.get"]
}

resource "google_project_iam_member" "game_server_compute_cli_lookup" {
  for_each = local.game_server_access_members

  project = var.project_id
  role    = google_project_iam_custom_role.game_server_compute_cli_lookup.name
  member  = each.value
}

resource "google_project_iam_custom_role" "iap_policy_viewer" {
  role_id     = "relayOperationsIapPolicyViewer"
  title       = "Relay Operations IAP policy viewer"
  description = "Read-only access to per-instance IAP tunnel IAM policies for Terraform plan."
  stage       = "GA"
  permissions = ["iap.tunnelInstances.getIamPolicy"]
}

resource "google_project_iam_member" "iap_policy_viewer" {
  project = var.project_id
  role    = google_project_iam_custom_role.iap_policy_viewer.name
  member  = "serviceAccount:${google_service_account.github_actions["plan"].email}"
}

resource "google_project_iam_custom_role" "iap_policy_admin" {
  role_id     = "relayOperationsIapPolicyAdmin"
  title       = "Relay Operations IAP policy admin"
  description = "Read/write access to per-instance IAP tunnel IAM policies for Terraform apply."
  stage       = "GA"
  permissions = [
    "iap.tunnelInstances.getIamPolicy",
    "iap.tunnelInstances.setIamPolicy",
  ]
}

resource "google_project_iam_member" "iap_policy_admin" {
  project = var.project_id
  role    = google_project_iam_custom_role.iap_policy_admin.name
  member  = "serviceAccount:${google_service_account.github_actions["apply"].email}"
}

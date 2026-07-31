locals {
  terraform_plan_artifact_registry_roles = toset([
    "roles/artifactregistry.reader",
  ])
  terraform_apply_artifact_registry_roles = toset([
    "roles/artifactregistry.admin",
    "roles/serviceusage.serviceUsageAdmin",
  ])
}

resource "google_project_iam_member" "terraform_plan_artifact_registry_reader" {
  for_each = local.terraform_plan_artifact_registry_roles

  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.github_actions["plan"].email}"
}

resource "google_project_iam_member" "terraform_apply_artifact_registry_admin" {
  for_each = local.terraform_apply_artifact_registry_roles

  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.github_actions["apply"].email}"
}

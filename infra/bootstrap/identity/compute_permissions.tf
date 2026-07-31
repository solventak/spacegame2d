locals {
  terraform_plan_compute_roles = toset([
    "roles/compute.viewer",
  ])
  terraform_apply_compute_roles = toset([
    "roles/compute.instanceAdmin.v1",
    "roles/compute.networkAdmin",
    "roles/compute.securityAdmin",
  ])
}

resource "google_project_iam_member" "terraform_plan_compute_viewer" {
  for_each = local.terraform_plan_compute_roles

  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.github_actions["plan"].email}"
}

resource "google_project_iam_member" "terraform_apply_compute_admin" {
  for_each = local.terraform_apply_compute_roles

  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.github_actions["apply"].email}"
}

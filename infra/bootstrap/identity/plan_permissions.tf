locals {
  terraform_plan_reader_roles = toset([
    "roles/iam.securityReviewer",
    "roles/iam.serviceAccountViewer",
    "roles/iam.workloadIdentityPoolViewer",
    "roles/serviceusage.serviceUsageViewer",
  ])
}

resource "google_project_iam_member" "terraform_plan_reader" {
  for_each = local.terraform_plan_reader_roles

  project = var.project_id
  role    = each.value
  member  = join("", ["serviceAccount:", lookup(google_service_account.github_actions, "plan").email])
}

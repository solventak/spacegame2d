locals {
  service_accounts = {
    plan = {
      account_id   = "gha-tf-plan"
      display_name = "GitHub Actions Terraform plan"
    }
    apply = {
      account_id   = "gha-tf-apply"
      display_name = "GitHub Actions Terraform apply"
    }
    release = {
      account_id   = "gha-server-release"
      display_name = "GitHub Actions server release"
    }
  }
}

resource "google_service_account" "github_actions" {
  for_each = local.service_accounts

  account_id   = each.value.account_id
  display_name = each.value.display_name
  description  = "Federated identity. Resource permissions are granted by the owning infrastructure ticket."

  depends_on = [google_project_service.required]
}

resource "google_service_account_iam_member" "github_actions_workload_identity_user" {
  for_each = google_service_account.github_actions

  service_account_id = each.value.name
  role               = "roles/iam.workloadIdentityUser"
  member = join("/", [
    "principalSet://iam.googleapis.com/projects/${var.project_number}/locations/global/workloadIdentityPools/${google_iam_workload_identity_pool.github_actions.workload_identity_pool_id}/attribute.ci_role",
    each.key,
  ])
}

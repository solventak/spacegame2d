resource "google_project_iam_custom_role" "terraform_plan_billing_budget" {
  role_id     = "relayOperationsBillingBudgetViewer"
  title       = "Relay Operations billing budget viewer"
  description = "Read-only project-scoped billing budget access for Terraform plan."
  stage       = "GA"
  permissions = [
    "billing.resourcebudgets.read",
    "resourcemanager.projects.get",
  ]
}

resource "google_project_iam_member" "terraform_plan_billing_budget" {
  project = var.project_id
  role    = google_project_iam_custom_role.terraform_plan_billing_budget.name
  member  = "serviceAccount:${google_service_account.github_actions["plan"].email}"
}

resource "google_project_iam_custom_role" "terraform_apply_billing_budget" {
  role_id     = "relayOperationsBillingBudgetAdmin"
  title       = "Relay Operations billing budget administrator"
  description = "Read/write project-scoped billing budget access for Terraform apply."
  stage       = "GA"
  permissions = [
    "billing.resourcebudgets.read",
    "billing.resourcebudgets.write",
    "resourcemanager.projects.get",
  ]
}

resource "google_project_iam_member" "terraform_apply_billing_budget" {
  project = var.project_id
  role    = google_project_iam_custom_role.terraform_apply_billing_budget.name
  member  = "serviceAccount:${google_service_account.github_actions["apply"].email}"
}

resource "google_project_iam_member" "terraform_plan_monitoring_channel" {
  project = var.project_id
  role    = "roles/monitoring.notificationChannelViewer"
  member  = "serviceAccount:${google_service_account.github_actions["plan"].email}"
}

resource "google_project_iam_member" "terraform_apply_monitoring_channel" {
  project = var.project_id
  role    = "roles/monitoring.notificationChannelEditor"
  member  = "serviceAccount:${google_service_account.github_actions["apply"].email}"
}

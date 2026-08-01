mock_provider "google" {}

run "plans_game_server_identity_contract" {
  command = plan

  variables {
    project_id           = "relayoperations"
    project_number       = "926404861741"
    github_owner         = "solventak"
    github_owner_id      = "155677178"
    github_repository    = "solventak/spacegame2d"
    github_repository_id = "1310387780"
  }

  assert {
    condition     = google_service_account.game_server_runtime.account_id == "relay-server-runtime"
    error_message = "The VM runtime identity must use the documented service-account ID."
  }

  assert {
    condition     = length(google_service_account_iam_member.game_server_runtime_user) == 3
    error_message = "The VM runtime identity must be usable by Alex, Terraform apply, and server release."
  }

  assert {
    condition     = contains(google_project_iam_custom_role.game_server_compute_cli_lookup.permissions, "compute.projects.get")
    error_message = "The CLI lookup role must contain only the required Compute project lookup permission."
  }

  assert {
    condition     = contains(google_project_iam_custom_role.iap_policy_viewer.permissions, "iap.tunnelInstances.getIamPolicy")
    error_message = "The Terraform plan identity must be able to read per-instance IAP policy."
  }

  assert {
    condition = contains(google_project_iam_custom_role.iap_policy_admin.permissions, "iap.tunnelInstances.getIamPolicy") && contains(
      google_project_iam_custom_role.iap_policy_admin.permissions,
      "iap.tunnelInstances.setIamPolicy",
    )
    error_message = "The Terraform apply identity must be able to manage per-instance IAP policy."
  }
}

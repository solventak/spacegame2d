output "project_id" {
  value = var.project_id
}

output "workload_identity_provider" {
  value = {
    for key, provider in google_iam_workload_identity_pool_provider.github :
    key => provider.name
  }
}

output "service_account_email" {
  value = {
    for key, service_account in google_service_account.github_actions :
    key => service_account.email
  }
}

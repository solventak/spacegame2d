locals {
  provider_attribute_mapping = {
    "google.subject"                = "assertion.sub"
    "attribute.environment"         = "assertion.environment"
    "attribute.event_name"          = "assertion.event_name"
    "attribute.ref"                 = "assertion.ref"
    "attribute.repository_id"       = "assertion.repository_id"
    "attribute.repository_owner_id" = "assertion.repository_owner_id"
    "attribute.workflow_ref"        = "assertion.workflow_ref"
  }

  github_providers = {
    plan = {
      id = "terraform-plan"
      condition = join(" && ", [
        "assertion.repository_id == '${var.github_repository_id}'",
        "assertion.repository_owner_id == '${var.github_owner_id}'",
        "assertion.event_name == 'pull_request'",
        "(assertion.base_ref == 'dev' || assertion.base_ref == 'main')",
        "assertion.workflow_ref.startsWith('${var.github_repository}/.github/workflows/terraform-plan.yml@')",
      ])
    }
    apply = {
      id = "terraform-apply"
      condition = join(" && ", [
        "assertion.repository_id == '${var.github_repository_id}'",
        "assertion.repository_owner_id == '${var.github_owner_id}'",
        "assertion.event_name == 'workflow_dispatch'",
        "assertion.ref == 'refs/heads/main'",
        "assertion.environment == 'production'",
        "assertion.workflow_ref == '${var.github_repository}/.github/workflows/terraform-apply.yml@refs/heads/main'",
      ])
    }
    release = {
      id = "server-release"
      condition = join(" && ", [
        "assertion.repository_id == '${var.github_repository_id}'",
        "assertion.repository_owner_id == '${var.github_owner_id}'",
        "assertion.event_name == 'workflow_dispatch'",
        "assertion.ref == 'refs/heads/main'",
        "assertion.environment == 'production'",
        "assertion.workflow_ref == '${var.github_repository}/.github/workflows/release-server.yml@refs/heads/main'",
      ])
    }
  }
}

resource "google_iam_workload_identity_pool" "github_actions" {
  workload_identity_pool_id = "github-actions"
  display_name              = "GitHub Actions"
  description               = "Federated GitHub Actions identities for spacegame2d."

  depends_on = [google_project_service.required]
}

resource "google_iam_workload_identity_pool_provider" "github" {
  for_each = local.github_providers

  workload_identity_pool_id          = google_iam_workload_identity_pool.github_actions.workload_identity_pool_id
  workload_identity_pool_provider_id = each.value.id
  display_name                       = "GitHub Actions ${title(each.key)}"
  description                        = "GitHub Actions ${each.key} identity for ${var.github_repository}."
  attribute_mapping = merge(local.provider_attribute_mapping, {
    "attribute.ci_role" = "'${each.key}'"
  })
  attribute_condition = each.value.condition

  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
}

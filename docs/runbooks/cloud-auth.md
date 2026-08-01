# Cloud authentication bootstrap

SWA-64 establishes keyless GitHub Actions authentication for the `relayoperations`
project. Terraform state is intentionally local only for this bootstrap root. SWA-67 migrates
the state to the versioned GCS backend before infrastructure resources are managed in CI.

## Bootstrap with personal ADC

Use Alex's personal Google identity. Do not create or download a service-account key.

```bash
gcloud auth application-default login
gcloud auth application-default set-quota-project relayoperations
cp infra/bootstrap/identity/terraform.tfvars.example \
  infra/bootstrap/identity/terraform.tfvars
terraform -chdir=infra/bootstrap/identity init
terraform -chdir=infra/bootstrap/identity fmt -check
terraform -chdir=infra/bootstrap/identity validate
terraform -chdir=infra/bootstrap/identity plan -out=swa-64.tfplan
terraform -chdir=infra/bootstrap/identity apply swa-64.tfplan
terraform -chdir=infra/bootstrap/identity plan -detailed-exitcode
```

The final command exits `0` when no drift remains. Keep the local state file secure and out of
Git until SWA-67 migrates it.

## Configure GitHub repository settings

Before changing the environment, save its current configuration outside the repository:

```bash
gh api repos/solventak/spacegame2d/environments/production \
  > production-environment.before.json
```

Create the `production` environment with Alex (`155677178`) as the only required reviewer,
self-approval allowed, and no wait timer. Then add `main` as its only allowed deployment branch.
Use the GitHub Environments REST API; do not add a GitHub provider or a long-lived token to
Terraform.

```bash
gh api --method PUT repos/solventak/spacegame2d/environments/production \
  --input production-environment.json
gh api --method POST \
  repos/solventak/spacegame2d/environments/production/deployment-branch-policies \
  -f name=main -f type=branch
```

`production-environment.json` contains:

```json
{
  "wait_timer": 0,
  "prevent_self_review": false,
  "reviewers": [{"type": "User", "id": 155677178}],
  "deployment_branch_policy": {
    "protected_branches": false,
    "custom_branch_policies": true
  }
}
```

Populate these repository variables from Terraform outputs. They are identifiers, not secrets.

| Variable | Terraform output key |
| --- | --- |
| `GCP_PROJECT_ID` | `project_id` |
| `GCP_TERRAFORM_PLAN_WIF_PROVIDER` | `workload_identity_provider.plan` |
| `GCP_TERRAFORM_APPLY_WIF_PROVIDER` | `workload_identity_provider.apply` |
| `GCP_SERVER_RELEASE_WIF_PROVIDER` | `workload_identity_provider.release` |
| `GCP_CLIENT_RELEASE_WIF_PROVIDER` | `workload_identity_provider.client_release` |
| `GCP_TERRAFORM_PLAN_SERVICE_ACCOUNT` | `service_account_email.plan` |
| `GCP_TERRAFORM_APPLY_SERVICE_ACCOUNT` | `service_account_email.apply` |
| `GCP_SERVER_RELEASE_SERVICE_ACCOUNT` | `service_account_email.release` |
| `GCP_CLIENT_RELEASE_SERVICE_ACCOUNT` | `service_account_email.client_release` |

The bootstrap root also outputs `game_server_runtime_service_account_email`. It is attached to
the VM by the production root and is not a GitHub Actions identity.

## Verification and recovery

After the credentials variables and environment are configured:

- Open a same-repository PR changing `infra/` and confirm `Terraform Plan` authenticates.
- Confirm a fork PR skips its authentication job.
- Run `Terraform Apply` and `Release Server` from `main`; each requires the `production`
  approval and reports its own service-account identity.

If a trust policy is wrong, use personal ADC to remove the affected service-account
`roles/iam.workloadIdentityUser` binding or disable the affected provider, then apply the corrected
Terraform configuration. Do not immediately destroy and recreate a workload identity pool or
provider: deleted identifiers can remain unavailable during the recovery period. Restore the
saved GitHub environment configuration with the GitHub API if its protection policy needs to be
reverted.

## Terraform state and infrastructure delivery

SWA-67 keeps the identity root on remote GCS state, supplies the protected Terraform plan/apply
workflows, and preserves the provider names, service-account emails, and workflow filenames
created here. Follow [the Terraform state and delivery runbook](terraform.md) for bucket
bootstrap, state migration, normal CI delivery, and recovery.

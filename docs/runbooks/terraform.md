# Terraform state and delivery runbook

SWA-67 keeps Terraform state in a recoverable Cloud Storage backend and uses keyless GitHub
Actions identities for infrastructure planning and production applies. Use this runbook for the
one-time backend bootstrap, normal administration, and recovery.

## Guardrails

- The state bucket is `relayoperations-terraform-state-926404861741` in `us-west1`.
- The bucket is created manually with Alex's Google identity; it is not a Terraform resource.
- Do not create or download service-account JSON keys.
- Do not commit a state file, `*.tfvars`, `*.tfplan`, or `gha-creds-*.json` file.
- Do not run a local plan or apply while the GitHub Terraform workflow is active.
- The production apply workflow manages only `infra/`. Keep
  `infra/bootstrap/identity/` personal-ADC administered so the production identity cannot
  modify its own Workload Identity Federation trust.
- The public host uses a dedicated VPC. It has one public ingress rule: the configured game
  TCP port. It deliberately has no public SSH rule. IAP/OS Login access is added by SWA-69,
  and the server runtime is added by SWA-65.

## Terraform CI transition checklist

Use this checklist when a Terraform change adds a GitHub variable, `gcloud` fallback, Google API,
or Workload Identity Federation permission:

1. Preserve the workflow's existing trusted GitHub event unless the trust model is deliberately
   changing. In particular, `pull_request_target` evaluates the workflow from the PR base branch,
   not the workflow file in the PR head.
2. Account for that base-branch behavior during rollout: a repository-variable mapping added in a
   PR is not available to that PR's existing `pull_request_target` workflow until the PR merges.
   Keep a fail-loud fallback for the transition, or make the one-time prerequisite before opening
   the PR.
3. Enable every Google API the fallback calls before relying on it. For example, the
   billing-account fallback requires the Cloud Billing API:

   ```bash
   gcloud services enable cloudbilling.googleapis.com --project=<project-id>
   ```

4. Check the full dependency chain in this order: GitHub event, Workload Identity Federation
   attribute condition, repository-variable visibility, enabled Google APIs, then IAM roles for
   the plan identity.
5. After changing workflow triggers or authentication, inspect the runs with
   `gh run list --branch <branch>`. Confirm there is one intended Terraform Plan run and review
   its cloud-aware plan output; local `terraform test` cannot validate GitHub event semantics or
   live Google API access.
## One-time bucket bootstrap

Set the values below in a shell that is authenticated as Alex. The bucket name is globally
unique because it includes the Google Cloud project number; stop if Cloud Storage reports that
it belongs to another project.

```bash
gcloud auth login
gcloud auth application-default login
gcloud auth application-default set-quota-project relayoperations

export STATE_BUCKET=relayoperations-terraform-state-926404861741
export GCP_PROJECT=relayoperations
export PLAN_SERVICE_ACCOUNT=gha-tf-plan@relayoperations.iam.gserviceaccount.com
export APPLY_SERVICE_ACCOUNT=gha-tf-apply@relayoperations.iam.gserviceaccount.com

gcloud storage buckets create "gs://${STATE_BUCKET}" \
  --project="${GCP_PROJECT}" \
  --location=us-west1 \
  --uniform-bucket-level-access \
  --public-access-prevention

gcloud storage buckets update "gs://${STATE_BUCKET}" --versioning
```

The new bucket has no lifecycle rule by default. Do not add a lifecycle `Delete` rule for
state objects or noncurrent object versions. Leave Cloud Storage soft delete enabled as the
additional bounded recovery mechanism for accidental bucket deletion; Object Versioning is the
mechanism that retains replaced and deleted state-object versions indefinitely.

Grant only the backend object permissions needed by CI:

```bash
gcloud storage buckets add-iam-policy-binding "gs://${STATE_BUCKET}" \
  --member="serviceAccount:${PLAN_SERVICE_ACCOUNT}" \
  --role=roles/storage.objectViewer

gcloud storage buckets add-iam-policy-binding "gs://${STATE_BUCKET}" \
  --member="serviceAccount:${APPLY_SERVICE_ACCOUNT}" \
  --role=roles/storage.objectAdmin
```

Do not grant either CI service account `roles/storage.admin`: neither needs permission to
delete, reconfigure, or change IAM on the state bucket.

Verify the completed bucket before migrating state:

```bash
gcloud storage buckets describe "gs://${STATE_BUCKET}" \
  --format='yaml(location,storageClass,iamConfiguration,versioning,lifecycle,softDeletePolicy)'
gcloud storage buckets get-iam-policy "gs://${STATE_BUCKET}"
```

Confirm that the location is `US-WEST1`, Object Versioning is enabled, uniform bucket-level
access and public-access prevention are enabled, no lifecycle delete rule is present, and the
two CI bindings have only the roles above.

## Initialize and migrate Terraform state

The repository uses independent state prefixes in the same bucket:

| Root | GCS prefix | Administration path |
| --- | --- | --- |
| `infra/` | `production` | Protected GitHub apply workflow or Alex ADC |
| `infra/bootstrap/identity/` | `bootstrap/identity` | Alex ADC only |

First initialize the production root. It has no infrastructure resources until SWA-62, but
this verifies that the backend is reachable without an out-of-band state file.

```bash
terraform -chdir=infra init
terraform -chdir=infra validate
```

The SWA-64 identity root currently has local state. Before migration, secure a local backup
outside the repository and confirm it is mode `0600` or otherwise inaccessible to other users.
Then migrate it interactively with ADC:

```bash
terraform -chdir=infra/bootstrap/identity init -migrate-state
terraform -chdir=infra/bootstrap/identity plan \
  -var-file=terraform.tfvars \
  -detailed-exitcode
```

The final command must exit `0` before local state remnants are removed. After migration,
confirm that both roots can initialize from a fresh clone with no copied state file.

## Pull-request planning

The `Terraform Plan` workflow runs only when Terraform source, the Terraform version file, or
Terraform workflow definitions change.

- A fork PR receives `terraform fmt -check` and `terraform validate` only. Its workflow has no
  OIDC token permission and never receives Google credentials.
- A same-repository PR additionally authenticates as `gha-tf-plan`, reads the remote state, and
  publishes speculative plans for both roots to the Actions job summary.
- The plan uses `-lock=false` because its state identity is intentionally read-only. The shared
  `terraform-production` concurrency group prevents it from overlapping a CI apply.
- Review the plan in the job summary. No PR comment or binary plan artifact is created.

For the identity root, the plan identity has only the read roles required to inspect service
accounts, Workload Identity pools, Service Usage, and IAM policy. The first no-change cloud
plan is the verification that these bindings are sufficient.

## Production apply

Run `Terraform Apply` manually from `main` only. The job enters the protected `production`
environment before it receives the apply Workload Identity Federation token.

After approval, it:

1. checks out the dispatched `main` SHA;
2. creates a fresh, state-locked production plan;
3. writes its human-readable form to the job summary;
4. applies that exact saved plan in the same job; and
5. deletes its temporary plan file before the runner exits.

The job summary reports the run, commit, plan/apply result, and only the following outputs when
they exist and Terraform marks them non-sensitive:

- `server_endpoint`
- `game_port`
- `vm_name`
- `vm_zone`

Do not add a generic `terraform output -json` command to logs or job summaries: JSON output
prints sensitive values in cleartext.

## Public game-server host

SWA-62 provisions the network foundation only: an `e2-micro` Ubuntu 24.04 LTS VM, a regional
static IPv4 address, a dedicated custom VPC/subnet, and the public game-port firewall rule. The
VM has no startup script, Docker installation, server process, or attached runtime service
account in this stage.

Before the production workflow can create Compute Engine resources, apply the matching
`infra/bootstrap/identity/` change with Alex's ADC. This grants the existing Terraform plan
identity Compute Viewer and the existing apply identity the Compute instance, network, and
firewall administration roles. It does not change the Workload Identity Federation setup or
the GitHub workflows.

After the infrastructure PR is merged to `main`, run **Terraform Apply** from `main` and approve
the protected `production` environment. The workflow reports these non-sensitive outputs:

- `server_endpoint` — the reserved `static-ip:port` value for a future client build.
- `game_port` — the configured public TCP port, defaulting to `4000`.
- `vm_name` and `vm_zone` — the host location for the later IAP/OS Login runbook.

The static address is a separate regional resource. Recreating only the VM therefore retains the
same client endpoint. To request a capacity fallback, set `TF_VAR_zone` for the approved
Terraform command; it must remain within the configured region. Do not change the endpoint
outside a reviewed infrastructure apply.

### Refresh the public client endpoint

After a reviewed production apply changes `server_endpoint`, dispatch **Release Client** from the
corresponding `main` commit. The workflow reads the deployed value with
`terraform -chdir=infra output -raw server_endpoint` using its dedicated read-only identity,
validates it before compilation, and injects it only into the client build. Verify the workflow
summary shows the expected endpoint and source commit before distributing its artifact. It fails
before compiling if the output is missing or invalid; never replace it with localhost for a public
release.

### Verify the host foundation

After apply, inspect the outputs and Compute resources with Alex's authenticated `gcloud` CLI:

```bash
terraform -chdir=infra output server_endpoint
terraform -chdir=infra output game_port
terraform -chdir=infra output vm_name
terraform -chdir=infra output vm_zone

gcloud compute instances describe relay-operations-server \
  --project=relayoperations \
  --zone=us-west1-a

gcloud compute firewall-rules describe relay-operations-public-game-port \
  --project=relayoperations
```

Confirm that the instance uses `e2-micro`, the Ubuntu 24.04 LTS image family, the static address,
and only the public TCP game-port rule. A connection to port 4000 cannot be accepted until SWA-65
installs and starts the server service; SWA-62 verifies that the public network path is configured.
Do not add a public TCP/22 rule as a diagnostic shortcut.

### Controlled destroy and recreation

Before publishing the endpoint to clients, inspect a destroy plan to confirm every host resource
can be removed cleanly:

```bash
terraform -chdir=infra plan -destroy -input=false
```

Only run `terraform destroy` during an approved test window: it removes the VM, network, firewall,
and static address, so a subsequent apply receives a new endpoint. To test VM replacement without
changing the endpoint, replace only `google_compute_instance.game_server`; the separate static
address must remain in place.

## State recovery

### Refresh ADC

If local administration fails because ADC expired, refresh it:

```bash
gcloud auth application-default login
gcloud auth application-default set-quota-project relayoperations
```

### Inspect and restore an object version

List all generations of a state object, identify the last known-good generation, then copy it
back over the live object. Do this only after recording the current state and verifying no other
apply is in progress.

```bash
gcloud storage ls --all-versions \
  "gs://relayoperations-terraform-state-926404861741/production/"

terraform -chdir=infra state pull > production-state-before-recovery.json

gcloud storage cp \
  "gs://relayoperations-terraform-state-926404861741/production/default.tfstate#GENERATION" \
  "gs://relayoperations-terraform-state-926404861741/production/default.tfstate"
```

Replace `GENERATION` only after reviewing the object listing. The copy creates a new live
generation and retains the previously live state as a noncurrent version.

### Recover a soft-deleted bucket

Object Versioning alone cannot recover a deleted bucket. Cloud Storage soft delete provides a
separate recovery window. If the entire bucket is deleted, stop all Terraform work, use the
Cloud Storage bucket-recovery procedure within the configured soft-delete window, recheck IAM
and bucket configuration, and run a no-change plan with Alex's ADC before re-enabling CI.

### Backend write failure or stale lock

Terraform can write an emergency local state file when a remote backend write fails. Keep that
file private. First repair backend access and run `terraform state pull`; compare it with the
emergency file before considering `terraform state push`. State push overwrites remote state and
is exceptional.

Use `terraform force-unlock LOCK_ID` only when the lock is confirmed stale and `LOCK_ID` is the
exact identifier reported by Terraform. It does not modify infrastructure, but unlocking an
active operation can corrupt state.

### Broken Workload Identity Federation

If CI authentication fails, use Alex's ADC against `infra/bootstrap/identity/` to inspect and
correct the trust configuration. Do not destroy and recreate the pool or provider as a first
response; deleted identifiers can remain unavailable during the recovery period.

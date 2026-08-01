mock_provider "google" {
  mock_data "google_project" {
    defaults = {
      number = "926404861741"
    }
  }
}

mock_provider "external" {
  mock_data "external" {
    defaults = {
      result = {
        billing_account_id = "fallback-billing-account"
      }
    }
  }
}

run "plans_the_default_public_host" {
  command = plan

  variables {
    project_id         = "relayoperations"
    billing_account_id = "test-billing-account"
  }

  assert {
    condition     = google_compute_instance.game_server.machine_type == "e2-micro"
    error_message = "The public host must use the Always Free-eligible e2-micro machine type."
  }

  assert {
    condition     = google_compute_instance.game_server.zone == "us-west1-a"
    error_message = "The public host must default to us-west1-a."
  }

  assert {
    condition     = google_compute_instance.game_server.boot_disk[0].initialize_params[0].image == "ubuntu-os-cloud/ubuntu-2404-lts-amd64"
    error_message = "The public host must use Ubuntu 24.04 LTS."
  }

  assert {
    condition     = google_compute_network.game_server.auto_create_subnetworks == false
    error_message = "The public host must use an isolated custom VPC."
  }

  assert {
    condition     = google_compute_firewall.public_game_port.source_ranges == toset(["0.0.0.0/0"])
    error_message = "Friend playtests must be able to reach the public game port."
  }

  assert {
    condition = anytrue([
      for rule in google_compute_firewall.public_game_port.allow :
      rule.protocol == "tcp" && length(rule.ports) == 1 && contains(rule.ports, "4000")
    ])
    error_message = "The default public firewall rule must expose TCP port 4000 only."
  }

  assert {
    condition     = output.game_port == 4000 && output.vm_zone == "us-west1-a"
    error_message = "The machine-readable defaults must report the configured port and zone."
  }

  assert {
    condition     = google_compute_firewall.iap_ssh.source_ranges == toset(["35.235.240.0/20"])
    error_message = "SSH must be reachable only from the IAP TCP forwarding range."
  }

  assert {
    condition = anytrue([
      for rule in google_compute_firewall.iap_ssh.allow :
      rule.protocol == "tcp" && contains(rule.ports, "22")
    ])
    error_message = "The IAP firewall rule must expose TCP port 22."
  }

  assert {
    condition     = google_compute_instance.game_server.metadata["enable-oslogin"] == "TRUE"
    error_message = "The VM must enforce OS Login."
  }

  assert {
    condition     = google_compute_instance.game_server.service_account[0].email == "relay-server-runtime@relayoperations.iam.gserviceaccount.com"
    error_message = "The VM must use the dedicated runtime service account."
  }

  assert {
    condition = strcontains(google_compute_instance.game_server.metadata_startup_script, "relay-operations-deploy") && strcontains(
      google_compute_instance.game_server.metadata_startup_script,
      "GAME_PORT=4000",
    )
    error_message = "The VM startup contract must install the deployment helper and port configuration."
  }

  assert {
    condition = strcontains(google_compute_instance.game_server.metadata_startup_script, "--log-driver=journald") && strcontains(
      google_compute_instance.game_server.metadata_startup_script,
      "7d",
    )
    error_message = "The runtime must preserve container logs in journald and bound failed-release files to seven days."
  }

  assert {
    condition     = length(google_compute_instance_iam_member.os_admin_login) == 2 && length(google_iap_tunnel_instance_iam_member.tunnel_access) == 2
    error_message = "Alex and the release identity must receive VM-scoped OS Login and IAP access."
  }

  assert {
    condition     = google_artifact_registry_repository_iam_member.game_server_runtime_reader.member == "serviceAccount:relay-server-runtime@relayoperations.iam.gserviceaccount.com"
    error_message = "The runtime identity must have repository-scoped Artifact Registry reader access."
  }

  assert {
    condition     = google_billing_budget.playtest.budget_filter[0].calendar_period == "MONTH" && google_billing_budget.playtest.budget_filter[0].projects == toset(["projects/926404861741"])
    error_message = "The monthly budget must be scoped only to the playtest project."
  }

  assert {
    condition     = google_billing_budget.playtest.ownership_scope == "ALL_USERS"
    error_message = "The single-project budget must permit access through project-scoped billing IAM."
  }

  assert {
    condition     = google_billing_budget.playtest.amount[0].specified_amount[0].currency_code == "USD" && google_billing_budget.playtest.amount[0].specified_amount[0].units == "5"
    error_message = "The playtest budget must be five US dollars."
  }

  assert {
    condition     = toset([for rule in google_billing_budget.playtest.threshold_rules : rule.threshold_percent]) == toset([0.5, 0.9, 1])
    error_message = "The budget must alert at 50%, 90%, and 100% of current spend."
  }

  assert {
    condition     = google_monitoring_notification_channel.billing_email.type == "email" && google_monitoring_notification_channel.billing_email.labels["email_address"] == "akennedy4155@gmail.com"
    error_message = "Budget alerts must use the playtest operator's email notification channel."
  }

  assert {
    condition     = length([for policy in google_artifact_registry_repository.server_images.cleanup_policies : policy if policy.id == "keep-two-most-recent" && policy.action == "KEEP" && policy.most_recent_versions[0].keep_count == 2]) == 1
    error_message = "The repository must explicitly keep the two newest image versions."
  }

  assert {
    condition     = length([for policy in google_artifact_registry_repository.server_images.cleanup_policies : policy if policy.id == "delete-older-than-thirty-days" && policy.action == "DELETE" && policy.condition[0].older_than == "2592000s"]) == 1
    error_message = "Non-current repository versions older than 30 days must be eligible for deletion."
  }
}

run "accepts_a_capacity_zone_override" {
  command = plan

  variables {
    project_id         = "relayoperations"
    billing_account_id = "test-billing-account"
    zone               = "us-west1-b"
  }

  assert {
    condition     = output.vm_zone == "us-west1-b"
    error_message = "An explicit zone override within the configured region must be preserved."
  }
}

run "falls_back_to_the_project_billing_account" {
  command = plan

  variables {
    project_id = "relayoperations"
  }

  assert {
    condition     = google_billing_budget.playtest.billing_account == "fallback-billing-account"
    error_message = "A plan without an injected billing account must resolve the account attached to the project."
  }
}

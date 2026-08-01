mock_provider "google" {}

run "plans_the_default_public_host" {
  command = plan

  variables {
    project_id = "relayoperations"
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
}

run "accepts_a_capacity_zone_override" {
  command = plan

  variables {
    project_id = "relayoperations"
    zone       = "us-west1-b"
  }

  assert {
    condition     = output.vm_zone == "us-west1-b"
    error_message = "An explicit zone override within the configured region must be preserved."
  }
}

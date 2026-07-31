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

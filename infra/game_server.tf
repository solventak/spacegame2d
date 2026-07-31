locals {
  game_server_network_tag = "relay-operations-server"
}

resource "google_project_service" "compute_engine" {
  project            = var.project_id
  service            = "compute.googleapis.com"
  disable_on_destroy = false
}

resource "google_compute_network" "game_server" {
  name                    = "relay-operations-playtest"
  auto_create_subnetworks = false
  routing_mode            = "REGIONAL"

  depends_on = [google_project_service.compute_engine]
}

resource "google_compute_subnetwork" "game_server" {
  name          = "relay-operations-playtest"
  ip_cidr_range = var.game_server_subnet_cidr
  network       = google_compute_network.game_server.id
  region        = var.region
}

resource "google_compute_address" "game_server" {
  name         = "relay-operations-server"
  address_type = "EXTERNAL"
  network_tier = "STANDARD"
  region       = var.region

  depends_on = [google_project_service.compute_engine]
}

resource "google_compute_firewall" "public_game_port" {
  name        = "relay-operations-public-game-port"
  description = "Allows friend-playtest clients to reach the game server."
  network     = google_compute_network.game_server.name

  direction     = "INGRESS"
  source_ranges = ["0.0.0.0/0"]
  target_tags   = [local.game_server_network_tag]

  allow {
    protocol = "tcp"
    ports    = [tostring(var.game_port)]
  }
}

resource "google_compute_instance" "game_server" {
  name         = var.game_server_name
  machine_type = "e2-micro"
  zone         = var.zone

  tags                = [local.game_server_network_tag]
  deletion_protection = false

  boot_disk {
    initialize_params {
      image = "ubuntu-os-cloud/ubuntu-2404-lts-amd64"
      size  = 10
      type  = "pd-standard"
    }
  }

  network_interface {
    subnetwork = google_compute_subnetwork.game_server.id

    access_config {
      nat_ip       = google_compute_address.game_server.address
      network_tier = "STANDARD"
    }
  }

  depends_on = [google_project_service.compute_engine]
}

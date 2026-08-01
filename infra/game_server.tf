locals {
  game_server_network_tag = "relay-operations-server"
  iap_ssh_source_range    = "35.235.240.0/20"
  release_identity        = "serviceAccount:gha-server-release@${var.project_id}.iam.gserviceaccount.com"
  runtime_identity        = "${var.runtime_service_account_id}@${var.project_id}.iam.gserviceaccount.com"
  image_prefix            = "${var.region}-docker.pkg.dev/${var.project_id}/${var.server_image_repository_id}/${var.server_image_name}"
  vm_access_members       = toset([var.operator_identity, local.release_identity])
}

resource "google_project_service" "compute_engine" {
  project            = var.project_id
  service            = "compute.googleapis.com"
  disable_on_destroy = false
}

resource "google_project_service" "iap" {
  project            = var.project_id
  service            = "iap.googleapis.com"
  disable_on_destroy = false
}

resource "google_project_service" "os_login" {
  project            = var.project_id
  service            = "oslogin.googleapis.com"
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

resource "google_compute_firewall" "iap_ssh" {
  name        = "relay-operations-iap-ssh"
  description = "Allows SSH management only through Identity-Aware Proxy."
  network     = google_compute_network.game_server.name

  direction     = "INGRESS"
  source_ranges = [local.iap_ssh_source_range]
  target_tags   = [local.game_server_network_tag]

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }
}

resource "google_compute_instance" "game_server" {
  name         = var.game_server_name
  machine_type = "e2-micro"
  zone         = var.zone

  tags                = [local.game_server_network_tag]
  deletion_protection = false
  metadata = {
    enable-oslogin = "TRUE"
  }
  metadata_startup_script = templatefile("${path.module}/templates/game-server-startup.sh.tftpl", {
    deploy_script_b64 = filebase64("${path.module}/runtime/relay-operations-deploy")
    game_port         = var.game_port
    health_script_b64 = filebase64("${path.module}/runtime/relay-operations-health")
    image_prefix      = local.image_prefix
    run_script_b64    = filebase64("${path.module}/runtime/relay-operations-run")
    service_unit_b64  = filebase64("${path.module}/runtime/relay-operations-server.service")
    tmpfiles_b64      = filebase64("${path.module}/runtime/relay-operations-tmpfiles.conf")
  })

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

  service_account {
    email  = local.runtime_identity
    scopes = ["https://www.googleapis.com/auth/cloud-platform"]
  }

  depends_on = [
    google_project_service.compute_engine,
    google_project_service.iap,
    google_project_service.os_login,
  ]
}

resource "google_compute_instance_iam_member" "os_admin_login" {
  for_each = local.vm_access_members

  project       = var.project_id
  zone          = google_compute_instance.game_server.zone
  instance_name = google_compute_instance.game_server.name
  role          = "roles/compute.osAdminLogin"
  member        = each.value
  depends_on    = [google_project_service.os_login]
}

resource "google_iap_tunnel_instance_iam_member" "tunnel_access" {
  for_each = local.vm_access_members

  project    = var.project_id
  zone       = google_compute_instance.game_server.zone
  instance   = google_compute_instance.game_server.name
  role       = "roles/iap.tunnelResourceAccessor"
  member     = each.value
  depends_on = [google_project_service.iap]
}

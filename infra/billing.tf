data "google_project" "playtest" {
  project_id = var.project_id
}

resource "google_project_service" "billing_budgets" {
  project            = var.project_id
  service            = "billingbudgets.googleapis.com"
  disable_on_destroy = false
}

resource "google_project_service" "monitoring" {
  project            = var.project_id
  service            = "monitoring.googleapis.com"
  disable_on_destroy = false
}

resource "google_monitoring_notification_channel" "billing_email" {
  display_name = "Relay Operations playtest budget alerts"
  type         = "email"

  labels = {
    email_address = "akennedy4155@gmail.com"
  }

  depends_on = [google_project_service.monitoring]
}

resource "google_billing_budget" "playtest" {
  billing_account = var.billing_account_id
  display_name    = "Relay Operations playtest monthly budget"
  ownership_scope = "ALL_USERS"

  budget_filter {
    calendar_period = "MONTH"
    projects        = ["projects/${data.google_project.playtest.number}"]
  }

  amount {
    specified_amount {
      currency_code = "USD"
      units         = "5"
    }
  }

  all_updates_rule {
    monitoring_notification_channels = [google_monitoring_notification_channel.billing_email.name]
  }

  threshold_rules {
    threshold_percent = 0.5
  }

  threshold_rules {
    threshold_percent = 0.9
  }

  threshold_rules {
    threshold_percent = 1
  }

  depends_on = [google_project_service.billing_budgets]
}

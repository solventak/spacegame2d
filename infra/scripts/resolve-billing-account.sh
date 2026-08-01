#!/usr/bin/env bash
set -euo pipefail

query="$(cat)"
project_id="$(jq -er '.project_id' <<<"${query}")"
billing_account="$(gcloud billing projects describe "${project_id}" --format='value(billingAccountName)')"

if [[ "${billing_account}" != billingAccounts/* ]]; then
  echo "project ${project_id} has no linked billing account" >&2
  exit 1
fi

jq -cn --arg billing_account_id "${billing_account#billingAccounts/}" \
  '{billing_account_id: $billing_account_id}'

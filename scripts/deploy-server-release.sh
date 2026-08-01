#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: deploy-server-release.sh <image-ref> <image-path> <vm-name> <vm-zone> <endpoint>" >&2
  exit 64
}

[[ "$#" -eq 5 ]] || usage

candidate="$1"
image_path="$2"
vm_name="$3"
vm_zone="$4"
endpoint="$5"

project_id="${GCP_PROJECT_ID:?GCP_PROJECT_ID is required}"
gcloud_bin="${GCLOUD_BIN:-gcloud}"
timeout_bin="${TIMEOUT_BIN:-timeout}"
sleep_bin="${SLEEP_BIN:-sleep}"
tcp_check_bin="${TCP_CHECK_BIN:-}"

write_output() {
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf '%s=%s\n' "$1" "$2" >> "$GITHUB_OUTPUT"
  fi
}

if [[ "$candidate" != "$image_path@sha256:"* ]] || ! [[ "$candidate" =~ @sha256:[0-9a-f]{64}$ ]]; then
  echo "image must be an immutable digest from the configured Artifact Registry repository" >&2
  exit 64
fi

if ! [[ "$vm_name" =~ ^[a-z]([-a-z0-9]*[a-z0-9])?$ ]]; then
  echo "vm name must be a valid Compute Engine instance name" >&2
  exit 64
fi

if ! [[ "$vm_zone" =~ ^[a-z0-9-]+$ ]]; then
  echo "vm zone must be a valid Compute Engine zone" >&2
  exit 64
fi

if ! [[ "$endpoint" =~ ^([0-9]{1,3}(\.[0-9]{1,3}){3}):([0-9]{1,5})$ ]]; then
  echo "endpoint must be an IPv4 address and TCP port" >&2
  exit 64
fi

endpoint_host="${BASH_REMATCH[1]}"
endpoint_port="${BASH_REMATCH[3]}"
IFS=. read -r octet_1 octet_2 octet_3 octet_4 <<< "$endpoint_host"
for octet in "$octet_1" "$octet_2" "$octet_3" "$octet_4"; do
  (( octet <= 255 )) || { echo "endpoint contains an invalid IPv4 address" >&2; exit 64; }
done
(( endpoint_port <= 65535 )) || { echo "endpoint contains an invalid TCP port" >&2; exit 64; }

ssh_args=(
  compute ssh "$vm_name"
  --project="$project_id"
  --zone="$vm_zone"
  --tunnel-through-iap
  --quiet
)

remote() {
  "$gcloud_bin" "${ssh_args[@]}" --command="$1"
}

current_image_command="if sudo test -s /var/lib/relay-operations/current-image; then sudo cat /var/lib/relay-operations/current-image; fi"
previous_image="$(remote "$current_image_command")"
if [[ -n "$previous_image" ]] && {
  [[ "$previous_image" != "$image_path@sha256:"* ]] || ! [[ "$previous_image" =~ @sha256:[0-9a-f]{64}$ ]];
}; then
  echo "managed runtime reported an invalid current image" >&2
  exit 1
fi

check_local() {
  remote "sudo /usr/local/sbin/relay-operations-health"
}

check_external() {
  local attempt
  for attempt in {1..30}; do
    if [[ -n "$tcp_check_bin" ]]; then
      if "$tcp_check_bin" "$endpoint_host" "$endpoint_port"; then
        return 0
      fi
    elif "$timeout_bin" 2 bash -c ":</dev/tcp/${endpoint_host}/${endpoint_port}" >/dev/null 2>&1; then
      return 0
    fi
    "$sleep_bin" 2
  done
  return 1
}

rollback() {
  if [[ -n "$previous_image" ]]; then
    echo "restoring previous image=$previous_image"
    remote "sudo /usr/local/sbin/relay-operations-deploy '$previous_image'"
    check_local
    check_external
    echo "rollback verified image=$previous_image"
  else
    echo "no previous image; leaving first deployment stopped"
    remote "sudo systemctl stop relay-operations-server.service || true; sudo rm -f /var/lib/relay-operations/current-image; sudo docker rm --force relay-operations-server >/dev/null 2>&1 || true"
  fi
}

echo "deploying image=$candidate vm=$vm_name zone=$vm_zone endpoint=$endpoint"
if ! remote "sudo /usr/local/sbin/relay-operations-deploy '$candidate'"; then
  write_output rollback_status handled_by_vm_helper
  echo "VM deployment failed; the VM helper handled local rollback" >&2
  exit 1
fi

verification_failed=false
if ! check_local; then
  verification_failed=true
elif ! check_external; then
  verification_failed=true
fi

if [[ "$verification_failed" == true ]]; then
  echo "post-deployment verification failed" >&2
  if ! rollback; then
    write_output rollback_status failed
    echo "automatic rollback failed" >&2
  else
    write_output rollback_status succeeded
  fi
  exit 1
fi

write_output rollback_status not_needed
echo "deployment verified image=$candidate endpoint=$endpoint"

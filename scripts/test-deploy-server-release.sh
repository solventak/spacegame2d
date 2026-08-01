#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
deploy_script="$script_dir/deploy-server-release.sh"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

fake_gcloud="$test_root/gcloud"
fake_tcp="$test_root/tcp-check"
log_file="$test_root/commands.log"
scenario_file="$test_root/scenario"

cat > "$fake_gcloud" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

command="${*: -1}"
printf '%s\n' "$command" >> "$FAKE_GCLOUD_LOG"
scenario="$(<"$FAKE_SCENARIO")"

if [[ "$command" == *"current-image"* ]]; then
  if [[ "$scenario" == "first-external-failure" ]]; then
    exit 0
  fi
  printf '%s\n' "$FAKE_IMAGE_PATH@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  exit 0
fi

if [[ "$command" == *"relay-operations-deploy"* ]]; then
  if [[ "$scenario" == "helper-failure" && "$command" != *"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"* ]]; then
    exit 1
  fi
  exit 0
fi

if [[ "$command" == *"relay-operations-health"* ]]; then
  exit 0
fi

exit 0
EOF

cat > "$fake_tcp" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

count_file="$FAKE_TCP_COUNT"
count=0
if [[ -f "$count_file" ]]; then
  count="$(<"$count_file")"
fi
count=$((count + 1))
printf '%s\n' "$count" > "$count_file"

if [[ "$(<"$FAKE_SCENARIO")" == "external-failure" && "$count" -le 30 ]]; then
  exit 1
fi
if [[ "$(<"$FAKE_SCENARIO")" == "first-external-failure" ]]; then
  exit 1
fi
EOF

chmod +x "$fake_gcloud" "$fake_tcp"

if "$deploy_script" invalid tag relay-operations-server us-west1-a 203.0.113.10:4000 >/dev/null 2>&1; then
  echo "mutable image was accepted" >&2
  exit 1
fi

valid_image="us-west1-docker.pkg.dev/relayoperations/spacegame2d-server/spacegame2d-server@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

run_valid_case() {
  local scenario="$1"
  local expected_status="$2"
  : > "$log_file"
  printf '%s\n' "$scenario" > "$scenario_file"
  : > "$test_root/tcp.count"
  set +e
  GCP_PROJECT_ID=relayoperations \
  GCLOUD_BIN="$fake_gcloud" \
  TCP_CHECK_BIN="$fake_tcp" \
  SLEEP_BIN=true \
  FAKE_GCLOUD_LOG="$log_file" \
  FAKE_SCENARIO="$scenario_file" \
  FAKE_IMAGE_PATH="us-west1-docker.pkg.dev/relayoperations/spacegame2d-server/spacegame2d-server" \
  FAKE_TCP_COUNT="$test_root/tcp.count" \
    "$deploy_script" "$valid_image" \
      "us-west1-docker.pkg.dev/relayoperations/spacegame2d-server/spacegame2d-server" \
      relay-operations-server us-west1-a 203.0.113.10:4000 \
      >/dev/null 2>&1
  local status=$?
  set -e
  test "$status" -eq "$expected_status"
}

run_valid_case success 0
run_valid_case external-failure 1
grep -q 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' "$log_file"
run_valid_case first-external-failure 1
grep -q 'current-image' "$log_file"

echo "deployment workflow tests passed"

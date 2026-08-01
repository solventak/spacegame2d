#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
runtime_dir="$repo_root/infra/runtime"
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT

mock_bin="$test_root/mock-bin"
runtime_root="$test_root/root"
mkdir --parents "$mock_bin" "$runtime_root/etc/relay-operations" \
  "$runtime_root/var/lib/relay-operations" "$runtime_root/var/log/relay-operations/failed-releases" \
  "$runtime_root/run/lock"

cat > "$mock_bin/docker" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  login|logout) exit 0 ;;
  pull)
    printf '%s\n' "$2" > "${DOCKER_PULL_LOG:?}"
    exit "${DOCKER_PULL_STATUS:-0}"
    ;;
  *) exit 0 ;;
esac
MOCK

cat > "$mock_bin/curl" <<'MOCK'
#!/usr/bin/env bash
printf '{"access_token":"test-token"}\n'
MOCK

cat > "$mock_bin/systemctl" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  is-active) test "${SYSTEMCTL_ACTIVE:-1}" = 1 ;;
  restart)
    printf 'restart\n' >> "${SYSTEMCTL_LOG:?}"
    test "${SYSTEMCTL_RESTART_STATUS:-0}" = 0
    ;;
  stop) printf 'stop\n' >> "${SYSTEMCTL_LOG:?}" ;;
  *) exit 0 ;;
esac
MOCK

cat > "$mock_bin/timeout" <<'MOCK'
#!/usr/bin/env bash
test "${TCP_HEALTH:-1}" = 1
MOCK

cat > "$mock_bin/journalctl" <<'MOCK'
#!/usr/bin/env bash
printf 'mock failed-release journal\n'
MOCK

cat > "$mock_bin/logger" <<'MOCK'
#!/usr/bin/env bash
exit 0
MOCK

cat > "$mock_bin/install" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
args=()
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --owner=*|--group=*) shift ;;
    *) args+=("$1"); shift ;;
  esac
done
exec /usr/bin/install "${args[@]}"
MOCK

chmod 0755 "$mock_bin"/*

image_prefix="us-west1-docker.pkg.dev/relayoperations/spacegame2d-server/spacegame2d-server"
cat > "$runtime_root/etc/relay-operations/server.env" <<EOF
GAME_PORT=4000
SERVER_LISTEN_ADDRESS=0.0.0.0:4000
SPACEGAME_LOG_FORMAT=json
ALLOWED_IMAGE_PREFIX=$image_prefix
HEALTH_TIMEOUT_SECONDS=1
EOF

export RELAY_OPERATIONS_ROOT="$runtime_root"
export DOCKER_BIN="$mock_bin/docker"
export CURL_BIN="$mock_bin/curl"
export SYSTEMCTL_BIN="$mock_bin/systemctl"
export TIMEOUT_BIN="$mock_bin/timeout"
export HEALTH_BIN="$runtime_dir/relay-operations-health"
export JOURNALCTL_BIN="$mock_bin/journalctl"
export LOGGER_BIN="$mock_bin/logger"
export INSTALL_BIN="$mock_bin/install"
export DOCKER_PULL_LOG="$test_root/pull.log"
export SYSTEMCTL_LOG="$test_root/systemctl.log"
export PATH="$mock_bin:$PATH"

assert_file_contains() {
  local file="$1"
  local expected="$2"
  grep -F -- "$expected" "$file" >/dev/null
}

run_deploy() {
  "$runtime_dir/relay-operations-deploy" "$1"
}

first_image="$image_prefix@sha256:1111111111111111111111111111111111111111111111111111111111111111"
second_image="$image_prefix@sha256:2222222222222222222222222222222222222222222222222222222222222222"

export SYSTEMCTL_ACTIVE=1
export SYSTEMCTL_RESTART_STATUS=0
export TCP_HEALTH=1
run_deploy "$first_image"
assert_file_contains "$runtime_root/var/lib/relay-operations/current-image" "$first_image"
test ! -e "$runtime_root/var/lib/relay-operations/previous-image"

run_deploy "$second_image"
assert_file_contains "$runtime_root/var/lib/relay-operations/current-image" "$second_image"
assert_file_contains "$runtime_root/var/lib/relay-operations/previous-image" "$first_image"

if run_deploy "${image_prefix}:latest" 2>/dev/null; then
  echo "mutable tag was accepted" >&2
  exit 1
fi

rm -f "$runtime_root/var/lib/relay-operations/current-image" "$runtime_root/var/lib/relay-operations/previous-image"
export TCP_HEALTH=0
if run_deploy "$first_image"; then
  echo "first failed deployment returned success" >&2
  exit 1
fi
test ! -e "$runtime_root/var/lib/relay-operations/current-image"
test -n "$(find "$runtime_root/var/log/relay-operations/failed-releases" -type f -print -quit)"

printf '%s\n' "$first_image" > "$runtime_root/var/lib/relay-operations/current-image"
export TCP_HEALTH=0
if run_deploy "$second_image"; then
  echo "rollback failure returned success" >&2
  exit 1
fi
assert_file_contains "$runtime_root/var/lib/relay-operations/current-image" "$first_image"

echo "game-server runtime tests passed"

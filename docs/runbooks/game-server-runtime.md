# Managed game-server runtime

SWA-65 and SWA-69 provision the VM runtime and access boundary. SWA-66 owns the manually dispatched
GitHub Actions workflow that verifies, publishes, and deploys the immutable server image.

## Apply order

1. Apply `infra/bootstrap/identity/` with Alex's ADC. This creates the dedicated
   `relay-server-runtime` identity, grants the narrowly scoped Terraform/IAP permissions, and
   grants service-account use to Alex, Terraform apply, and the release identity.
2. Apply `infra/` through the protected production Terraform workflow.
3. Confirm the VM startup contract is ready. The game service remains inactive until SWA-66
   supplies its first full image digest.

The startup script may replace the VM when the runtime contract changes. The regional static
address is separate and must remain unchanged.

## Access

Alex connects through IAP and OS Login:

```bash
gcloud compute ssh relay-operations-server \
  --project=relayoperations \
  --zone=us-west1-a \
  --tunnel-through-iap \
  --command='sudo systemctl status relay-operations-server --no-pager'
```

Inspect service logs with:

```bash
gcloud compute ssh relay-operations-server \
  --project=relayoperations \
  --zone=us-west1-a \
  --tunnel-through-iap \
  --command='sudo journalctl -u relay-operations-server -n 100 --no-pager'
```

TCP/22 is reachable only from Google's IAP forwarding range `35.235.240.0/20`. Do not add a
public SSH rule for diagnosis or recovery.

## Deployment contract

SWA-66 invokes this VM-side command over IAP/OS Login:

```bash
sudo /usr/local/sbin/relay-operations-deploy \
  us-west1-docker.pkg.dev/relayoperations/spacegame2d-server/spacegame2d-server@sha256:<64-lowercase-hex-digest>
```

The helper rejects mutable tags, obtains short-lived Artifact Registry credentials from the VM
runtime identity, pulls the digest, and restarts the systemd service. The workflow follows that
with a second local health check and an external TCP 4000 check. Runtime configuration lives in
`/etc/relay-operations/server.env`; the container has no persistent game-data volume and logs
through journald.

Deployment health requires systemd active status and local TCP 4000 acceptance within 60 seconds.
The service intentionally restarts during planned releases, interrupting active sessions.

## Failure handling

The current digest is recorded as the preceding digest before each replacement. A failed first
deployment leaves the service stopped and stores diagnostics in journald plus
`/var/log/relay-operations/failed-releases/`. That archive is bounded to seven days.

A later failed deployment restores the preceding digest, reruns local and external health
verification, and returns a failure result even when rollback succeeds. A failed first deployment
stops the service and removes the current-image marker. There is no manual rollback command in
this scope.

Recovery uses Alex's personal IAP/OS Login access and, if necessary, a Terraform revert. It never
requires public SSH.

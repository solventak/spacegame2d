## Release

Run **Release Server** manually from `main`. The workflow first runs the repository Rust test
gate, the 85% coverage gate, and builds a local container image with the full commit SHA embedded
as `SPACEGAME_GIT_SHA`. It smoke-tests the exact image for its TCP listener, non-root runtime
user, absence of Rust and Cargo, and JSON startup metadata before the protected `production` job
is approved.

The approved job pushes that tested image to:

```text
us-west1-docker.pkg.dev/<project-id>/spacegame2d-server/spacegame2d-server:<full-git-sha>
```

After approval, the workflow pushes the tested image, resolves its immutable digest, and invokes
the VM deployment helper through IAP and OS Login. It then checks both the VM-local service health
and external TCP reachability on port 4000. Releases and later deployments must use that digest,
never `latest` or another mutable tag.

The workflow has no ref or version inputs. It deploys only the `main` SHA associated with the
manual dispatch. A second run queues behind an active release; it never cancels an in-progress
deployment. Every release intentionally interrupts active sessions.

## Build cache

The workflow uses the GitHub Actions BuildKit cache in the `spacegame2d-server` scope with
`mode=max`. The Dockerfile separately caches Cargo registry, Git, and target artifacts, so a
repeat build reuses unchanged dependencies while recompiling the release binary with its new Git
SHA. Credentials are never part of a Docker build layer or cache entry.

## Rollback

This workflow never deletes images. The deployed image and the immediately previous image remain
available as digest-addressed rollback inputs. The VM helper handles failures during local
deployment. If post-deployment local or external verification fails, the workflow automatically
restores the previously active digest and rechecks it, but still reports the release as failed.
If there is no previous digest, the first deployment is stopped and the service is left inactive.

The deployment boundary is the VM-side helper described in the [managed game-server runtime
runbook](game-server-runtime.md). The workflow records the Git SHA, immutable image digest,
simulation version, public endpoint, deployment result, and rollback result in its summary.

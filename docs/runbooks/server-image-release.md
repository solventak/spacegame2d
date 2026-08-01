## Publishing

Run **Release Server** manually from `main`. The workflow first runs the repository Rust test
gate and builds a local container image with the full commit SHA embedded as `SPACEGAME_GIT_SHA`.
It smoke-tests the exact image for its TCP listener, non-root runtime user, absence of Rust and
Cargo, and JSON startup metadata before the protected `production` job is approved.

The approved job pushes that tested image to:

```text
us-west1-docker.pkg.dev/<project-id>/spacegame2d-server/spacegame2d-server:<full-git-sha>
```

The workflow summary records the immutable image digest. Releases and later deployments must use
that digest, never `latest` or another mutable tag.

## Build cache

The workflow uses the GitHub Actions BuildKit cache in the `spacegame2d-server` scope with
`mode=max`. The Dockerfile separately caches Cargo registry, Git, and target artifacts, so a
repeat build reuses unchanged dependencies while recompiling the release binary with its new Git
SHA. Credentials are never part of a Docker build layer or cache entry.

## Rollback inputs

This workflow never deletes images. The deployed image and the immediately previous image remain
available as digest-addressed rollback inputs.

SWA-66 passes the selected digest to the VM-side deployment helper described in the [managed
game-server runtime runbook](game-server-runtime.md). Image publication and VM deployment remain
separate operations.

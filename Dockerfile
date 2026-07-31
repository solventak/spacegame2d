# syntax=docker/dockerfile:1.7

FROM rust:1.91-bookworm AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY crates/logging/Cargo.toml crates/logging/Cargo.toml
COPY crates/protocol/Cargo.toml crates/protocol/Cargo.toml
COPY crates/server/Cargo.toml crates/server/Cargo.toml
COPY crates/simulation/Cargo.toml crates/simulation/Cargo.toml
COPY crates/spacegame2d/Cargo.toml crates/spacegame2d/Cargo.toml
COPY crates/ui-protocol/Cargo.toml crates/ui-protocol/Cargo.toml

RUN mkdir -p \
      crates/logging/src \
      crates/protocol/src \
      crates/server/src \
      crates/simulation/src \
      crates/spacegame2d/src \
      crates/ui-protocol/src && \
    touch \
      crates/logging/src/lib.rs \
      crates/protocol/src/lib.rs \
      crates/server/src/main.rs \
      crates/simulation/src/lib.rs \
      crates/spacegame2d/src/main.rs \
      crates/ui-protocol/src/lib.rs

RUN --mount=type=cache,id=spacegame2d-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=spacegame2d-cargo-git,target=/usr/local/cargo/git \
    cargo fetch --locked

COPY crates crates

ARG GIT_SHA=unknown

RUN --mount=type=cache,id=spacegame2d-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=spacegame2d-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=spacegame2d-target-rust-1.91,target=/workspace/target \
    SPACEGAME_GIT_SHA="$GIT_SHA" cargo build --locked --release --package spacegame2d-server && \
    install --directory /out && \
    install --mode=0755 target/release/spacegame2d-server /out/spacegame2d-server

FROM debian:bookworm-slim AS runtime

RUN groupadd --system --gid 10001 spacegame && \
    useradd --system --uid 10001 --gid spacegame --create-home --home-dir /var/lib/spacegame \
        --shell /usr/sbin/nologin spacegame && \
    install --directory --owner=spacegame --group=spacegame --mode=0755 /var/log/spacegame

COPY --from=builder --chown=spacegame:spacegame /out/spacegame2d-server /usr/local/bin/spacegame2d-server

USER spacegame

ENV SPACEGAME_LOG_DIR=/var/log/spacegame

EXPOSE 4000/tcp

ENTRYPOINT ["/usr/local/bin/spacegame2d-server", "0.0.0.0:4000"]

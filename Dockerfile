# CI/dev environment for windmeter: the host Rust toolchain, the
# STM32H723ZG embedded target this project actually builds for, the
# sysml v2 CLI (validates requirements/model/*.sysml), and cargo-llvm-cov
# for coverage.
#
# Deliberately NOT included: cargo-mutants. Mutation testing is run
# locally/on demand (see PROJECT_PRINCIPLES.md § Test Quality), not in
# this per-commit CI image -- a full-crate run takes tens of minutes,
# which is too expensive to pay on every push.

FROM rust:1-slim-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        tar \
    && rm -rf /var/lib/apt/lists/*

# sysml / sysml-lsp (Open-MBEE/OpenSysML) -- validates
# requirements/model/*.sysml. Pinned for reproducible builds; bump
# deliberately, not automatically.
ARG OPENSYSML_VERSION=v0.8.1
RUN curl -fsSL "https://github.com/Open-MBEE/OpenSysML/releases/download/${OPENSYSML_VERSION}/opensysml-linux-amd64.tar.gz" \
        -o /tmp/opensysml.tar.gz \
    && tar -xzf /tmp/opensysml.tar.gz -C /usr/local/bin sysml sysml-lsp \
    && rm /tmp/opensysml.tar.gz \
    && chmod +x /usr/local/bin/sysml /usr/local/bin/sysml-lsp

# Only the target this project's firmware actually builds for
# (STM32H723ZG). rust-toolchain.toml lists more targets, inherited from
# the embassy template this project was generated from, for chips this
# project doesn't use -- CI has no reason to install those.
RUN rustup target add thumbv7em-none-eabihf \
    && rustup component add rustfmt llvm-tools

RUN cargo install cargo-llvm-cov --locked

# CARGO_HOME's registry dir is bind-mounted over in CI to cache
# downloaded crates across runs; keep it writable by any UID docker run
# is invoked with (matches how the base rust image already sets this up
# for /usr/local/cargo, kept explicit here since CI overrides the mount).
RUN mkdir -p /usr/local/cargo/registry && chmod -R a+rwX /usr/local/cargo

WORKDIR /workspace

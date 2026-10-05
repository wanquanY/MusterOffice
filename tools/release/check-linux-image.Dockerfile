# syntax=docker/dockerfile:1
# Check the actual archives in Linux, independent of any receiving application.
FROM python:3.13.7-bookworm@sha256:c900d35aba5fe4c1dc1cd358408baae2902ff2a2926a1d15cc5002c6061ddb2e AS python
FROM rust:1.99.0-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0 AS check
COPY --from=python /usr/local/ /usr/local/
WORKDIR /src
ENV RUSTUP_TOOLCHAIN=1.92.0 CARGO_TARGET_DIR=/target
# `cargo metadata` inspects the portable SDK graph, including target-specific
# crates not compiled by the Linux worker. Populate that exact locked graph
# before the independent consumer's deliberately offline acceptance.
COPY Cargo.toml Cargo.lock rust-toolchain.toml /producer/
COPY crates /producer/crates
COPY tools /producer/tools
RUN --mount=type=cache,id=musteroffice-linux-x64-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    cargo fetch --locked --manifest-path /producer/Cargo.toml
COPY Cargo.lock /src/Cargo.lock
COPY tools/sdk/example /src/tools/sdk/example
COPY tools/release/check.py /src/tools/release/check.py
COPY --from=release . /component/
COPY --from=export-input . /input/
ARG MUSTEROFFICE_RELEASE_SHA256
RUN --mount=type=cache,id=musteroffice-linux-x64-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=musteroffice-linux-x64-release-check,target=/target,sharing=locked \
    if ! python3 tools/release/check.py --release /component --sha256 "$MUSTEROFFICE_RELEASE_SHA256" \
      --target linux-x64 --input /input --output /result; then \
      test ! -f /result/cargo.log || tail -n 120 /result/cargo.log; exit 1; \
    fi
FROM scratch AS artifact
COPY --from=check /result/ /

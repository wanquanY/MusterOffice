# syntax=docker/dockerfile:1
# Check the actual archives in Linux, independent of any receiving application.
FROM python:3.13.7-bookworm@sha256:c900d35aba5fe4c1dc1cd358408baae2902ff2a2926a1d15cc5002c6061ddb2e AS python
FROM rust:1.92.0-bookworm@sha256:e90e846de4124376164ddfbaab4b0774c7bdeef5e738866295e5a90a34a307a2 AS check
COPY --from=python /usr/local/ /usr/local/
WORKDIR /src
COPY Cargo.lock /src/Cargo.lock
COPY tools/sdk/example /src/tools/sdk/example
COPY tools/release/check.py /src/tools/release/check.py
COPY --from=release . /component/
COPY --from=export-input . /input/
ARG MUSTEROFFICE_RELEASE_SHA256
ENV RUSTUP_TOOLCHAIN=1.92.0 CARGO_TARGET_DIR=/target
RUN --mount=type=cache,id=musteroffice-linux-x64-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=musteroffice-linux-x64-release-check,target=/target,sharing=locked \
    if ! python3 tools/release/check.py --release /component --sha256 "$MUSTEROFFICE_RELEASE_SHA256" \
      --target linux-x64 --input /input --output /result; then \
      test ! -f /result/cargo.log || tail -n 120 /result/cargo.log; exit 1; \
    fi
FROM scratch AS artifact
COPY --from=check /result/ /

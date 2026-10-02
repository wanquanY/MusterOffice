# syntax=docker/dockerfile:1
# Build the independent native component, never a receiving application's source.
FROM rust:1.92.0-bookworm@sha256:e90e846de4124376164ddfbaab4b0774c7bdeef5e738866295e5a90a34a307a2 AS toolchain
ENV RUSTUP_TOOLCHAIN=1.92.0
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang llvm cmake nasm python3 patch pkg-config ninja-build && \
    rm -rf /var/lib/apt/lists/*
RUN git clone https://github.com/ninja-build/ninja.git /opt/ninja && \
    git -C /opt/ninja checkout --detach 3441b633c2fe2c494e958780ba0f4227b1327634 && \
    cmake -S /opt/ninja -B /opt/ninja/out -DBUILD_TESTING=OFF -DCMAKE_BUILD_TYPE=Release && \
    cmake --build /opt/ninja/out -j 4 && \
    install -m 0755 /opt/ninja/out/ninja /usr/local/bin/ninja && \
    test "$(ninja --version)" = 1.13.2
RUN git clone https://gn.googlesource.com/gn /opt/gn && \
    git -C /opt/gn checkout --detach b2afae122eeb6ce09c52d63f67dc53fc517dbdc8 && \
    cd /opt/gn && python3 build/gen.py && ninja -C out -j 4 && \
    test "$(out/gn --version)" = '2175 (b2afae122eeb)'

FROM toolchain AS components
WORKDIR /src
COPY components /src/components
COPY tools/components /src/tools/components
COPY tools/verification/skia-probe.cpp tools/verification/harfbuzz-probe.cpp /src/tools/verification/
# Named context contains only archives whose lengths and hashes are in locks.
COPY --from=component-archives . /opt/component-archives/
RUN mkdir -p .codex-work/linux/skia .codex-work/linux/harfbuzz && \
    cp /opt/component-archives/skia-*.tar.gz .codex-work/linux/skia/ && \
    cp /opt/component-archives/harfbuzz-*.tar.xz .codex-work/linux/harfbuzz/ && \
    python3 tools/components/build-image-codecs.py --target native \
      --archives /opt/component-archives --directory .codex-work/linux/codecs --jobs 4 && \
    python3 tools/components/build-harfbuzz.py --target native \
      --directory .codex-work/linux/harfbuzz --archiver /usr/bin/llvm-ar && \
    python3 tools/components/build-skia.py --target native \
      --directory .codex-work/linux/skia --gn /opt/gn/out/gn \
      --ninja /usr/local/bin/ninja --archiver /usr/bin/llvm-ar \
      --image-codecs .codex-work/linux/codecs --jobs 4

FROM components AS worker
COPY Cargo.toml Cargo.lock rust-toolchain.toml /src/
COPY crates /src/crates
COPY tools /src/tools
COPY fixtures /src/fixtures
ENV MO_SKIA_LIB_DIR=/src/.codex-work/linux/skia \
    MO_HARFBUZZ_LIB_DIR=/src/.codex-work/linux/harfbuzz
RUN --mount=type=cache,id=musteroffice-linux-x64-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=musteroffice-linux-x64-cargo-target,target=/src/target,sharing=locked \
    cargo build --locked --release -p mo-export-worker -j 4 && \
    mkdir /release && cp target/release/mo-export-worker /release/ && \
    cargo test --locked --release -p mo-export-worker -p mo-native-io -p mo-native-export \
      -p mo-native-worker -p mo-native-compute -j 4 > /release/tests.log 2>&1 && \
    ldd /release/mo-export-worker > /release/dynamic-libraries.txt && \
    python3 tools/release/record-linux-worker.py /release

FROM scratch AS artifact
COPY --from=worker /release/ /

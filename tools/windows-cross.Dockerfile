# Rust 1.98.1, pinned to the official image used to validate this build.
FROM rust:1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e
RUN apt-get update && apt-get install -y --no-install-recommends clang llvm lld \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-pc-windows-msvc
ENV CARGO_TARGET_DIR=/build CARGO_BUILD_JOBS=4
WORKDIR /workspace

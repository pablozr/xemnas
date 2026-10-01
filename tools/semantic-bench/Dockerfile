FROM rust:1-bookworm@sha256:93ce27a88655056a51dbdd8f5f2d7ddc071c7b0070fb288a37b5a285fc83971e
RUN rustup component add clippy rustfmt
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler libprotobuf-dev cmake time \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_HOME=/cargo CARGO_TARGET_DIR=/build
WORKDIR /source

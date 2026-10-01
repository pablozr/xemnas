#!/bin/sh
set -eu
archive=/bench/lancedb-0.39.0.crate
if [ ! -f "$archive" ]; then
    cached=$(find /cargo/registry/cache -name lancedb-0.39.0.crate -print -quit)
    if [ -n "$cached" ]; then cp "$cached" "$archive";
    else curl --fail --location --retry 3 \
        https://static.crates.io/crates/lancedb/lancedb-0.39.0.crate --output "$archive"; fi
fi
printf '%s  %s\n' 2801e8ea61db659b07f45aa663e2a63ff6d7052485206c7c6c75ea01d08eea9a "$archive" | sha256sum -c -
mkdir -p /bench/lancedb-patched
tar -xzf "$archive" --strip-components=1 -C /bench/lancedb-patched
patch --batch --forward -p1 -d /bench/lancedb-patched < /source/lancedb-embedded.patch

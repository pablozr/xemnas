#!/bin/sh
set -eu
out="${1:?Provide a fresh output directory}"
bin=/build/release/semantic-bench
test ! -e "$out"
mkdir -p "$out"
export TOKENIZERS_PARALLELISM=false RAYON_NUM_THREADS=4
export OPENBLAS_NUM_THREADS=4 OMP_NUM_THREADS=4
export ORT_DYLIB_PATH=/bench/ort/onnxruntime-linux-x64-1.30.0/lib/libonnxruntime.so.1.30.0
if [ ! -f "$ORT_DYLIB_PATH" ]; then
    tar -xzf /bench/ort/onnxruntime-linux-x64-1.30.0.tgz -C /bench/ort
fi
unset HF_HOME
{ uname -a; rustc --version; cargo --version; lscpu; cat /sys/fs/cgroup/memory.max;
  cat /sys/fs/cgroup/memory.swap.max; cat /sys/fs/cgroup/cpu.max;
  df -hT /bench /scratch; } > "$out/environment.txt"
"$bin" fixture "$out/fixture.json"
for model in e5 gemma-q4; do
    echo "Preparing $model (network excluded from measured runs)"
    "$bin" prepare "$model" 4 /bench/models "$out/prepare-$model" > "$out/prepare-$model.log" 2>&1
done
# Alternate model order across repetitions to reduce order bias.
for repeat in 1 2 3; do
    if [ "$repeat" = 2 ]; then models="gemma-q4 e5"; else models="e5 gemma-q4"; fi
    for model in $models; do
        echo "Embedding $model, 4 threads, repetition $repeat"
        /usr/bin/time -v -o "$out/$model-t4-r$repeat.time" "$bin" embed "$model" 4 /bench/models \
            "$out/$model-t4-r$repeat" > "$out/$model-t4-r$repeat.log" 2>&1
    done
done
for model in e5 gemma-q4; do
    echo "Embedding $model, 1 thread"
    /usr/bin/time -v -o "$out/$model-t1.time" "$bin" embed "$model" 1 /bench/models \
        "$out/$model-t1" > "$out/$model-t1.log" 2>&1
done
for model in e5 gemma-q4; do
    vectors="$out/$model-t4-r1/vectors.json"
    for rows in 48 1000 10000 100000; do
        for repeat in 1 2 3; do
            if [ "$repeat" = 2 ]; then engines="lance sqlite"; else engines="sqlite lance"; fi
            for engine in $engines; do
                name="$engine-$model-$rows-r$repeat"
                echo "Storage $name"
                /usr/bin/time -v -o "$out/$name.time" "$bin" "$engine" "$vectors" "$rows" \
                    "/scratch/$name" > "$out/$name.log" 2>&1
                mkdir -p "$out/$name"
                cp "/scratch/$name/storage.json" "$out/$name/storage.json"
                if [ "$repeat" = 1 ] && [ "$rows" != 48 ]; then
                    if [ "$engine" = sqlite ]; then database="/scratch/$name/index.sqlite";
                    else database="/scratch/$name/lancedb"; fi
                    echo "End-to-end retrieval: $name"
                    /usr/bin/time -v -o "$out/$name-pipeline.time" "$bin" pipeline "$engine" \
                        "$model" 4 /bench/models "$database" "$out/$name" \
                        > "$out/$name-pipeline.log" 2>&1
                fi
                # Only this freshly-created benchmark DB; keep JSON and timings.
                case "$name" in sqlite-*|lance-*) rm -rf "/scratch/$name";; *) exit 1;; esac
            done
        done
    done
done
for model in e5 gemma-q4; do
    for language in por_Latn eng_Latn; do
        echo "Public retrieval quality: $model, $language"
        /usr/bin/time -v -o "$out/belebele-$model-$language.time" "$bin" evaluate "$model" 4 /bench/models \
            "/bench/datasets/belebele-$language.json" "$out/belebele-$model-$language" \
            > "$out/belebele-$model-$language.log" 2>&1
    done
done
find -L /bench/models -type f -not -path '*/.locks/*' -print0 | sort -z | xargs -0 sha256sum > "$out/model-artifacts.sha256"
du -ah /bench/models > "$out/model-artifacts-sizes.txt"
find -L /bench/models -path '*/snapshots/*' -type f -printf '%s %p\n' \
    > "$out/model-file-bytes.txt"
stat -c '%s %n' "$bin" "$ORT_DYLIB_PATH" > "$out/runtime-file-bytes.txt"
sha256sum /source/Cargo.lock /source/Cargo.toml /source/Dockerfile /source/*.ps1 \
    /source/*.sh /source/*.patch /source/src/*.rs /bench/lancedb-patched/src/job.rs \
    /bench/lancedb-0.39.0.crate \
    "$bin" "$ORT_DYLIB_PATH" > "$out/experiment.sha256"
echo "Completed: $out"

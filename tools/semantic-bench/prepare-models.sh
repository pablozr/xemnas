#!/bin/sh
set -eu
# Populate only the experiment cache, using immutable model revisions.
cache=/bench/models
for spec in \
    'intfloat/multilingual-e5-small:614241f622f53c4eeff9890bdc4f31cfecc418b3' \
    'onnx-community/embeddinggemma-300m-ONNX:5090578d9565bb06545b4552f76e6bc2c93e4a66'; do
    repo=${spec%:*}
    revision=${spec#*:}
    directory="$cache/models--$(printf '%s' "$repo" | sed 's|/|--|g')"
    snapshot="$directory/snapshots/$revision"
    files='config.json tokenizer.json tokenizer_config.json special_tokens_map.json'
    case "$repo" in
        intfloat/*) files="$files onnx/model.onnx";;
        onnx-community/*) files="$files onnx/model_q4.onnx onnx/model_q4.onnx_data";;
        *) exit 1;;
    esac
    for file in $files; do
        if [ ! -f "$snapshot/$file" ]; then
            mkdir -p "$(dirname "$snapshot/$file")"
            curl --fail --location --retry 3 \
                "https://huggingface.co/$repo/resolve/$revision/$file" \
                --output "$snapshot/$file.partial"
            mv "$snapshot/$file.partial" "$snapshot/$file"
        fi
    done
    mkdir -p "$directory/refs"
    printf '%s' "$revision" > "$directory/refs/main"
done

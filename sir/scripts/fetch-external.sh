#!/usr/bin/env bash
# Clone Embench and compile the four external probes with fixed flags so
# the external evidence is reproducible.
set -euo pipefail

DEST="${1:-/tmp/external}"
mkdir -p "$DEST"
if [ ! -d "$DEST/embench/.git" ]; then
    git clone --depth 1 https://github.com/embench/embench-iot.git "$DEST/embench"
fi

cd "$DEST/embench"
FLAGS=(-O1 -fno-inline-functions -fno-vectorize -fno-slp-vectorize
       -DGLOBAL_SCALE_FACTOR=1 -DLOCAL_SCALE_FACTOR=1)
for entry in "matmult-int:matmult-int.c" "crc32:crc_32.c" "edn:libedn.c" "statemate:libstatemate.c"; do
    name="${entry%%:*}"
    source="${entry##*:}"
    clang "${FLAGS[@]}" -I support -I "src/$name" -S -emit-llvm \
        "src/$name/$source" -o "$DEST/$name.ll"
    echo "EXTERNAL $name $DEST/$name.ll"
done

#!/usr/bin/env bash
# Clone Embench and compile the four external probes with fixed flags so
# the external evidence is reproducible.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# Default to the persisted workspace, never /tmp: a corpus that vanishes with
# the temp directory is a gate input that cannot be reproduced.
DEST="${1:-$ROOT/work/external}"
mkdir -p "$DEST"
if [ ! -d "$DEST/embench/.git" ]; then
    git clone --depth 1 https://github.com/embench/embench-iot.git "$DEST/embench"
fi

cd "$DEST/embench"
FLAGS=(-O1 -fno-inline-functions -fno-vectorize -fno-slp-vectorize
       -DGLOBAL_SCALE_FACTOR=1 -DLOCAL_SCALE_FACTOR=1)
for entry in \
    "matmult-int:matmult-int.c" "crc32:crc_32.c" "edn:libedn.c" "statemate:libstatemate.c" \
    "aha-mont64:mont64.c" "depthconv:depthconv.c" "huffbench:libhuffbench.c" "md5sum:md5.c" \
    "nettle-aes:nettle-aes.c" "nettle-sha256:nettle-sha256.c" "nsichneu:libnsichneu.c" \
    "picojpeg:libpicojpeg.c" "qrduino:qrencode.c" "sglib-combined:combined.c" \
    "slre:libslre.c" "tarfind:tarfind.c" "ud:libud.c" "wikisort:libwikisort.c" \
    "xgboost:testbench.c"; do
    name="${entry%%:*}"
    source="${entry##*:}"
    clang "${FLAGS[@]}" -I support -I "src/$name" -S -emit-llvm \
        "src/$name/$source" -o "$DEST/$name.ll"
    echo "EXTERNAL $name $DEST/$name.ll"
done
